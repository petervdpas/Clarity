// Secret settings in the system's protected store (DESIGN 5.27): kept, read
// back and forgotten, apart from settings.json. Needs a store, such as the
// Secret Service of a desktop session; without one it checks nothing, unless
// CLARITY_SECRET_STORE=1 says the machine has one.

#include <stdlib.h>
#include <string.h>

#include "check.h"
#include "clarity/db.h"
#include "clarity/settings.h"
#include "secrets/secrets.h"
#include "tests.h"

#define PROGRAM "clarity-runtime-test"

static bool store_works(void) {
    bool works = clarity_secret_set(PROGRAM, "probe", "1") && clarity_secret_get(PROGRAM, "probe") && strcmp(clarity_secret_get(PROGRAM, "probe"), "1") == 0;
    clarity_secret_set(PROGRAM, "probe", "");
    const char* said = getenv("CLARITY_SECRET_STORE");
    if (said && strcmp(said, "1") == 0) CHECK(works);
    return works;
}

void a_secret_is_kept_read_and_forgotten(void) {
    if (!store_works()) return;
    CHECK(clarity_secret_set(PROGRAM, "token", "s3cr3t"));
    const char* got = clarity_secret_get(PROGRAM, "token");
    CHECK(got && strcmp(got, "s3cr3t") == 0);
    CHECK(clarity_secret_set(PROGRAM, "token", ""));
    CHECK(clarity_secret_get(PROGRAM, "token") == NULL);
    CHECK(clarity_secret_set(PROGRAM, "token", ""));
}

void settings_keep_their_secrets_out_of_the_file(void) {
    if (!store_works()) return;
    clarity_secrets_use();
#ifndef _WIN32
    // Beside the tests, where they run, as the settings test keeps its own.
    setenv("XDG_CONFIG_HOME", clarity_format("clarity-secret-settings-%d", rand()), 1);
#endif
    const char* program = "clarity-runtime-test-settings";
    cJSON* values = cJSON_Parse("{\"pageSize\": 20, \"apiToken\": \"t0k3n\"}");
    cJSON* secrets = cJSON_Parse("[\"apiToken\"]");
    CHECK(clarity_keep_settings_with(program, values, secrets));
    cJSON* file = clarity_read_settings(program);
    CHECK(cJSON_GetObjectItemCaseSensitive(file, "apiToken") == NULL);
    CHECK(cJSON_GetObjectItemCaseSensitive(file, "pageSize") != NULL);
    cJSON* read = clarity_read_settings_with(program, secrets);
    const cJSON* token = cJSON_GetObjectItemCaseSensitive(read, "apiToken");
    CHECK(cJSON_IsString(token) && strcmp(token->valuestring, "t0k3n") == 0);
    clarity_secret_set(program, "apiToken", "");
    cJSON_Delete(values);
    cJSON_Delete(secrets);
    cJSON_Delete(file);
    cJSON_Delete(read);
    clarity_secret_store = (ClaritySecretStore){0};
}

// A database connects with a secret setting (DESIGN 5.27): its connection,
// password and all, kept in the protected store, not in the program.
void a_database_connects_with_a_secret_setting(void) {
    if (!store_works()) return;
    clarity_secrets_use();
    const char* file = clarity_format("%s/clarity-secret-connection-c.db", temp_folder());
    CHECK(clarity_secret_set(PROGRAM, "shopDb", file));
    ClaritySource source = {"main", CLARITY_SQLITE, "setting:" PROGRAM "/shopDb"};
    ClarityDb* db = NULL;
    CHECK(clarity_db_open(&source, 1, &db));
    clarity_secret_set(PROGRAM, "shopDb", "");
    if (db) clarity_db_close(db);
    CHECK(!clarity_db_open(&source, 1, &db));
    CHECK(strstr(clarity_error()->message, "the secret setting shopDb is not kept yet") != NULL);
    remove(file);
    clarity_secret_store = (ClaritySecretStore){0};
}
