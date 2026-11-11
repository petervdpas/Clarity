// The databases of an application: one per data source. Records are saved,
// deleted and read through here, with the entity's data events around a
// save or delete, and migration steps are applied here at startup.

package clarity

import (
	"errors"
	"fmt"
	"reflect"
	"strings"
)

// Source is the database of one data source: its name, its kind and where
// it is (a file for SQLite, a connection for the others). env:NAME takes
// the location from that environment variable.
type Source struct {
	Name    string
	Dialect Dialect
	At      string
}

type sourceDb struct {
	name   string
	driver driver
	// depth is how many saves and deletes with data events are running in
	// it; the outermost one holds the transaction.
	depth int
}

// Db is the databases of an application.
type Db struct {
	// sources by data source name; the first is the default.
	sources []*sourceDb
	// stores are the folders of the file stores, by name.
	stores map[string]string
	// storeSettings are the file stores whose folder is a folder setting, by name.
	storeSettings map[string]storeSetting
	// history is what undo takes back and redo does again (DESIGN 5.29).
	history history
	// people is who uses the program and how they sign in (signin.go); nil without a signin block.
	people *people
}

// Connect opens a database per data source.
func Connect(sources ...Source) (*Db, error) {
	db := &Db{}
	for _, s := range sources {
		d, err := open(s.Dialect, s.At)
		if err != nil {
			db.Close()
			return nil, err
		}
		db.sources = append(db.sources, &sourceDb{name: s.Name, driver: d})
	}
	return db, nil
}

// Open is one SQLite database, used for every record.
func Open(path string) (*Db, error) { return Connect(Source{"", Sqlite, path}) }

// Memory is a SQLite database that lives in memory, for tests.
func Memory() (*Db, error) { return Open(":memory:") }

func (db *Db) Close() error {
	var first error
	for _, s := range db.sources {
		if err := s.driver.close(); err != nil && first == nil {
			first = err
		}
	}
	return first
}

// source is the data source named source. With one database, every record is in it.
func (db *Db) source(source string) (*sourceDb, error) {
	single := len(db.sources) == 1
	for _, s := range db.sources {
		if single || source == "" || s.name == source {
			return s, nil
		}
	}
	return nil, &StorageError{What: fmt.Sprintf("there is no data source '%s'", source)}
}

// Rows is the rows SQL reads in the database of source, for tools and tests.
func (db *Db) Rows(source, sql string, params ...any) ([][]any, error) {
	s, err := db.source(source)
	if err != nil {
		return nil, err
	}
	return s.driver.rows(sql, params)
}

// Batch runs SQL without parameters in the database of source, for tools and tests.
func (db *Db) Batch(source, sql string) error {
	s, err := db.source(source)
	if err != nil {
		return err
	}
	return s.driver.batch(sql)
}

// Dialect is the kind of database source is.
func (db *Db) Dialect(source string) (Dialect, error) {
	s, err := db.source(source)
	if err != nil {
		return 0, err
	}
	return s.driver.dialect(), nil
}

// SchemaVersion is the number of migration steps applied so far to the default database.
func (db *Db) SchemaVersion() (int, error) {
	s, err := db.source("")
	if err != nil {
		return 0, err
	}
	return schemaVersion(s.driver)
}

// Migrate is MigrateSource for the default database.
func (db *Db) Migrate(steps [][]string) (int, error) { return db.MigrateSource("", steps) }

// MigrateSource applies the steps of source its database has not had yet, each on its own.
func (db *Db) MigrateSource(source string, steps [][]string) (int, error) {
	s, err := db.source(source)
	if err != nil {
		return 0, err
	}
	done, err := schemaVersion(s.driver)
	if err != nil {
		return 0, err
	}
	// A database that already has a schema is backed up before it changes (docs/BACKUP-PLAN.md).
	if done > 0 && done < len(steps) {
		if err := backupBeforeMigration(s.driver, done+1); err != nil {
			return 0, err
		}
	}
	for i := done; i < len(steps); i++ {
		if err := s.driver.apply(i+1, steps[i]); err != nil {
			return i - done, err
		}
	}
	return max(0, len(steps)-done), nil
}

