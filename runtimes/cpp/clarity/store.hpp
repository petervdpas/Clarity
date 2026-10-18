// The connection to one database, behind the same few calls for every
// kind: run a statement, insert and get the new key, read rows, and apply a
// migration step. SQLite comes first; the server databases follow.

#pragma once

#include <memory>
#include <string>
#include <vector>

#include "clarity/dialect.hpp"
#include "clarity/value.hpp"

namespace clarity {

using Row = std::vector<SqlValue>;

class Driver {
public:
    virtual ~Driver() = default;
    virtual Dialect dialect() const = 0;
    /// Runs a statement; gives the number of rows it changed.
    virtual int64_t execute(const std::string& sql, const std::vector<SqlValue>& params) = 0;
    /// Runs an INSERT from `insert` and gives the new key.
    virtual int64_t insert(const std::string& sql, const std::vector<SqlValue>& params) = 0;
    virtual std::vector<Row> rows(const std::string& sql, const std::vector<SqlValue>& params) = 0;
    /// Runs SQL without parameters, such as a statement that creates a table.
    virtual void batch(const std::string& sql) = 0;
    /// Runs the statements of migration step `version` and records it, all or nothing.
    virtual void apply(int version, const std::vector<std::string>& statements) = 0;
};

/// The statement that records migration step `version`.
std::string applied(int version);
/// Where a database or store is: as written, or from the environment
/// variable `env:NAME` names, so passwords stay out of the program.
std::string location(const std::string& text);
/// Connects to the database of a data source.
std::unique_ptr<Driver> open_driver(Dialect dialect, const std::string& at);
std::unique_ptr<Driver> open_sqlite(const std::string& path);

}  // namespace clarity
