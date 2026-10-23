#include "clarity/record_file.hpp"

#include <algorithm>
#include <cctype>
#include <cstdlib>
#include <filesystem>
#include <fstream>

#include "clarity/errors.hpp"
#include "clarity/sql.hpp"

namespace clarity {

namespace {

std::string last_part(const std::string& path) {
    auto at = path.find_last_of("/\\");
    return at == std::string::npos ? path : path.substr(at + 1);
}

std::string system_os() {
#if defined(_WIN32)
    return "windows";
#elif defined(__APPLE__)
    return "macos";
#else
    return "linux";
#endif
}

std::optional<std::string> from_system(const std::string& name) {
    const char* value = std::getenv(name.c_str());
    return value ? std::optional<std::string>(value) : std::nullopt;
}

void write(const std::filesystem::path& file, const Bytes& bytes) {
    std::ofstream out(file, std::ios::binary | std::ios::trunc);
    out.write(reinterpret_cast<const char*>(bytes.data()), std::streamsize(bytes.size()));
    if (!out) throw StorageError("cannot write " + file.string());
}

}  // namespace

std::string clean_name(const std::string& chosen) {
    std::string last = last_part(chosen);
    auto start = last.find_first_not_of(" \t\r\n");
    auto end = last.find_last_not_of(" \t\r\n");
    return start == std::string::npos ? "file" : last.substr(start, end - start + 1);
}

std::string store_path(const Bytes& bytes, const std::string& name) { return sha256_hex(bytes).substr(0, 32) + "/" + clean_name(name); }

std::string name_of(const std::string& path) { return last_part(path); }

std::string extension_of(const std::string& name) {
    auto dot = name.find_last_of('.');
    if (dot == std::string::npos || dot == 0) return "";
    std::string extension = name.substr(dot + 1);
    std::transform(extension.begin(), extension.end(), extension.begin(), [](unsigned char c) { return char(std::tolower(c)); });
    return extension;
}

std::string record_file_name(const std::optional<RecordFile>& file) { return file ? file->name : ""; }

std::string record_file_extension(const std::optional<RecordFile>& file) { return extension_of(record_file_name(file)); }

std::optional<std::string> downloads_in(const std::string& os, const std::function<std::optional<std::string>(const std::string&)>& env) {
    auto home = env(os == "windows" ? "USERPROFILE" : "HOME");
    if (!home || home->empty()) return std::nullopt;
    return *home + "/Downloads";
}

std::string free_name(const std::string& name, const std::function<bool(const std::string&)>& taken) {
    if (!taken(name)) return name;
    // A leading dot starts a name, not an extension.
    auto dot = name.find_last_of('.');
    bool has_extension = dot != std::string::npos && dot > 0;
    std::string stem = has_extension ? name.substr(0, dot) : name;
    std::string extension = has_extension ? name.substr(dot) : "";
    for (int n = 2;; ++n) {
        std::string candidate = stem + " (" + std::to_string(n) + ")" + extension;
        if (!taken(candidate)) return candidate;
    }
}

std::string save_copy(const std::string& name, const Bytes& bytes) {
    auto folder = downloads_in(system_os(), from_system);
    if (!folder) throw InvalidError("this system names no Downloads folder");
    std::filesystem::path root(*folder);
    std::filesystem::create_directories(root);
    std::string free = free_name(clean_name(name), [&](const std::string& n) { return std::filesystem::exists(root / n); });
    auto file = root / free;
    write(file, bytes);
    return file.string();
}

std::string temporary_copy(const std::string& name, const Bytes& bytes) {
    auto folder = std::filesystem::temp_directory_path() / ("clarity-open-" + sha256_hex(bytes).substr(0, 32));
    std::filesystem::create_directories(folder);
    auto file = folder / clean_name(name);
    write(file, bytes);
    return file.string();
}

SqlValue file_content(const std::optional<RecordFile>& file) { return sql::picture(file ? std::optional<Picture>(file->content) : std::nullopt); }

SqlValue file_name_value(const std::optional<RecordFile>& file) { return file ? file->name : std::string(); }

std::optional<RecordFile> get_file(const std::string& column, const SqlValue& content, const SqlValue& name) {
    auto picture = sql::get_picture(column, content);
    if (!picture) return std::nullopt;
    auto text = std::get_if<std::string>(&name);
    return RecordFile{clean_name(text ? *text : ""), *picture};
}

void to_json(nlohmann::json& out, const RecordFile& file) {
    out = nlohmann::json(file.content);
    out["name"] = file.name;
}

void from_json(const nlohmann::json& in, RecordFile& file) {
    file.content = in.get<Picture>();
    file.name = clean_name(in.value("name", ""));
}

}  // namespace clarity
