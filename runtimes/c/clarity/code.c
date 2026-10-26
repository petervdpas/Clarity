#include "clarity/code.h"

#include <stdlib.h>
#include <string.h>

const char* clarity_show_integer(int64_t value) { return clarity_format("%lld", (long long)value); }
const char* clarity_show_decimal(ClarityDecimal value) { return clarity_decimal_text(value); }
const char* clarity_show_bool(bool value) { return value ? "yes" : "no"; }
const char* clarity_show_date(ClarityDate value) { return clarity_date_text(value); }

const char* clarity_show_date_time(ClarityDateTime value) {
    if (clarity_date_time_is_none(value)) return "";
    return clarity_format("%04d-%02d-%02d %02d:%02d", value.year, value.month, value.day, value.hour, value.minute);
}

const char* clarity_join(const char* a, const char* b) { return clarity_format("%s%s", a ? a : "", b ? b : ""); }

bool clarity_same_text(const char* a, const char* b) { return strcmp(a ? a : "", b ? b : "") == 0; }

ClarityFileList clarity_file_list(size_t count) { return (ClarityFileList){CLARITY_NEW(ClarityFile, count ? count : 1), 0}; }

ClarityFileOrder clarity_asc(const char* detail) { return (ClarityFileOrder){detail, false}; }
ClarityFileOrder clarity_desc(const char* detail) { return (ClarityFileOrder){detail, true}; }

static int compare_files(const ClarityFile* a, const ClarityFile* b, const char* detail) {
    if (strcmp(detail, "name") == 0) return strcmp(a->name, b->name);
    if (strcmp(detail, "extension") == 0) return strcmp(a->extension, b->extension);
    if (strcmp(detail, "size") == 0) return a->size < b->size ? -1 : a->size > b->size;
    if (strcmp(detail, "modified") == 0) return clarity_date_time_compare(a->modified, b->modified);
    return strcmp(a->path, b->path);
}

static int sign(int n) { return n < 0 ? -1 : n > 0; }

void clarity_sort_files(ClarityFileList* files, size_t count, const ClarityFileOrder* keys) {
    // Insertion sort: stable, and file lists are short.
    for (size_t i = 1; i < files->count; ++i) {
        ClarityFile item = files->items[i];
        size_t at = i;
        while (at > 0) {
            int c = 0;
            for (size_t k = 0; k < count && c == 0; ++k) {
                c = sign(compare_files(&files->items[at - 1], &item, keys[k].detail));
                if (keys[k].descending) c = -c;
            }
            if (c <= 0) break;
            files->items[at] = files->items[at - 1];
            --at;
        }
        files->items[at] = item;
    }
}

bool clarity_outside(int64_t position, size_t count) {
    return clarity_fail(CLARITY_VALUE, "there is no item %lld in a list of %zu", (long long)position, count);
}

// The lists of each kind of value alike: made, read, set and kept.
#define CLARITY_VALUES(List, Item, name)                                                  \
    List clarity_##name(size_t count, const Item* items) {                                \
        List list = {CLARITY_NEW(Item, count ? count : 1), count};                        \
        if (count) memcpy(list.items, items, sizeof(Item) * count);                       \
        return list;                                                                      \
    }                                                                                     \
    bool clarity_##name##_at(List list, int64_t position, Item* out) {                    \
        if (position < 1 || (uint64_t)position > list.count) return clarity_outside(position, list.count); \
        *out = list.items[position - 1];                                                  \
        return true;                                                                      \
    }                                                                                     \
    bool clarity_##name##_set(List list, int64_t position, Item value) {                  \
        if (position < 1 || (uint64_t)position > list.count) return clarity_outside(position, list.count); \
        list.items[position - 1] = value;                                                 \
        return true;                                                                      \
    }                                                                                     \
    List clarity_##name##_join(List a, List b) {                                          \
        List list = {CLARITY_NEW(Item, a.count + b.count ? a.count + b.count : 1), a.count + b.count}; \
        if (a.count) memcpy(list.items, a.items, sizeof(Item) * a.count);                 \
        if (b.count) memcpy(list.items + a.count, b.items, sizeof(Item) * b.count);       \
        return list;                                                                      \
    }

CLARITY_VALUES(ClarityIntegers, int64_t, integers)
CLARITY_VALUES(ClarityDecimals, ClarityDecimal, decimals)
CLARITY_VALUES(ClarityBools, bool, bools)
CLARITY_VALUES(ClarityDates, ClarityDate, dates)

bool clarity_integers_holds(ClarityIntegers list, int64_t item) {
    for (size_t i = 0; i < list.count; ++i) if (list.items[i] == item) return true;
    return false;
}

bool clarity_decimals_holds(ClarityDecimals list, ClarityDecimal item) {
    for (size_t i = 0; i < list.count; ++i) if (clarity_decimal_compare(list.items[i], item) == 0) return true;
    return false;
}

bool clarity_bools_holds(ClarityBools list, bool item) {
    for (size_t i = 0; i < list.count; ++i) if (list.items[i] == item) return true;
    return false;
}

bool clarity_texts_holds(ClarityTexts list, const char* item) {
    for (size_t i = 0; i < list.count; ++i) if (clarity_same_text(list.items[i], item)) return true;
    return false;
}

bool clarity_dates_holds(ClarityDates list, ClarityDate item) {
    for (size_t i = 0; i < list.count; ++i)
        if (list.items[i].year == item.year && list.items[i].month == item.month && list.items[i].day == item.day) return true;
    return false;
}

ClarityTexts clarity_texts(size_t count, const char* const* items) {
    ClarityTexts list = {CLARITY_NEW(const char*, count ? count : 1), count};
    if (count) memcpy(list.items, items, sizeof(const char*) * count);
    return list;
}

ClarityTexts clarity_texts_join(ClarityTexts a, ClarityTexts b) {
    ClarityTexts list = {CLARITY_NEW(const char*, a.count + b.count ? a.count + b.count : 1), a.count + b.count};
    if (a.count) memcpy(list.items, a.items, sizeof(const char*) * a.count);
    if (b.count) memcpy(list.items + a.count, b.items, sizeof(const char*) * b.count);
    return list;
}

bool clarity_texts_at(ClarityTexts list, int64_t position, const char** out) {
    if (position < 1 || (uint64_t)position > list.count) return clarity_outside(position, list.count);
    *out = list.items[position - 1];
    return true;
}

bool clarity_texts_set(ClarityTexts list, int64_t position, const char* value) {
    if (position < 1 || (uint64_t)position > list.count) return clarity_outside(position, list.count);
    list.items[position - 1] = value;
    return true;
}

