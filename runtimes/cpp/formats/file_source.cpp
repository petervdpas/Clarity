#include "formats/file_source.hpp"

#include <cctype>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <map>
#include <sstream>

#include <nlohmann/json.hpp>

#include "formats/formats.hpp"

namespace clarity {

namespace {

namespace fs = std::filesystem;
using nlohmann::json;

std::string sql_name(const std::string& name) { return quote(Dialect::Sqlite, name); }

/// What went wrong, without the kind before it.
std::string problem_of(const std::exception& e) {
    if (auto storage = dynamic_cast<const StorageError*>(&e)) return storage->problem();
    return e.what();
}

/// A name without `_`, `-` and spaces, in lower case, as Clarity.Formats matches columns.
std::string plain(const std::string& name) {
    std::string out;
    for (char c : name)
        if (c != '_' && c != '-' && c != ' ') out += char(std::tolower(static_cast<unsigned char>(c)));
    return out;
}

/// The SQLite type a mapped column of `kind` gets, so its values are kept as the generated code reads them.
std::string column_type(const std::string& kind) {
    if (kind == "integer" || kind == "bool") return " INTEGER";
    if (kind == "text" || kind == "decimal" || kind == "date" || kind == "datetime" || kind == "enum" || kind == "values") return " TEXT";
    return "";
}

/// The type of file column `column`, by the kind of the field it maps.
std::string type_of(const json& fields, const std::string& column) {
    if (!fields.is_array()) return "";
    for (const auto& field : fields) {
        if (!field.is_object() || !field.contains("name") || !field.contains("kind") || !field["name"].is_string() || !field["kind"].is_string()) continue;
        if (plain(field["name"].get<std::string>()) == plain(column)) return column_type(field["kind"].get<std::string>());
    }
    return "";
}

/// A cell read as SQLite keeps it: a whole number as an integer, other numbers as their text.
SqlValue cell(const json& value) {
    if (value.is_null()) return null();
    if (value.is_boolean()) return int64_t(value.get<bool>() ? 1 : 0);
    if (value.is_number_integer() && !(value.is_number_unsigned() && value.get<uint64_t>() > uint64_t(INT64_MAX))) return value.get<int64_t>();
    if (value.is_string()) return value.get<std::string>();
    return value.dump();
}

/// A value of a row as a cell of the table written back.
json tree(const SqlValue& value) {
    if (auto n = std::get_if<int64_t>(&value)) return *n;
    if (auto r = std::get_if<double>(&value)) return *r;
    if (auto t = std::get_if<std::string>(&value)) return *t;
    return nullptr;
}

std::string read_file(const fs::path& path) {
    std::ifstream in(path, std::ios::binary);
    if (!in) throw StorageError("cannot read " + path.string());
    std::ostringstream bytes;
    bytes << in.rdbuf();
    if (in.bad()) throw StorageError("cannot read " + path.string());
    return bytes.str();
}

/// The table a nested table's rows sit in: `lines/notes` in `lines`, `lines` in the source's own ("").
std::string parent_of(const std::string& name) {
    auto slash = name.rfind('/');
    return slash == std::string::npos ? "" : name.substr(0, slash);
}

/// The records field the table name `name` (`lines/notes`) leads to among `fields`, or null.
const json* records_field(const json& fields, const std::string& name) {
    const json* here = &fields;
    const json* found = nullptr;
    std::stringstream steps(name);
    std::string step;
    while (std::getline(steps, step, '/')) {
        found = nullptr;
        if (!here->is_array()) return nullptr;
        for (const auto& field : *here)
            if (field.is_object() && field.value("name", std::string()) == step) found = &field;
        if (!found || !found->contains("fields")) return nullptr;
        here = &(*found)["fields"];
    }
    return found;
}

/// One table read from the file and kept in SQLite.
struct FileTable {
    /// Its name in the file's tables: "" for the source's own, `lines` or `lines/notes` for records inside.
    std::string name;
    std::string table;
    std::string key;
    /// The file's columns, in its order; for records inside, the second links a row to its row.
    std::vector<std::string> columns;
    /// Whether the second column is a link: the place of the row in the table it sits in.
    bool linked = false;
};

class FileDriver final : public Driver {
public:
    /// Reads the file at `path` (already found as an SQLite file's location is) into its tables.
    FileDriver(const FileSource& source, const fs::path& path)
        : db_(open_sqlite(":memory:")), path_(path), format_(source.format), options_(source.options), fields_(source.fields), readonly_(source.readonly) {
        auto about = [&](const std::string& text) { return StorageError("data source " + source.name + ": " + text); };
        try {
            // A file not there yet is a new one, made at the first save; a readonly source needs it.
            if (source.readonly || fs::exists(path)) original_ = read_file(path);
        } catch (const StorageError& e) {
            throw about(e.problem());
        }
        json read;
        try {
            read = json::parse(formats_read_table(source.format, original_, source.options, source.fields));
        } catch (const ValueError& e) {
            throw about(e.what());
        } catch (const json::exception&) {
            throw about("the table read is no table");
        }
        several_ = read.is_object() && read.contains("tables") && read["tables"].is_array();
        json tables = several_ ? read["tables"] : json::array({read});
        json fields = json::parse(source.fields, nullptr, false);
        // The keys of each table's rows, in the order read, so a link to a place becomes a key.
        std::map<std::string, std::vector<int64_t>> keys;
        for (const auto& table : tables) {
            if (!table.is_object() || !table.contains("columns") || !table.contains("rows") || !table["columns"].is_array() || !table["rows"].is_array())
                throw about("the table read is no table");
            FileTable kept;
            kept.name = table.value("name", std::string());
            kept.linked = !kept.name.empty();
            kept.table = source.table;
            kept.key = source.key;
            const json* kinds = &fields;
            if (kept.linked) {
                const json* field = records_field(fields, kept.name);
                if (!field || !(*field).contains("table") || !(*field)["table"].is_string() || !(*field).contains("key") || !(*field)["key"].is_string())
                    throw about("the records inside '" + kept.name + "' are no field of the fields given");
                kept.table = (*field)["table"].get<std::string>();
                kept.key = (*field)["key"].get<std::string>();
                kinds = &(*field)["fields"];
            }
            for (const auto& c : table["columns"]) {
                if (!c.is_string()) throw about("the table read is no table");
                kept.columns.push_back(c.get<std::string>());
            }
            bool keyed = false;
            for (const auto& c : kept.columns) keyed = keyed || c == kept.key;
            std::string create;
            // The file has no key column: the row's place keys it, never written back.
            if (!keyed) create = sql_name(kept.key) + " INTEGER PRIMARY KEY";
            std::string names, marks;
            for (size_t i = 0; i < kept.columns.size(); ++i) {
                const auto& c = kept.columns[i];
                std::string type = c == kept.key ? " INTEGER PRIMARY KEY" : kept.linked && i == 1 ? " INTEGER" : type_of(*kinds, c);
                create += (create.empty() ? "" : ", ") + sql_name(c) + type;
                names += (names.empty() ? "" : ", ") + sql_name(c);
                marks += marks.empty() ? "?" : ", ?";
            }
            try {
                db_->batch("CREATE TABLE " + sql_name(kept.table) + " (" + create + ")");
            } catch (const Error& e) {
                throw about(problem_of(e));
            }
            std::string insert = "INSERT INTO " + sql_name(kept.table) + " (" + names + ") VALUES (" + marks + ")";
            const std::vector<int64_t> parent = kept.linked ? keys[parent_of(kept.name)] : std::vector<int64_t>();
            std::vector<int64_t> inserted;
            size_t number = 0;
            for (const auto& row : table["rows"]) {
                ++number;
                std::vector<SqlValue> values;
                for (const auto& value : row) values.push_back(cell(value));
                if (kept.linked && values.size() > 1) {
                    // The place of the row it sits in, from 1, is that row's key.
                    auto place = std::get_if<int64_t>(&values[1]);
                    values[1] = place && *place >= 1 && size_t(*place) <= parent.size() ? SqlValue(parent[size_t(*place) - 1]) : null();
                }
                try {
                    inserted.push_back(db_->insert(insert, values));
                } catch (const Error& e) {
                    throw about("row " + std::to_string(number) + ": " + problem_of(e));
                }
            }
            keys[kept.name] = std::move(inserted);
            tables_.push_back(std::move(kept));
        }
    }

