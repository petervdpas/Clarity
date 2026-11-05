// A backup before migration steps change a database (docs/BACKUP-PLAN.md),
// named and pruned as every stack does it (conformance/backups.json).

#include <algorithm>
#include <filesystem>
#include <fstream>

#include "check.hpp"
#include "clarity/backup.hpp"
#include "clarity/db.hpp"
#include "conformance.hpp"

using namespace clarity;
namespace fs = std::filesystem;

namespace {

std::vector<std::string> names_in(const fs::path& folder) {
    std::vector<std::string> names;
    for (const auto& e : fs::directory_iterator(folder)) names.push_back(e.path().filename().string());
    std::sort(names.begin(), names.end());
    return names;
}

}  // namespace

TEST(backups_are_named_and_pruned_as_every_stack_does) {
    const auto& c = conformance("backups.json");
    for (const auto& s : c["stamps"]) CHECK_EQ(backup::stamp(s["seconds"].get<int64_t>()), s["stamp"].get<std::string>());
    for (const auto& n : c["names"])
        CHECK_EQ(backup::file_name(n["database"].get<std::string>(), n["seconds"].get<int64_t>(), n["step"].get<int>()), n["file"].get<std::string>());
    CHECK_EQ(c["kept"].get<int>(), backup::kept_automatic);
    const auto& p = c["prune"];
    auto dir = fs::temp_directory_path() / "clarity-cpp-prune";
    fs::remove_all(dir);
    fs::create_directories(dir / "backups");
    for (const auto& f : p["files"]) std::ofstream(dir / "backups" / f.get<std::string>());
    backup::prune((dir / (p["database"].get<std::string>() + ".db")).string());
    CHECK(names_in(dir / "backups") == p["left"].get<std::vector<std::string>>());
    fs::remove_all(dir);
}

TEST(a_database_with_a_schema_is_backed_up_before_new_steps) {
    auto dir = fs::temp_directory_path() / "clarity-cpp-backup";
    fs::remove_all(dir);
    fs::create_directories(dir);
    auto path = (dir / "shop.db").string();
    Step first{"CREATE TABLE item (id INTEGER PRIMARY KEY, name TEXT NOT NULL DEFAULT '')"};
    Step second{"ALTER TABLE item ADD COLUMN price INTEGER NOT NULL DEFAULT 0"};
    {
        Db db = Db::open(path);
        db.migrate({first});
        db.batch("", "INSERT INTO item (name) VALUES ('kept')");
    }
    CHECK(!fs::exists(dir / "backups"));
    {
        Db db = Db::open(path);
        CHECK_EQ(db.migrate({first, second}), 1);
    }
    auto backups = names_in(dir / "backups");
    REQUIRE(backups.size() == 1);
    CHECK(backups[0].rfind("shop-", 0) == 0);
    CHECK(backups[0].find("-before-migration-0002.db") != std::string::npos);
    {
        Db copy = Db::open((dir / "backups" / backups[0]).string());
        CHECK_EQ(copy.schema_version(), 1);
    }
    fs::remove_all(dir);
}
