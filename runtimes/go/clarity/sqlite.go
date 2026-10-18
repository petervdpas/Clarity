// SQLite through modernc.org/sqlite, in pure Go: one file per data source.
// Decimals and dates are stored as text, bools as 0 and 1. One connection
// holds the transaction of a save with its data events.

package clarity

import (
	"database/sql"
	"errors"
	"os"
	"path/filepath"

	"github.com/shopspring/decimal"
	"modernc.org/sqlite"
)

// sqliteForeignKey is SQLITE_CONSTRAINT_FOREIGNKEY.
const sqliteForeignKey = 787

type sqliteDriver struct{ db *sql.DB }

func openSqlite(path string) (driver, error) {
	if path != ":memory:" {
		if dir := filepath.Dir(path); dir != "" {
			if err := os.MkdirAll(dir, 0o755); err != nil {
				return nil, &StorageError{What: err.Error(), Err: err}
			}
		}
	}
	db, err := sql.Open("sqlite", "file:"+path+"?_pragma=foreign_keys(1)")
	if err != nil {
		return nil, &StorageError{What: err.Error(), Err: err}
	}
	// One connection: a transaction and the pragmas belong to it.
	db.SetMaxOpenConns(1)
	if err := db.Ping(); err != nil {
		db.Close()
		return nil, &StorageError{What: err.Error(), Err: err}
	}
	return &sqliteDriver{db}, nil
}

func (*sqliteDriver) dialect() Dialect { return Sqlite }

// stored is a value as SQLite keeps it.
func sqliteValue(value any) any {
	switch v := value.(type) {
	case decimal.Decimal:
		return v.String()
	case bool:
		if v {
			return int64(1)
		}
		return int64(0)
	case *int64:
		if v == nil {
			return nil
		}
		return *v
	case *Date:
		if v == nil {
			return nil
		}
		return v.String()
	case Date:
		return v.String()
	case *DateTime:
		if v == nil {
			return nil
		}
		return v.String()
	case DateTime:
		return v.String()
	default:
		return v
	}
}

func sqliteParams(params []any) []any {
	out := make([]any, len(params))
	for i, p := range params {
		out[i] = sqliteValue(p)
	}
	return out
}

// failed tells a broken reference apart from other failures.
func sqliteFailed(err error) error {
	var e *sqlite.Error
	if errors.As(err, &e) && e.Code() == sqliteForeignKey {
		return &referenceFailure{err}
	}
	return &StorageError{What: err.Error(), Err: err}
}

func (d *sqliteDriver) execute(query string, params []any) (int64, error) {
	result, err := d.db.Exec(query, sqliteParams(params)...)
	if err != nil {
		return 0, sqliteFailed(err)
	}
	return result.RowsAffected()
}

func (d *sqliteDriver) insert(query string, params []any) (int64, error) {
	result, err := d.db.Exec(query, sqliteParams(params)...)
	if err != nil {
		return 0, sqliteFailed(err)
	}
	return result.LastInsertId()
}

func (d *sqliteDriver) rows(query string, params []any) ([][]any, error) {
	rows, err := d.db.Query(query, sqliteParams(params)...)
	if err != nil {
		return nil, sqliteFailed(err)
	}
	return readRows(rows)
}

// readRows reads every row as values.
func readRows(rows *sql.Rows) ([][]any, error) {
	defer rows.Close()
	columns, err := rows.Columns()
	if err != nil {
		return nil, &StorageError{What: err.Error(), Err: err}
	}
	var out [][]any
	for rows.Next() {
		row := make([]any, len(columns))
		pointers := make([]any, len(columns))
		for i := range row {
			pointers[i] = &row[i]
		}
		if err := rows.Scan(pointers...); err != nil {
			return nil, &StorageError{What: err.Error(), Err: err}
		}
		out = append(out, row)
	}
	if err := rows.Err(); err != nil {
		return nil, &StorageError{What: err.Error(), Err: err}
	}
	return out, nil
}

func (d *sqliteDriver) batch(query string) error {
	if _, err := d.db.Exec(query); err != nil {
		return sqliteFailed(err)
	}
	return nil
}

// apply runs a step with references unchecked, since a step may build a
// table again that others refer to; they are checked before the step is kept.
func (d *sqliteDriver) apply(version int, statements []string) error {
	if err := d.batch("PRAGMA foreign_keys = OFF"); err != nil {
		return err
	}
	defer d.batch("PRAGMA foreign_keys = ON")
	tx, err := d.db.Begin()
	if err != nil {
		return sqliteFailed(err)
	}
	defer tx.Rollback()
	for _, statement := range statements {
		if _, err := tx.Exec(statement); err != nil {
			return sqliteFailed(err)
		}
	}
	var broken int64
	if err := tx.QueryRow("SELECT COUNT(*) FROM pragma_foreign_key_check").Scan(&broken); err != nil {
		return sqliteFailed(err)
	}
	if broken > 0 {
		return &StorageError{What: "migration step leaves references to records that do not exist"}
	}
	if _, err := tx.Exec(applied(version)); err != nil {
		return sqliteFailed(err)
	}
	if err := tx.Commit(); err != nil {
		return sqliteFailed(err)
	}
	return nil
}

func (d *sqliteDriver) close() error { return d.db.Close() }
