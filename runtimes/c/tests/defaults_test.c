// Clarity's defaults are the spec's (conformance/defaults.json); the copies
// this runtime carries say the same.

#include "auth/auth.h"
#include "check.h"
#include "clarity/backup.h"
#include "clarity/db.h"
#include "clarity/edits.h"
#include "http/http.h"
#include "service/serve.h"
#include "tests.h"

static double number(const cJSON* in, const char* path) {
    const cJSON* at = in;
    char name[64];
    for (const char* p = path; at && *p;) {
        size_t n = strcspn(p, ".");
        snprintf(name, sizeof name, "%.*s", (int)n, p);
        at = cJSON_GetObjectItemCaseSensitive(at, name);
        p += n + (p[n] == '.');
    }
    return cJSON_IsNumber(at) ? at->valuedouble : -1;
}

void the_defaults_are_the_spec_s(void) {
    cJSON* file = conformance("defaults.json");
    REQUIRE(file);
    CHECK(CLARITY_SIGNIN_SESSION_SECONDS == number(file, "signin.sessionMinutes") * 60);
    CHECK(CLARITY_SIGNIN_TRIES == number(file, "signin.tries"));
    CHECK(CLARITY_SIGNIN_TRIES_SECONDS == number(file, "signin.triesMinutes") * 60);
    CHECK(CLARITY_UNDO_STEPS == number(file, "undoSteps"));
    CHECK(CLARITY_KEPT_BACKUPS == number(file, "keptBackups"));
    CHECK(CLARITY_HTTP_TIMEOUT == number(file, "httpSeconds"));
    CHECK(CLARITY_OWNERS_PER_READ == number(file, "ownersPerRead"));
    CHECK(CLARITY_EDIT_BUS_KEEPS == number(file, "editBusKeeps"));
    CHECK(CLARITY_QUIET_MS == number(file, "quietMinutes") * 60 * 1000);
    cJSON_Delete(file);
}
