// CLARITY_DATA moves every SQLite database of a program into a folder of its
// own, under its own file name: a run in the IDE or by hand (a live check on
// a copy) chooses its data without editing the generated program. The same
// as the Rust and C# runtimes.

#include <cstdlib>
#include <filesystem>

#include "check.hpp"
#include "clarity/db.hpp"
#include "clarity/store.hpp"

using namespace clarity;
namespace fs = std::filesystem;

namespace {

std::function<const char*(const char*)> env(const char* folder) {
    return [folder](const char* name) -> const char* { return std::string(name) == data_variable ? folder : nullptr; };
}

void set_data_folder(const std::string* folder) {
#ifdef _WIN32
    _putenv_s(data_variable, folder ? folder->c_str() : "");
#else
    if (folder)
        setenv(data_variable, folder->c_str(), 1);
    else
        unsetenv(data_variable);
#endif
}

}  // namespace

TEST(clarity_data_moves_an_sqlite_file_into_its_folder) {
    std::string at = "/home/someone/noughts/data/noughts.db";
    CHECK_EQ(data_folder_sqlite(at, env("/copy")), (fs::path("/copy") / "noughts.db").string());
    // Unset or empty, the file is where the project put it.
    CHECK_EQ(data_folder_sqlite(at, env(nullptr)), at);
    CHECK_EQ(data_folder_sqlite(at, env("")), at);
    // A location from the environment or a secret setting is as it says, and so is SQLite's database in memory.
    CHECK_EQ(data_folder_sqlite("env:SHOP_DB", env("/copy")), "env:SHOP_DB");
    CHECK_EQ(data_folder_sqlite("setting:shop/db", env("/copy")), "setting:shop/db");
    CHECK_EQ(data_folder_sqlite(":memory:", env("/copy")), ":memory:");
}

// The process's variable, read when a database opens.
TEST(a_database_opens_in_the_folder_clarity_data_names) {
    fs::path folder = fs::temp_directory_path() / "clarity-data-cpp";
    fs::remove_all(folder);
    fs::create_directories(folder);
    std::string named = folder.string();
    set_data_folder(&named);
    fs::path elsewhere = folder / "elsewhere" / "shop.db";
    bool opened = false;
    try {
        Db db({{"main", Dialect::Sqlite, elsewhere.string()}});
        opened = true;
    } catch (...) {
    }
    set_data_folder(nullptr);
    CHECK(opened);
    CHECK(fs::exists(folder / "shop.db"));
    CHECK(!fs::exists(elsewhere));
    fs::remove_all(folder);
}
