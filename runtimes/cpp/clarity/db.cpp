#include "clarity/db.hpp"

#include <algorithm>
#include <fstream>
#include <set>
#include <tuple>

namespace clarity {

Db::Db(const std::vector<Source>& sources) {
    for (const auto& s : sources) sources_.push_back({s.name, open_driver(s.dialect, s.at), 0});
}

Db Db::open(const std::string& path) {
    Db db;
    db.sources_.push_back({"", open_sqlite(path), 0});
    return db;
}

Db Db::memory() { return open(":memory:"); }

Db::SourceDb& Db::at(const std::string& source) {
    bool single = sources_.size() == 1;
    for (auto& s : sources_)
        if (single || source.empty() || s.name == source) return s;
    throw StorageError("there is no data source '" + source + "'");
}

void Db::batch(const std::string& source, const std::string& sql) { at(source).driver->batch(sql); }
Dialect Db::dialect(const std::string& source) { return at(source).driver->dialect(); }

static int version_of(Driver& d) {
    d.batch(version_table(d.dialect()));
    auto rows = d.rows("SELECT MAX(version) FROM clarity_schema", {});
    if (rows.empty() || std::holds_alternative<std::monostate>(rows[0][0])) return 0;
    return int(std::get<int64_t>(rows[0][0]));
}

int Db::schema_version() { return version_of(*at("").driver); }

int Db::migrate_source(const std::string& source, const std::vector<Step>& steps) {
    Driver& d = *at(source).driver;
    int done = version_of(d);
    for (int i = done; i < int(steps.size()); ++i) d.apply(i + 1, steps[size_t(i)]);
    return std::max(0, int(steps.size()) - done);
}

std::string Db::select_columns(Dialect d, const std::string& key, const std::vector<std::string>& columns, const std::vector<PictureColumn>& pictures) {
    std::string out = quote(d, key);
    for (const auto& c : columns) {
        bool in_database = false;
        for (const auto& p : pictures)
            if (p.column == c && !p.store) in_database = true;
        // Whether there is a picture, not the picture.
        out += ", " + (in_database ? presence(d, c) : quote(d, c));
    }
    return out;
}

std::vector<Row> Db::select(const std::string& source, const std::string& sql, const std::vector<SqlValue>& params) {
    return at(source).driver->rows(sql, params);
}

/// A failed save: a reference to a record that is not there reads better
/// than the database's own words.
template <typename F>
static auto unfit(F run) {
    try {
        return run();
    } catch (const ReferenceFailure& e) {
        throw InvalidError(std::string("a reference does not fit: ") + e.what());
    }
}

int64_t Db::store_record(const std::string& source, const std::string& table, const std::string& key, const std::vector<std::string>& columns,
                         const std::vector<PictureColumn>& pictures, std::vector<SqlValue> values, int64_t id) {
    if (!recording_) return store_row(source, table, key, columns, pictures, std::move(values), id);
    std::optional<Row> before;
    if (id != 0) before = row(source, table, key, columns, id);
    int64_t stored = store_row(source, table, key, columns, pictures, std::move(values), id);
    auto after = row(source, table, key, columns, stored);
    recording_->push_back({source, table, key, columns, stored, std::move(before), std::move(after)});
    return stored;
}

int64_t Db::store_row(const std::string& source, const std::string& table, const std::string& key, const std::vector<std::string>& columns,
                      const std::vector<PictureColumn>& pictures, std::vector<SqlValue> values, int64_t id) {
    // A new picture is checked, and written to its file store when it has
    // one, its column then holding the file's name.
    auto picture_of = [&](const std::string& column) -> const PictureColumn* {
        for (const auto& p : pictures)
            if (p.column == column) return &p;
        return nullptr;
    };
    for (size_t i = 0; i < columns.size(); ++i) {
        const PictureColumn* p = picture_of(columns[i]);
        auto bytes = std::get_if<Bytes>(&values[i]);
        if (!p || !bytes) continue;
        // A file of a file field is kept under its own name, in a folder named by its contents.
        if (p->name) {
            if (!p->store) continue;
            std::string named;
            for (size_t j = 0; j < columns.size(); ++j)
                if (columns[j] == *p->name)
                    if (auto text = std::get_if<std::string>(&values[j])) named = *text;
            std::string path = store_path(*bytes, named);
            auto file = in_store(store_folder(*p->store), path);
            if (!std::filesystem::exists(file)) {
                std::filesystem::create_directories(file.parent_path());
                std::ofstream out(file, std::ios::binary);
                out.write(reinterpret_cast<const char*>(bytes->data()), std::streamsize(bytes->size()));
                if (!out) throw StorageError("cannot write " + file.string());
            }
            values[i] = path;
            continue;
        }
        std::string name = file_name(*bytes);
        if (!p->store) continue;
        auto file = store_folder(*p->store) / name;
        if (!std::filesystem::exists(file)) {
            std::ofstream out(file, std::ios::binary);
            out.write(reinterpret_cast<const char*>(bytes->data()), std::streamsize(bytes->size()));
            if (!out) throw StorageError("cannot write " + file.string());
        }
        values[i] = name;
    }
    Driver& d = *at(source).driver;
    Dialect dialect = d.dialect();
    auto is_none = [](const SqlValue& v) { return std::holds_alternative<std::monostate>(v) || std::holds_alternative<Kept>(v); };
    if (id == 0) {
        // An empty or kept picture is left to the column's NULL, so no NULL
        // is bound to a binary column.
        std::vector<std::string> names;
        std::vector<SqlValue> params;
        for (size_t i = 0; i < columns.size(); ++i) {
            if (picture_of(columns[i]) && is_none(values[i])) continue;
            names.push_back(columns[i]);
            params.push_back(values[i]);
        }
        return unfit([&] { return d.insert(insert(dialect, table, key, names), params); });
    }
    // A kept picture stays as it is; an emptied one is set to NULL as text.
    std::string sets;
    std::vector<SqlValue> params;
    for (size_t i = 0; i < columns.size(); ++i) {
        if (std::holds_alternative<Kept>(values[i])) continue;
        if (!sets.empty()) sets += ", ";
        if (picture_of(columns[i]) && std::holds_alternative<std::monostate>(values[i])) {
            sets += quote(dialect, columns[i]) + " = NULL";
            continue;
        }
        params.push_back(values[i]);
        sets += quote(dialect, columns[i]) + " = " + mark(dialect, int(params.size()));
    }
    auto exists = [&] {
        return !d.rows("SELECT " + quote(dialect, key) + " FROM " + quote(dialect, table) + " WHERE " + quote(dialect, key) + " = " + mark(dialect, 1), {id}).empty();
    };
    if (sets.empty()) {
        if (!exists()) throw NotFoundError(table, id);
        return id;
    }
    params.emplace_back(id);
    std::string sql = "UPDATE " + quote(dialect, table) + " SET " + sets + " WHERE " + quote(dialect, key) + " = " + mark(dialect, int(params.size()));
    // MySQL counts only the rows whose values changed, so an update that
    // changes nothing is told apart from a missing record.
    if (unfit([&] { return d.execute(sql, params); }) == 0 && !exists()) throw NotFoundError(table, id);
    return id;
}

void Db::remove_record(const std::string& source, const std::string& table, const std::string& key, const std::vector<std::string>& columns, int64_t id) {
    if (!recording_) return remove_row(source, table, key, id);
    auto before = row(source, table, key, columns, id);
    remove_row(source, table, key, id);
    recording_->push_back({source, table, key, columns, id, std::move(before), std::nullopt});
}

void Db::remove_row(const std::string& source, const std::string& table, const std::string& key, int64_t id) {
    Driver& d = *at(source).driver;
    Dialect dialect = d.dialect();
    try {
        d.execute("DELETE FROM " + quote(dialect, table) + " WHERE " + quote(dialect, key) + " = " + mark(dialect, 1), {id});
    } catch (const ReferenceFailure&) {
        throw InUseError(table, id);
    }
}

void Db::put_back(const std::vector<Change>& changes) {
    for (auto c = changes.rbegin(); c != changes.rend(); ++c) {
        try {
            put(*c, c->before);
        } catch (...) {
            // The error of the handler is the one to report.
        }
    }
}

bool Db::replay_step(int64_t id, bool back) {
    auto found = steps_.find(id);
    if (found == steps_.end() || found->second.undone == back) return false;
    std::vector<Change> order = found->second.changes;
    if (back) std::reverse(order.begin(), order.end());
    // Each row as the step left it (its last change undone, its first redone) must still be so.
    std::set<std::tuple<std::string, std::string, int64_t>> seen;
    std::vector<std::string> sources;
    for (const auto& c : order) {
        if (seen.insert({c.source, c.table, c.id}).second) {
            const auto& expected = back ? c.after : c.before;
            if (row(c.source, c.table, c.key, c.columns, c.id) != expected) refuse(Text::key("undoChangedSince"));
        }
        if (std::find(sources.begin(), sources.end(), c.source) == sources.end()) sources.push_back(c.source);
    }
    within(sources, 0, [&] {
        for (const auto& c : order) put(c, back ? c.before : c.after);
    });
    found->second.undone = back;
    return true;
}

void Db::within(const std::vector<std::string>& sources, size_t from, const std::function<void()>& run) {
    if (from == sources.size()) return run();
    atomic(sources[from], [&] { within(sources, from + 1, run); });
}

std::optional<Row> Db::row(const std::string& source, const std::string& table, const std::string& key, const std::vector<std::string>& columns, int64_t id) {
    Driver& d = *at(source).driver;
    Dialect dialect = d.dialect();
    std::string names;
    for (const auto& c : columns) names += (names.empty() ? "" : ", ") + quote(dialect, c);
    if (names.empty()) names = quote(dialect, key);
    auto rows = d.rows("SELECT " + names + " FROM " + quote(dialect, table) + " WHERE " + quote(dialect, key) + " = " + mark(dialect, 1), {id});
    if (rows.empty()) return std::nullopt;
    if (columns.empty()) return Row{};
    return rows[0];
}

void Db::put(const Change& c, const std::optional<Row>& values) {
    auto now = row(c.source, c.table, c.key, c.columns, c.id);
    Driver& d = *at(c.source).driver;
    Dialect dialect = d.dialect();
    std::string table = quote(dialect, c.table), key = quote(dialect, c.key);
    if (!values) {
        if (now) d.execute("DELETE FROM " + table + " WHERE " + key + " = " + mark(dialect, 1), {c.id});
        return;
    }
    if (now) {
        if (c.columns.empty()) return;
        std::string sets;
        for (size_t i = 0; i < c.columns.size(); ++i) sets += (i ? ", " : "") + quote(dialect, c.columns[i]) + " = " + mark(dialect, int(i + 1));
        std::vector<SqlValue> params = *values;
        params.emplace_back(c.id);
        d.execute("UPDATE " + table + " SET " + sets + " WHERE " + key + " = " + mark(dialect, int(c.columns.size() + 1)), params);
        return;
    }
    std::string names = key, marks = mark(dialect, 1);
    for (size_t i = 0; i < c.columns.size(); ++i) {
        names += ", " + quote(dialect, c.columns[i]);
        marks += ", " + mark(dialect, int(i + 2));
    }
    std::string insert = "INSERT INTO " + table + " (" + names + ") VALUES (" + marks + ")";
    std::vector<SqlValue> params{c.id};
    params.insert(params.end(), values->begin(), values->end());
    // SQL Server takes a key of its own counting only when asked to.
    if (dialect != Dialect::Mssql) {
        d.execute(insert, params);
        return;
    }
    d.batch("SET IDENTITY_INSERT " + table + " ON");
    try {
        d.execute(insert, params);
    } catch (...) {
        d.batch("SET IDENTITY_INSERT " + table + " OFF");
        throw;
    }
    d.batch("SET IDENTITY_INSERT " + table + " OFF");
}

std::optional<Bytes> Db::picture_bytes(const std::string& source, const std::string& table, const std::string& key, const PictureColumn& picture, int64_t id) {
    Driver& d = *at(source).driver;
    Dialect dialect = d.dialect();
    auto rows = d.rows("SELECT " + quote(dialect, picture.column) + " FROM " + quote(dialect, table) + " WHERE " + quote(dialect, key) + " = " + mark(dialect, 1), {id});
    if (rows.empty()) throw NotFoundError(table, id);
    const SqlValue& value = rows[0][0];
    if (std::holds_alternative<std::monostate>(value)) return std::nullopt;
    if (!picture.store) {
        if (auto bytes = std::get_if<Bytes>(&value)) return *bytes;
        throw StorageError(picture.column + " holds no picture");
    }
    std::string path = std::holds_alternative<Bytes>(value) ? std::string(std::get<Bytes>(value).begin(), std::get<Bytes>(value).end()) : std::get<std::string>(value);
    if (path.empty()) return std::nullopt;
    return read_file(*picture.store, path);
}

}  // namespace clarity
