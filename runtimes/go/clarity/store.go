// The connection to one database, behind the same few calls for every
// kind: run a statement, insert and get the new key, read rows, and apply a
// migration step. SQLite is always there; a server database is there when
// the application has its driver's file (server.go has what they share).

package clarity

import (
	"fmt"
	"os"
	"strings"
)

type driver interface {
	dialect() Dialect
	// execute runs a statement and gives the number of rows it changed.
	execute(sql string, params []any) (int64, error)
	// insert runs an INSERT from Dialect.insert and gives the new key.
	insert(sql string, params []any) (int64, error)
	rows(sql string, params []any) ([][]any, error)
	// batch runs SQL without parameters, such as a statement that creates a table.
	batch(sql string) error
	// apply runs the statements of migration step version and records that
	// the database has it, all or nothing where the database can undo
	// changes to tables.
	apply(version int, statements []string) error
	close() error
}

// applied is the statement that records migration step version.
func applied(version int) string {
	return fmt.Sprintf("INSERT INTO clarity_schema (version) VALUES (%d)", version)
}

// location is where a database is: as written, or from the environment
// variable env:NAME names, so passwords stay out of the program.
func location(text string) (string, error) {
	name, ok := strings.CutPrefix(text, "env:")
	if !ok {
		return text, nil
	}
	value, set := os.LookupEnv(name)
	if !set {
		return "", &StorageError{What: "the environment variable " + name + " is not set"}
	}
	return value, nil
}

// open connects to the database of a data source.
func open(dialect Dialect, at string) (driver, error) {
	where, err := location(at)
	if err != nil {
		return nil, err
	}
	if opener, ok := servers[dialect]; ok {
		return opener(where)
	}
	if dialect == Sqlite {
		return openSqlite(where)
	}
	return nil, &StorageError{What: "this application was built without " + dialect.Label()}
}

// servers are the server drivers this build has, by dialect: those whose
// package the application imports (UseServer).
var servers = map[Dialect]func(location string) (driver, error){}

// HasServer tells whether the server database's package is imported.
func HasServer(dialect Dialect) bool {
	_, ok := servers[dialect]
	return ok
}
