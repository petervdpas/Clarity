// A value written into JSON as every stack writes it for parts, reports, the
// wire and `json of` (conformance/values-json.json): the clarity_json_*
// writers a generated record's JSON is made of, printed as a program prints it.

#include <stdlib.h>

#include "check.h"
#include "clarity/code.h"
#include "clarity/json.h"
#include "tests.h"

/// The JSON a generated record writes for a field of `kind` holding `value`.
static cJSON* written(const char* kind, const cJSON* value) {
    const char* text = cJSON_IsString(value) ? value->valuestring : NULL;
    if (!strcmp(kind, "decimal") || !strcmp(kind, "money")) return clarity_json_decimal(clarity_decimal_from_text(text));
    if (!strcmp(kind, "integer")) return clarity_json_integer(strtoll(text, NULL, 10));
    if (!strcmp(kind, "date")) {
        ClarityDate d = {0};
        if (text) clarity_date_parse(text, &d);
        return clarity_json_date(d);
    }
    if (!strcmp(kind, "datetime")) {
        ClarityDateTime d = {0};
        if (text) clarity_date_time_parse(text, &d);
        return clarity_json_date_time(d);
    }
    if (!strcmp(kind, "ref")) return clarity_json_reference(text ? strtoll(text, NULL, 10) : 0);
    if (!strcmp(kind, "bool")) return clarity_json_bool(cJSON_IsTrue(value));
    if (!strcmp(kind, "text")) return clarity_json_text(text);
    if (!strcmp(kind, "list")) return cJSON_CreateArray();
    return NULL;
}

void every_value_is_written_as_every_stack_writes_it(void) {
    cJSON* file = conformance("values-json.json");
    REQUIRE(file);
    cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        cJSON* json = written(cJSON_GetObjectItem(c, "kind")->valuestring, cJSON_GetObjectItem(c, "value"));
        REQUIRE(json);
        char* printed = cJSON_PrintUnformatted(json);
        const char* got = clarity_format("%s: %s", cJSON_GetObjectItem(c, "name")->valuestring, printed);
        const char* want = clarity_format("%s: %s", cJSON_GetObjectItem(c, "name")->valuestring, cJSON_GetObjectItem(c, "json")->valuestring);
        CHECK_TEXT(got, want);
        cJSON_Delete(json);
    }
}
