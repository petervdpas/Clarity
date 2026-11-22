// A program checks its database's migration steps at start and does not migrate
// (docs/ledger/migrations.md): behind or ahead, it stops with one English line
// (conformance/migrate-check.json).

#include "check.h"
#include "clarity/db.h"
#include "clarity/store.h"
#include "apply_steps.h"
#include "tests.h"

void a_database_is_checked_as_the_cases_say(void) {
    cJSON* file = conformance("migrate-check.json");
    REQUIRE(file);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const char* source = cJSON_GetObjectItem(c, "source")->valuestring;
        int had = cJSON_GetObjectItem(c, "had")->valueint, steps = cJSON_GetObjectItem(c, "steps")->valueint;
        const cJSON* line = cJSON_GetObjectItem(c, "line");
        ClaritySource sources[] = {{source, CLARITY_SQLITE, ":memory:"}};
        ClarityDb* db = NULL;
        REQUIRE(clarity_db_open(sources, 1, &db));
        ClarityStep made[8];
        const char* tables[8][1];
        for (int n = 0; n < had; ++n) {
            tables[n][0] = clarity_format("CREATE TABLE t%d (id INTEGER PRIMARY KEY)", n);
            made[n] = (ClarityStep){tables[n], 1};
        }
        REQUIRE(apply_steps(db, source, made, (size_t)had, NULL));
        bool ok = clarity_check_steps(db, source, steps);
        if (cJSON_IsNull(line)) {
            CHECK(ok);
        } else {
            CHECK(!ok);
            CHECK(clarity_error()->kind == CLARITY_STEPS);
            CHECK_TEXT(clarity_error()->message, line->valuestring);
        }
        clarity_db_close(db);
    }
    cJSON_Delete(file);
}
