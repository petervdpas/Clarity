// Where an application keeps its settings, as every runtime finds it
// (conformance/settings.json, `file`), and keeping them.

#include <cstdlib>
#include <filesystem>

#include "check.hpp"
#include "clarity/db.hpp"
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

TEST(a_folder_setting_is_where_every_runtime_puts_a_store) {
    json cases = conformance("settings.json");
    for (const auto& c : cases["folders"]) {
        auto found = store_folder_in(c["os"].get<std::string>(), c["settingsFolder"].get<std::string>(), c["store"].get<std::string>(), c["kept"], c["default"].get<std::string>());
        CHECK(found == c["folder"].get<std::string>());
    }
}

// A store whose folder is a setting (filestore photos "setting:photosFolder")
// keeps its files where the person's settings say, read again at every use,
// so choosing another folder takes effect at once.
TEST(a_store_keeps_its_files_in_a_folder_setting) {
    auto home = std::filesystem::temp_directory_path() / "clarity-folder-cpp";
    std::filesystem::remove_all(home);
#ifdef _WIN32
    const char* had = std::getenv("APPDATA");
    std::string was = had ? had : "";
    _putenv_s("APPDATA", home.string().c_str());
#elif defined(__APPLE__)
    std::string was = std::getenv("HOME");
    setenv("HOME", home.string().c_str(), 1);
#else
    setenv("XDG_CONFIG_HOME", home.string().c_str(), 1);
#endif
    const std::string program = "clarity-folder-test";
    auto settings_folder = std::filesystem::path(*settings_file(program)).parent_path();
    {
        Db db = Db::memory();
        db.add_store_setting("photos", program, "photosFolder", "photos");
        // Not chosen yet: the default, in the settings' folder.
        db.write_text("photos", "first.txt", "first");
        CHECK(std::filesystem::exists(settings_folder / "photos" / "first.txt"));
        // Chosen while the program runs: the next file goes there.
        auto chosen = home / "chosen";
        keep_settings(program, json{{"photosFolder", chosen.string()}});
        db.write_text("photos", "second.txt", "second");
        CHECK(std::filesystem::exists(chosen / "second.txt"));
    }
    std::filesystem::remove_all(home);
#ifdef _WIN32
    _putenv_s("APPDATA", was.c_str());
#elif defined(__APPLE__)
    setenv("HOME", was.c_str(), 1);
#endif
}
