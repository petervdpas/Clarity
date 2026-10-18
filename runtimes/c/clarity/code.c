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