func schemaVersion(d driver) (int, error) {
	if err := d.batch(d.dialect().versionTable()); err != nil {
		return 0, err
	}
	rows, err := d.rows("SELECT MAX(version) FROM clarity_schema", nil)
	if err != nil {
		return 0, err
	}
	if len(rows) == 0 {
		return 0, nil
	}
	n, _ := integer(rows[0][0])
	return int(n), nil
}

// atomic runs run in a transaction of source, or in the one that is open
// already: a save that an event of another save makes is part of it.
func (db *Db) atomic(source string, run func() error) error {
	s, err := db.source(source)
	if err != nil {
		return err
	}
	outer := s.depth == 0
	d := s.driver.dialect()
	if outer {
		if err := s.driver.batch(d.begin()); err != nil {
			return err
		}
	}
	s.depth++
	err = run()
	s.depth--
	// A step that holds the transaction ends it itself.
	if s.depth == 0 {
		if err != nil {
			// The error that made it roll back is the one to report.
			s.driver.batch(d.rollback())
			return err
		}
		// A commit that fails leaves nothing half kept.
		if err := s.driver.batch(d.commit()); err != nil {
			s.driver.batch(d.rollback())
			return err
		}
	}
	return err
}

// SavedSinceText is why a save or a delete is refused: someone saved the
// record since it was read (conformance/versions.json), in Clarity's own
// words with their English.
func SavedSinceText() Text {
	return KeyedText("savedSince", "Someone else changed this record since you opened it. Reload it to see their change.")
}

// Save inserts a new record (and gives it its id) or updates a stored one,
// with the entity's data events around it.
// A record whose required field is empty is refused (required.go), and so
// is one someone saved since it was read (SavedSinceText), changing nothing.
func Save[R Record](db *Db, record R) error {
	return saveWith(db, record, false)
}

// SaveAnyway saves a record regardless of its version (save ... anyway):
// what someone saved since is written over, on purpose.
func SaveAnyway[R Record](db *Db, record R) error {
	return saveWith(db, record, true)
}

// Reload reads a record again as it is stored now, with its version.
func Reload[T any, P interface {
	*T
	Record
}](db *Db, record P) error {
	found, err := Get[T, P](db, record.GetID())
	if err != nil {
		return err
	}
	*record = found
	return nil
}

func saveWith[R Record](db *Db, record R, anyway bool) error {
	// A person's secret changed by this save closes their other sessions (signin.go).
	secrets := db.secretsOf(record.Table(), record.GetID())
	err := saveRecord(db, record, anyway)
	if err == nil {
		db.secretsSaved(record.Table(), record.GetID(), secrets)
	}
	return err
}

func saveRecord[R Record](db *Db, record R, anyway bool) error {
	if !hasEvents(record) {
		if err := checkRequired(record); err != nil {
			return err
		}
		return store(db, record, anyway)
	}
	id, version := record.GetID(), versionValue(record)
	err := db.atomic(sourceOf(record), func() error {
		if e, ok := any(record).(BeforeSaver); ok {
			if err := e.BeforeSave(db); err != nil {
				return err
			}
		}
		// A required field is checked after before save, which may fill it.
		if err := checkRequired(record); err != nil {
			return err
		}
		if err := store(db, record, anyway); err != nil {
			return err
		}
		if e, ok := any(record).(AfterSaver); ok {
			return e.AfterSave(db)
		}
		return nil
	})
	// Nothing was kept, so a new record is still new, at the version it was read with.
	if err != nil {
		record.SetID(id)
		if v, ok := any(record).(Versioned); ok {
			v.SetVersion(version)
		}
	}
	return err
}

// versionValue is the version a record holds; 0 when it has none.
func versionValue(r Record) int64 {
	if v, ok := r.(Versioned); ok {
		return v.GetVersion()
	}
	return 0
}

