// Secret settings in the system's protected store (DESIGN 5.27): kept, read
// back and forgotten, apart from settings.json. Needs a store, such as the
// Secret Service of a desktop session; without one it checks nothing, unless
// CLARITY_SECRET_STORE=1 says the machine has one.

#include <cstdlib>
#include <filesystem>
#include <string>

#include "check.hpp"
#include "clarity/db.hpp"
#include "clarity/errors.hpp"
#include "clarity/settings.hpp"
#include "secrets/secrets.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

const std::string Program = "clarity-runtime-test";

bool store_works() {
    bool works = false;
    try {
        secret_set(Program, "probe", "1");
        works = secret_get(Program, "probe") == std::optional<std::string>("1");
        secret_set(Program, "probe", "");
    } catch (const Error&) {
    }
    const char* said = std::getenv("CLARITY_SECRET_STORE");
    if (said && std::string(said) == "1") CHECK(works);
    return works;
}

}  // namespace

TEST(a_secret_is_kept_read_and_forgotten) {
    if (!store_works()) return;
    secret_set(Program, "token", "s3cr3t");
    CHECK(secret_get(Program, "token") == std::optional<std::string>("s3cr3t"));
    secret_set(Program, "token", "");
    CHECK(!secret_get(Program, "token"));
    secret_set(Program, "token", "");
}

TEST(settings_keep_their_secrets_out_of_the_file) {
    if (!store_works()) return;
    secrets_use();
#ifdef _WIN32
    // A folder of the test's own, not the user's settings.
    auto folder = std::filesystem::temp_directory_path() / ("clarity-secret-settings-cpp-" + std::to_string(std::rand()));
    const char* had = std::getenv("APPDATA");
    std::string was = had ? had : "";
    _putenv_s("APPDATA", folder.string().c_str());
#else
    // Beside the tests, where they run, as the settings test keeps its own.
    setenv("XDG_CONFIG_HOME", ("clarity-secret-settings-" + std::to_string(std::rand())).c_str(), 1);
#endif
    const std::string program = "clarity-runtime-test-settings";
    keep_settings_with(program, json{{"pageSize", 20}, {"apiToken", "t0k3n"}}, json::array({"apiToken"}));
    json file = read_settings(program);
    CHECK(!file.contains("apiToken"));
    CHECK(file.contains("pageSize"));
    json read = read_settings_with(program, json::array({"apiToken"}));
    CHECK(read.value("apiToken", "") == "t0k3n");
    secret_set(program, "apiToken", "");
    secret_store = SecretStore{};
#ifdef _WIN32
    std::filesystem::remove_all(folder);
    _putenv_s("APPDATA", was.c_str());
#endif
}

// A database connects with a secret setting (DESIGN 5.27): its connection,
// password and all, kept in the protected store, not in the program.
TEST(a_database_connects_with_a_secret_setting) {
    if (!store_works()) return;
    secrets_use();
    auto file = (std::filesystem::temp_directory_path() / "clarity-secret-connection-cpp.db").string();
    secret_set(Program, "shopDb", file);
    try {
        Db db({{"main", Dialect::Sqlite, "setting:" + Program + "/shopDb"}});
    } catch (const std::exception&) {
        secret_set(Program, "shopDb", "");
        CHECK(false);
    }
    secret_set(Program, "shopDb", "");
    bool refused = false;
    try {
        Db db({{"main", Dialect::Sqlite, "setting:" + Program + "/shopDb"}});
    } catch (const StorageError& e) {
        refused = std::string(e.what()).find("the secret setting shopDb is not kept yet") != std::string::npos;
    }
    CHECK(refused);
    std::filesystem::remove(file);
    secret_store = SecretStore{};
}
