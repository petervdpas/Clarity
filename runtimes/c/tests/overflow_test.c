// Whole numbers and decimals that do not fit, as every stack treats them
// (conformance/overflow.json): generated C works them out through clarity_whole_*
// and clarity_decimal_*, a value error, never a wrapped number.

#include <stdlib.h>
#include "check.h"
#include "clarity/code.h"
#include "tests.h"

static const char* whole(const char* op, const char* a, const char* b) {
    int64_t x = strtoll(a, NULL, 10), y = strtoll(b, NULL, 10), n = 0;
    bool ok = !strcmp(op, "+")     ? clarity_whole_add(x, y, &n)
              : !strcmp(op, "-")   ? clarity_whole_sub(x, y, &n)
              : !strcmp(op, "*")   ? clarity_whole_mul(x, y, &n)
              : !strcmp(op, "div") ? clarity_whole_div(x, y, &n)
              : !strcmp(op, "mod") ? clarity_whole_mod(x, y, &n)
                                   : clarity_whole_neg(x, &n);
    return ok ? clarity_format("%lld", (long long)n) : clarity_error()->kind == CLARITY_VALUE ? "a value error" : "another error";
}

static const char* decimal(const char* op, const char* a, const char* b) {
    ClarityDecimal x = clarity_decimal_from_text(a), y = clarity_decimal_from_text(b), d = {0};
    bool ok = !strcmp(op, "+")   ? clarity_decimal_add(x, y, &d)
              : !strcmp(op, "-") ? clarity_decimal_sub(x, y, &d)
              : !strcmp(op, "*") ? clarity_decimal_mul(x, y, &d)
                                 : (d = clarity_decimal_neg(x), true);
    return ok ? clarity_decimal_text(d) : clarity_error()->kind == CLARITY_VALUE ? "a value error" : "another error";
}

void numbers_that_do_not_fit_are_a_value_error(void) {
    cJSON* file = conformance("overflow.json");
    REQUIRE(file);
    cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const char* kind = cJSON_GetObjectItem(c, "kind")->valuestring;
        const char* op = cJSON_GetObjectItem(c, "op")->valuestring;
        const char* a = cJSON_GetObjectItem(c, "a")->valuestring;
        cJSON* bItem = cJSON_GetObjectItem(c, "b");
        const char* b = bItem ? bItem->valuestring : "0";
        const char* got = !strcmp(kind, "whole") ? whole(op, a, b) : decimal(op, a, b);
        const char* want = cJSON_IsTrue(cJSON_GetObjectItem(c, "error")) ? "a value error" : cJSON_GetObjectItem(c, "result")->valuestring;
        CHECK_TEXT(clarity_format("%s %s %s %s = %s", kind, a, op, b, got), clarity_format("%s %s %s %s = %s", kind, a, op, b, want));
    }
}
