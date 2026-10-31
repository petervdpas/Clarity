#include "clarity/charts.h"

#include <ctype.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

#include "clarity/core.h"

typedef struct {
    const cJSON* key;
    int64_t count, numbers;
    double sum;
} Group;

/// The number a row measures, or none for an empty one.
static bool number_of(const cJSON* value, double* out) {
    if (cJSON_IsNumber(value)) {
        *out = value->valuedouble;
        return true;
    }
    if (!cJSON_IsString(value) || !*value->valuestring) return false;
    char* end;
    *out = strtod(value->valuestring, &end);
    while (isspace((unsigned char)*end)) ++end;
    return *end == '\0';
}

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

/// Where `value` is in an enum's values; -1 when it is none of them.
static int place_in(const cJSON* order, const cJSON* value) {
    int i = 0;
    const cJSON* item;
    cJSON_ArrayForEach(item, order) {
        if (cJSON_Compare(item, value, true)) return i;
        ++i;
    }
    return -1;
}

/// What groups sort by: the kind of their values, and an enum's values.
typedef struct {
    const char* by;
    const cJSON* order;
} Sorting;

/// The order of two groups by their values, as the kind of value says.
static int before(const void* a, const void* b, void* context) {
    const char* sorting_by = ((const Sorting*)context)->by;
    const cJSON* sorting_order = ((const Sorting*)context)->order;
    const cJSON* x = ((const Group*)a)->key;
    const cJSON* y = ((const Group*)b)->key;
    if (strcmp(sorting_by, "number") == 0) {
        double nx = 0, ny = 0;
        number_of(x, &nx);
        number_of(y, &ny);
        return nx < ny ? -1 : nx > ny;
    }
    if (strcmp(sorting_by, "enum") == 0) return place_in(sorting_order, x) - place_in(sorting_order, y);
    if (strcmp(sorting_by, "bool") == 0) return cJSON_IsTrue(x) - cJSON_IsTrue(y);
    const char* sx = text_of(x);
    const char* sy = text_of(y);
    if (strcmp(sorting_by, "date") != 0) {
        int by_case = strcmp(lower(sx), lower(sy));
        if (by_case) return by_case;
    }
    return strcmp(sx, sy);
}

cJSON* clarity_chart_groups(const cJSON* rows, const char* by, const char* measure, const cJSON* order) {
    Group* groups = CLARITY_NEW(Group, cJSON_GetArraySize(rows) + 1);
    size_t count = 0;
    const cJSON* row;
    cJSON_ArrayForEach(row, rows) {
        const cJSON* key = cJSON_GetObjectItemCaseSensitive(row, "by");
        // A row with no value is left out.
        if (!key || cJSON_IsNull(key) || (cJSON_IsString(key) && !*key->valuestring)) continue;
        size_t g = 0;
        while (g < count && !cJSON_Compare(groups[g].key, key, true)) ++g;
        if (g == count) groups[count++].key = key;
        groups[g].count++;
        double n;
        if (number_of(cJSON_GetObjectItemCaseSensitive(row, "of"), &n)) {
            groups[g].sum += n;
            groups[g].numbers++;
        }
    }
    Sorting sorting = {by, order};
    clarity_sort(groups, count, sizeof *groups, before, &sorting);
    cJSON* out = cJSON_CreateArray();
    for (size_t g = 0; g < count; ++g) {
        double value = strcmp(measure, "count") == 0 ? (double)groups[g].count
                       : strcmp(measure, "sum") == 0 ? groups[g].sum
                       : groups[g].numbers > 0       ? groups[g].sum / (double)groups[g].numbers
                                                     : 0;
        cJSON* group = cJSON_CreateObject();
        cJSON_AddItemToObject(group, "key", cJSON_Duplicate(groups[g].key, true));
        // Worked out to 12 decimals, as on every stack.
        cJSON_AddNumberToObject(group, "value", round(value * 1e12) / 1e12);
        cJSON_AddItemToArray(out, group);
    }
    return out;
}
