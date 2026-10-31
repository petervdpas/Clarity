#include "clarity/settings.hpp"

#include <cctype>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <map>
#include <mutex>
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

std::string settings_os() {
#if defined(_WIN32)
    return "windows";
#elif defined(__APPLE__)
    return "macos";
#else
    return "linux";
#endif
}

std::string store_folder_in(const std::string& os, const std::string& settings_folder, const std::string& store, const nlohmann::json& kept, const std::string& fallback) {
    std::string chosen = kept.is_string() && !kept.get<std::string>().empty() ? kept.get<std::string>() : fallback;
    if (chosen.empty()) chosen = store;
    bool absolute;
    if (os == "windows") {
        bool drive = chosen.size() > 2 && std::isalpha(static_cast<unsigned char>(chosen[0])) && chosen[1] == ':' && (chosen[2] == '/' || chosen[2] == '\\');
        absolute = drive || chosen.starts_with("\\\\") || chosen.starts_with("//");
    } else {
        absolute = chosen.starts_with('/');
    }
    return absolute ? chosen : settings_folder + "/" + chosen;
}

std::optional<std::string> settings_file(const std::string& program) {
    return settings_file_in(settings_os(), [](const std::string& name) -> std::optional<std::string> {
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

SecretStore secret_store;

namespace {
// Each secret as the store holds it now, as read or kept, by program and
// name; one never kept is "". Only a secret that changed goes to the store:
// another setting changing never needs it, nor fails where it cannot be reached.
std::map<std::pair<std::string, std::string>, std::string> kept_secrets;
std::mutex kept_secrets_lock;
}  // namespace

nlohmann::json read_settings_with(const std::string& program, const nlohmann::json& secrets) {
    nlohmann::json values = read_settings(program);
    if (!secrets.is_array()) return values;
    for (const auto& name : secrets) {
        if (!name.is_string()) continue;
        values.erase(name.get<std::string>());
        if (!secret_store.get) continue;
        if (auto secret = secret_store.get(program, name.get<std::string>())) {
            values[name.get<std::string>()] = *secret;
            std::lock_guard held(kept_secrets_lock);
            kept_secrets[{program, name.get<std::string>()}] = *secret;
        }
    }
    return values;
}

void keep_settings_with(const std::string& program, const nlohmann::json& values, const nlohmann::json& secrets) {
    nlohmann::json plain = values;
    if (secrets.is_array()) {
        for (const auto& name : secrets) {
            if (!name.is_string() || !plain.contains(name.get<std::string>())) continue;
            const auto& value = plain[name.get<std::string>()];
            const std::string text = value.is_string() ? value.get<std::string>() : "";
            if (secret_store.set) {
                std::lock_guard held(kept_secrets_lock);
                auto& was = kept_secrets[{program, name.get<std::string>()}];
                if (was != text) {
                    secret_store.set(program, name.get<std::string>(), text);
                    was = text;
                }
            }
            plain.erase(name.get<std::string>());
        }
    }
    keep_settings(program, plain);
}

}  // namespace clarity
