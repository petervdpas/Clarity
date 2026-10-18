// PostgreSQL through pgx: decimals are NUMERIC, dates DATE and TIMESTAMP,
// bools BOOLEAN. pgx reads the location as the project file writes it, a
// URL or libpq's key=value words. A migration step runs in one transaction,
// so a step that fails leaves the database as it was.

package clarity

import (
	"database/sql"
	"errors"

	"github.com/jackc/pgx/v5/pgconn"
	// The driver registers itself as "pgx".
	_ "github.com/jackc/pgx/v5/stdlib"
)

// postgresForeignKey is foreign_key_violation.
const postgresForeignKey = "23503"

func init() { servers[Postgres] = openPostgres }

func openPostgres(location string) (driver, error) {
	db, err := sql.Open("pgx", location)
	if err != nil {
		return nil, &StorageError{What: "cannot connect to PostgreSQL: " + err.Error(), Err: err}
	}
	d, err := connect(Postgres, db)
	if err != nil {
		return nil, err
	}
	isReference := func(err error) bool {
		var e *pgconn.PgError
		return errors.As(err, &e) && e.Code == postgresForeignKey
	}
	d.isReference, d.value, d.returning, d.undoes = isReference, asTime, true, true
	return d, nil
}
