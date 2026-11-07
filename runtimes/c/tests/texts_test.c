// Text functions alike in every runtime (conformance/texts.json).

#include <string.h>

#include "check.h"
#include "clarity/code.h"
#include "tests.h"

static const char* arg_text(const cJSON* args, int i) { return cJSON_GetArrayItem(args, i)->valuestring; }
static int64_t arg_number(const cJSON* args, int i) { return (int64_t)cJSON_GetArrayItem(args, i)->valuedouble; }

static void check_texts(ClarityTexts got, const cJSON* want, const char* function) {
    CHECK_INT((int64_t)got.count, (int64_t)cJSON_GetArraySize(want));
    for (size_t i = 0; i < got.count && (int)i < cJSON_GetArraySize(want); ++i) CHECK_TEXT(got.items[i], cJSON_GetArrayItem(want, (int)i)->valuestring);
    (void)function;
}

void text_functions_read_as_every_runtime_reads_them(void) {
    cJSON* cases = conformance("texts.json");
    REQUIRE(cases);
    const cJSON* item;
    cJSON_ArrayForEach(item, cJSON_GetObjectItem(cases, "cases")) {
        const char* function = cJSON_GetObjectItem(item, "function")->valuestring;
        const cJSON* a = cJSON_GetObjectItem(item, "args");
        const cJSON* want = cJSON_GetObjectItem(item, "result");
        if (strcmp(function, "words") == 0) check_texts(clarity_text_words(arg_text(a, 0)), want, function);
        else if (strcmp(function, "split") == 0) check_texts(clarity_text_split(arg_text(a, 0), arg_text(a, 1)), want, function);
        else if (strcmp(function, "trim") == 0) CHECK_TEXT(clarity_text_trim(arg_text(a, 0)), want->valuestring);
        else if (strcmp(function, "lower") == 0) CHECK_TEXT(clarity_text_lower(arg_text(a, 0)), want->valuestring);
        else if (strcmp(function, "upper") == 0) CHECK_TEXT(clarity_text_upper(arg_text(a, 0)), want->valuestring);
        else if (strcmp(function, "replace") == 0) CHECK_TEXT(clarity_text_replace(arg_text(a, 0), arg_text(a, 1), arg_text(a, 2)), want->valuestring);
        else if (strcmp(function, "startsWith") == 0) CHECK(clarity_text_starts_with(arg_text(a, 0), arg_text(a, 1)) == cJSON_IsTrue(want));
        else if (strcmp(function, "endsWith") == 0) CHECK(clarity_text_ends_with(arg_text(a, 0), arg_text(a, 1)) == cJSON_IsTrue(want));
        else if (strcmp(function, "slice") == 0) CHECK_TEXT(clarity_text_slice(arg_text(a, 0), arg_number(a, 1), arg_number(a, 2)), want->valuestring);
        else if (strcmp(function, "indexOf") == 0) CHECK_INT(clarity_text_index_of(arg_text(a, 0), arg_text(a, 1)), (int64_t)want->valuedouble);
    }
}
