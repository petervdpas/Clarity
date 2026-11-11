// The connection to one database, behind the same few calls for every
// kind: run a statement, insert and get the new key, read rows, and apply a
// migration step. SQLite comes first; the server databases follow.

#pragma once

#include <functional>
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

/// The environment variable that names the folder a program's SQLite databases open in.
inline constexpr const char* data_variable = "CLARITY_DATA";
/// The file of an SQLite database at `location`, with `env` reading the
/// environment (the process's when empty): in the folder CLARITY_DATA
/// names, under its own file name, or where the project put it when the
/// variable is unset or empty. A location from the environment (`env:`) or
/// a secret setting (`setting:`) is as it says, and so is SQLite's own
/// `:memory:`. The same as the Rust and C# runtimes.
std::string data_folder_sqlite(const std::string& location, const std::function<const char*(const char*)>& env = {});
// data_folder_sqlite on system `os` ("windows", "macos" or another): a
// data:<program>/<file> location opens in CLARITY_DATA's folder, else in the
// person's folder for application data (conformance/data-folder.json).
std::string data_folder_sqlite_on(const std::string& os, const std::string& location, const std::function<const char*(const char*)>& env = {});
/// Connects to the database of a data source.
std::unique_ptr<Driver> open_driver(Dialect dialect, const std::string& at);
std::unique_ptr<Driver> open_sqlite(const std::string& path);
/// Whether a connection open_sqlite made is outside a transaction.
bool sqlite_autocommit(const Driver& driver);

}  // namespace clarity
