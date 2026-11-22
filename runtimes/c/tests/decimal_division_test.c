// Dividing decimals as every stack divides them (conformance/decimal-division.json):
// generated C divides through clarity_decimal_div, a value error for dividing by zero.

#include "check.h"
#include "clarity/code.h"
#include "tests.h"

void decimals_divide_as_every_stack_divides_them(void) {
    cJSON* file = conformance("decimal-division.json");
    REQUIRE(file);
    cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const char* a = cJSON_GetObjectItem(c, "a")->valuestring;
        const char* b = cJSON_GetObjectItem(c, "b")->valuestring;
        ClarityDecimal q = {0};
        bool ok = clarity_decimal_div(clarity_decimal_from_text(a), clarity_decimal_from_text(b), &q);
        if (cJSON_IsTrue(cJSON_GetObjectItem(c, "error"))) {
            CHECK_TEXT(clarity_format("%s / %s: %s", a, b, ok ? "a quotient" : clarity_error()->kind == CLARITY_VALUE ? "a value error" : "another error"),
                       clarity_format("%s / %s: a value error", a, b));
            continue;
        }
        CHECK_TEXT(clarity_format("%s / %s = %s", a, b, ok ? clarity_decimal_text(q) : "an error"), clarity_format("%s / %s = %s", a, b, cJSON_GetObjectItem(c, "quotient")->valuestring));
    }
}
