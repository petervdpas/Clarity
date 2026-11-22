#include "clarity/decimal.h"
#include <math.h>

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

ClarityDecimal clarity_decimal_round(ClarityDecimal value, int places) {
    if (places < 0) places = 0;
    return value.scale <= places ? value : clarity_decimal_rescaled(value, places);
}

double clarity_units(ClarityDecimal value) { return (double)value.units / (double)power(value.scale); }

/// An angle function of degrees as a decimal with 10 decimals.
static ClarityDecimal angle(ClarityDecimal degrees, double (*of)(double)) {
    double radians = clarity_units(degrees) * 3.14159265358979323846 / 180.0;
    long long units = llround(of(radians) * 1e10);
    return (ClarityDecimal){units, 10};
}

ClarityDecimal clarity_decimal_sin(ClarityDecimal degrees) { return angle(degrees, sin); }

ClarityDecimal clarity_decimal_cos(ClarityDecimal degrees) { return angle(degrees, cos); }

ClarityDecimal clarity_decimal_neg(ClarityDecimal a) { return (ClarityDecimal){-a.units, a.scale}; }

/// A whole number of units wider than a decimal holds, for the sums and products
/// worked out on the way: 256 bits, the lowest 32 first.
typedef struct {
    uint32_t w[8];
} Wide;

static Wide wide_of(__int128 units) {
    unsigned __int128 v = (unsigned __int128)(units < 0 ? -units : units);
    Wide x = {{0}};
    for (int i = 0; i < 4; ++i, v >>= 32) x.w[i] = (uint32_t)v;
    return x;
}

static Wide wide_mul(Wide a, Wide b) {
    Wide x = {{0}};
    for (int i = 0; i < 8; ++i) {
        uint64_t carry = 0;
        for (int j = 0; i + j < 8; ++j) {
            uint64_t t = (uint64_t)a.w[i] * b.w[j] + x.w[i + j] + carry;
            x.w[i + j] = (uint32_t)t;
            carry = t >> 32;
        }
    }
    return x;
}

/// a times ten to the n.
static Wide wide_scaled(Wide a, int n) {
    Wide ten = wide_of(10);
    for (int i = 0; i < n; ++i) a = wide_mul(a, ten);
    return a;
}

static int wide_compare(Wide a, Wide b) {
    for (int i = 7; i >= 0; --i)
        if (a.w[i] != b.w[i]) return a.w[i] < b.w[i] ? -1 : 1;
    return 0;
}

static Wide wide_add(Wide a, Wide b) {
    uint64_t carry = 0;
    for (int i = 0; i < 8; ++i) {
        uint64_t t = (uint64_t)a.w[i] + b.w[i] + carry;
        a.w[i] = (uint32_t)t;
        carry = t >> 32;
    }
    return a;
}

/// a less b, where b is not more than a.
static Wide wide_sub(Wide a, Wide b) {
    int64_t borrow = 0;
    for (int i = 0; i < 8; ++i) {
        int64_t t = (int64_t)a.w[i] - b.w[i] - borrow;
        borrow = t < 0;
        a.w[i] = (uint32_t)(t + (borrow << 32));
    }
    return a;
}

/// a without its last digit; the digit.
static uint32_t wide_tenth(Wide* a) {
    uint64_t rest = 0;
    for (int i = 7; i >= 0; --i) {
        uint64_t t = (rest << 32) | a->w[i];
        a->w[i] = (uint32_t)(t / 10);
        rest = t % 10;
    }
    return (uint32_t)rest;
}

/// a without its last `digits` digits, rounded to the nearest, a tie to the even one.
static Wide wide_rounded_off(Wide a, int digits) {
    uint32_t last = 0;
    bool below = false;
    for (int i = 0; i < digits; ++i) {
        below |= last != 0;
        last = wide_tenth(&a);
    }
    if (last > 5 || (last == 5 && (below || (a.w[0] & 1)))) a = wide_add(a, wide_of(1));
    return a;
}

/// Whether the units fit in 96 bits.
static bool wide_fits(Wide a) {
    for (int i = 3; i < 8; ++i)
        if (a.w[i]) return false;
    return true;
}

