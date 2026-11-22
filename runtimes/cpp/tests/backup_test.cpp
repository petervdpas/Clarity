// A backup before migration steps change a database (docs/ledger/backups.md),
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

