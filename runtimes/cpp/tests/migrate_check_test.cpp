// A program checks its database's migration steps at start and does not migrate
// (docs/ledger/migrations.md): behind or ahead, it stops with one English line
// (conformance/migrate-check.json).

#include <string>

#include "apply_steps.hpp"
#include "check.hpp"
#include "conformance.hpp"
#include "clarity/db.hpp"
#include "clarity/errors.hpp"

using namespace clarity;

TEST(a_database_is_checked_as_the_cases_say) {
    for (const auto& c : conformance("migrate-check.json")["cases"]) {
        std::string source = c["source"];
        Db db({Source{source, Dialect::Sqlite, ":memory:"}});
        std::vector<Step> made;
        for (int n = 0; n < c["had"].get<int>(); ++n) made.push_back({"CREATE TABLE t" + std::to_string(n) + " (id INTEGER PRIMARY KEY)"});
        apply_steps(db, source, made);
        std::string got = "opens";
        try {
            db.check_steps(source, c["steps"].get<int>());
        } catch (const StepsError& e) {
            got = e.what();
        }
        CHECK_EQ(got, c["line"].is_null() ? std::string("opens") : c["line"].get<std::string>());
    }
}
