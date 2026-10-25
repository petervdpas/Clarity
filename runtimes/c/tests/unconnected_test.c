// Why the databases did not open (DESIGN 5.27), in Clarity's own words that
// the project translates, as every runtime says it (conformance/unconnected.json).

#include "check.h"
#include "clarity/db.h"
#include "clarity/store.h"
#include "clarity/text.h"

static void check_reason(const ClarityError* error, const cJSON* reason) {
    CHECK(check_json(clarity_text_json(clarity_unconnected_reason(error)), cJSON_PrintUnformatted(reason)));
}

void a_secret_connection_not_kept_says_so(void) {
    const cJSON* not_kept = cJSON_GetObjectItem(conformance("unconnected.json"), "notKept");
    ClaritySource source = {"main", CLARITY_SQLITE, cJSON_GetObjectItem(not_kept, "location")->valuestring};
    ClarityDb* db = NULL;
    REQUIRE(!clarity_db_open(&source, 1, &db));
    check_reason(clarity_error(), cJSON_GetObjectItem(not_kept, "reason"));
    // Its message stays as it was, for logs.
    CHECK(strstr(clarity_error()->message, "the secret setting neverKept is not kept yet") != NULL);
}

void any_other_failure_says_the_runtimes_message(void) {
    const cJSON* failed = cJSON_GetObjectItem(conformance("unconnected.json"), "failed");
    clarity_fail(CLARITY_STORAGE, "%s", cJSON_GetObjectItem(failed, "storage")->valuestring);
    check_reason(clarity_error(), cJSON_GetObjectItem(failed, "reason"));
}