// unfit is a failed save: a reference to a record that is not there reads
// better than the database's own words.
func unfit(err error) error {
	var reference *referenceFailure
	if errors.As(err, &reference) {
		return &InvalidError{"a reference does not fit: " + reference.Error()}
	}
	return err
}

func store(db *Db, record Record, anyway bool) error {
	if err := db.enlist(sourceOf(record)); err != nil {
		return err
	}
	if db.history.recording == nil {
		return storeRow(db, record, anyway)
	}
	c := change{source: sourceOf(record), table: record.Table(), key: keyOf(record), columns: record.Columns(), version: versionOf(record)}
	if record.GetID() != 0 {
		var err error
		if c.before, c.beforeVersion, err = db.state(c, record.GetID()); err != nil {
			return err
		}
	}
	if err := storeRow(db, record, anyway); err != nil {
		return err
	}
	c.id = record.GetID()
	var err error
	if c.after, c.afterVersion, err = db.state(c, c.id); err != nil {
		return err
	}
	db.history.record(c)
	return nil
}

// refusal is the refusal of a save or delete of a record someone saved
// since, in its entity's words about the record as stored now.
func (db *Db) refusal(record Record) error {
	if _, own := record.(SavedSincer); own {
		current := reflectNew(record)
		if err := db.reread(current, record.GetID()); err == nil {
			if words, ok := current.(SavedSincer); ok {
				return Refuse(words.SavedSince(db))
			}
		}
	}
	return Refuse(SavedSinceText())
}

func storeRow(db *Db, record Record, anyway bool) error {
	s, err := db.source(sourceOf(record))
	if err != nil {
		return err
	}
	d := s.driver.dialect()
	table, key, columns := record.Table(), keyOf(record), record.Columns()
	values, err := db.savedValues(record)
	if err != nil {
		return err
	}
	isPicture := func(c string) bool {
		_, ok := pictureColumn(record, c)
		return ok
	}
	if record.GetID() == 0 {
		// An empty or kept picture is left to the column's NULL, so no NULL
		// is bound to a binary column (SQL Server refuses that).
		var names []string
		var params []any
		for i, c := range columns {
			_, kept := values[i].(keptValue)
			if isPicture(c) && (values[i] == nil || kept) {
				continue
			}
			names = append(names, c)
			params = append(params, values[i])
		}
		// A new record starts at version 1.
		version, versioned := any(record).(Versioned)
		if versioned {
			names = append(names, version.VersionColumn())
			params = append(params, int64(1))
		}
		id, err := s.driver.insert(d.insert(table, key, names), params)
		if err != nil {
			return unfit(err)
		}
		record.SetID(id)
		if versioned {
			version.SetVersion(1)
		}
		return nil
	}
	// A kept picture stays as it is; an emptied one is set to NULL as text.
	var sets []string
	var params []any
	for i, c := range columns {
		if _, kept := values[i].(keptValue); kept {
			continue
		}
		if values[i] == nil && isPicture(c) {
			sets = append(sets, d.Quote(c)+" = NULL")
			continue
		}
		params = append(params, values[i])
		sets = append(sets, d.Quote(c)+" = "+d.Mark(len(params)))
	}
	// Each save adds one to the version; a checked one writes only over the version it read.
	version, versioned := any(record).(Versioned)
	if versioned {
		q := d.Quote(version.VersionColumn())
		sets = append(sets, q+" = "+q+" + 1")
	}
	if len(sets) == 0 {
		return exists(s.driver, d, table, key, record.GetID())
	}
	params = append(params, record.GetID())
	sql := "UPDATE " + d.Quote(table) + " SET " + strings.Join(sets, ", ") + " WHERE " + d.Quote(key) + " = " + d.Mark(len(params))
	checked := versioned && !anyway
	if checked {
		params = append(params, version.GetVersion())
		sql += " AND " + d.Quote(version.VersionColumn()) + " = " + d.Mark(len(params))
	}
	changed, err := s.driver.execute(sql, params)
	if err != nil {
		return unfit(err)
	}
	// MySQL counts only the rows whose values changed, so an update that
	// changes nothing is told apart from a missing record (one with a
	// version always changes).
	if changed == 0 {
		if err := exists(s.driver, d, table, key, record.GetID()); err != nil {
			return err
		}
		if checked {
			return db.refusal(record)
		}
	}
	switch {
	case versioned && anyway:
		rows, err := s.driver.rows("SELECT "+d.Quote(version.VersionColumn())+" FROM "+d.Quote(table)+" WHERE "+d.Quote(key)+" = "+d.Mark(1), []any{record.GetID()})
		if err != nil {
			return err
		}
		if len(rows) > 0 {
			n, _ := integer(rows[0][0])
			version.SetVersion(n)
		}
	case versioned:
		version.SetVersion(version.GetVersion() + 1)
	}
	return nil
}

