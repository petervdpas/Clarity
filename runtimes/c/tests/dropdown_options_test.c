// The dropdown's options of U1c, against the cases every runtime shares
// (conformance/dropdown-options.json).

#include "check.h"
#include "clarity/dropdown_options.h"
#include "tests.h"

void a_dropdown_offers_what_every_runtime_offers(void) {
    cJSON* cases = conformance("dropdown-options.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "suggestions")) {
        const char* typed = cJSON_GetObjectItem(c, "typed")->valuestring;
        CHECK_JSON(clarity_suggestions(cJSON_GetObjectItem(c, "values"), typed), cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "gives")));
    }
    const cJSON* search = cJSON_GetObjectItem(cases, "search");
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(search, "cases")) {
        cJSON* got = clarity_searched(cJSON_GetObjectItem(search, "options"), NULL, cJSON_GetObjectItem(c, "typed")->valuestring);
        cJSON* ids = cJSON_CreateArray();
        const cJSON* option;
        cJSON_ArrayForEach(option, got) cJSON_AddItemToArray(ids, cJSON_Duplicate(cJSON_GetObjectItem(option, "id"), true));
        CHECK_JSON(ids, cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "gives")));
    }
    // Only the fields shown are searched.
    cJSON* city = cJSON_Parse("[\"city\"]");
    cJSON* name = cJSON_Parse("[\"name\"]");
    CHECK(cJSON_GetArraySize(clarity_searched(cJSON_GetObjectItem(search, "options"), city, "gent")) == 1);
    CHECK(cJSON_GetArraySize(clarity_searched(cJSON_GetObjectItem(search, "options"), name, "gent")) == 0);
    cJSON_Delete(cases);
}
