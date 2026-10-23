// The SQL dialects of the databases a data source can be. Statements that
// read and write records are written here, so the rest of the runtime works
// the same on every database. The other runtimes have the same rules;
// conformance/queries.json keeps them equal.

#pragma once

#include <string>
#include <vector>

namespace clarity {

enum class Dialect { Sqlite, Postgres, Mysql, Mssql };

/// The database's name, for messages.
std::string label(Dialect d);
/// A table or column name as SQL needs it, so names like order work.
std::string quote(Dialect d, const std::string& name);
/// The mark of parameter `n`, counted from 1.
std::string mark(Dialect d, int n);
/// A decimal as a comparison reads it: SQLite stores decimals as text.
std::string number(Dialect d, const std::string& expr);
/// `column` containing the LIKE pattern `mark`, ignoring case; wildcards are escaped with !.
std::string contains(Dialect d, const std::string& column, const std::string& mark);
/// Whether the JSON list kept in `column` holds the item `mark` (a list field).
std::string holds(Dialect d, const std::string& column, const std::string& mark);
/// Text that LIKE matches as it is.
std::string escape_like(Dialect d, const std::string& text);
/// A bool column that is true.
std::string is_true(Dialect d, const std::string& column);
/// The INSERT of a new record, giving its key back on PostgreSQL and SQL Server.
std::string insert(Dialect d, const std::string& table, const std::string& key, const std::vector<std::string>& columns);
std::string begin(Dialect d);
std::string commit(Dialect d);
std::string rollback(Dialect d);
/// Creates the table that remembers how many migration steps a database has had.
std::string version_table(Dialect d);

}  // namespace clarity
