// A data source that is a file in a format (docs/FORMATS-PLAN.md, "A file
// as a data source"): `datasource logins csv "imports/logins.csv" external`.
// The file is read through Clarity.Formats into one table of an in-memory
// SQLite database (one per entity when its mapping holds records inside
// records, each linked to the row it sits in by that row's key), so the generated code finds, saves and deletes as it
// does in any SQLite database. After each change that is kept (outside a
// transaction, or when it commits) the table is written back to a new file
// that then replaces the old one, so a crash never leaves half a file; the
// file's own column order and the columns no entity maps are kept.

package clarity

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

// TableReader reads a file in a format as a table, JSON
// {"columns":[...],"rows":[[...],...]}; fields are the columns an entity
// maps with their kinds. The program's call into Clarity.Formats.
type TableReader func(format string, input []byte, options, fields string) (string, error)

// TableWriter writes a table, as TableReader gives it, back into a file in
// a format, inside original where the format keeps more than the table.
type TableWriter func(format string, original []byte, table, options, fields string) ([]byte, error)

// FileSource is what the generator says of a file source: its format and
// options, the entity's table and key column, and the columns the entity
// maps with their kinds (JSON [{"name":"site","kind":"text"},...]).
type FileSource struct {
	Name     string
	Format   string
	Path     string
	Options  string
	Table    string
	Key      string
	Fields   string
	Readonly bool
}

type fileDriver struct {
	*sqliteDriver
	path    string
	format  string
	options string
	fields  string
	// tables are the tables the file is read into: one, or with records inside (a mapping's nested) one per entity.
	tables []fileTable
	// several is whether the file holds records inside its records, so its tables go back as several.
	several bool
	// original is the file as it was last read or written, for formats that keep more than the table.
	original []byte
	write    TableWriter
	readonly bool
	dirty    bool
	// inTransaction is whether a transaction is open, so a change waits for its commit.
	inTransaction bool
	// seen is the file's size and modification time when it was last read
	// or written: a change by another program refuses the next save.
	seen fileStamp
}

// fileStamp is a file's size and modification time; zero when it is not there.
type fileStamp struct {
	size     int64
	modified int64
	there    bool
}

func stampOf(path string) fileStamp {
	info, err := os.Stat(path)
	if err != nil {
		return fileStamp{}
	}
	return fileStamp{info.Size(), info.ModTime().UnixNano(), true}
}

// unchangedOnDisk refuses a change when another program changed the file
// since it was read or written: its size or modification time differ, and
// so do its bytes.
func (d *fileDriver) unchangedOnDisk() error {
	if d.readonly {
		return nil
	}
	now := stampOf(d.path)
	if now == d.seen {
		return nil
	}
	read, err := os.ReadFile(d.path)
	if err != nil && !(os.IsNotExist(err) && len(d.original) == 0) {
		return Refuse(SavedSinceText())
	}
	if !bytes.Equal(read, d.original) {
		return Refuse(SavedSinceText())
	}
	// The same bytes written again: no change.
	d.seen = now
	return nil
}

// fileTable is one table read from the file and kept in SQLite.
type fileTable struct {
	// name is its name in the file's tables: "" for the source's own, lines or lines/notes for records inside.
	name  string
	table string
	key   string
	// columns are the file's columns, in its order; for records inside, the second links a row to its row.
	columns []string
	// linked is whether the second column is a link: the place of the row in the table it sits in.
	linked bool
}

// fileField is a field of a file source's fields; a records field says the table and key its records are kept in.
type fileField struct {
	Name   string      `json:"name"`
	Kind   string      `json:"kind"`
	Fields []fileField `json:"fields"`
	Table  string      `json:"table"`
	Key    string      `json:"key"`
}

// fileParent is the table a nested table's rows sit in: lines/notes in lines, lines in the source's own.
func fileParent(name string) string {
	if at := strings.LastIndex(name, "/"); at >= 0 {
		return name[:at]
	}
	return ""
}

