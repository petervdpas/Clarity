#include "clarity/tables.h"

#include <ctype.h>
#include <stdlib.h>
#include <string.h>

/// What a value sorts by: a number, or text; neither when it is empty.
typedef struct {
    bool empty, number;
    double n;
    const char* text;
} Key;

static const char* text_of(const cJSON* value) {
    if (cJSON_IsString(value)) return value->valuestring;
    if (cJSON_IsNumber(value)) return clarity_format("%.15g", value->valuedouble);
    return cJSON_IsTrue(value) ? "true" : "false";
}

static char* lower(const char* text) {
    char* out = clarity_copy(text);
    for (char* at = out; *at; ++at) *at = (char)tolower((unsigned char)*at);
    return out;
}

static Key key_of(const cJSON* column, const cJSON* value) {
    Key key = {.empty = true};
    if (!value || cJSON_IsNull(value) || (cJSON_IsString(value) && !*value->valuestring)) return key;
    const cJSON* kind = cJSON_GetObjectItemCaseSensitive(column, "kind");
    const char* k = cJSON_IsString(kind) ? kind->valuestring : "text";
    if (strcmp(k, "choice") == 0) {
        // A choice sorts by the label it shows.
        const char* label = text_of(value);
        const cJSON* choice;
        cJSON_ArrayForEach(choice, cJSON_GetObjectItemCaseSensitive(column, "choices")) {
            if (cJSON_Compare(cJSON_GetArrayItem(choice, 0), value, true)) {
                const cJSON* shown = cJSON_GetArrayItem(choice, 1);
                label = cJSON_IsString(shown) ? shown->valuestring : "";
                break;
            }
        }
        if (*label) key = (Key){.text = lower(label)};
        return key;
    }
    if (strcmp(k, "number") == 0) {
        if (cJSON_IsNumber(value)) return (Key){.number = true, .n = value->valuedouble};
        if (cJSON_IsBool(value)) return (Key){.number = true, .n = cJSON_IsTrue(value)};
        char* end;
        double n = strtod(value->valuestring, &end);
        while (isspace((unsigned char)*end)) ++end;
        if (*end) return key;
        return (Key){.number = true, .n = n};
    }
    if (strcmp(k, "bool") == 0) return (Key){.number = true, .n = cJSON_IsTrue(value)};
    return (Key){.text = lower(text_of(value))};
}

typedef struct {
    const Key* keys;
    bool desc;
} Sorting;

static int shown_before(const void* a, const void* b, void* context) {
    const Sorting* sorting = context;
    const Key* x = &sorting->keys[*(const size_t*)a];
    const Key* y = &sorting->keys[*(const size_t*)b];
    // An empty value comes last either way; equal rows keep their order.
    if (x->empty || y->empty) return x->empty == y->empty ? 0 : x->empty ? 1 : -1;
    int by;
    if (x->number && y->number) by = x->n < y->n ? -1 : x->n > y->n;
    else by = strcmp(x->number ? clarity_format("%.15g", x->n) : x->text, y->number ? clarity_format("%.15g", y->n) : y->text);
    return sorting->desc ? -by : by;
}

size_t* clarity_shown_order(const cJSON* rows, const cJSON* columns, const cJSON* order) {
    size_t n = (size_t)cJSON_GetArraySize(rows);
    size_t* places = CLARITY_NEW(size_t, n + 1);
    for (size_t i = 0; i < n; ++i) places[i] = i;
    const cJSON* field = cJSON_GetObjectItemCaseSensitive(order, "field");
    if (!cJSON_IsString(field)) return places;
    const cJSON* column = NULL;
    const cJSON* c;
    cJSON_ArrayForEach(c, columns) {
        const cJSON* f = cJSON_GetObjectItemCaseSensitive(c, "field");
        if (cJSON_IsString(f) && strcmp(f->valuestring, field->valuestring) == 0) column = c;
    }
    if (!column) return places;
    Key* keys = CLARITY_NEW(Key, n + 1);
    size_t i = 0;
    const cJSON* row;
    cJSON_ArrayForEach(row, rows) keys[i++] = key_of(column, cJSON_GetObjectItemCaseSensitive(row, field->valuestring));
    Sorting sorting = {keys, cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(order, "desc"))};
    clarity_sort(places, n, sizeof *places, shown_before, &sorting);
    return places;
}

cJSON* clarity_next_order(const cJSON* order, const char* field) {
    const cJSON* was = cJSON_GetObjectItemCaseSensitive(order, "field");
    bool same = cJSON_IsString(was) && strcmp(was->valuestring, field) == 0;
    cJSON* out = cJSON_CreateObject();
    cJSON_AddStringToObject(out, "field", field);
    cJSON_AddBoolToObject(out, "desc", same && !cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(order, "desc")));
    return out;
}

ClarityPage clarity_page_of(int64_t count, int64_t size, int64_t page) {
    if (size < 1) size = 1;
    int64_t pages = count > 0 ? (count + size - 1) / size : 1;
    if (count == 0) return (ClarityPage){1, 1, 0, 0};
    int64_t shown = page < 1 ? 1 : page > pages ? pages : page;
    int64_t to = shown * size < count ? shown * size : count;
    return (ClarityPage){shown, pages, (shown - 1) * size + 1, to};
}

const char* clarity_page_text(ClarityPage page, int64_t count) {
    if (page.to == 0) return "0 / 0";
    return clarity_format("%lld-%lld / %lld", (long long)page.from, (long long)page.to, (long long)count);
}

int64_t clarity_page_with(int64_t size, int64_t at) { return at / (size < 1 ? 1 : size) + 1; }
