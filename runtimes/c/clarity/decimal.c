#include "clarity/decimal.h"

static __int128 power(int n) {
    __int128 p = 1;
    for (int i = 0; i < n; ++i) p *= 10;
    return p;
}

/// `units` with `drop` fewer decimals, rounded half away from zero.
static __int128 shorten(__int128 units, int drop) {
    if (drop <= 0) return units * power(-drop);
    __int128 p = power(drop);
    __int128 q = units / p;
    __int128 r = units % p;
    if (r < 0) r = -r;
    if (r * 2 >= p) q += units < 0 ? -1 : 1;
    return q;
}

ClarityDecimal clarity_decimal(int64_t units, int scale) { return (ClarityDecimal){units, scale}; }

ClarityDecimal clarity_decimal_of(int64_t whole) { return (ClarityDecimal){whole, 0}; }

bool clarity_decimal_parse(const char* text, ClarityDecimal* out) {
    ClarityDecimal d = {0, 0};
    const char* at = text ? text : "";
    bool negative = false;
    if (*at == '-' || *at == '+') negative = *at++ == '-';
    bool digits = false, point = false;
    for (; *at; ++at) {
        if (*at == '.' && !point) {
            point = true;
        } else if (*at >= '0' && *at <= '9') {
            digits = true;
            d.units = d.units * 10 + (*at - '0');
            if (point) ++d.scale;
            if (d.scale > CLARITY_MAX_SCALE) return clarity_fail(CLARITY_INVALID, "'%s' has more than 28 decimals", text);
        } else {
            digits = false;
            break;
        }
    }
    if (!digits) return clarity_fail(CLARITY_INVALID, "'%s' is no number", text ? text : "");
    if (negative) d.units = -d.units;
    *out = d;
    return true;
}

ClarityDecimal clarity_decimal_from_text(const char* text) {
    ClarityDecimal d = {0, 0};
    clarity_decimal_parse(text, &d);
    return d;
}

const char* clarity_decimal_text(ClarityDecimal value) {
    char digits[64];
    int n = 0;
    __int128 v = value.units < 0 ? -value.units : value.units;
    do {
        digits[n++] = (char)('0' + (int)(v % 10));
        v /= 10;
    } while (v > 0);
    while (n <= value.scale) digits[n++] = '0';
    char out[72];
    int length = 0;
    if (value.units < 0) out[length++] = '-';
    for (int i = n - 1; i >= 0; --i) {
        out[length++] = digits[i];
        if (i == value.scale && i > 0) out[length++] = '.';
    }
    return clarity_copy_n(out, (size_t)length);
}

ClarityDecimal clarity_decimal_rescaled(ClarityDecimal value, int scale) {
    if (scale < 0) scale = 0;
    if (scale > CLARITY_MAX_SCALE) scale = CLARITY_MAX_SCALE;
    return (ClarityDecimal){shorten(value.units, value.scale - scale), scale};
}

ClarityDecimal clarity_decimal_neg(ClarityDecimal a) { return (ClarityDecimal){-a.units, a.scale}; }

ClarityDecimal clarity_decimal_add(ClarityDecimal a, ClarityDecimal b) {
    int scale = a.scale > b.scale ? a.scale : b.scale;
    return (ClarityDecimal){a.units * power(scale - a.scale) + b.units * power(scale - b.scale), scale};
}

ClarityDecimal clarity_decimal_sub(ClarityDecimal a, ClarityDecimal b) { return clarity_decimal_add(a, clarity_decimal_neg(b)); }

ClarityDecimal clarity_decimal_mul(ClarityDecimal a, ClarityDecimal b) {
    ClarityDecimal d = {a.units * b.units, a.scale + b.scale};
    return d.scale > CLARITY_MAX_SCALE ? clarity_decimal_rescaled(d, CLARITY_MAX_SCALE) : d;
}

bool clarity_decimal_div(ClarityDecimal a, ClarityDecimal b, ClarityDecimal* out) {
    if (b.units == 0) return clarity_fail(CLARITY_INVALID, "division by zero");
    // a / b with CLARITY_DIVISION_SCALE + 1 decimals, then rounded to CLARITY_DIVISION_SCALE.
    int extra = CLARITY_DIVISION_SCALE + 1 - a.scale + b.scale;
    __int128 top = a.units * power(extra > 0 ? extra : 0);
    __int128 bottom = b.units * power(extra < 0 ? -extra : 0);
    ClarityDecimal d = {shorten(top / bottom, 1), CLARITY_DIVISION_SCALE};
    while (d.scale > 0 && d.units % 10 == 0) {
        d.units /= 10;
        --d.scale;
    }
    *out = d;
    return true;
}

int clarity_decimal_compare(ClarityDecimal a, ClarityDecimal b) {
    int scale = a.scale > b.scale ? a.scale : b.scale;
    __int128 x = a.units * power(scale - a.scale);
    __int128 y = b.units * power(scale - b.scale);
    return x < y ? -1 : x > y ? 1 : 0;
}
