#include "clarity/db.hpp"

#include "clarity/backup.hpp"

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
    // A database that already has a schema is backed up before it changes (docs/BACKUP-PLAN.md).
    if (done > 0 && done < int(steps.size())) backup::before_migration(d, done + 1);
    for (int i = done; i < int(steps.size()); ++i) d.apply(i + 1, steps[size_t(i)]);
    return std::max(0, int(steps.size()) - done);
}

Text saved_since() { return Text::keyed("savedSince", "Someone else changed this record since you opened it. Reload it to see their change."); }
Text undo_changed_since() { return Text::keyed("undoChangedSince", "This was changed since, so it cannot be taken back"); }
Text undo_too_old() { return Text::keyed("undoTooOld", "This was done too long ago to take back"); }

std::string Db::select_columns(Dialect d, const std::string& key, const std::vector<std::string>& columns, const std::vector<PictureColumn>& pictures,
                               const std::string& version) {
    std::string out = quote(d, key);
    for (const auto& c : columns) {
        bool in_database = false;
        for (const auto& p : pictures)
            if (p.column == c && !p.store) in_database = true;
        // Whether there is a picture, not the picture.
        out += ", " + (in_database ? presence(d, c) : quote(d, c));
    }
    // The version last, read with every record.
    if (!version.empty()) out += ", " + quote(d, version);
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
                         const std::vector<PictureColumn>& pictures, std::vector<SqlValue> values, int64_t id, const std::string& version, int64_t* held,
                         bool anyway) {
    // A person's secret that a save changes signs them out elsewhere (people.hpp, step 4).
    std::vector<std::string> secrets = people_ && id != 0 ? people_->secret_columns(source, table) : std::vector<std::string>{};
    if (!secrets.empty()) {
        auto kept = row(source, table, key, secrets, id);
        int64_t stored = recorded(source, table, key, columns, pictures, std::move(values), id, version, held, anyway);
        if (kept && row(source, table, key, secrets, stored) != kept) people_->secret_changed(*this, stored);
        return stored;
    }
    return recorded(source, table, key, columns, pictures, std::move(values), id, version, held, anyway);
}

int64_t Db::recorded(const std::string& source, const std::string& table, const std::string& key, const std::vector<std::string>& columns,
                     const std::vector<PictureColumn>& pictures, std::vector<SqlValue> values, int64_t id, const std::string& version, int64_t* held,
                     bool anyway) {
    enlist(source);
    if (!recording_) return store_row(source, table, key, columns, pictures, std::move(values), id, version, held, anyway);
    Change change{source, table, key, columns, version, 0, std::nullopt, std::nullopt, std::nullopt, std::nullopt};
    if (id != 0) std::tie(change.before, change.before_version) = state(source, table, key, columns, version, id);
    change.id = store_row(source, table, key, columns, pictures, std::move(values), id, version, held, anyway);
    std::tie(change.after, change.after_version) = state(source, table, key, columns, version, change.id);
    int64_t stored = change.id;
    recording_->push_back(std::move(change));
    return stored;
}

void Db::enlist(const std::string& source) {
    if (!recording_) return;
    SourceDb& s = at(source);
    if (std::find(held_.begin(), held_.end(), s.name) != held_.end()) return;
    if (s.depth == 0) s.driver->batch(begin(s.driver->dialect()));
    ++s.depth;
    held_.push_back(s.name);
}

void Db::release(bool keep) {
    std::exception_ptr failed;
    for (const auto& name : held_) {
        SourceDb& s = at(name);
        if (--s.depth > 0) continue;
        Dialect d = s.driver->dialect();
        if (keep) {
            try {
                s.driver->batch(commit(d));
                continue;
            } catch (...) {
                // When one cannot be kept, the ones not kept yet are rolled back too.
                failed = std::current_exception();
                keep = false;
            }
        }
        try {
            s.driver->batch(rollback(d));
        } catch (...) {
        }
    }
    held_.clear();
    if (failed) std::rethrow_exception(failed);
}

