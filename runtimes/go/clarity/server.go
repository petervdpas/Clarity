// What the server databases share over database/sql: one connection, held
// for the driver's life rather than taken from the pool for each call (the
// pool resets a connection between calls, which would lose the transaction
// of a save with its data events), values written as the drivers take
// them, and a broken reference told apart from other failures. Each
// database adds how it gives the new key, which error is a broken
// reference, and how it runs a migration step. A generated application
// keeps the files of the databases it uses; each one adds itself to servers.

package clarity

import (
	"context"
	"database/sql"
	"fmt"
	"reflect"
	"time"

	"github.com/shopspring/decimal"
)

var background = context.Background()

type serverDriver struct {
	db   *sql.DB
	conn *sql.Conn
	kind Dialect
	// isReference tells whether an error means a record refers to one that
	// is not there, or is referred to.
	isReference func(error) bool
	// value is a value as the database's driver takes it, for the values
	// serverValue leaves alone.
	value func(any) any
	// returning is whether the INSERT gives the new key back as a row.
	returning bool
	// undoes is whether the database can undo changes to tables, so a
	// migration step runs in one transaction.
	undoes bool
}

// connect opens the one connection of a server database.
func connect(kind Dialect, db *sql.DB) (*serverDriver, error) {
	conn, err := db.Conn(background)
	if err == nil {
		err = conn.PingContext(background)
	}
	if err != nil {
		db.Close()
		return nil, &StorageError{What: fmt.Sprintf("cannot connect to %s: %v", kind.Label(), err), Err: err}
	}
	return &serverDriver{db: db, conn: conn, kind: kind}, nil
}

func (d *serverDriver) dialect() Dialect { return d.kind }

// serverValue is a value as a server takes it: an enum as its name, a
// decimal as its text, a date as its driver takes it.
func (d *serverDriver) serverValue(value any) any {
	switch v := value.(type) {
	case nil:
		return nil
	case decimal.Decimal:
		return v.String()
	case *int64:
		if v == nil {
			return nil
		}
		return *v
	case *Date:
		if v == nil {
			return nil
		}
		return d.value(*v)
	case *DateTime:
		if v == nil {
			return nil
		}
		return d.value(*v)
	case Date, DateTime:
		return d.value(v)
	}
	if r := reflect.ValueOf(value); r.Kind() == reflect.String {
		return r.String()
	}
	return value
}

// asTime is a date or a date and time as a time in UTC, as most drivers take it.
func asTime(value any) any {
	switch v := value.(type) {
	case Date:
		return time.Date(v.Year, v.Month, v.Day, 0, 0, 0, 0, time.UTC)
	case DateTime:
		return time.Date(v.Year, v.Month, v.Day, v.Hour, v.Minute, v.Second, 0, time.UTC)
	}
	return value
}

func (d *serverDriver) params(params []any) []any {
	out := make([]any, len(params))
	for i, p := range params {
		out[i] = d.serverValue(p)
	}
	return out
}

func (d *serverDriver) failed(err error) error {
	if d.isReference(err) {
		return &referenceFailure{err}
	}
	return &StorageError{What: err.Error(), Err: err}
}

func (d *serverDriver) execute(query string, params []any) (int64, error) {
	result, err := d.conn.ExecContext(background, query, d.params(params)...)
	if err != nil {
		return 0, d.failed(err)
	}
	return result.RowsAffected()
}

func (d *serverDriver) insert(query string, params []any) (int64, error) {
	if !d.returning {
		result, err := d.conn.ExecContext(background, query, d.params(params)...)
		if err != nil {
			return 0, d.failed(err)
		}
		return result.LastInsertId()
	}
	var id any
	if err := d.conn.QueryRowContext(background, query, d.params(params)...).Scan(&id); err != nil {
		return 0, d.failed(err)
	}
	n, ok := integer(id)
	if !ok {
		return 0, &StorageError{What: fmt.Sprintf("the new key %v is no number", id)}
	}
	return n, nil
}

func (d *serverDriver) rows(query string, params []any) ([][]any, error) {
	rows, err := d.conn.QueryContext(background, query, d.params(params)...)
	if err != nil {
		return nil, d.failed(err)
	}
	return readRows(rows)
}

func (d *serverDriver) batch(query string) error {
	if _, err := d.conn.ExecContext(background, query); err != nil {
		return d.failed(err)
	}
	return nil
}

// apply runs every statement of a step and records it: all or nothing where
// the database can undo changes to tables. Where it cannot, a step that
// fails halfway keeps what ran before the failure, and says so.
func (d *serverDriver) apply(version int, statements []string) error {
	if !d.undoes {
		for i, statement := range statements {
			if err := d.batch(statement); err != nil {
				if i == 0 {
					return err
				}
				return &StorageError{What: fmt.Sprintf("%v; %s cannot undo the %d statement(s) of step %d that ran before it", err, d.kind.Label(), i, version), Err: err}
			}
		}
		return d.batch(applied(version))
	}
	tx, err := d.conn.BeginTx(background, nil)
	if err != nil {
		return d.failed(err)
	}
	defer tx.Rollback()
	for _, statement := range append(statements, applied(version)) {
		if _, err := tx.Exec(statement); err != nil {
			return d.failed(err)
		}
	}
	if err := tx.Commit(); err != nil {
		return d.failed(err)
	}
	return nil
}

func (d *serverDriver) close() error {
	d.conn.Close()
	return d.db.Close()
}
