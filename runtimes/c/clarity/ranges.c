#include "clarity/ranges.h"

#include <ctype.h>
#include <string.h>

#include "clarity/decimal.h"

/// A number as text, trimmed, into `out`; false, with the error set, for text that is none.
static bool exact(const char* text, ClarityDecimal* out) {
    const char* at = text ? text : "";
    while (isspace((unsigned char)*at)) ++at;
    size_t length = strlen(at);
    while (length > 0 && isspace((unsigned char)at[length - 1])) --length;
    return clarity_decimal_parse(clarity_copy_n(at, length), out);
}

/// The most decimals of the numbers given; a NULL is no number and counts for none.
static bool scale_of(const char* const* texts, size_t count, int* scale) {
    *scale = 0;
    for (size_t i = 0; i < count; ++i) {
        ClarityDecimal d;
        if (!texts[i]) continue;
        if (!exact(texts[i], &d)) return false;
        if (d.scale > *scale) *scale = d.scale;
    }
    return true;
}

/// A number as whole units at `scale` decimals.
static bool at_scale(const char* text, int scale, __int128* out) {
    ClarityDecimal d;
    if (!exact(text, &d)) return false;
    __int128 n = d.units;
    for (int i = d.scale; i < scale; ++i) n *= 10;
    *out = n;
    return true;
}

/// Whole units at `scale` decimals as text, without trailing zeros.
static const char* text_of(__int128 n, int scale) {
    char* text = clarity_copy(clarity_decimal_text((ClarityDecimal){n, scale}));
    if (strchr(text, '.')) {
        size_t length = strlen(text);
        while (text[length - 1] == '0') text[--length] = '\0';
        if (text[length - 1] == '.') text[--length] = '\0';
    }
    return text;
}

/// `n` within the ends that are there.
static __int128 within(__int128 n, const __int128* low, const __int128* high) {
    if (high && n > *high) n = *high;
    if (low && n < *low) n = *low;
    return n;
}

/// The range's ends at `scale`: `low` and `high` point at them, or are NULL.
typedef struct {
    __int128 min, max;
    const __int128* low;
    const __int128* high;
} Ends;

static bool ends_of(ClarityRange range, int scale, Ends* ends) {
    ends->low = ends->high = NULL;
    if (range.min) {
        if (!at_scale(range.min, scale, &ends->min)) return false;
        ends->low = &ends->min;
    }
    if (range.max) {
        if (!at_scale(range.max, scale, &ends->max)) return false;
        ends->high = &ends->max;
    }
    return true;
}

const char* clarity_range_keyed(const char* value, ClarityRange range, const char* key) {
    if (value && !*value) value = NULL;
    const char* texts[] = {value, range.min, range.max, range.step};
    int scale;
    if (!scale_of(texts, 4, &scale)) return NULL;
    Ends ends;
    __int128 step, now;
    if (!ends_of(range, scale, &ends) || !at_scale(range.step, scale, &step)) return NULL;
    if (value) {
        if (!at_scale(value, scale, &now)) return NULL;
    } else {
        now = ends.low ? *ends.low : 0;
    }
    __int128 next;
    if (strcmp(key, "up") == 0) next = now + step;
    else if (strcmp(key, "down") == 0) next = now - step;
    else if (strcmp(key, "pageUp") == 0) next = now + 10 * step;
    else if (strcmp(key, "pageDown") == 0) next = now - 10 * step;
    else if (strcmp(key, "home") == 0) next = ends.low ? *ends.low : now;
    else if (strcmp(key, "end") == 0) next = ends.high ? *ends.high : now;
    else {
        clarity_fail(CLARITY_INVALID, "%s is no key of a range", key);
        return NULL;
    }
    return text_of(within(next, ends.low, ends.high), scale);
}

const char* clarity_range_dragged(const char* at, ClarityRange range) {
    const char* texts[] = {at, range.min, range.max, range.step};
    int scale;
    if (!scale_of(texts, 4, &scale)) return NULL;
    __int128 low = 0, step, where;
    if (range.min && !at_scale(range.min, scale, &low)) return NULL;
    if (!at_scale(range.step, scale, &step) || !at_scale(at, scale, &where)) return NULL;
    if (step <= 0) {
        clarity_fail(CLARITY_INVALID, "a range's step is above zero, not %s", range.step);
        return NULL;
    }
    __int128 from = where - low;
    // Rounded half up, as a whole number of steps.
    __int128 steps = from >= 0 ? (2 * from + step) / (2 * step) : -((-2 * from + step - 1) / (2 * step));
    __int128 none = 0, most = 0;
    if (range.max) {
        __int128 high;
        if (!at_scale(range.max, scale, &high)) return NULL;
        most = (high - low) / step;
    }
    steps = within(steps, &none, range.max ? &most : NULL);
    return text_of(low + steps * step, scale);
}

const char* clarity_range_typed(const char* value, ClarityRange range) {
    const char* texts[] = {value, range.min, range.max};
    int scale;
    if (!scale_of(texts, 3, &scale)) return NULL;
    Ends ends;
    __int128 n;
    if (!ends_of(range, scale, &ends) || !at_scale(value, scale, &n)) return NULL;
    return text_of(within(n, ends.low, ends.high), scale);
}
