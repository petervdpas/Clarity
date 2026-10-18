#include "clarity/store.hpp"

#include <cstdlib>

#include "clarity/errors.hpp"

namespace clarity {

std::string applied(int version) { return "INSERT INTO clarity_schema (version) VALUES (" + std::to_string(version) + ")"; }

std::string location(const std::string& text) {
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
