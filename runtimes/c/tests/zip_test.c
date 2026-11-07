// zip(paths, texts) alike in every runtime (conformance/zip.json).

#include <stdio.h>
#include <string.h>

#include "check.h"
#include "clarity/code.h"
#include "tests.h"

static ClarityTexts texts_of(const cJSON* list) {
    ClarityTexts texts = {CLARITY_NEW(const char*, cJSON_GetArraySize(list) + 1), (size_t)cJSON_GetArraySize(list)};
    for (int i = 0; i < cJSON_GetArraySize(list); ++i) texts.items[i] = cJSON_GetArrayItem(list, i)->valuestring;
    return texts;
}

void a_zip_is_written_as_every_runtime_writes_it(void) {
    cJSON* cases = conformance("zip.json");
    REQUIRE(cases);
    const cJSON* item;
    cJSON_ArrayForEach(item, cJSON_GetObjectItem(cases, "cases")) {
        ClarityBytes got = clarity_zip(texts_of(cJSON_GetObjectItem(item, "paths")), texts_of(cJSON_GetObjectItem(item, "texts")));
        char* hex = CLARITY_NEW(char, got.size * 2 + 1);
        for (size_t i = 0; i < got.size; ++i) snprintf(hex + 2 * i, 3, "%02x", got.data[i]);
        hex[got.size * 2] = 0;
        CHECK_TEXT(hex, cJSON_GetObjectItem(item, "hex")->valuestring);
    }
}
