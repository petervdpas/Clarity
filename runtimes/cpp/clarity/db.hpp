// The databases of an application: one per data source. Records are saved,
// deleted and read through here, with the entity's data events around a
// save or delete, and migration steps are applied here at startup.
//
// A record is a plain struct with its `id`, the `table`, `columns()`,
// `values()` and `from_values()` of its entity, and, when the entity has
// them, its `source`, `key`, `pictures()` and data events (`before_save`,
// `after_save`, `before_delete`, `after_delete`).

#pragma once

#include <concepts>
#include <filesystem>
#include <map>
#include <optional>
#include <string>
#include <vector>

#include "clarity/errors.hpp"
#include "clarity/record_file.hpp"
#include "clarity/picture.hpp"
#include "clarity/query.hpp"
#include "clarity/store.hpp"

namespace clarity {

class Db;

template <typename R>
concept Record = requires(R r, const R c, int64_t id, const Row& values) {
    { std::string(R::table) };
    { R::columns() } -> std::convertible_to<const std::vector<std::string>&>;
    { c.values() } -> std::same_as<std::vector<SqlValue>>;
    { R::from_values(id, values) } -> std::same_as<R>;
    { r.id } -> std::convertible_to<int64_t>;
};

/// The database of one data source: its name, its kind and where it is (a
/// file for SQLite). `env:NAME` takes the location from that environment variable.
struct Source {
    std::string name;
    Dialect dialect;
    std::string at;
};

/// The statements of one migration step.
using Step = std::vector<std::string>;

/// A file of a file store.
struct FileEntry {
    std::string store;
    /// The path inside the store, with `/` between folders.
    std::string path;
    std::string name;
    /// In lower case, without the dot; empty without one.
    std::string extension;
    /// In bytes.
    int64_t size = 0;
    /// When the file last changed, in UTC to the second.
    std::optional<DateTime> modified;
};

namespace detail {

template <typename R>
std::string source_of() {
    if constexpr (requires { R::source; }) return R::source;
    else return "";
}

template <typename R>
std::string key_of() {
    if constexpr (requires { R::key; }) return R::key;
    else return "id";
}

template <typename R>
std::vector<PictureColumn> pictures_of() {
    if constexpr (requires { R::pictures(); }) return R::pictures();
    else return {};
}

template <typename R>
constexpr bool has_events = requires(R r, Db& db) { r.before_save(db); } || requires(R r, Db& db) { r.after_save(db); } ||
                            requires(R r, Db& db) { r.before_delete(db); } || requires(R r, Db& db) { r.after_delete(db); };

}  // namespace detail

class Db {
public:
    /// A database per data source.
    explicit Db(const std::vector<Source>& sources);
    /// One SQLite database, used for every record.
    static Db open(const std::string& path);
    /// A SQLite database that lives in memory, for tests.
    static Db memory();

    Db(Db&&) noexcept = default;
    Db& operator=(Db&&) noexcept = default;

    /// Runs SQL without parameters in the database of `source`, for tools and tests.
    void batch(const std::string& source, const std::string& sql);
    Dialect dialect(const std::string& source);
    /// The number of migration steps applied so far to the default database.
    int schema_version();
    int migrate(const std::vector<Step>& steps) { return migrate_source("", steps); }
    /// Applies the steps of `source` its database has not had yet, each on its own.
    int migrate_source(const std::string& source, const std::vector<Step>& steps);

    /// Inserts a new record (and gives it its id) or updates a stored one,
    /// with the entity's data events around it.
    template <Record R>
    void save(R& record) {
        if constexpr (!detail::has_events<R>) {
            write(record);
        } else {
            int64_t id = record.id;
            try {
                atomic(detail::source_of<R>(), [&] {
                    if constexpr (requires { record.before_save(*this); }) record.before_save(*this);
                    write(record);
                    if constexpr (requires { record.after_save(*this); }) record.after_save(*this);
                });
            } catch (...) {
                // Nothing was kept, so a new record is still new.
                record.id = id;
                throw;
            }
        }
    }

    /// Removes a stored record, with the entity's data events around it.
    template <Record R>
    void remove(const R& record) {
        if constexpr (!detail::has_events<R>) {
            remove_record(detail::source_of<R>(), R::table, detail::key_of<R>(), record.id);
        } else {
            R copy = record;
            atomic(detail::source_of<R>(), [&] {
                if constexpr (requires { copy.before_delete(*this); }) copy.before_delete(*this);
                remove_record(detail::source_of<R>(), R::table, detail::key_of<R>(), copy.id);
                if constexpr (requires { copy.after_delete(*this); }) copy.after_delete(*this);
            });
        }
    }

    /// The records a query finds, in its order.
    template <Record R>
    std::vector<R> find(const Query& query) {
        std::string source = detail::source_of<R>();
        Dialect d = dialect(source);
        std::string columns = select_columns(d, detail::key_of<R>(), R::columns(), detail::pictures_of<R>());
        auto [sql, params] = query.sql(d, R::table, detail::key_of<R>(), columns);
        return records<R>(source, sql, params);
    }

