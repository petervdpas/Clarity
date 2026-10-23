// Where an application keeps its settings, as every runtime finds it
// (conformance/settings.json, `file`), and keeping them.

#include <stdlib.h>

#include "check.h"
#include "clarity/settings.h"
#include "tests.h"

static const char* from_case(const char* name, void* context) {
    const cJSON* value = cJSON_GetObjectItemCaseSensitive((const cJSON*)context, name);
    return cJSON_IsString(value) ? value->valuestring : NULL;
}

void the_settings_file_is_where_every_runtime_puts_it(void) {
    cJSON* cases = conformance("settings.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "file")) {
        const char* found = clarity_settings_file_in(cJSON_GetObjectItem(c, "os")->valuestring, from_case, cJSON_GetObjectItem(c, "env"), cJSON_GetObjectItem(c, "program")->valuestring);
        const cJSON* wanted = cJSON_GetObjectItem(c, "file");
        if (cJSON_IsNull(wanted)) CHECK(found == NULL);
        else CHECK_TEXT(found, wanted->valuestring);
    }
    cJSON_Delete(cases);
}

void settings_kept_are_read_back(void) {
#ifndef _WIN32
    // Beside the tests, where they run.
    const char* folder = clarity_format("clarity-settings-%d", rand());
    setenv("XDG_CONFIG_HOME", folder, 1);
#endif
    const char* program = "clarity-settings-test";
    cJSON* none = clarity_read_settings(program);
    CHECK(cJSON_GetArraySize(none) == 0);
    cJSON_Delete(none);
    cJSON* values = cJSON_Parse("{\"pageSize\":20,\"language\":\"nl\"}");
    REQUIRE(clarity_keep_settings(program, values));
    cJSON* kept = clarity_read_settings(program);
    CHECK(cJSON_Compare(kept, values, 1));
    cJSON_Delete(kept);
    cJSON_Delete(values);
}
