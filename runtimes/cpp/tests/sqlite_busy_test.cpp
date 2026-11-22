// Another program holding the database a moment (Clarity's data view,
// clarity-migrate, a second copy of the app) makes a write wait for it, up to
// five seconds, instead of failing with SQLITE_BUSY at once, as on every stack.

#include <filesystem>

#include "check.hpp"
#include "clarity/db.hpp"

using namespace clarity;
namespace fs = std::filesystem;

TEST(a_write_waits_while_another_program_holds_the_database) {
    auto dir = fs::temp_directory_path() / "clarity-cpp-busy";
    fs::remove_all(dir);
    {
        Db db = Db::open((dir / "shop.db").string());
        auto rows = db.rows("", "PRAGMA busy_timeout", {});
        REQUIRE(rows.size() == 1);
        CHECK_EQ(std::get<int64_t>(rows[0][0]), int64_t(5000));
    }
    fs::remove_all(dir);
}