// recordsField is the records field the name (lines/notes) leads to among fields.
func recordsField(fields []fileField, name string) *fileField {
	var found *fileField
	for _, step := range strings.Split(name, "/") {
		found = nil
		for i := range fields {
			if fields[i].Name == step {
				found = &fields[i]
				break
			}
		}
		if found == nil {
			return nil
		}
		fields = found.Fields
	}
	return found
}

// AddFile adds the file source, found as a SQLite file's location is, read
// through read and written back through write.
func (db *Db) AddFile(source FileSource, read TableReader, write TableWriter) error {
	at, err := location(DataFolderSqlite(source.Path, os.Getenv))
	if err != nil {
		return err
	}
	d, err := openFile(source, at, read, write)
	if err != nil {
		return err
	}
	kept := db.sources[:0]
	for _, s := range db.sources {
		if s.name == source.Name {
			s.driver.close()
			continue
		}
		kept = append(kept, s)
	}
	db.sources = append(kept, &sourceDb{name: source.Name, driver: d})
	return nil
}

// plainName is a name as names are matched: without _, - and spaces, in lower case.
func plainName(name string) string {
	return strings.ToLower(strings.NewReplacer("_", "", "-", "", " ", "").Replace(name))
}

// fileColumnType is the SQLite type a mapped column of kind gets, so its values are kept as the generated code reads them.
func fileColumnType(kind string) string {
	switch kind {
	case "integer", "bool":
		return " INTEGER"
	case "text", "decimal", "date", "datetime", "enum", "values":
		return " TEXT"
	}
	return ""
}

// fileCell is a cell of the table as SQLite keeps it.
func fileCell(cell json.RawMessage) any {
	var value any
	decoder := json.NewDecoder(bytes.NewReader(cell))
	decoder.UseNumber()
	if decoder.Decode(&value) != nil {
		return nil
	}
	switch v := value.(type) {
	case nil:
		return nil
	case bool:
		if v {
			return int64(1)
		}
		return int64(0)
	case json.Number:
		if n, err := strconv.ParseInt(string(v), 10, 64); err == nil {
			return n
		}
		return string(v)
	case string:
		return v
	default:
		// An object or list inside is kept as its JSON text.
		return string(cell)
	}
}

// tableCell is a value SQLite kept as a cell of the table.
func tableCell(value any) any {
	switch v := value.(type) {
	case int64:
		return v
	case float64:
		return json.Number(strconv.FormatFloat(v, 'f', -1, 64))
	case string:
		return v
	case bool:
		if v {
			return int64(1)
		}
		return int64(0)
	default:
		return nil
	}
}

