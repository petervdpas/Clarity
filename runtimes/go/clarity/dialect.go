// The SQL dialects of the databases a data source can be. Statements that
// read and write records are written here, so the rest of the runtime works
// the same on every database. The Rust and C# runtimes have the same rules;
// conformance/queries.json keeps them equal.

package clarity

import (
	"fmt"
	"strings"
)

// Dialect is the database a data source is.
type Dialect int

const (
	Sqlite Dialect = iota
	Postgres
	Mysql
	Mssql
)

// Label is the database's name, for messages.
func (d Dialect) Label() string {
	switch d {
	case Sqlite:
		return "SQLite"
	case Postgres:
		return "PostgreSQL"
	case Mysql:
		return "MySQL"
	default:
		return "SQL Server"
	}
}

// Quote writes a table or column name as SQL needs it, so names like order work.
func (d Dialect) Quote(name string) string {
	switch d {
	case Mysql:
		return "`" + strings.ReplaceAll(name, "`", "``") + "`"
	case Mssql:
		return "[" + strings.ReplaceAll(name, "]", "]]") + "]"
	default:
		return `"` + strings.ReplaceAll(name, `"`, `""`) + `"`
	}
}

// Mark is the mark of parameter n, counted from 1.
func (d Dialect) Mark(n int) string {
	switch d {
	case Sqlite:
		return fmt.Sprintf("?%d", n)
	case Postgres:
		return fmt.Sprintf("$%d", n)
	case Mysql:
		return "?"
	default:
		return fmt.Sprintf("@P%d", n)
	}
}

// number is a decimal value as a comparison reads it. SQLite stores
// decimals as text, so it compares them as numbers; the others store numbers.
func (d Dialect) number(expr string) string {
	if d == Sqlite {
		return "CAST(" + expr + " AS REAL)"
	}
	return expr
}

// contains is column containing the LIKE pattern mark, ignoring case. The
// pattern escapes its wildcards with !.
func (d Dialect) contains(column, mark string) string {
	if d == Sqlite {
		// SQLite's LIKE ignores case already.
		return d.Quote(column) + " LIKE " + mark + " ESCAPE '!'"
	}
	return "LOWER(" + d.Quote(column) + ") LIKE LOWER(" + mark + ") ESCAPE '!'"
}

// escapeLike is text that LIKE matches as it is: wildcards are escaped with !.
func (d Dialect) escapeLike(text string) string {
	var out strings.Builder
	for _, c := range text {
		if c == '!' || c == '%' || c == '_' || (c == '[' && d == Mssql) {
			out.WriteRune('!')
		}
		out.WriteRune(c)
	}
	return out.String()
}

// isTrue is a bool column that is true.
func (d Dialect) isTrue(column string) string {
	if d == Postgres {
		return d.Quote(column) + " = TRUE"
	}
	return d.Quote(column) + " = 1"
}

// insert is the INSERT of a new record into table, with a mark per column.
// On PostgreSQL and SQL Server it returns the new key; SQLite and MySQL tell
// it afterwards.
func (d Dialect) insert(table, key string, columns []string) string {
	t, k := d.Quote(table), d.Quote(key)
	names := make([]string, len(columns))
	marks := make([]string, len(columns))
	for i, c := range columns {
		names[i] = d.Quote(c)
		marks[i] = d.Mark(i + 1)
	}
	list, values := strings.Join(names, ", "), strings.Join(marks, ", ")
	if len(columns) == 0 {
		switch d {
		case Mysql:
			return "INSERT INTO " + t + " () VALUES ()"
		case Sqlite:
			return "INSERT INTO " + t + " DEFAULT VALUES"
		case Postgres:
			return "INSERT INTO " + t + " DEFAULT VALUES RETURNING " + k
		default:
			return "INSERT INTO " + t + " OUTPUT INSERTED." + k + " DEFAULT VALUES"
		}
	}
	switch d {
	case Postgres:
		return "INSERT INTO " + t + " (" + list + ") VALUES (" + values + ") RETURNING " + k
	case Mssql:
		return "INSERT INTO " + t + " (" + list + ") OUTPUT INSERTED." + k + " VALUES (" + values + ")"
	default:
		return "INSERT INTO " + t + " (" + list + ") VALUES (" + values + ")"
	}
}

func (d Dialect) begin() string {
	switch d {
	case Mysql:
		return "START TRANSACTION"
	case Mssql:
		return "BEGIN TRANSACTION"
	default:
		return "BEGIN"
	}
}

func (d Dialect) commit() string {
	if d == Mssql {
		return "COMMIT TRANSACTION"
	}
	return "COMMIT"
}

func (d Dialect) rollback() string {
	if d == Mssql {
		return "ROLLBACK TRANSACTION"
	}
	return "ROLLBACK"
}

// versionTable creates the table that remembers how many migration steps a
// database has had.
func (d Dialect) versionTable() string {
	if d == Mssql {
		return "IF OBJECT_ID(N'clarity_schema', N'U') IS NULL CREATE TABLE clarity_schema (version INT NOT NULL)"
	}
	return "CREATE TABLE IF NOT EXISTS clarity_schema (version INTEGER NOT NULL)"
}
