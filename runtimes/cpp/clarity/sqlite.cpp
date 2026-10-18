// SQLite through its own C library: one file per data source. Decimals and
// dates are stored as text, bools as 0 and 1. One connection holds the
// transaction of a save with its data events.

#include <filesystem>

#include <sqlite3.h>

#include "clarity/errors.hpp"
#include "clarity/store.hpp"

namespace clarity {

namespace {

class Statement {
public:
    Statement(sqlite3* db, const std::string& sql);
    ~Statement() { sqlite3_finalize(stmt_); }
    Statement(const Statement&) = delete;
    Statement& operator=(const Statement&) = delete;
    void bind(const std::vector<SqlValue>& params);
    /// Steps once; false at the end.
    bool step();
    Row row() const;

private:
    sqlite3* db_;
    sqlite3_stmt* stmt_ = nullptr;
};

[[noreturn]] void failed(sqlite3* db) {
    int code = sqlite3_extended_errcode(db);
    std::string what = sqlite3_errmsg(db);
    if (code == SQLITE_CONSTRAINT_FOREIGNKEY) throw ReferenceFailure(what);
    throw StorageError(what);
}

Statement::Statement(sqlite3* db, const std::string& sql) : db_(db) {
    if (sqlite3_prepare_v2(db, sql.c_str(), int(sql.size()), &stmt_, nullptr) != SQLITE_OK) failed(db);
}

void Statement::bind(const std::vector<SqlValue>& params) {
    for (size_t i = 0; i < params.size(); ++i) {
        int at = int(i) + 1;
        int rc = std::visit(
            [&](const auto& v) -> int {
                using T = std::decay_t<decltype(v)>;
                if constexpr (std::is_same_v<T, std::monostate> || std::is_same_v<T, Kept>) return sqlite3_bind_null(stmt_, at);
                else if constexpr (std::is_same_v<T, int64_t>) return sqlite3_bind_int64(stmt_, at, v);
                else if constexpr (std::is_same_v<T, double>) return sqlite3_bind_double(stmt_, at, v);
                else if constexpr (std::is_same_v<T, bool>) return sqlite3_bind_int64(stmt_, at, v ? 1 : 0);
                else if constexpr (std::is_same_v<T, std::string>) return sqlite3_bind_text(stmt_, at, v.c_str(), int(v.size()), SQLITE_TRANSIENT);
                else if constexpr (std::is_same_v<T, Bytes>) return sqlite3_bind_blob(stmt_, at, v.data(), int(v.size()), SQLITE_TRANSIENT);
                else {
                    std::string text = v.str();
                    return sqlite3_bind_text(stmt_, at, text.c_str(), int(text.size()), SQLITE_TRANSIENT);
                }
            },
            params[i]);
        if (rc != SQLITE_OK) failed(db_);
    }
}

bool Statement::step() {
    int rc = sqlite3_step(stmt_);
    if (rc == SQLITE_ROW) return true;
    if (rc == SQLITE_DONE) return false;
    failed(db_);
}

Row Statement::row() const {
    Row out;
    for (int i = 0, n = sqlite3_column_count(stmt_); i < n; ++i) {
        switch (sqlite3_column_type(stmt_, i)) {
        case SQLITE_INTEGER: out.emplace_back(int64_t(sqlite3_column_int64(stmt_, i))); break;
        case SQLITE_FLOAT: out.emplace_back(sqlite3_column_double(stmt_, i)); break;
        case SQLITE_TEXT:
            out.emplace_back(std::string(reinterpret_cast<const char*>(sqlite3_column_text(stmt_, i)), size_t(sqlite3_column_bytes(stmt_, i))));
            break;
        case SQLITE_BLOB: {
            auto data = static_cast<const uint8_t*>(sqlite3_column_blob(stmt_, i));
            out.emplace_back(Bytes(data, data + sqlite3_column_bytes(stmt_, i)));
            break;
        }
        default: out.emplace_back(std::monostate{});
        }
    }
    return out;
}

class Sqlite final : public Driver {
public:
    explicit Sqlite(sqlite3* db) : db_(db) {}
    ~Sqlite() override { sqlite3_close(db_); }
    Dialect dialect() const override { return Dialect::Sqlite; }

    int64_t execute(const std::string& sql, const std::vector<SqlValue>& params) override {
        Statement s(db_, sql);
        s.bind(params);
        while (s.step()) {
        }
        return sqlite3_changes64(db_);
    }

    int64_t insert(const std::string& sql, const std::vector<SqlValue>& params) override {
        execute(sql, params);
        return sqlite3_last_insert_rowid(db_);
    }

    std::vector<Row> rows(const std::string& sql, const std::vector<SqlValue>& params) override {
        Statement s(db_, sql);
        s.bind(params);
        std::vector<Row> out;
        while (s.step()) out.push_back(s.row());
        return out;
    }

    void batch(const std::string& sql) override {
        if (sqlite3_exec(db_, sql.c_str(), nullptr, nullptr, nullptr) != SQLITE_OK) failed(db_);
    }

    // A step runs with references unchecked, since it may build a table again
    // that others refer to; they are checked before the step is kept.
    void apply(int version, const std::vector<std::string>& statements) override {
        batch("PRAGMA foreign_keys = OFF");
        try {
            batch("BEGIN");
            for (const auto& statement : statements) batch(statement);
            if (!rows("SELECT 1 FROM pragma_foreign_key_check", {}).empty())
                throw StorageError("migration step leaves references to records that do not exist");
            batch(applied(version));
            batch("COMMIT");
        } catch (...) {
            sqlite3_exec(db_, "ROLLBACK", nullptr, nullptr, nullptr);
            sqlite3_exec(db_, "PRAGMA foreign_keys = ON", nullptr, nullptr, nullptr);
            throw;
        }
        batch("PRAGMA foreign_keys = ON");
    }

private:
    sqlite3* db_;
};

}  // namespace

std::unique_ptr<Driver> open_sqlite(const std::string& path) {
    if (path != ":memory:") {
        std::filesystem::path folder = std::filesystem::path(path).parent_path();
        std::error_code ignored;
        if (!folder.empty()) std::filesystem::create_directories(folder, ignored);
    }
    sqlite3* db = nullptr;
    if (sqlite3_open(path.c_str(), &db) != SQLITE_OK) {
        std::string what = db ? sqlite3_errmsg(db) : "out of memory";
        sqlite3_close(db);
        throw StorageError("cannot open " + path + ": " + what);
    }
    sqlite3_extended_result_codes(db, 1);
    auto driver = std::make_unique<Sqlite>(db);
    driver->batch("PRAGMA foreign_keys = ON");
    return driver;
}

}  // namespace clarity
