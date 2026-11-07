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


int64_t clarity_snapped(int64_t value, int64_t step) {
    if (step <= 0) return value;
    int64_t n = 2 * value + step, q = n / (2 * step);
    if (n % (2 * step) != 0 && n < 0) --q;
    return q * step;
}

ClarityIntegers clarity_range(int64_t from, int64_t to) {
    size_t count = to < from ? 0 : (size_t)(to - from) + 1;
    ClarityIntegers list = {clarity_alloc(sizeof(int64_t) * (count ? count : 1)), count};
    for (size_t i = 0; i < count; ++i) list.items[i] = from + (int64_t)i;
    return list;
}

int64_t clarity_length(const char* text) {
    int64_t count = 0;
    for (const unsigned char* at = (const unsigned char*)text; at && *at; ++at)
        if ((*at & 0xC0) != 0x80) ++count;
    return count;
}

const char* clarity_joined(ClarityTexts texts, const char* separator) {
    size_t gap = strlen(separator), size = 1;
    for (size_t i = 0; i < texts.count; ++i) size += strlen(texts.items[i]) + (i ? gap : 0);
    char* out = clarity_alloc(size);
    char* at = out;
    for (size_t i = 0; i < texts.count; ++i) {
        if (i) {
            memcpy(at, separator, gap);
            at += gap;
        }
        size_t length = strlen(texts.items[i]);
        memcpy(at, texts.items[i], length);
        at += length;
    }
    *at = '\0';
    return out;
}

/* The text functions of code (conformance/texts.json): characters counted
 * once however many bytes they take in UTF-8, positions from 1. */

/* The code point at `at` and how many bytes it takes; a broken sequence is one byte. */
static uint32_t utf8_at(const unsigned char* at, size_t* size) {
    if (at[0] < 0x80) { *size = 1; return at[0]; }
    if ((at[0] & 0xE0) == 0xC0 && (at[1] & 0xC0) == 0x80) { *size = 2; return ((uint32_t)(at[0] & 0x1F) << 6) | (at[1] & 0x3F); }
    if ((at[0] & 0xF0) == 0xE0 && (at[1] & 0xC0) == 0x80 && (at[2] & 0xC0) == 0x80) { *size = 3; return ((uint32_t)(at[0] & 0x0F) << 12) | ((uint32_t)(at[1] & 0x3F) << 6) | (at[2] & 0x3F); }
    if ((at[0] & 0xF8) == 0xF0 && (at[1] & 0xC0) == 0x80 && (at[2] & 0xC0) == 0x80 && (at[3] & 0xC0) == 0x80) { *size = 4; return ((uint32_t)(at[0] & 0x07) << 18) | ((uint32_t)(at[1] & 0x3F) << 12) | ((uint32_t)(at[2] & 0x3F) << 6) | (at[3] & 0x3F); }
    *size = 1;
    return at[0];
}

static size_t utf8_put(uint32_t cp, char* out) {
    if (cp < 0x80) { out[0] = (char)cp; return 1; }
    if (cp < 0x800) { out[0] = (char)(0xC0 | (cp >> 6)); out[1] = (char)(0x80 | (cp & 0x3F)); return 2; }
    if (cp < 0x10000) { out[0] = (char)(0xE0 | (cp >> 12)); out[1] = (char)(0x80 | ((cp >> 6) & 0x3F)); out[2] = (char)(0x80 | (cp & 0x3F)); return 3; }
    out[0] = (char)(0xF0 | (cp >> 18)); out[1] = (char)(0x80 | ((cp >> 12) & 0x3F)); out[2] = (char)(0x80 | ((cp >> 6) & 0x3F)); out[3] = (char)(0x80 | (cp & 0x3F));
    return 4;
}