    Dialect dialect() const override { return Dialect::Sqlite; }

    int64_t execute(const std::string& sql, const std::vector<SqlValue>& params) override {
        int64_t changed = db_->execute(sql, params);
        dirty_ = dirty_ || changed > 0;
        keep();
        return changed;
    }

    int64_t insert(const std::string& sql, const std::vector<SqlValue>& params) override {
        int64_t id = db_->insert(sql, params);
        dirty_ = true;
        keep();
        return id;
    }

    std::vector<Row> rows(const std::string& sql, const std::vector<SqlValue>& params) override { return db_->rows(sql, params); }

    /// A transaction's end keeps what it changed.
    void batch(const std::string& sql) override {
        db_->batch(sql);
        keep();
    }

    /// A file source is external: Clarity writes no steps for it.
    void apply(int, const std::vector<std::string>&) override {}

private:
    /// Writes the table back when a change is kept: to a new file beside the old one, which then replaces it.
    void keep() {
        if (!dirty_ || readonly_ || !sqlite_autocommit(*db_)) return;
        dirty_ = false;
        // Each table's rows in their order, a link turned back into the place of its row among the rows of the table it sits in.
        std::map<std::string, std::map<int64_t, int64_t>> places;
        json list = json::array();
        for (const auto& kept : tables_) {
            std::string names;
            for (const auto& c : kept.columns) names += ", " + sql_name(c);
            const auto& parent = places[parent_of(kept.name)];
            std::map<int64_t, int64_t> own;
            json rows = json::array();
            int64_t place = 0;
            for (const auto& row : db_->rows("SELECT " + sql_name(kept.key) + names + " FROM " + sql_name(kept.table) + " ORDER BY rowid", {})) {
                ++place;
                if (auto key = std::get_if<int64_t>(&row[0])) own[*key] = place;
                json cells = json::array();
                for (size_t i = 1; i < row.size(); ++i) cells.push_back(tree(row[i]));
                if (kept.linked && cells.size() > 1) {
                    auto key = std::get_if<int64_t>(&row[2]);
                    auto found = key ? parent.find(*key) : parent.end();
                    cells[1] = found != parent.end() ? json(found->second) : json(nullptr);
                }
                rows.push_back(std::move(cells));
            }
            places[kept.name] = std::move(own);
            json table = {{"columns", kept.columns}, {"rows", std::move(rows)}};
            if (several_) table["name"] = kept.name;
            list.push_back(std::move(table));
        }
        json table = several_ ? json{{"tables", std::move(list)}} : std::move(list[0]);
        std::string bytes;
        try {
            bytes = formats_write_table(format_, original_, table.dump(), options_, fields_);
        } catch (const ValueError& e) {
            throw StorageError(e.what());
        }
        std::error_code made;
        if (path_.has_parent_path()) fs::create_directories(path_.parent_path(), made);
        fs::path written = path_;
        written += ".new";
        {
            std::ofstream out(written, std::ios::binary | std::ios::trunc);
            out.write(bytes.data(), std::streamsize(bytes.size()));
            if (!out) throw StorageError("cannot write " + written.string());
        }
        std::error_code failed;
        fs::rename(written, path_, failed);
        if (failed) throw StorageError("cannot replace " + path_.string() + ": " + failed.message());
        original_ = std::move(bytes);
    }

    std::unique_ptr<Driver> db_;
    fs::path path_;
    std::string format_, options_, fields_;
    /// The tables the file is read into: one, or with records inside (a mapping's `nested`) one per entity.
    std::vector<FileTable> tables_;
    /// Whether the file holds records inside its records, so its tables go back as several.
    bool several_ = false;
    /// The file as it was last read or written, for formats that keep more than the table.
    std::string original_;
    bool readonly_ = false;
    bool dirty_ = false;
};

}  // namespace

void Db::add_file(const FileSource& source) {
    fs::path path(location(data_folder_sqlite(source.path)));
    auto driver = std::make_unique<FileDriver>(source, path);
    // A source of the same name is replaced.
    for (auto& s : sources_) {
        if (s.name != source.name) continue;
        s.driver = std::move(driver);
        s.depth = 0;
        return;
    }
    sources_.push_back({source.name, std::move(driver), 0});
}

}  // namespace clarity