// exists is nil when the record id is stored, else a NotFoundError.
func exists(conn driver, d Dialect, table, key string, id int64) error {
	rows, err := conn.rows("SELECT "+d.Quote(key)+" FROM "+d.Quote(table)+" WHERE "+d.Quote(key)+" = "+d.Mark(1), []any{id})
	if err != nil {
		return err
	}
	if len(rows) == 0 {
		return &NotFoundError{table, &id}
	}
	return nil
}

// Delete removes a stored record, with the entity's data events around it.
func Delete[R Record](db *Db, record R) error {
	if !hasEvents(record) {
		return remove(db, record)
	}
	return db.atomic(sourceOf(record), func() error {
		if e, ok := any(record).(BeforeDeleter); ok {
			if err := e.BeforeDelete(db); err != nil {
				return err
			}
		}
		// What cascades from it goes first, so no reference is left behind.
		if c, ok := any(record).(Cascader); ok {
			if err := c.Cascade(db); err != nil {
				return err
			}
		}
		if err := remove(db, record); err != nil {
			return err
		}
		if e, ok := any(record).(AfterDeleter); ok {
			return e.AfterDelete(db)
		}
		return nil
	})
}

func remove(db *Db, record Record) error {
	if err := db.enlist(sourceOf(record)); err != nil {
		return err
	}
	if db.history.recording == nil {
		return removeRow(db, record)
	}
	c := change{source: sourceOf(record), table: record.Table(), key: keyOf(record), columns: record.Columns(), version: versionOf(record), id: record.GetID()}
	var err error
	if c.before, c.beforeVersion, err = db.state(c, c.id); err != nil {
		return err
	}
	if err := removeRow(db, record); err != nil {
		return err
	}
	db.history.record(c)
	return nil
}

func removeRow(db *Db, record Record) error {
	s, err := db.source(sourceOf(record))
	if err != nil {
		return err
	}
	d := s.driver.dialect()
	sql := "DELETE FROM " + d.Quote(record.Table()) + " WHERE " + d.Quote(keyOf(record)) + " = " + d.Mark(1)
	params := []any{record.GetID()}
	// A record someone saved since it was read is not deleted.
	version, versioned := any(record).(Versioned)
	if versioned {
		sql += " AND " + d.Quote(version.VersionColumn()) + " = " + d.Mark(2)
		params = append(params, version.GetVersion())
	}
	changed, err := s.driver.execute(sql, params)
	var reference *referenceFailure
	if errors.As(err, &reference) {
		return &InUseError{record.Table(), record.GetID()}
	}
	if err == nil && changed == 0 && versioned && exists(s.driver, d, record.Table(), keyOf(record), record.GetID()) == nil {
		return db.refusal(record)
	}
	return err
}

// selectColumns is the key and the stored columns of a record, for SELECT.
func selectColumns(d Dialect, record Record) string {
	columns := []string{d.Quote(keyOf(record))}
	for _, c := range record.Columns() {
		// Whether there is a picture, not the picture.
		if p, ok := pictureColumn(record, c); ok && p.Store == "" {
			columns = append(columns, Presence(d, c))
			continue
		}
		columns = append(columns, d.Quote(c))
	}
	// The version last, read with every record.
	if v := versionOf(record); v != "" {
		columns = append(columns, d.Quote(v))
	}
	return strings.Join(columns, ", ")
}

