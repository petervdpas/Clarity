#include "clarity/store.hpp"

#include <cstdlib>
#include <filesystem>

#include "clarity/errors.hpp"
#include "clarity/settings.hpp"

namespace clarity {

std::string applied(int version) { return "INSERT INTO clarity_schema (version) VALUES (" + std::to_string(version) + ")"; }

std::string location(const std::string& text) {
    // A secret setting (DESIGN 5.27): setting:<program>/<name>, from the system's protected store.
    if (text.rfind("setting:", 0) == 0) {
        auto at = text.substr(8);
        auto slash = at.find('/');
        auto name = slash == std::string::npos ? at : at.substr(slash + 1);
        std::optional<std::string> value;
        if (slash != std::string::npos && secret_store.get) value = secret_store.get(at.substr(0, slash), name);
        if (!value || value->empty()) throw NotKeptError(name);
        return *value;
    }
    if (text.rfind("env:", 0) != 0) return text;
    std::string name = text.substr(4);
    const char* value = std::getenv(name.c_str());
    if (!value) throw StorageError("the environment variable " + name + " is not set");
    return value;
}

std::string data_folder_sqlite(const std::string& location, const std::function<const char*(const char*)>& env) {
#if defined(_WIN32)
    return data_folder_sqlite_on("windows", location, env);
#elif defined(__APPLE__)
    return data_folder_sqlite_on("macos", location, env);
#else
    return data_folder_sqlite_on("linux", location, env);
#endif
}

std::string data_folder_sqlite_on(const std::string& os, const std::string& location, const std::function<const char*(const char*)>& env) {
    if (location.rfind("env:", 0) == 0 || location.rfind("setting:", 0) == 0 || location == ":memory:") return location;
    auto get = [&](const char* name) -> std::string {
        const char* value = env ? env(name) : std::getenv(name);
        return value ? value : "";
    };
    // A database the project keeps beside it: data:<program>/<file>.
    bool kept = location.rfind("data:", 0) == 0;
    std::string rest = kept ? location.substr(5) : location;
    std::string name = rest.substr(rest.find_last_of("/\\") == std::string::npos ? 0 : rest.find_last_of("/\\") + 1);
    std::string folder = get(data_variable);
    if (!folder.empty() && !name.empty() && name != "." && name != "..") return (std::filesystem::path(folder) / name).string();
    if (!kept) return location;
    // The person's folder for application data, under the program's name.
    std::string home = get("HOME");
    std::string data;
    if (os == "windows") {
        data = get("APPDATA");
    } else if (os == "macos") {
        if (!home.empty()) data = home + "/Library/Application Support";
    } else {
        data = get("XDG_DATA_HOME");
        if (data.empty() && !home.empty()) data = home + "/.local/share";
    }
    return data.empty() ? name : data + "/" + rest;
}

std::unique_ptr<Driver> open_driver(Dialect dialect, const std::string& at) {
    // An SQLite file opens in the folder CLARITY_DATA names, when it is set.
    std::string where = location(dialect == Dialect::Sqlite ? data_folder_sqlite(at) : at);
    if (dialect == Dialect::Sqlite) return open_sqlite(where);
    throw StorageError("this application was built without " + label(dialect));
}

}  // namespace clarity
