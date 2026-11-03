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
    if (location.rfind("env:", 0) == 0 || location.rfind("setting:", 0) == 0 || location == ":memory:") return location;
    const char* folder = env ? env(data_variable) : std::getenv(data_variable);
    if (!folder || !*folder) return location;
    std::filesystem::path name = std::filesystem::path(location).filename();
    if (name.empty() || name == "." || name == "..") return location;
    return (std::filesystem::path(folder) / name).string();
}

std::unique_ptr<Driver> open_driver(Dialect dialect, const std::string& at) {
    // An SQLite file opens in the folder CLARITY_DATA names, when it is set.
    std::string where = location(dialect == Dialect::Sqlite ? data_folder_sqlite(at) : at);
    if (dialect == Dialect::Sqlite) return open_sqlite(where);
    throw StorageError("this application was built without " + label(dialect));
}

}  // namespace clarity
