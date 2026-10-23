#include "clarity/dialect.hpp"

namespace clarity {

namespace {

std::string replaced(const std::string& text, char from, const std::string& to) {
    std::string out;
    for (char c : text) {
        if (c == from) out += to;
        else out += c;
    }
    return out;
}

std::string joined(const std::vector<std::string>& parts) {
    std::string out;
    for (size_t i = 0; i < parts.size(); ++i) out += (i ? ", " : "") + parts[i];
    return out;
}

}  // namespace

std::string label(Dialect d) {
    switch (d) {
    case Dialect::Sqlite: return "SQLite";
    case Dialect::Postgres: return "PostgreSQL";
    case Dialect::Mysql: return "MySQL";
    default: return "SQL Server";
    }
}

std::string quote(Dialect d, const std::string& name) {
    switch (d) {
    case Dialect::Mysql: return "`" + replaced(name, '`', "``") + "`";
    case Dialect::Mssql: return "[" + replaced(name, ']', "]]") + "]";
    default: return "\"" + replaced(name, '"', "\"\"") + "\"";
    }
}

std::string mark(Dialect d, int n) {
    switch (d) {
    case Dialect::Sqlite: return "?" + std::to_string(n);
    case Dialect::Postgres: return "$" + std::to_string(n);
    case Dialect::Mysql: return "?";
    default: return "@P" + std::to_string(n);
    }
}

std::string number(Dialect d, const std::string& expr) { return d == Dialect::Sqlite ? "CAST(" + expr + " AS REAL)" : expr; }

std::string contains(Dialect d, const std::string& column, const std::string& mark) {
    // SQLite's LIKE ignores case already.
    if (d == Dialect::Sqlite) return quote(d, column) + " LIKE " + mark + " ESCAPE '!'";
    return "LOWER(" + quote(d, column) + ") LIKE LOWER(" + mark + ") ESCAPE '!'";
}

std::string holds(Dialect d, const std::string& column, const std::string& mark) {
    std::string quoted = quote(d, column);
    switch (d) {
    case Dialect::Sqlite: return "EXISTS (SELECT 1 FROM json_each(" + quoted + ") WHERE value = " + mark + ")";
    case Dialect::Postgres: return quoted + "::jsonb @> " + mark + "::jsonb";
    case Dialect::Mysql: return "JSON_CONTAINS(" + quoted + ", " + mark + ")";
    default: return "EXISTS (SELECT 1 FROM OPENJSON(" + quoted + ") WHERE [value] = " + mark + ")";
    }
}

std::string escape_like(Dialect d, const std::string& text) {
    std::string out;
    for (char c : text) {
        if (c == '!' || c == '%' || c == '_' || (c == '[' && d == Dialect::Mssql)) out += '!';
        out += c;
    }
    return out;
}

std::string is_true(Dialect d, const std::string& column) { return quote(d, column) + (d == Dialect::Postgres ? " = TRUE" : " = 1"); }

std::string insert(Dialect d, const std::string& table, const std::string& key, const std::vector<std::string>& columns) {
    std::string t = quote(d, table), k = quote(d, key);
    if (columns.empty()) {
        switch (d) {
        case Dialect::Mysql: return "INSERT INTO " + t + " () VALUES ()";
        case Dialect::Sqlite: return "INSERT INTO " + t + " DEFAULT VALUES";
        case Dialect::Postgres: return "INSERT INTO " + t + " DEFAULT VALUES RETURNING " + k;
        default: return "INSERT INTO " + t + " OUTPUT INSERTED." + k + " DEFAULT VALUES";
        }
    }
    std::vector<std::string> names, marks;
    for (size_t i = 0; i < columns.size(); ++i) {
        names.push_back(quote(d, columns[i]));
        marks.push_back(mark(d, int(i) + 1));
    }
    std::string list = joined(names), values = joined(marks);
    switch (d) {
    case Dialect::Postgres: return "INSERT INTO " + t + " (" + list + ") VALUES (" + values + ") RETURNING " + k;
    case Dialect::Mssql: return "INSERT INTO " + t + " (" + list + ") OUTPUT INSERTED." + k + " VALUES (" + values + ")";
    default: return "INSERT INTO " + t + " (" + list + ") VALUES (" + values + ")";
    }
}

std::string begin(Dialect d) {
    switch (d) {
    case Dialect::Mysql: return "START TRANSACTION";
    case Dialect::Mssql: return "BEGIN TRANSACTION";
    default: return "BEGIN";
    }
}

std::string commit(Dialect d) { return d == Dialect::Mssql ? "COMMIT TRANSACTION" : "COMMIT"; }
std::string rollback(Dialect d) { return d == Dialect::Mssql ? "ROLLBACK TRANSACTION" : "ROLLBACK"; }

std::string version_table(Dialect d) {
    if (d == Dialect::Mssql) return "IF OBJECT_ID(N'clarity_schema', N'U') IS NULL CREATE TABLE clarity_schema (version INT NOT NULL)";
    return "CREATE TABLE IF NOT EXISTS clarity_schema (version INTEGER NOT NULL)";
}

}  // namespace clarity