    /// Every stored record of an entity, in the order they were stored.
    template <Record R>
    std::vector<R> all() {
        return find<R>(Query());
    }

    template <Record R>
    R get(int64_t id) {
        std::string source = detail::source_of<R>();
        Dialect d = dialect(source);
        std::string key = detail::key_of<R>();
        std::string sql = "SELECT " + select_columns(d, key, R::columns(), detail::pictures_of<R>()) + " FROM " + quote(d, R::table) + " WHERE " + quote(d, key) + " = " + mark(d, 1);
        auto found = records<R>(source, sql, {id});
        if (found.empty()) throw NotFoundError(R::table, id);
        return found.back();
    }

    /// The record a reference field points at.
    template <Record R>
    R load(const std::optional<int64_t>& id) {
        if (!id) throw NotFoundError(R::table, std::nullopt);
        return get<R>(*id);
    }

    /// Adds the file store `name` at a folder (made when it is not there
    /// yet); `env:NAME` takes it from that environment variable.
    void add_store(const std::string& name, const std::string& at);

    /// The picture of field `column` of record `id`, read from the database
    /// or from its file store; none when the record has none.
    template <Record R>
    std::optional<Bytes> picture(int64_t id, const std::string& column) {
        for (const auto& p : detail::pictures_of<R>())
            if (p.column == column) return picture_bytes(detail::source_of<R>(), R::table, detail::key_of<R>(), p, id);
        throw StorageError(std::string(R::table) + " has no picture '" + column + "'");
    }

    /// Every file of a store and its folders, but those whose name or
    /// folder starts with a dot, sorted by path.
    std::vector<FileEntry> files(const std::string& store);
    /// The text a file holds.
    std::string read_text(const FileEntry& file);
    /// Writes `text` as the file `path` of a store, making its folders.
    void write_text(const std::string& store, const std::string& path, const std::string& text);
    /// The contents of a file as they are.
    Bytes read_bytes(const FileEntry& file);
    /// Writes `bytes` as they are as the file `path` of a store, making its folders.
    void write_bytes(const std::string& store, const std::string& path, const Bytes& bytes);
    /// A file as the picture of a field kept in `field_store` (none: in the
    /// database): the file itself when it is in that store, else a copy.
    Picture picture_from(const FileEntry& file, const std::optional<std::string>& field_store);
    /// A file of a store as the file of a file field, under its name: linked
    /// when it is in the field's own store, else copied.
    RecordFile record_file_from(const FileEntry& file, const std::optional<std::string>& field_store);
    /// The bytes of the file `path` of a store: a picture linked from a file
    /// before its record is saved.
    Bytes store_file(const std::string& store, const std::string& path);

private:
    struct SourceDb {
        std::string name;
        std::unique_ptr<Driver> driver;
        /// How many saves and deletes with data events are running in it;
        /// the outermost one holds the transaction.
        int depth = 0;
    };
    std::vector<SourceDb> sources_;
    std::map<std::string, std::filesystem::path> stores_;

    Db() = default;
    SourceDb& at(const std::string& source);
    const std::filesystem::path& store_folder(const std::string& name);
    Bytes read_file(const std::string& store, const std::string& path);

    /// Runs `run` in a transaction of `source`, or in the one that is open
    /// already: a save that an event of another save makes is part of it.
    template <typename F>
    void atomic(const std::string& source, F run) {
        SourceDb& s = at(source);
        bool outer = s.depth == 0;
        Dialect d = s.driver->dialect();
        if (outer) s.driver->batch(begin(d));
        ++s.depth;
        try {
            run();
        } catch (...) {
            --s.depth;
            // The error that made it roll back is the one to report.
            if (outer) {
                try {
                    s.driver->batch(rollback(d));
                } catch (...) {
                }
            }
            throw;
        }
        --s.depth;
        if (outer) s.driver->batch(commit(d));
    }

    template <Record R>
    void write(R& record) {
        record.id = store_record(detail::source_of<R>(), R::table, detail::key_of<R>(), R::columns(), detail::pictures_of<R>(), record.values(), record.id);
    }

    template <Record R>
    std::vector<R> records(const std::string& source, const std::string& sql, const std::vector<SqlValue>& params) {
        std::vector<R> out;
        for (auto& row : select(source, sql, params)) {
            int64_t id = std::get<int64_t>(row.at(0));
            row.erase(row.begin());
            out.push_back(R::from_values(id, row));
        }
        return out;
    }

    int64_t store_record(const std::string& source, const std::string& table, const std::string& key, const std::vector<std::string>& columns,
                         const std::vector<PictureColumn>& pictures, std::vector<SqlValue> values, int64_t id);
    void remove_record(const std::string& source, const std::string& table, const std::string& key, int64_t id);
    std::vector<Row> select(const std::string& source, const std::string& sql, const std::vector<SqlValue>& params);
    static std::string select_columns(Dialect d, const std::string& key, const std::vector<std::string>& columns, const std::vector<PictureColumn>& pictures);
    std::optional<Bytes> picture_bytes(const std::string& source, const std::string& table, const std::string& key, const PictureColumn& picture, int64_t id);
};

}  // namespace clarity