int64_t Db::store_row(const std::string& source, const std::string& table, const std::string& key, const std::vector<std::string>& columns,
                      const std::vector<PictureColumn>& pictures, std::vector<SqlValue> values, int64_t id, const std::string& version, int64_t* held,
                      bool anyway) {
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
        // A new record starts at version 1.
        if (held) {
            names.push_back(version);
            params.emplace_back(int64_t{1});
        }
        int64_t made = unfit([&] { return d.insert(insert(dialect, table, key, names), params); });
        if (held) *held = 1;
        return made;
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
    // Each save adds one to the version; a checked one writes only over the version it read.
    if (held) sets += (sets.empty() ? "" : ", ") + quote(dialect, version) + " = " + quote(dialect, version) + " + 1";
    if (sets.empty()) {
        if (!exists()) throw NotFoundError(table, id);
        return id;
    }
    params.emplace_back(id);
    std::string sql = "UPDATE " + quote(dialect, table) + " SET " + sets + " WHERE " + quote(dialect, key) + " = " + mark(dialect, int(params.size()));
    bool checked = held && !anyway;
    if (checked) {
        params.emplace_back(*held);
        sql += " AND " + quote(dialect, version) + " = " + mark(dialect, int(params.size()));
    }
    // MySQL counts only the rows whose values changed, so an update that
    // changes nothing is told apart from a missing record (one with a
    // version always changes).
    if (unfit([&] { return d.execute(sql, params); }) == 0) {
        if (!exists()) throw NotFoundError(table, id);
        if (checked) throw SavedSinceError();
    }
    if (held && anyway) *held = state(source, table, key, {}, version, id).second.value_or(0);
    else if (held) ++*held;
    return id;
}

void Db::remove_record(const std::string& source, const std::string& table, const std::string& key, const std::vector<std::string>& columns,
                       const std::string& version, int64_t held, int64_t id) {
    enlist(source);
    if (!recording_) return remove_row(source, table, key, version, held, id);
    Change change{source, table, key, columns, version, id, std::nullopt, std::nullopt, std::nullopt, std::nullopt};
    std::tie(change.before, change.before_version) = state(source, table, key, columns, version, id);
    remove_row(source, table, key, version, held, id);
    recording_->push_back(std::move(change));
}

void Db::remove_row(const std::string& source, const std::string& table, const std::string& key, const std::string& version, int64_t held, int64_t id) {
    Driver& d = *at(source).driver;
    Dialect dialect = d.dialect();
    std::string sql = "DELETE FROM " + quote(dialect, table) + " WHERE " + quote(dialect, key) + " = " + mark(dialect, 1);
    std::vector<SqlValue> params{id};
    // A record someone saved since it was read is not deleted.
    if (!version.empty()) {
        sql += " AND " + quote(dialect, version) + " = " + mark(dialect, 2);
        params.emplace_back(held);
    }
    int64_t changed = 0;
    try {
        changed = d.execute(sql, params);
    } catch (const ReferenceFailure&) {
        throw InUseError(table, id);
    }
    if (changed == 0 && !version.empty() && state(source, table, key, {}, "", id).first) throw SavedSinceError();
}

bool Db::replay_steps(const std::vector<int64_t>& ids, bool back) {
    std::vector<Done*> taken;
    for (int64_t id : ids) {
        auto found = steps_.find(id);
        // A step never made is nothing to replay; one made and no longer kept is refused.
        if (found == steps_.end()) {
            if (id != 0 && id <= last_step_) refuse(undo_too_old());
            continue;
        }
        if (found->second.undone != back) taken.push_back(&found->second);
    }
    if (taken.empty()) return false;
    // The changes in the order they are replayed: the steps as given, each one's changes backwards for an undo.
    std::vector<Change*> order;
    for (Done* done : taken) {
        auto& changes = done->changes;
        for (size_t i = 0; i < changes.size(); ++i) order.push_back(&changes[back ? changes.size() - 1 - i : i]);
    }
    replay(order, back);
    for (Done* done : taken) done->undone = back;
    return true;
}

void Db::replay(std::vector<Change*>& order, bool back) {
    std::vector<std::string> sources;
    for (const Change* c : order)
        if (std::find(sources.begin(), sources.end(), c->source) == sources.end()) sources.push_back(c->source);
    within(sources, 0, [&] {
        // Each row as the steps left it (its last change undone, its first redone) must still be so.
        std::set<std::tuple<std::string, std::string, int64_t>> seen;
        for (const Change* c : order) {
            if (!seen.insert({c->source, c->table, c->id}).second) continue;
            auto [now, version] = state(c->source, c->table, c->key, c->columns, c->version, c->id);
            bool unchanged = c->version.empty() ? now == (back ? c->after : c->before) : version == (back ? c->after_version : c->before_version);
            if (!unchanged) refuse(undo_changed_since());
        }
        for (const Change* c : order) put(*c, back ? c->before : c->after, back ? c->before_version : c->after_version);
        // The version each row has now is what the next replay finds: noted on its last change in this order.
        std::set<std::tuple<std::string, std::string, int64_t>> noted;
        for (auto c = order.rbegin(); c != order.rend(); ++c) {
            if ((*c)->version.empty() || !noted.insert({(*c)->source, (*c)->table, (*c)->id}).second) continue;
            auto version = state((*c)->source, (*c)->table, (*c)->key, (*c)->columns, (*c)->version, (*c)->id).second;
            if (back) (*c)->before_version = version;
            else (*c)->after_version = version;
        }
    });
}

