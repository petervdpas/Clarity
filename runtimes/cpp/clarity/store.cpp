#include "clarity/store.hpp"

#include <cstdlib>

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

std::unique_ptr<Driver> open_driver(Dialect dialect, const std::string& at) {
    std::string where = location(at);
    if (dialect == Dialect::Sqlite) return open_sqlite(where);
    throw StorageError("this application was built without " + label(dialect));
}

}  // namespace clarity