static bool is_space(uint32_t cp) {
    return cp == ' ' || (cp >= 0x09 && cp <= 0x0D) || cp == 0x85 || cp == 0xA0 || cp == 0x1680 || (cp >= 0x2000 && cp <= 0x200A) || cp == 0x2028 || cp == 0x2029 || cp == 0x202F || cp == 0x205F || cp == 0x3000;
}

static uint32_t to_lower(uint32_t c) {
    if (c >= 'A' && c <= 'Z') return c + 32;
    if (c >= 0xC0 && c <= 0xDE && c != 0xD7) return c + 32;
    if ((c >= 0x100 && c <= 0x137) || (c >= 0x14A && c <= 0x177)) return c % 2 == 0 ? c + 1 : c;
    if ((c >= 0x139 && c <= 0x148) || (c >= 0x179 && c <= 0x17E)) return c % 2 == 1 ? c + 1 : c;
    if (c == 0x178) return 0xFF;
    if (c >= 0x391 && c <= 0x3A9 && c != 0x3A2) return c + 32;
    if (c == 0x386) return 0x3AC;
    if (c >= 0x388 && c <= 0x38A) return c + 37;
    if (c == 0x38C) return 0x3CC;
    if (c == 0x38E || c == 0x38F) return c + 63;
    if (c >= 0x410 && c <= 0x42F) return c + 32;
    if (c >= 0x400 && c <= 0x40F) return c + 80;
    return c;
}

static uint32_t to_upper(uint32_t c) {
    if (c >= 'a' && c <= 'z') return c - 32;
    if (c >= 0xE0 && c <= 0xFE && c != 0xF7) return c - 32;
    if (c == 0xFF) return 0x178;
    if ((c >= 0x100 && c <= 0x137) || (c >= 0x14A && c <= 0x177)) return c % 2 == 1 ? c - 1 : c;
    if ((c >= 0x139 && c <= 0x148) || (c >= 0x179 && c <= 0x17E)) return c % 2 == 0 ? c - 1 : c;
    if (c == 0x3C2) return 0x3A3;
    if (c >= 0x3B1 && c <= 0x3C9) return c - 32;
    if (c == 0x3AC) return 0x386;
    if (c >= 0x3AD && c <= 0x3AF) return c - 37;
    if (c == 0x3CC) return 0x38C;
    if (c == 0x3CD || c == 0x3CE) return c - 63;
    if (c >= 0x430 && c <= 0x44F) return c - 32;
    if (c >= 0x450 && c <= 0x45F) return c - 80;
    return c;
}

static const char* copied(const char* from, size_t length) {
    char* out = clarity_alloc(length + 1);
    memcpy(out, from, length);
    out[length] = '\0';
    return out;
}

/* A list grown one text at a time. */
static void texts_add(ClarityTexts* list, size_t* room, const char* text) {
    if (list->count == *room) {
        size_t more = *room ? *room * 2 : 8;
        const char** items = CLARITY_NEW(const char*, more);
        if (list->count) memcpy(items, list->items, sizeof(const char*) * list->count);
        list->items = items;
        *room = more;
    }
    list->items[list->count++] = text;
}

ClarityTexts clarity_text_words(const char* text) {
    ClarityTexts list = {NULL, 0};
    size_t room = 0;
    const unsigned char* at = (const unsigned char*)(text ? text : "");
    const unsigned char* start = NULL;
    while (*at) {
        size_t size;
        uint32_t cp = utf8_at(at, &size);
        if (is_space(cp)) {
            if (start) texts_add(&list, &room, copied((const char*)start, (size_t)(at - start)));
            start = NULL;
        } else if (!start) {
            start = at;
        }
        at += size;
    }
    if (start) texts_add(&list, &room, copied((const char*)start, (size_t)(at - start)));
    if (!list.items) list.items = CLARITY_NEW(const char*, 1);
    return list;
}

