// A date compared with an empty one, as every stack compares them
// (conformance/compare-empty.json): generated C orders dates through
// clarity_date_compare and clarity_date_time_compare.

#include "check.h"
#include "clarity/code.h"
#include "tests.h"

void an_empty_date_is_smallest_as_every_stack_compares_it(void) {
    cJSON* file = conformance("compare-empty.json");
    REQUIRE(file);
    static const char* const ops[] = {"<", "<=", ">", ">="};
    cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const cJSON* a = cJSON_GetObjectItem(c, "a");
        const cJSON* b = cJSON_GetObjectItem(c, "b");
        int order;
        if (!strcmp(cJSON_GetObjectItem(c, "kind")->valuestring, "date")) {
            ClarityDate x = {0}, y = {0};
            if (cJSON_IsString(a)) clarity_date_parse(a->valuestring, &x);
            if (cJSON_IsString(b)) clarity_date_parse(b->valuestring, &y);
            order = clarity_date_compare(x, y);
        } else {
            ClarityDateTime x = {0}, y = {0};
            if (cJSON_IsString(a)) clarity_date_time_parse(a->valuestring, &x);
            if (cJSON_IsString(b)) clarity_date_time_parse(b->valuestring, &y);
            order = clarity_date_time_compare(x, y);
        }
        bool got[4] = {order < 0, order <= 0, order > 0, order >= 0};
        for (int i = 0; i < 4; i++) {
            const char* said = clarity_format("%s %s %s", cJSON_IsString(a) ? a->valuestring : "none", ops[i], cJSON_IsString(b) ? b->valuestring : "none");
            CHECK_TEXT(clarity_format("%s: %s", said, got[i] ? "true" : "false"), clarity_format("%s: %s", said, cJSON_IsTrue(cJSON_GetObjectItem(c, ops[i])) ? "true" : "false"));
        }
    }
}
