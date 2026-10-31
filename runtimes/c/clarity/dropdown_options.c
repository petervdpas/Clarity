#include "clarity/dropdown_options.h"

#include <ctype.h>
#include <stdlib.h>
#include <string.h>

#include "clarity/core.h"

/// `text` in lower case, in the scratch arena.
static char* lower(const char* text) {
    char* out = clarity_copy(text);
    for (char* at = out; *at; ++at) *at = (char)tolower((unsigned char)*at);
    return out;
}

/// A value as the text it shows: text as it is, a number written out, nothing for null.
static const char* text_of(const cJSON* value) {
    if (cJSON_IsString(value)) return value->valuestring;
    if (cJSON_IsNumber(value)) return clarity_format("%.15g", value->valuedouble);
    if (cJSON_IsBool(value)) return cJSON_IsTrue(value) ? "true" : "false";
    return "";
}

/// Ignoring case first, then as written.
static int before(const void* a, const void* b) {
    const char* x = *(const char* const*)a;
    const char* y = *(const char* const*)b;
    int by_case = strcmp(lower(x), lower(y));
    return by_case ? by_case : strcmp(x, y);
}

cJSON* clarity_suggestions(const cJSON* values, const char* typed) {
    // What is typed, trimmed and in lower case.
    const char* from = typed ? typed : "";
    while (isspace((unsigned char)*from)) ++from;
    size_t length = strlen(from);
    while (length > 0 && isspace((unsigned char)from[length - 1])) --length;
    const char* wanted = lower(clarity_copy_n(from, length));
    size_t count = 0;
    const char** kept = CLARITY_NEW(const char*, cJSON_GetArraySize(values) + 1);
    const cJSON* value;
    cJSON_ArrayForEach(value, values) {
        const char* text = text_of(value);
        if (!*text || !strstr(lower(text), wanted)) continue;
        size_t i = 0;
        while (i < count && strcmp(kept[i], text) != 0) ++i;
        if (i == count) kept[count++] = text;
    }
    qsort(kept, count, sizeof *kept, before);
    cJSON* out = cJSON_CreateArray();
    for (size_t i = 0; i < count; ++i) cJSON_AddItemToArray(out, cJSON_CreateString(kept[i]));
    return out;
}

/// Whether `word` is in the option's text or in a field it shows.
static bool holds(const cJSON* option, const cJSON* show, const char* word) {
    if (strstr(lower(text_of(cJSON_GetObjectItemCaseSensitive(option, "text"))), word)) return true;
    const cJSON* fields = cJSON_GetObjectItemCaseSensitive(option, "fields");
    const cJSON* field;
    if (!show) {
        cJSON_ArrayForEach(field, fields) {
            if (strstr(lower(text_of(field)), word)) return true;
        }
        return false;
    }
    const cJSON* name;
    cJSON_ArrayForEach(name, show) {
        field = cJSON_IsString(name) ? cJSON_GetObjectItemCaseSensitive(fields, name->valuestring) : NULL;
        if (strstr(lower(text_of(field)), word)) return true;
    }
    return false;
}

cJSON* clarity_searched(const cJSON* options, const cJSON* show, const char* typed) {
    // The words typed, in lower case.
    char* words = lower(typed ? typed : "");
    const char* each[64];
    size_t count = 0;
    for (char* at = words; *at && count < sizeof each / sizeof *each;) {
        while (*at && isspace((unsigned char)*at)) *at++ = '\0';
        if (!*at) break;
        each[count++] = at;
        while (*at && !isspace((unsigned char)*at)) ++at;
    }
    cJSON* out = cJSON_CreateArray();
    const cJSON* option;
    cJSON_ArrayForEach(option, options) {
        bool all = true;
        for (size_t i = 0; i < count && all; ++i) all = holds(option, show, each[i]);
        if (all) cJSON_AddItemToArray(out, cJSON_Duplicate(option, true));
    }
    return out;
}