void Db::within(const std::vector<std::string>& sources, size_t from, const std::function<void()>& run) {
    if (from == sources.size()) return run();
    atomic(sources[from], [&] { within(sources, from + 1, run); });
}

std::pair<std::optional<Row>, std::optional<int64_t>> Db::state(const std::string& source, const std::string& table, const std::string& key,
                                                                 const std::vector<std::string>& columns, const std::string& version, int64_t id) {
    Driver& d = *at(source).driver;
    Dialect dialect = d.dialect();
    std::string names = quote(dialect, key);
    for (const auto& c : columns) names += ", " + quote(dialect, c);
    if (!version.empty()) names += ", " + quote(dialect, version);
    auto rows = d.rows("SELECT " + names + " FROM " + quote(dialect, table) + " WHERE " + quote(dialect, key) + " = " + mark(dialect, 1), {id});
    if (rows.empty()) return {std::nullopt, std::nullopt};
    Row row = rows[0];
    row.erase(row.begin());
    std::optional<int64_t> stored;
    if (!version.empty()) {
        stored = std::get<int64_t>(row.back());
        row.pop_back();
    }
    return {row, stored};
}

void Db::put(const Change& c, const std::optional<Row>& values, std::optional<int64_t> version) {
    auto [now, current] = state(c.source, c.table, c.key, c.columns, c.version, c.id);
    int64_t next = std::max(current.value_or(0), version.value_or(0)) + 1;
    Driver& d = *at(c.source).driver;
    Dialect dialect = d.dialect();
    std::string table = quote(dialect, c.table), key = quote(dialect, c.key);
    if (!values) {
        if (now) d.execute("DELETE FROM " + table + " WHERE " + key + " = " + mark(dialect, 1), {c.id});
        return;
    }
    if (now) {
        std::string sets;
        std::vector<SqlValue> params = *values;
        for (size_t i = 0; i < c.columns.size(); ++i) sets += (i ? ", " : "") + quote(dialect, c.columns[i]) + " = " + mark(dialect, int(i + 1));
        if (!c.version.empty()) {
            params.emplace_back(next);
            sets += (sets.empty() ? "" : ", ") + quote(dialect, c.version) + " = " + mark(dialect, int(params.size()));
        }
        if (sets.empty()) return;
        params.emplace_back(c.id);
        d.execute("UPDATE " + table + " SET " + sets + " WHERE " + key + " = " + mark(dialect, int(params.size())), params);
        return;
    }
    std::string names = key;
    std::vector<SqlValue> params{c.id};
    params.insert(params.end(), values->begin(), values->end());
    for (const auto& column : c.columns) names += ", " + quote(dialect, column);
    if (!c.version.empty()) {
        names += ", " + quote(dialect, c.version);
        params.emplace_back(next);
    }
    std::string marks;
    for (size_t i = 0; i < params.size(); ++i) marks += (i ? ", " : "") + mark(dialect, int(i + 1));
    std::string insert = "INSERT INTO " + table + " (" + names + ") VALUES (" + marks + ")";
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

void check_required(const std::vector<Required>& required, const std::vector<std::string>& columns, const std::vector<SqlValue>& values) {
    auto empty = [](const SqlValue& v) {
        if (std::holds_alternative<std::monostate>(v)) return true;
        auto text = std::get_if<std::string>(&v);
        return text && text->empty();
    };
    for (const auto& field : required) {
        for (size_t i = 0; i < columns.size() && i < values.size(); ++i) {
            if (columns[i] != field.column || !empty(values[i])) continue;
            // A label written as @key takes the key's value; a plain one is itself.
            std::string label = field.label.starts_with("@") ? text_of(field.label.substr(1)) : field.label;
            refuse(Text::key("fieldRequired").with("field", slot_text(label)));
        }
    }
}

}  // namespace clarity
