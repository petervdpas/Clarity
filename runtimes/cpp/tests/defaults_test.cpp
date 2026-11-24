// Clarity's defaults are the spec's (conformance/defaults.json); the copies
// this runtime carries say the same.

#include "check.hpp"
#include "conformance.hpp"
#include "clarity/db.hpp"
#include "clarity/edits.hpp"
#include "clarity/people.hpp"
#include "http/http.hpp"
#include "shell/serve.hpp"

using namespace clarity;

TEST(the_defaults_are_the_spec_s) {
    auto file = conformance("defaults.json");
    CHECK_EQ(signin_session_seconds, file["signin"]["sessionMinutes"].get<int64_t>() * 60);
    CHECK_EQ(signin_tries, file["signin"]["tries"].get<int64_t>());
    CHECK_EQ(signin_tries_seconds, file["signin"]["triesMinutes"].get<int64_t>() * 60);
    CHECK_EQ(int64_t(undo_steps), file["undoSteps"].get<int64_t>());
    CHECK_EQ(int64_t(HTTP_TIMEOUT), file["httpSeconds"].get<int64_t>());
    CHECK_EQ(int64_t(owners_per_read), file["ownersPerRead"].get<int64_t>());
    CHECK_EQ(int64_t(edit_bus_keeps), file["editBusKeeps"].get<int64_t>());
    CHECK_EQ(int64_t(std::chrono::duration_cast<std::chrono::minutes>(shell::Server::quiet_by_default).count()), file["quietMinutes"].get<int64_t>());
}