/// An exact result as a decimal keeps it (conformance/overflow.json): rounded to
/// at most CLARITY_MAX_SCALE places, and to fewer while its units do not fit in
/// 96 bits; a value error when they do not fit with no places.
static bool fitted(bool negative, Wide units, int places, ClarityDecimal* out) {
    if (places > CLARITY_MAX_SCALE) {
        units = wide_rounded_off(units, places - CLARITY_MAX_SCALE);
        places = CLARITY_MAX_SCALE;
    }
    while (!wide_fits(units) && places > 0) {
        units = wide_rounded_off(units, 1);
        --places;
    }
    if (!wide_fits(units)) return clarity_too_large();
    __int128 v = 0;
    for (int i = 2; i >= 0; --i) v = (v << 32) | units.w[i];
    out->units = negative ? -v : v;
    out->scale = places;
    return true;
}

bool clarity_decimal_add(ClarityDecimal a, ClarityDecimal b, ClarityDecimal* out) {
    int scale = a.scale > b.scale ? a.scale : b.scale;
    Wide x = wide_scaled(wide_of(a.units), scale - a.scale);
    Wide y = wide_scaled(wide_of(b.units), scale - b.scale);
    if ((a.units < 0) == (b.units < 0)) return fitted(a.units < 0, wide_add(x, y), scale, out);
    // Signs that differ: the larger less the smaller, with the larger one's sign.
    return wide_compare(x, y) >= 0 ? fitted(a.units < 0, wide_sub(x, y), scale, out) : fitted(b.units < 0, wide_sub(y, x), scale, out);
}

bool clarity_decimal_sub(ClarityDecimal a, ClarityDecimal b, ClarityDecimal* out) { return clarity_decimal_add(a, clarity_decimal_neg(b), out); }

bool clarity_decimal_mul(ClarityDecimal a, ClarityDecimal b, ClarityDecimal* out) {
    return fitted((a.units < 0) != (b.units < 0), wide_mul(wide_of(a.units), wide_of(b.units)), a.scale + b.scale, out);
}

/// The largest number of units a quotient holds: 2^96 - 1, as Rust and C# keep decimals.
static const __int128 MOST_UNITS = ((__int128)1 << 96) - 1;

/// `a / b`, as every stack divides decimals (conformance/decimal-division.json): up to
/// CLARITY_MAX_SCALE places in 96 bits, the last digit rounded to the nearest (a tie to
/// the even one), worked out a digit at a time so nothing overflows on the way; an exact
/// quotient keeps only the places it needs. Dividing by zero is a value error.
bool clarity_decimal_div(ClarityDecimal a, ClarityDecimal b, ClarityDecimal* out) {
    if (b.units == 0) return clarity_division_by_zero();
    bool negative = (a.units < 0) != (b.units < 0);
    __int128 top = a.units < 0 ? -a.units : a.units;
    __int128 bottom = b.units < 0 ? -b.units : b.units;
    // a / b = (top / bottom) / 10^shift; the quotient's places are its digits after the point plus shift.
    int shift = a.scale - b.scale;
    __int128 quotient = top / bottom, rest = top % bottom;
    if (quotient > MOST_UNITS && shift <= 0) return clarity_too_large();
    int digits = 0;
    while (rest != 0 && digits + shift < CLARITY_MAX_SCALE && quotient <= (MOST_UNITS - 9) / 10) {
        rest *= 10;
        quotient = quotient * 10 + rest / bottom;
        rest %= bottom;
        ++digits;
    }
    bool exact = rest == 0;
    if (!exact) {
        rest *= 10;
        __int128 digit = rest / bottom, left = rest % bottom;
        if (digit > 5 || (digit == 5 && (left != 0 || quotient % 2 == 1))) ++quotient;
        if (quotient > MOST_UNITS) {
            quotient /= 10;
            --digits;
        }
    }
    int places = digits + shift;
    while (places < 0) {
        quotient *= 10;
        ++places;
    }
    // An exact quotient keeps only the places it needs: 6.0 / 3 is 2.
    while (exact && places > 0 && quotient % 10 == 0) {
        quotient /= 10;
        --places;
    }
    out->units = negative ? -quotient : quotient;
    out->scale = places;
    return true;
}

int clarity_decimal_compare(ClarityDecimal a, ClarityDecimal b) {
    if ((a.units < 0) != (b.units < 0)) return a.units < 0 ? -1 : 1;
    int scale = a.scale > b.scale ? a.scale : b.scale;
    int magnitude = wide_compare(wide_scaled(wide_of(a.units), scale - a.scale), wide_scaled(wide_of(b.units), scale - b.scale));
    return a.units < 0 ? -magnitude : magnitude;
}
