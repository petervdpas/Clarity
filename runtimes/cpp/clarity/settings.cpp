#include "clarity/settings.hpp"

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <sstream>

#include "clarity/errors.hpp"

namespace clarity {

std::optional<std::string> settings_file_in(const std::string& os, const std::function<std::optional<std::string>(const std::string&)>& env, const std::string& program) {
    auto set = [&](const std::string& name) -> std::optional<std::string> {
        auto value = env(name);
        return value && !value->empty() ? value : std::nullopt;
    };
    std::optional<std::string> folder;
    if (os == "windows") {
        folder = set("APPDATA");
    } else if (os == "macos") {
        if (auto home = set("HOME")) folder = *home + "/Library/Application Support";
    } else {
        folder = set("XDG_CONFIG_HOME");
        if (auto home = set("HOME"); !folder && home) folder = *home + "/.config";
    }
    if (!folder) return std::nullopt;
    return *folder + "/" + program + "/settings.json";
}

std::optional<std::string> settings_file(const std::string& program) {
#if defined(_WIN32)
    const std::string os = "windows";
#elif defined(__APPLE__)
    const std::string os = "macos";
#else
    const std::string os = "linux";
#endif
    return settings_file_in(os, [](const std::string& name) -> std::optional<std::string> {
        const char* value = std::getenv(name.c_str());
        return value ? std::optional<std::string>(value) : std::nullopt;
    }, program);
}

nlohmann::json read_settings(const std::string& program) {
    auto file = settings_file(program);
    if (!file) return nlohmann::json::object();
    std::ifstream in(*file, std::ios::binary);
    if (!in) return nlohmann::json::object();
    std::stringstream text;
    text << in.rdbuf();
    auto kept = nlohmann::json::parse(text.str(), nullptr, false);
    return kept.is_object() ? kept : nlohmann::json::object();
}

void keep_settings(const std::string& program, const nlohmann::json& values) {
    auto file = settings_file(program);
    if (!file) throw InvalidError("this system names no folder for application settings");
    namespace fs = std::filesystem;
    fs::path path(*file);
    std::error_code failed;
    fs::create_directories(path.parent_path(), failed);
    fs::path written = path;
    written += ".new";
    {
        std::ofstream out(written, std::ios::binary | std::ios::trunc);
        out << values.dump(2);
        if (!out) throw StorageError("cannot keep the settings in " + path.string());
    }
    fs::rename(written, path, failed);
    if (failed) throw StorageError("cannot keep the settings in " + path.string() + ": " + failed.message());
}

}  // namespace clarity
