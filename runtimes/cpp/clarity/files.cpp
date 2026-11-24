// The files of a file store, which code reaches through `files f in
// store`, `read text of f` and `write t to store "path"`: only folders the
// project settings name. A file found can become a record's picture: in the
// field's own store by its path, elsewhere as a copy. The other runtimes do
// the same (conformance/files.json).

#include <algorithm>
#include <chrono>
#include <fstream>
#include <set>
#include <sstream>

#include "clarity/db.hpp"
#include "clarity/settings.hpp"

namespace clarity {

namespace fs = std::filesystem;

// data:<program>/<name> is in the program's data folder, and with env:NAME
// unset the store says so where it is used, so the program does not stop
// (conformance/data-folder.json, stores).
void Db::add_store(const std::string& name, const std::string& at) {
    stores_not_set_.erase(name);
    fs::path folder;
    try {
        folder = fs::path(data_folder_store(at));
    } catch (const NotSetError& e) {
        stores_.erase(name);
        store_settings_.erase(name);
        stores_not_set_[name] = e.variable;
        return;
    }
    std::error_code error;
    fs::create_directories(folder, error);
    if (error) throw StorageError("cannot make the file store " + folder.string() + ": " + error.message());
    store_settings_.erase(name);
    stores_[name] = folder;
}

void Db::add_store_setting(const std::string& name, const std::string& program, const std::string& setting, const std::string& fallback) {
    stores_.erase(name);
    stores_not_set_.erase(name);
    store_settings_[name] = {program, setting, fallback};
    store_folder(name);
}

void Db::hold_settings(const nlohmann::json& settings) { held_settings_ = settings.is_object() ? settings : nlohmann::json::object(); }

void Db::hold_setting(const std::string& setting, const std::string& value) { held_settings_[setting] = value; }

fs::path Db::store_folder(const std::string& name) {
    // A folder that is no store, by its path: just a folder.
    if (name.starts_with(folder_store)) {
        std::string path = name.substr(folder_store.size());
        if (path.empty()) throw no_folder_chosen();
        return fs::path(path);
    }
    if (auto unset = stores_not_set_.find(name); unset != stores_not_set_.end()) throw NotSetError(unset->second);
    if (auto setting = store_settings_.find(name); setting != store_settings_.end()) {
        // A relative folder is in the program's data folder (conformance/settings.json, folders).
        nlohmann::json kept = read_settings(setting->second.program);
        std::string data = fs::path(data_folder(setting->second.program)).generic_string();
        fs::path folder = store_folder_in(settings_os(), data, name, nlohmann::json(setting_now(held_settings_, kept, setting->second.setting)), setting->second.fallback);
        std::error_code error;
        fs::create_directories(folder, error);
        if (error) throw StorageError("cannot make the file store " + folder.string() + ": " + error.message());
        return folder;
    }
    auto found = stores_.find(name);
    if (found == stores_.end()) throw StorageError("there is no file store '" + name + "'");
    return found->second;
}

Bytes Db::read_file(const std::string& store, const std::string& path) {
    fs::path file = in_store(store_folder(store), path);
    std::ifstream in(file, std::ios::binary);
    if (!in) throw StorageError("cannot read " + file.string());
    return Bytes(std::istreambuf_iterator<char>(in), {});
}

static DateTime utc(fs::file_time_type at) {
    auto seconds = std::chrono::floor<std::chrono::seconds>(std::chrono::clock_cast<std::chrono::system_clock>(at));
    auto day = std::chrono::floor<std::chrono::days>(seconds);
    std::chrono::year_month_day date{day};
    std::chrono::hh_mm_ss time{seconds - day};
    return DateTime{Date{int(date.year()), int(unsigned(date.month())), int(unsigned(date.day()))}, int(time.hours().count()),
                    int(time.minutes().count()), int(time.seconds().count())};
}

static void walk(const std::string& store, const fs::path& folder, const std::string& prefix, std::vector<FileEntry>& out, bool hidden = false) {
    std::error_code error;
    for (const auto& entry : fs::directory_iterator(folder, error)) {
        std::string name = entry.path().filename().string();
        if (name.starts_with(".") && !hidden) continue;
        if (entry.is_directory()) {
            walk(store, entry.path(), prefix + name + "/", out, hidden);
        } else if (entry.is_regular_file()) {
            std::string extension = entry.path().extension().string();
            if (!extension.empty()) extension.erase(0, 1);
            std::transform(extension.begin(), extension.end(), extension.begin(), [](unsigned char c) { return char(std::tolower(c)); });
            out.push_back({store, prefix + name, name, extension, int64_t(entry.file_size()), utc(entry.last_write_time())});
        }
    }
    if (error) throw StorageError("cannot read " + folder.string() + ": " + error.message());
}

std::vector<FileEntry> Db::files(const std::string& store) {
    std::vector<FileEntry> out;
    walk(store, store_folder(store), "", out);
    std::sort(out.begin(), out.end(), [](const FileEntry& a, const FileEntry& b) { return a.path < b.path; });
    return out;
}

static std::string lower(std::string text) {
    std::transform(text.begin(), text.end(), text.begin(), [](unsigned char c) { return char(std::tolower(c)); });
    return text;
}

bool folder_takes(const std::vector<std::string>& patterns, bool hidden, const std::string& name) {
    if (name.starts_with(".") && !hidden) return false;
    if (patterns.empty() || std::find(patterns.begin(), patterns.end(), "*.*") != patterns.end()) return true;
    // `.env` has no extension, `.notes.md` has md.
    std::string extension = fs::path(name).extension().string();
    if (!extension.empty()) extension.erase(0, 1);
    extension = lower(extension);
    for (const auto& p : patterns)
        if (p.starts_with("*.") && !extension.empty() && extension == lower(p.substr(2))) return true;
    return false;
}

void check_file_type(const std::string& field, const std::vector<std::string>& patterns, bool hidden, const std::string& name) {
    size_t cut = name.find_last_of("/\\");
    std::string last = cut == std::string::npos ? name : name.substr(cut + 1);
    if (last.empty() || folder_takes(patterns, hidden, last)) return;
    std::string holds;
    if (last.starts_with(".") && !hidden) {
        holds = "no hidden";
    } else if (patterns.empty()) {
        holds = "any";
    } else {
        for (const auto& p : patterns) holds += (holds.empty() ? "" : " ") + p;
    }
    refuse(Text::keyed("fileTypeRefused", "{field} takes {extensions} files, not {name}")
               .with("field", slot_text(field))
               .with("extensions", slot_text(holds))
               .with("name", slot_text(last)));
}

std::string Db::store_upload(const std::string& store, const std::string& name, const Bytes& bytes) {
    std::string inside = store_path(bytes, name);
    write_bytes(store, inside, bytes);
    return (store_folder(store) / fs::path(inside)).string();
}

nlohmann::json Db::stores() {
    std::set<std::string> names;
    for (const auto& [name, _] : stores_) names.insert(name);
    for (const auto& [name, _] : store_settings_) names.insert(name);
    nlohmann::json out = nlohmann::json::array();
    for (const auto& name : names) {
        try {
            out.push_back({{"name", name}, {"folder", store_folder(name).string()}});
        } catch (const Error&) {
        }
    }
    return out;
}

std::vector<FileEntry> Db::files_in(const std::string& folder, const std::vector<std::string>& patterns, bool hidden) {
    std::vector<FileEntry> out;
    walk(folder, store_folder(folder), "", out, hidden);
    std::erase_if(out, [&](const FileEntry& f) { return !folder_takes(patterns, hidden, f.name); });
    std::sort(out.begin(), out.end(), [](const FileEntry& a, const FileEntry& b) { return a.path < b.path; });
    return out;
}

void Db::write_text_in(const std::string& folder, const std::vector<std::string>& patterns, bool hidden, const std::string& path, const std::string& text) {
    write_bytes_in(folder, patterns, hidden, path, Bytes(text.begin(), text.end()));
}

void Db::write_bytes_in(const std::string& folder, const std::vector<std::string>& patterns, bool hidden, const std::string& path, const Bytes& bytes) {
    std::string name = path.substr(path.rfind('/') == std::string::npos ? 0 : path.rfind('/') + 1);
    if (!folder_takes(patterns, hidden, name)) {
        std::string holds;
        if (name.starts_with(".") && !hidden) {
            holds = "no hidden";
        } else {
            for (const auto& p : patterns) holds += (holds.empty() ? "" : " ") + p;
        }
        throw InvalidError("the folder holds " + holds + " files, not '" + path + "'");
    }
    write_bytes(folder, path, bytes);
}

/// Whether text is UTF-8.
static bool utf8(const Bytes& b) {
    for (size_t i = 0; i < b.size();) {
        uint8_t c = b[i];
        size_t n = c < 0x80 ? 0 : (c >> 5) == 6 ? 1 : (c >> 4) == 14 ? 2 : (c >> 3) == 30 ? 3 : 9;
        if (n == 9 || i + n >= b.size()) return false;
        for (size_t k = 1; k <= n; ++k)
            if ((b[i + k] >> 6) != 2) return false;
        i += n + 1;
    }
    return true;
}

std::string Db::read_text(const FileEntry& file) {
    Bytes bytes = read_file(file.store, file.path);
    if (!utf8(bytes)) throw InvalidError(file.path + " is not text");
    return std::string(bytes.begin(), bytes.end());
}

void Db::write_text(const std::string& store, const std::string& path, const std::string& text) {
    write_bytes(store, path, Bytes(text.begin(), text.end()));
}

Bytes Db::read_bytes(const FileEntry& file) { return read_file(file.store, file.path); }

void Db::write_bytes(const std::string& store, const std::string& path, const Bytes& bytes) {
    fs::path file = in_store(store_folder(store), path);
    std::error_code error;
    fs::create_directories(file.parent_path(), error);
    std::ofstream out(file, std::ios::binary);
    out.write(reinterpret_cast<const char*>(bytes.data()), static_cast<std::streamsize>(bytes.size()));
    if (!out) throw StorageError("cannot write " + file.string());
}

Picture Db::picture_from(const FileEntry& file, const std::optional<std::string>& field_store) {
    Bytes bytes = read_file(file.store, file.path);
    checked(bytes);
    if (field_store && *field_store == file.store) return Picture::in_store(file.path);
    return Picture::of(std::move(bytes));
}

RecordFile Db::record_file_from(const FileEntry& file, const std::optional<std::string>& field_store) {
    if (field_store && *field_store == file.store) return {file.name, Picture::in_store(file.path)};
    return {file.name, Picture::of(read_file(file.store, file.path))};
}

Bytes Db::store_file(const std::string& store, const std::string& path) { return read_file(store, path); }

}  // namespace clarity
