// Lists of values as fields (U1c), kept in one column as JSON, against the
// cases every runtime shares (conformance/list-fields.json).

#include "check.h"
#include "clarity/list_fields.h"
#include "tests.h"

// The column a case's items are kept as: read from the wire, written back.
static const char* kept_as(const char* item, const cJSON* items) {
    cJSON* in = cJSON_CreateObject();
    cJSON_AddItemToObject(in, "items", cJSON_Duplicate(items, true));
    ClarityValue value = {0};
    bool ok = true;
    if (strcmp(item, "integer") == 0) {
        ClarityIntegers list = {0};
        ok = clarity_json_get_integers(in, "items", &list);
        value = clarity_integers_value(list);
    } else if (strcmp(item, "decimal") == 0) {
        ClarityDecimals list = {0};
        ok = clarity_json_get_decimals(in, "items", &list);
        value = clarity_decimals_value(list);
    } else if (strcmp(item, "date") == 0) {
        ClarityDates list = {0};
        ok = clarity_json_get_dates(in, "items", &list);
        value = clarity_dates_value(list);
    } else {
        ClarityTexts list = {0};
        ok = clarity_json_get_texts(in, "items", &list);
        value = clarity_texts_value(list);
    }
    cJSON_Delete(in);
    return ok ? value.text : NULL;
}

// The items a column holds, as they travel.
static cJSON* read_as(const char* item, ClarityValue column) {
    if (strcmp(item, "integer") == 0) {
        ClarityIntegers list = {0};
        return clarity_get_integers("tags", column, &list) ? clarity_json_integers(list) : NULL;
    }
    if (strcmp(item, "decimal") == 0) {
        ClarityDecimals list = {0};
        return clarity_get_decimals("tags", column, &list) ? clarity_json_decimals(list) : NULL;
    }
    if (strcmp(item, "date") == 0) {
        ClarityDates list = {0};
        return clarity_get_dates("tags", column, &list) ? clarity_json_dates(list) : NULL;
    }
    ClarityTexts list = {0};
    return clarity_get_texts("tags", column, &list) ? clarity_json_texts(list) : NULL;
}

void a_list_is_kept_as_compact_json(void) {
    cJSON* cases = conformance("list-fields.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "kept")) {
        CHECK_TEXT(kept_as(cJSON_GetObjectItem(c, "item")->valuestring, cJSON_GetObjectItem(c, "items")), cJSON_GetObjectItem(c, "column")->valuestring);
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "read")) {
        const cJSON* column = cJSON_GetObjectItem(c, "column");
        ClarityValue value = cJSON_IsString(column) ? clarity_text_value(column->valuestring) : clarity_null();
        cJSON* got = read_as(cJSON_GetObjectItem(c, "item")->valuestring, value);
        REQUIRE(got);
        char* expected = cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "items"));
        CHECK(check_json(got, expected));
        cJSON_free(expected);
        cJSON_Delete(got);
    }
    ClarityIntegers wrong = {0};
    CHECK(!clarity_get_integers("tags", clarity_text_value("[\"x\"]"), &wrong));
    cJSON_Delete(cases);
}

void a_typed_item_is_added_as_every_runtime_adds_it(void) {
    cJSON* cases = conformance("list-fields.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "added")) {
        cJSON* got = clarity_item_added(cJSON_GetObjectItem(c, "item")->valuestring, NULL, cJSON_GetObjectItem(c, "items"), cJSON_GetObjectItem(c, "typed")->valuestring);
        const char* typed = cJSON_GetObjectItem(c, "typed")->valuestring;
        if (cJSON_IsTrue(cJSON_GetObjectItem(c, "refused"))) {
            if (got) check_fail(__FILE__, __LINE__, clarity_format("'%s' was not refused", typed));
            continue;
        }
        if (!got || !check_json(got, cJSON_PrintUnformatted(cJSON_GetObjectItem(c, "gives"))))
            check_fail(__FILE__, __LINE__, clarity_format("'%s' gives %s", typed, got ? cJSON_PrintUnformatted(got) : "a refusal"));
    }
    // An enum's list takes only its values, and a date list only dates.
    cJSON* genres = cJSON_Parse("[\"Jazz\",\"Rock\"]");
    CHECK_JSON(clarity_item_added("enum", genres, cJSON_Parse("[\"Jazz\"]"), " Rock"), "[\"Jazz\",\"Rock\"]");
    CHECK(!clarity_item_added("enum", genres, cJSON_Parse("[\"Jazz\"]"), "Polka"));
    CHECK_JSON(clarity_item_added("date", NULL, cJSON_Parse("[]"), "2026-10-31"), "[\"2026-10-31\"]");
    CHECK(!clarity_item_added("date", NULL, cJSON_Parse("[]"), "tomorrow"));
    // A list not set yet is empty; 1.50 is the 1.5 the list has.
    CHECK_JSON(clarity_item_added("text", NULL, NULL, "jazz"), "[\"jazz\"]");
    CHECK_JSON(clarity_item_added("decimal", NULL, cJSON_Parse("[\"1.5\"]"), "1.50"), "[\"1.5\"]");
    cJSON_Delete(cases);
}
