#include "clarity/list_fields.h"

#include <ctype.h>
#include <stdlib.h>
#include <string.h>

static bool unfit(const char* name) { return clarity_fail(CLARITY_INVALID, "%s is no list of its values", name); }

// One item of a list's JSON as its value; false when it is none of its kind.
typedef bool (*ItemRead)(const cJSON* item, void* out);

static bool text_item(const cJSON* item, void* out) {
    if (!cJSON_IsString(item)) return false;
    *(const char**)out = clarity_copy(item->valuestring);
    return true;
}

static bool integer_item(const cJSON* item, void* out) {
    if (!cJSON_IsNumber(item)) return false;
    *(int64_t*)out = (int64_t)item->valuedouble;
    return true;
}

static bool decimal_item(const cJSON* item, void* out) {
    if (cJSON_IsString(item)) return clarity_decimal_parse(item->valuestring, out);
    if (!cJSON_IsNumber(item)) return false;
    return clarity_decimal_parse(clarity_format("%.15g", item->valuedouble), out);
}

static bool date_item(const cJSON* item, void* out) {
    if (cJSON_IsNull(item)) {
        *(ClarityDate*)out = (ClarityDate){0};
        return true;
    }
    return cJSON_IsString(item) && clarity_date_parse(item->valuestring, out);
}

// The items of a JSON array, each `size` bytes, in the scratch arena.
static bool items_of(const cJSON* array, size_t size, ItemRead read, const char* name, void** items, size_t* count) {
    if (!cJSON_IsArray(array)) return unfit(name);
    size_t n = (size_t)cJSON_GetArraySize(array);
    char* at = clarity_alloc(size * (n ? n : 1));
    size_t i = 0;
    const cJSON* item;
    cJSON_ArrayForEach(item, array) {
        if (!read(item, at + size * i++)) return unfit(name);
    }
    *items = at;
    *count = n;
    return true;
}

// The items a column holds: none for null or empty text.
static bool column_items(const char* column, ClarityValue value, size_t size, ItemRead read, void** items, size_t* count) {
    const char* text = NULL;
    if (!clarity_get_text(column, value, &text)) return false;
    if (!text || text[strspn(text, " \t\r\n")] == '\0') {
        *items = clarity_alloc(size);
        *count = 0;
        return true;
    }
    cJSON* array = cJSON_Parse(text);
    bool ok = items_of(array, size, read, column, items, count);
    cJSON_Delete(array);
    return ok;
}

// The items of field `name` of `in`, when it is there.
static bool given_items(const cJSON* in, const char* name, size_t size, ItemRead read, void** items, size_t* count) {
    const cJSON* array = clarity_json_given(in, name);
    if (!array) return true;
    return items_of(array, size, read, name, items, count);
}

// An array as its column keeps it; JSON lives in the scratch arena.
static ClarityValue printed(cJSON* array) {
    char* text = cJSON_PrintUnformatted(array);
    return clarity_text_value(text ? text : "[]");
}

cJSON* clarity_json_texts(ClarityTexts list) {
    cJSON* out = cJSON_CreateArray();
    for (size_t i = 0; i < list.count; ++i) cJSON_AddItemToArray(out, clarity_json_text(list.items[i]));
    return out;
}

cJSON* clarity_json_integers(ClarityIntegers list) {
    cJSON* out = cJSON_CreateArray();
    for (size_t i = 0; i < list.count; ++i) cJSON_AddItemToArray(out, clarity_json_integer(list.items[i]));
    return out;
}

cJSON* clarity_json_decimals(ClarityDecimals list) {
    cJSON* out = cJSON_CreateArray();
    for (size_t i = 0; i < list.count; ++i) cJSON_AddItemToArray(out, clarity_json_decimal(list.items[i]));
    return out;
}

cJSON* clarity_json_dates(ClarityDates list) {
    cJSON* out = cJSON_CreateArray();
    for (size_t i = 0; i < list.count; ++i) cJSON_AddItemToArray(out, clarity_json_date(list.items[i]));
    return out;
}

ClarityValue clarity_texts_value(ClarityTexts list) { return printed(clarity_json_texts(list)); }
ClarityValue clarity_integers_value(ClarityIntegers list) { return printed(clarity_json_integers(list)); }
ClarityValue clarity_decimals_value(ClarityDecimals list) { return printed(clarity_json_decimals(list)); }
ClarityValue clarity_dates_value(ClarityDates list) { return printed(clarity_json_dates(list)); }

