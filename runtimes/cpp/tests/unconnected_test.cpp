// Why the databases did not open (DESIGN 5.27), in Clarity's own words that
// the project translates, as every runtime says it (conformance/unconnected.json).

#include <string>

#include "check.hpp"
#include "clarity/db.hpp"
#include "clarity/errors.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

TEST(a_secret_connection_not_kept_says_so) {
    auto not_kept = conformance("unconnected.json")["notKept"];
    bool said = false;
    try {
        Db db({{"main", Dialect::Sqlite, not_kept["location"].get<std::string>()}});
    } catch (const std::exception& e) {
        CHECK_EQ(json(unconnected_reason(e)), not_kept["reason"]);
        // Its message stays as it was, for logs.
        said = std::string(e.what()).find("the secret setting neverKept is not kept yet") != std::string::npos;
    }
    CHECK(said);
}

TEST(any_other_failure_says_the_runtimes_message) {
    auto failed = conformance("unconnected.json")["failed"];
    CHECK_EQ(json(unconnected_reason(StorageError(failed["storage"].get<std::string>()))), failed["reason"]);
}
