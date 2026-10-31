// The slider and the spin box of U1c, against the cases every runtime
// shares (conformance/ranges.json).

#include "check.h"
#include "clarity/ranges.h"
#include "tests.h"

static const char* text_or_null(const cJSON* c, const char* name) {
    const cJSON* v = cJSON_GetObjectItem(c, name);
    return cJSON_IsString(v) ? v->valuestring : NULL;
}

static ClarityRange range_of(const cJSON* c) {
    const char* step = text_or_null(c, "step");
    return (ClarityRange){text_or_null(c, "min"), text_or_null(c, "max"), step ? step : "1"};
}

void every_range_case_moves_as_every_runtime_does(void) {
    cJSON* cases = conformance("ranges.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "keys")) {
        CHECK_TEXT(clarity_range_keyed(text_or_null(c, "value"), range_of(c), text_or_null(c, "key")), text_or_null(c, "after"));
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "drag")) {
        CHECK_TEXT(clarity_range_dragged(text_or_null(c, "at"), range_of(c)), text_or_null(c, "value"));
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "typed")) {
        CHECK_TEXT(clarity_range_typed(text_or_null(c, "value"), range_of(c)), text_or_null(c, "after"));
    }
    // Text that is no number, and a key that is none, say so.
    CHECK_FAILS(CLARITY_INVALID, clarity_range_typed("twelve", (ClarityRange){NULL, NULL, "1"}));
    CHECK_FAILS(CLARITY_INVALID, clarity_range_keyed("1", (ClarityRange){NULL, NULL, "1"}, "sideways"));
    cJSON_Delete(cases);
}
