// `contains` outside a query, as every stack does it (conformance/contains.json).

#include "check.h"
#include "clarity/code.h"
#include "tests.h"

void text_contains_a_part_case_ignored_as_every_stack_does(void) {
    cJSON* file = conformance("contains.json");
    REQUIRE(file);
    cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const char* text = cJSON_GetObjectItem(c, "text")->valuestring;
        const char* part = cJSON_GetObjectItem(c, "part")->valuestring;
        bool want = cJSON_IsTrue(cJSON_GetObjectItem(c, "contains"));
        const char* said = clarity_format("'%s' contains '%s': %s", text, part, clarity_text_contains(text, part) ? "yes" : "no");
        CHECK_TEXT(said, clarity_format("'%s' contains '%s': %s", text, part, want ? "yes" : "no"));
    }
}