// openFile reads the file at path (already found as a SQLite file's location is) into its table.
func openFile(source FileSource, path string, read TableReader, write TableWriter) (*fileDriver, error) {
	failed := func(what string) error {
		return &StorageError{What: fmt.Sprintf("data source %s: %s", source.Name, what)}
	}
	// A file not there yet is a new one, made at the first save; a readonly source needs it.
	original, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) && !source.Readonly {
		original, err = nil, nil
	}
	if err != nil {
		return nil, failed(fmt.Sprintf("cannot read %s: %v", path, err))
	}
	text, err := read(source.Format, original, source.Options, source.Fields)
	if err != nil {
		return nil, failed(err.Error())
	}
	type readTable struct {
		Name    string              `json:"name"`
		Columns []string            `json:"columns"`
		Rows    [][]json.RawMessage `json:"rows"`
	}
	var given struct {
		readTable
		Tables []readTable `json:"tables"`
	}
	if json.Unmarshal([]byte(text), &given) != nil || (given.Columns == nil && given.Tables == nil) {
		return nil, failed("the table read is no table")
	}
	several := given.Tables != nil
	tables := given.Tables
	if !several {
		tables = []readTable{given.readTable}
	}
	var fields []fileField
	json.Unmarshal([]byte(source.Fields), &fields)
	memory, err := openSqlite(":memory:")
	if err != nil {
		return nil, err
	}
	d := &fileDriver{
		sqliteDriver: memory.(*sqliteDriver),
		path:         path,
		format:       source.Format,
		options:      source.Options,
		fields:       source.Fields,
		several:      several,
		original:     original,
		write:        write,
		readonly:     source.Readonly,
		seen:         stampOf(path),
	}
	// keys are the keys of each table's rows, in the order read, so a link to a place becomes a key.
	keys := map[string][]int64{}
	for _, table := range tables {
		kept := fileTable{name: table.Name, table: source.Table, key: source.Key, columns: table.Columns, linked: table.Name != ""}
		kinds := fields
		if kept.linked {
			field := recordsField(fields, table.Name)
			if field == nil {
				d.close()
				return nil, failed(fmt.Sprintf("the records inside '%s' are no field of the fields given", table.Name))
			}
			kept.table, kept.key, kinds = field.Table, field.Key, field.Fields
		}
		kindOf := func(column string) string {
			for _, f := range kinds {
				if plainName(f.Name) == plainName(column) {
					return fileColumnType(f.Kind)
				}
			}
			return ""
		}
		keyed := false
		for _, c := range table.Columns {
			keyed = keyed || c == kept.key
		}
		var columns []string
		if !keyed {
			// The file has no key column: the row's place keys it, never written back.
			columns = append(columns, Sqlite.Quote(kept.key)+" INTEGER PRIMARY KEY")
		}
		names := make([]string, len(table.Columns))
		for i, c := range table.Columns {
			kind := kindOf(c)
			if c == kept.key {
				kind = " INTEGER PRIMARY KEY"
			} else if kept.linked && i == 1 {
				kind = " INTEGER"
			}
			columns = append(columns, Sqlite.Quote(c)+kind)
			names[i] = Sqlite.Quote(c)
		}
		if err := d.sqliteDriver.batch("CREATE TABLE " + Sqlite.Quote(kept.table) + " (" + strings.Join(columns, ", ") + ")"); err != nil {
			d.close()
			return nil, err
		}
		marks := strings.TrimSuffix(strings.Repeat("?, ", len(names)), ", ")
		insert := "INSERT INTO " + Sqlite.Quote(kept.table) + " (" + strings.Join(names, ", ") + ") VALUES (" + marks + ")"
		parentKeys := keys[fileParent(table.Name)]
		inserted := make([]int64, 0, len(table.Rows))
		for i, row := range table.Rows {
			values := make([]any, len(row))
			for n, cell := range row {
				values[n] = fileCell(cell)
			}
			if kept.linked && len(values) > 1 {
				// The place of the row it sits in, from 1, is that row's key.
				place, ok := values[1].(int64)
				values[1] = nil
				if ok && place >= 1 && int(place) <= len(parentKeys) {
					values[1] = parentKeys[place-1]
				}
			}
			id, err := d.sqliteDriver.insert(insert, values)
			if err != nil {
				d.close()
				return nil, failed(fmt.Sprintf("row %d: %v", i+1, err))
			}
			inserted = append(inserted, id)
		}
		keys[table.Name] = inserted
		d.tables = append(d.tables, kept)
	}
	return d, nil
}