ClarityTexts clarity_text_split(const char* text, const char* separator) {
    ClarityTexts list = {NULL, 0};
    size_t room = 0;
    text = text ? text : "";
    size_t gap = strlen(separator);
    if (gap == 0) {
        for (const unsigned char* at = (const unsigned char*)text; *at;) {
            size_t size;
            utf8_at(at, &size);
            texts_add(&list, &room, copied((const char*)at, size));
            at += size;
        }
    } else {
        const char* start = text;
        for (const char* found; (found = strstr(start, separator)); start = found + gap) texts_add(&list, &room, copied(start, (size_t)(found - start)));
        texts_add(&list, &room, copied(start, strlen(start)));
    }
    if (!list.items) list.items = CLARITY_NEW(const char*, 1);
    return list;
}

const char* clarity_text_trim(const char* text) {
    const unsigned char* at = (const unsigned char*)(text ? text : "");
    const unsigned char* first = NULL;
    const unsigned char* end = at;
    while (*at) {
        size_t size;
        uint32_t cp = utf8_at(at, &size);
        if (!is_space(cp)) {
            if (!first) first = at;
            end = at + size;
        }
        at += size;
    }
    return first ? copied((const char*)first, (size_t)(end - first)) : "";
}

static const char* cased(const char* text, uint32_t (*change)(uint32_t)) {
    text = text ? text : "";
    char* out = clarity_alloc(strlen(text) * 2 + 1);
    size_t length = 0;
    for (const unsigned char* at = (const unsigned char*)text; *at;) {
        size_t size;
        uint32_t cp = utf8_at(at, &size);
        length += utf8_put(change(cp), out + length);
        at += size;
    }
    out[length] = '\0';
    return out;
}

const char* clarity_text_lower(const char* text) { return cased(text, to_lower); }

const char* clarity_text_upper(const char* text) { return cased(text, to_upper); }

const char* clarity_text_replace(const char* text, const char* find, const char* with) {
    text = text ? text : "";
    size_t gap = strlen(find);
    if (gap == 0) return text;
    size_t count = 0;
    for (const char* at = text; (at = strstr(at, find)); at += gap) ++count;
    size_t put = strlen(with);
    char* out = clarity_alloc(strlen(text) + count * put + 1);
    char* to = out;
    const char* start = text;
    for (const char* found; (found = strstr(start, find)); start = found + gap) {
        memcpy(to, start, (size_t)(found - start));
        to += found - start;
        memcpy(to, with, put);
        to += put;
    }
    strcpy(to, start);
    return out;
}

bool clarity_text_starts_with(const char* text, const char* prefix) {
    return strncmp(text ? text : "", prefix, strlen(prefix)) == 0;
}

bool clarity_text_ends_with(const char* text, const char* suffix) {
    text = text ? text : "";
    size_t length = strlen(text), tail = strlen(suffix);
    return tail <= length && strcmp(text + length - tail, suffix) == 0;
}

/* The byte offset of the character at `position` (from 1), the end for one past it. */
static size_t offset_of(const char* text, int64_t position) {
    const unsigned char* at = (const unsigned char*)text;
    for (int64_t i = 1; i < position && *at; ++i) {
        size_t size;
        utf8_at(at, &size);
        at += size;
    }
    return (size_t)(at - (const unsigned char*)text);
}

const char* clarity_text_slice(const char* text, int64_t from, int64_t count) {
    text = text ? text : "";
    int64_t length = clarity_length(text);
    int64_t start = from > 1 ? from : 1;
    int64_t end = from + (count > 0 ? count : 0);
    if (end > length + 1) end = length + 1;
    if (end <= start) return "";
    size_t a = offset_of(text, start), b = offset_of(text, end);
    return copied(text + a, b - a);
}

int64_t clarity_text_index_of(const char* text, const char* find) {
    text = text ? text : "";
    const char* found = strstr(text, find);
    if (!found) return 0;
    return clarity_length(copied(text, (size_t)(found - text))) + 1;
}