// Find is the records a query finds, in its order.
func Find[T any, P interface {
	*T
	Record
}](db *Db, query Query) ([]T, error) {
	var probe T
	record := P(&probe)
	s, err := db.source(sourceOf(record))
	if err != nil {
		return nil, err
	}
	d := s.driver.dialect()
	sql, params := query.SQL(d, record.Table(), keyOf(record), selectColumns(d, record))
	return records[T, P](s.driver, sql, params)
}

// All is every stored record of an entity, in the order they were stored.
func All[T any, P interface {
	*T
	Record
}](db *Db) ([]T, error) {
	return Find[T, P](db, Query{})
}

// Get is the record with id; a NotFoundError when there is none.
func Get[T any, P interface {
	*T
	Record
}](db *Db, id int64) (T, error) {
	var probe T
	record := P(&probe)
	s, err := db.source(sourceOf(record))
	if err != nil {
		return probe, err
	}
	d := s.driver.dialect()
	key := keyOf(record)
	sql := "SELECT " + selectColumns(d, record) + " FROM " + d.Quote(record.Table()) + " WHERE " + d.Quote(key) + " = " + d.Mark(1)
	found, err := records[T, P](s.driver, sql, []any{id})
	if err != nil {
		return probe, err
	}
	if len(found) == 0 {
		return probe, &NotFoundError{record.Table(), &id}
	}
	return found[len(found)-1], nil
}

// Load is the record a reference field points at.
func Load[T any, P interface {
	*T
	Record
}](db *Db, id *int64) (T, error) {
	if id == nil {
		var probe T
		return probe, &NotFoundError{P(&probe).Table(), nil}
	}
	return Get[T, P](db, *id)
}

func records[T any, P interface {
	*T
	Record
}](d driver, sql string, params []any) ([]T, error) {
	rows, err := d.rows(sql, params)
	if err != nil {
		return nil, err
	}
	out := make([]T, 0, len(rows))
	for _, row := range rows {
		var item T
		id, ok := integer(row[0])
		if !ok {
			return nil, &InvalidError{fmt.Sprintf("the key %v is no number", row[0])}
		}
		if err := loadRow(P(&item), id, row[1:]); err != nil {
			return nil, err
		}
		out = append(out, item)
	}
	return out, nil
}

// loadRow fills a record from a row read with selectColumns: its values,
// then its version, last.
func loadRow(record Record, id int64, values []any) error {
	v, versioned := record.(Versioned)
	var version int64
	if versioned && len(values) > 0 {
		version, _ = integer(values[len(values)-1])
		values = values[:len(values)-1]
	}
	if err := record.Load(id, values); err != nil {
		return err
	}
	if versioned {
		v.SetVersion(version)
	}
	return nil
}

// reread fills record with the record id as it is stored now.
func (db *Db) reread(record Record, id int64) error {
	s, err := db.source(sourceOf(record))
	if err != nil {
		return err
	}
	d := s.driver.dialect()
	key := keyOf(record)
	rows, err := s.driver.rows("SELECT "+selectColumns(d, record)+" FROM "+d.Quote(record.Table())+" WHERE "+d.Quote(key)+" = "+d.Mark(1), []any{id})
	if err != nil {
		return err
	}
	if len(rows) == 0 {
		return &NotFoundError{record.Table(), &id}
	}
	return loadRow(record, id, rows[0][1:])
}

// reflectNew is a new, empty record of the type of record.
func reflectNew(record Record) Record {
	return reflect.New(reflect.TypeOf(record).Elem()).Interface().(Record)
}

// Ref is the record a reference field points at, to work on in place.
func Ref[T any, P interface {
	*T
	Record
}](db *Db, id *int64) (*T, error) {
	record, err := Load[T, P](db, id)
	if err != nil {
		return nil, err
	}
	return &record, nil
}