// tablesJSON is the tables as the file takes them back: each table's rows
// in their order, and a link turned back into the place of its row among
// the rows of the table it sits in.
func (d *fileDriver) tablesJSON() (string, error) {
	type written struct {
		Name    string   `json:"name"`
		Columns []string `json:"columns"`
		Rows    [][]any  `json:"rows"`
	}
	places := map[string]map[int64]int{}
	var out []written
	for _, kept := range d.tables {
		names := make([]string, len(kept.columns))
		for i, c := range kept.columns {
			names[i] = Sqlite.Quote(c)
		}
		rows, err := d.sqliteDriver.rows("SELECT "+Sqlite.Quote(kept.key)+", "+strings.Join(names, ", ")+" FROM "+Sqlite.Quote(kept.table)+" ORDER BY rowid", nil)
		if err != nil {
			return "", err
		}
		parent := places[fileParent(kept.name)]
		own := map[int64]int{}
		cells := make([][]any, len(rows))
		for i, row := range rows {
			if key, ok := row[0].(int64); ok {
				own[key] = i + 1
			}
			cells[i] = make([]any, len(row)-1)
			for n, value := range row[1:] {
				cells[i][n] = tableCell(value)
			}
			if kept.linked && len(cells[i]) > 1 {
				cells[i][1] = nil
				if key, ok := row[2].(int64); ok {
					if place, found := parent[key]; found {
						cells[i][1] = int64(place)
					}
				}
			}
		}
		places[kept.name] = own
		out = append(out, written{Name: kept.name, Columns: kept.columns, Rows: cells})
	}
	var table []byte
	var err error
	if d.several {
		table, err = json.Marshal(map[string]any{"tables": out})
	} else {
		table, err = json.Marshal(map[string]any{"columns": out[0].Columns, "rows": out[0].Rows})
	}
	if err != nil {
		return "", &StorageError{What: err.Error(), Err: err}
	}
	return string(table), nil
}

// keep writes the table back when a change is kept: to a new file beside the old one, which then replaces it.
func (d *fileDriver) keep() error {
	if !d.dirty || d.inTransaction || d.readonly {
		return nil
	}
	d.dirty = false
	table, err := d.tablesJSON()
	if err != nil {
		return err
	}
	written, err := d.write(d.format, d.original, table, d.options, d.fields)
	if err != nil {
		return &StorageError{What: err.Error(), Err: err}
	}
	if err := os.MkdirAll(filepath.Dir(d.path), 0o755); err != nil {
		return &StorageError{What: fmt.Sprintf("cannot make %s: %v", filepath.Dir(d.path), err), Err: err}
	}
	next := d.path + ".new"
	if err := os.WriteFile(next, written, 0o644); err != nil {
		return &StorageError{What: fmt.Sprintf("cannot write %s: %v", next, err), Err: err}
	}
	if err := os.Rename(next, d.path); err != nil {
		return &StorageError{What: fmt.Sprintf("cannot replace %s: %v", d.path, err), Err: err}
	}
	d.original = written
	d.seen = stampOf(d.path)
	return nil
}

func (d *fileDriver) execute(query string, params []any) (int64, error) {
	if err := d.unchangedOnDisk(); err != nil {
		return 0, err
	}
	changed, err := d.sqliteDriver.execute(query, params)
	if err != nil {
		return 0, err
	}
	d.dirty = d.dirty || changed > 0
	return changed, d.keep()
}

func (d *fileDriver) insert(query string, params []any) (int64, error) {
	if err := d.unchangedOnDisk(); err != nil {
		return 0, err
	}
	id, err := d.sqliteDriver.insert(query, params)
	if err != nil {
		return 0, err
	}
	d.dirty = true
	return id, d.keep()
}

// batch follows the transaction: its commit keeps what it changed, its
// rollback nothing; a file another program changed meanwhile rolls it back.
func (d *fileDriver) batch(query string) error {
	if query == Sqlite.commit() && d.dirty {
		if err := d.unchangedOnDisk(); err != nil {
			d.sqliteDriver.batch(Sqlite.rollback())
			d.inTransaction = false
			d.dirty = false
			return err
		}
	}
	if err := d.sqliteDriver.batch(query); err != nil {
		return err
	}
	switch query {
	case Sqlite.begin():
		d.inTransaction = true
	case Sqlite.rollback():
		d.inTransaction = false
		d.dirty = false
	case Sqlite.commit():
		d.inTransaction = false
	}
	return d.keep()
}

// apply does nothing: a file source is external, Clarity writes no steps for it.
func (*fileDriver) apply(int, []string) error { return nil }