static uint32_t zip_crc32(const uint8_t* data, size_t size) {
    uint32_t crc = 0xFFFFFFFFu;
    for (size_t i = 0; i < size; ++i) {
        crc ^= data[i];
        for (int k = 0; k < 8; ++k) crc = (crc & 1u) ? (crc >> 1) ^ 0xEDB88320u : crc >> 1;
    }
    return ~crc;
}

static uint8_t* zip_u16(uint8_t* at, uint32_t v) {
    at[0] = (uint8_t)v;
    at[1] = (uint8_t)(v >> 8);
    return at + 2;
}

static uint8_t* zip_u32(uint8_t* at, uint32_t v) {
    for (int i = 0; i < 4; ++i) at[i] = (uint8_t)(v >> (8 * i));
    return at + 4;
}

ClarityBytes clarity_zip(ClarityTexts paths, ClarityTexts texts) {
    size_t count = paths.count < texts.count ? paths.count : texts.count;
    size_t size = 22;
    for (size_t i = 0; i < count; ++i) size += 30 + 46 + 2 * strlen(paths.items[i]) + strlen(texts.items[i] ? texts.items[i] : "");
    uint8_t* out = CLARITY_NEW(uint8_t, size);
    uint32_t* crcs = CLARITY_NEW(uint32_t, count ? count : 1);
    uint32_t* offsets = CLARITY_NEW(uint32_t, count ? count : 1);
    uint8_t* at = out;
    for (size_t i = 0; i < count; ++i) {
        const char* name = paths.items[i];
        const char* data = texts.items[i] ? texts.items[i] : "";
        uint32_t name_len = (uint32_t)strlen(name), data_len = (uint32_t)strlen(data);
        crcs[i] = zip_crc32((const uint8_t*)data, data_len);
        offsets[i] = (uint32_t)(at - out);
        /* Version 20, UTF-8 names, stored, 1980-01-01 00:00. */
        at = zip_u32(at, 0x04034b50u);
        at = zip_u16(at, 20); at = zip_u16(at, 0x0800); at = zip_u16(at, 0); at = zip_u16(at, 0); at = zip_u16(at, 0x21);
        at = zip_u32(at, crcs[i]); at = zip_u32(at, data_len); at = zip_u32(at, data_len);
        at = zip_u16(at, name_len); at = zip_u16(at, 0);
        memcpy(at, name, name_len); at += name_len;
        memcpy(at, data, data_len); at += data_len;
    }
    uint8_t* central = at;
    for (size_t i = 0; i < count; ++i) {
        const char* name = paths.items[i];
        const char* data = texts.items[i] ? texts.items[i] : "";
        uint32_t name_len = (uint32_t)strlen(name), data_len = (uint32_t)strlen(data);
        at = zip_u32(at, 0x02014b50u);
        at = zip_u16(at, 20); at = zip_u16(at, 20); at = zip_u16(at, 0x0800); at = zip_u16(at, 0); at = zip_u16(at, 0); at = zip_u16(at, 0x21);
        at = zip_u32(at, crcs[i]); at = zip_u32(at, data_len); at = zip_u32(at, data_len);
        at = zip_u16(at, name_len); at = zip_u16(at, 0); at = zip_u16(at, 0); at = zip_u16(at, 0); at = zip_u16(at, 0);
        at = zip_u32(at, 0); at = zip_u32(at, offsets[i]);
        memcpy(at, name, name_len); at += name_len;
    }
    uint32_t central_size = (uint32_t)(at - central), central_at = (uint32_t)(central - out);
    at = zip_u32(at, 0x06054b50u);
    at = zip_u16(at, 0); at = zip_u16(at, 0); at = zip_u16(at, (uint32_t)count); at = zip_u16(at, (uint32_t)count);
    at = zip_u32(at, central_size); at = zip_u32(at, central_at); at = zip_u16(at, 0);
    ClarityBytes bytes = {out, (size_t)(at - out)};
    return bytes;
}
