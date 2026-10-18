#include "clarity/sql.h"

#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool unfit(const char* column, const char* what) { return clarity_fail(CLARITY_STORAGE, "%s: the stored value is no %s", column, what); }

bool clarity_get_text(const char* column, ClarityValue value, const char** out) {
    switch (value.kind) {
    case CLARITY_TEXT: *out = value.text; return true;
    case CLARITY_NULL: *out = ""; return true;
    case CLARITY_INTEGER: *out = clarity_format("%lld", (long long)value.integer); return true;
    case CLARITY_BYTES: *out = clarity_copy_n((const char*)value.bytes.data, value.bytes.size); return true;
    default: return unfit(column, "text");
    }
}

bool clarity_get_integer(const char* column, ClarityValue value, int64_t* out) {
    switch (value.kind) {
    case CLARITY_INTEGER: *out = value.integer; return true;
    case CLARITY_NULL: *out = 0; return true;
    case CLARITY_BOOL: *out = value.flag; return true;
    case CLARITY_REAL: *out = llround(value.real); return true;
    case CLARITY_TEXT: {
        // Text that is no number counts as none, as a migration step leaves it.
        if (!*value.text) {
            *out = 0;
            return true;
        }
        char* end;
        errno = 0;
        long long n = strtoll(value.text, &end, 10);
        if (*end == '\0' && errno == 0) {
            *out = n;
            return true;
        }
        return unfit(column, "whole number");
    }
    default: return unfit(column, "whole number");
    }
}

bool clarity_get_decimal(const char* column, ClarityValue value, ClarityDecimal* out) {
    switch (value.kind) {
    case CLARITY_DECIMAL: *out = value.decimal; return true;
    case CLARITY_TEXT:
        if (!*value.text) {
            *out = (ClarityDecimal){0, 0};
            return true;
        }
        return clarity_decimal_parse(value.text, out);
    case CLARITY_INTEGER: *out = clarity_decimal_of(value.integer); return true;
    case CLARITY_REAL: {
        char text[64];
        snprintf(text, sizeof text, "%.10g", value.real);
        return clarity_decimal_parse(text, out);
    }
    case CLARITY_NULL: *out = (ClarityDecimal){0, 0}; return true;
    default: return unfit(column, "number");
    }
}

bool clarity_get_bool(const char* column, ClarityValue value, bool* out) {
    if (value.kind == CLARITY_BOOL) {
        *out = value.flag;
        return true;
    }
    int64_t n = 0;
    if (!clarity_get_integer(column, value, &n)) return false;
    *out = n != 0;
    return true;
}

bool clarity_get_date(const char* column, ClarityValue value, ClarityDate* out) {
    if (value.kind == CLARITY_NULL) {
        *out = (ClarityDate){0};
        return true;
    }
    if (value.kind == CLARITY_DATE) {
        *out = value.date;
        return true;
    }
    // A date can be written with a time of day after it.
    if (value.kind == CLARITY_TEXT && clarity_date_parse(clarity_copy_n(value.text, strnlen(value.text, 10)), out)) return true;
    return unfit(column, "date");
}

bool clarity_get_date_time(const char* column, ClarityValue value, ClarityDateTime* out) {
    if (value.kind == CLARITY_NULL) {
        *out = (ClarityDateTime){0};
        return true;
    }
    if (value.kind == CLARITY_DATE_TIME) {
        *out = value.date_time;
        return true;
    }
    if (value.kind == CLARITY_TEXT && clarity_date_time_parse(value.text, out)) return true;
    return unfit(column, "date and time");
}

bool clarity_get_reference(const char* column, ClarityValue value, int64_t* out) {
    if (value.kind == CLARITY_NULL) {
        *out = 0;
        return true;
    }
    return clarity_get_integer(column, value, out);
}

bool clarity_get_picture(const char* column, ClarityValue value, ClarityPicture** out) {
    bool there;
    switch (value.kind) {
    case CLARITY_INTEGER: there = value.integer != 0; break;
    case CLARITY_TEXT: there = *value.text != '\0'; break;
    case CLARITY_BYTES: there = value.bytes.size != 0; break;
    case CLARITY_BOOL: there = value.flag; break;
    case CLARITY_NULL: there = false; break;
    default: return unfit(column, "picture");
    }
    *out = there ? clarity_picture_stored() : NULL;
    return true;
}