bool clarity_get_texts(const char* column, ClarityValue value, ClarityTexts* out) {
    return column_items(column, value, sizeof(const char*), text_item, (void**)&out->items, &out->count);
}

bool clarity_get_integers(const char* column, ClarityValue value, ClarityIntegers* out) {
    return column_items(column, value, sizeof(int64_t), integer_item, (void**)&out->items, &out->count);
}

bool clarity_get_decimals(const char* column, ClarityValue value, ClarityDecimals* out) {
    return column_items(column, value, sizeof(ClarityDecimal), decimal_item, (void**)&out->items, &out->count);
}

bool clarity_get_dates(const char* column, ClarityValue value, ClarityDates* out) {
    return column_items(column, value, sizeof(ClarityDate), date_item, (void**)&out->items, &out->count);
}

bool clarity_json_get_texts(const cJSON* in, const char* name, ClarityTexts* out) {
    return given_items(in, name, sizeof(const char*), text_item, (void**)&out->items, &out->count);
}

bool clarity_json_get_integers(const cJSON* in, const char* name, ClarityIntegers* out) {
    return given_items(in, name, sizeof(int64_t), integer_item, (void**)&out->items, &out->count);
}

bool clarity_json_get_decimals(const cJSON* in, const char* name, ClarityDecimals* out) {
    return given_items(in, name, sizeof(ClarityDecimal), decimal_item, (void**)&out->items, &out->count);
}

bool clarity_json_get_dates(const cJSON* in, const char* name, ClarityDates* out) {
    return given_items(in, name, sizeof(ClarityDate), date_item, (void**)&out->items, &out->count);
}

/// Whether `text` is digits, after a minus, with decimals after a point when `point`.
static bool is_number(const char* text, bool point) {
    const char* at = text + (*text == '-');
    if (!isdigit((unsigned char)*at)) return false;
    while (isdigit((unsigned char)*at)) ++at;
    if (point && *at == '.') {
        if (!isdigit((unsigned char)*++at)) return false;
        while (isdigit((unsigned char)*at)) ++at;
    }
    return *at == '\0';
}

/// Whether two items of a list of `item` are the same value: 1.50 is 1.5.
static bool same_item(const char* item, const cJSON* a, const cJSON* b) {
    if (strcmp(item, "decimal") == 0) {
        ClarityDecimal x, y;
        return decimal_item(a, &x) && decimal_item(b, &y) && clarity_decimal_compare(x, y) == 0;
    }
    return cJSON_Compare(a, b, true);
}

cJSON* clarity_item_added(const char* item, const cJSON* values, const cJSON* items, const char* typed) {
    cJSON* list = cJSON_IsArray(items) ? cJSON_Duplicate(items, true) : cJSON_CreateArray();
    // The typed text, trimmed.
    const char* from = typed ? typed : "";
    while (isspace((unsigned char)*from)) ++from;
    size_t length = strlen(from);
    while (length > 0 && isspace((unsigned char)from[length - 1])) --length;
    const char* text = clarity_copy_n(from, length);
    if (!*text) return list;
    cJSON* added;
    if (strcmp(item, "integer") == 0) {
        if (!is_number(text, false)) return NULL;
        added = cJSON_CreateNumber((double)strtoll(text, NULL, 10));
    } else if (strcmp(item, "decimal") == 0) {
        if (!is_number(text, true)) return NULL;
        added = cJSON_CreateString(text);
    } else if (strcmp(item, "date") == 0) {
        ClarityDate date;
        if (!clarity_date_parse(text, &date)) return NULL;
        added = cJSON_CreateString(text);
    } else {
        added = cJSON_CreateString(text);
        // An enum's list takes only its values.
        if (strcmp(item, "enum") == 0 && cJSON_IsArray(values)) {
            const cJSON* value;
            bool known = false;
            cJSON_ArrayForEach(value, values) known = known || cJSON_Compare(value, added, true);
            if (!known) return NULL;
        }
    }
    const cJSON* had;
    cJSON_ArrayForEach(had, list) {
        if (same_item(item, had, added)) return list;
    }
    cJSON_AddItemToArray(list, added);
    return list;
}
