// Reading JSON into records (DESIGN 5.23) against the cases every runtime
// shares (conformance/json.json): what is read is fitted to the entity.

#include "check.h"
#include "clarity/json_read.h"
#include "tests.h"

static ClarityJsonIs is_of(const char* word) {
    if (strcmp(word, "integer") == 0) return CLARITY_JSON_INTEGER;
    if (strcmp(word, "decimal") == 0) return CLARITY_JSON_DECIMAL;
    if (strcmp(word, "bool") == 0) return CLARITY_JSON_BOOL;
    if (strcmp(word, "date") == 0) return CLARITY_JSON_DATE;
    if (strcmp(word, "datetime") == 0) return CLARITY_JSON_DATE_TIME;
    if (strcmp(word, "enum") == 0) return CLARITY_JSON_ENUM;
    if (strcmp(word, "values") == 0) return CLARITY_JSON_VALUES;
    if (strcmp(word, "other") == 0) return CLARITY_JSON_OTHER;
    if (strcmp(word, "record") == 0) return CLARITY_JSON_RECORD;
    if (strcmp(word, "records") == 0) return CLARITY_JSON_RECORDS;
    return CLARITY_JSON_TEXT;
}

static ClarityJsonField* fields_of(const cJSON* listed, size_t* count);

static ClarityJsonKind kind_of(const cJSON* field, const char* word) {
    ClarityJsonKind kind = {is_of(word), NULL, 0, NULL, NULL, NULL, 0, NULL};
    const cJSON* values = cJSON_GetObjectItem(field, "values");
    if (values) {
        int n = cJSON_GetArraySize(values);
        const char** words = CLARITY_NEW(const char*, n);
        for (int i = 0; i < n; i++) words[i] = cJSON_GetArrayItem(values, i)->valuestring;
        kind.values = words;
        kind.count = (size_t)n;
    }
    if (kind.is == CLARITY_JSON_VALUES) {
        ClarityJsonKind* item = CLARITY_NEW(ClarityJsonKind, 1);
        *item = kind_of(field, cJSON_GetObjectItem(field, "item")->valuestring);
        kind.item = item;
    }
    if (kind.is == CLARITY_JSON_RECORD || kind.is == CLARITY_JSON_RECORDS) {
        kind.entity = cJSON_GetObjectItem(field, "entity")->valuestring;
        kind.fields = fields_of(cJSON_GetObjectItem(field, "fields"), &kind.field_count);
        kind.base = cJSON_GetObjectItem(field, "base");
    }
    return kind;
}

static ClarityJsonField* fields_of(const cJSON* listed, size_t* count) {
    int n = cJSON_GetArraySize(listed);
    ClarityJsonField* fields = CLARITY_NEW(ClarityJsonField, n);
    for (int i = 0; i < n; i++) {
        const cJSON* f = cJSON_GetArrayItem(listed, i);
        fields[i] = (ClarityJsonField){cJSON_GetObjectItem(f, "name")->valuestring, kind_of(f, cJSON_GetObjectItem(f, "kind")->valuestring)};
    }
    *count = (size_t)n;
    return fields;
}

void json_is_fitted_as_every_runtime_fits_it(void) {
    cJSON* cases = conformance("json.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "cases")) {
        size_t count = 0;
        ClarityJsonField* fields = fields_of(cJSON_GetObjectItem(c, "fields"), &count);
        const cJSON* at = cJSON_GetObjectItem(c, "at");
        cJSON* got = NULL;
        bool fitted = clarity_json_fit(cJSON_GetObjectItem(c, "json")->valuestring, cJSON_IsString(at) ? at->valuestring : NULL, cJSON_IsTrue(cJSON_GetObjectItem(c, "many")),
                                       cJSON_GetObjectItem(c, "entity")->valuestring, fields, count, cJSON_GetObjectItem(c, "base"), &got);
        const cJSON* error = cJSON_GetObjectItem(c, "error");
        if (cJSON_IsString(error)) {
            CHECK(!fitted);
            if (!fitted) CHECK_TEXT(clarity_error()->message, error->valuestring);
        } else {
            CHECK(fitted);
            if (fitted && !cJSON_Compare(got, cJSON_GetObjectItem(c, "fitted"), 1))
                check_fail(__FILE__, __LINE__, clarity_format("%s: %s", cJSON_GetObjectItem(c, "name")->valuestring, clarity_json_of(got)));
        }
    }
    cJSON_Delete(cases);
}
