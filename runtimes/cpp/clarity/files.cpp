// The files of a file store, which code reaches through `files f in
// store`, `read text of f` and `write t to store "path"`: only folders the
// project settings name. A file found can become a record's picture: in the
// field's own store by its path, elsewhere as a copy. The other runtimes do
// the same (conformance/files.json).

#include <algorithm>
#include <chrono>
#include <fstream>
#include <sstream>

#include "clarity/db.hpp"

namespace clarity {

namespace fs = std::filesystem;

void Db::add_store(const std::string& name, const std::string& at) {
    fs::path folder = location(at);
    std::error_code error;
    fs::create_directories(folder, error);
    if (error) throw StorageError("cannot make the file store " + folder.string() + ": " + error.message());
    stores_[name] = folder;
}

const fs::path& Db::store_folder(const std::string& name) {
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

static void walk(const std::string& store, const fs::path& folder, const std::string& prefix, std::vector<FileEntry>& out) {
    std::error_code error;
    for (const auto& entry : fs::directory_iterator(folder, error)) {
        std::string name = entry.path().filename().string();
        if (name.starts_with(".")) continue;
        if (entry.is_directory()) {
            walk(store, entry.path(), prefix + name + "/", out);
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
    fs::path file = in_store(store_folder(store), path);
    std::error_code error;
    fs::create_directories(file.parent_path(), error);
    std::ofstream out(file, std::ios::binary);
    out << text;
    if (!out) throw StorageError("cannot write " + file.string());
}

Picture Db::picture_from(const FileEntry& file, const std::optional<std::string>& field_store) {
    Bytes bytes = read_file(file.store, file.path);
    checked(bytes);
    if (field_store && *field_store == file.store) return Picture::in_store(file.path);
    return Picture::of(std::move(bytes));
}

Bytes Db::store_file(const std::string& store, const std::string& path) { return read_file(store, path); }

}  // namespace clarity
