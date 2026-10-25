// Where an application keeps its settings, as every runtime finds it
// (conformance/settings.json, `file`), and keeping them.

#include <cstdlib>
#include <filesystem>

#include "check.hpp"
#include "clarity/settings.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

TEST(the_settings_file_is_where_every_runtime_puts_it) {
    json cases = conformance("settings.json");
    for (const auto& c : cases["file"]) {
        const json& env = c["env"];
        auto found = settings_file_in(c["os"].get<std::string>(), [&](const std::string& name) -> std::optional<std::string> {
            return env.contains(name) ? std::optional<std::string>(env[name].get<std::string>()) : std::nullopt;
        }, c["program"].get<std::string>());
        if (c["file"].is_null()) CHECK(!found);
        else CHECK(found && *found == c["file"].get<std::string>());
    }
}

TEST(settings_kept_are_read_back) {
    // A folder of the test's own, not the user's settings.
    auto folder = std::filesystem::temp_directory_path() / "clarity-settings-cpp";
    std::filesystem::remove_all(folder);
#ifdef _WIN32
    const char* had = std::getenv("APPDATA");
    std::string was = had ? had : "";
    _putenv_s("APPDATA", folder.string().c_str());
#else
    setenv("XDG_CONFIG_HOME", folder.string().c_str(), 1);
#endif
    const std::string program = "clarity-settings-test";
    CHECK(read_settings(program).empty());
    json values = {{"pageSize", 20}, {"language", "nl"}};
    keep_settings(program, values);
    CHECK(read_settings(program) == values);
    std::filesystem::remove_all(folder);
#ifdef _WIN32
    _putenv_s("APPDATA", was.c_str());
#endif
}
