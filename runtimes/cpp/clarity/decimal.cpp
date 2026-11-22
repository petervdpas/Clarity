#include "clarity/decimal.hpp"

#include <cmath>

#include <algorithm>
#include <array>
#include <cstdint>
#include <ostream>

#include "clarity/code.hpp"
#include "clarity/errors.hpp"

namespace clarity {

namespace {

__int128 power(int n) {
    __int128 p = 1;
    for (int i = 0; i < n; ++i) p *= 10;
    return p;
}

/// `units` with `drop` fewer decimals, rounded half away from zero.
__int128 shorten(__int128 units, int drop) {
    if (drop <= 0) return units * power(-drop);
    __int128 p = power(drop);
    __int128 q = units / p;
    __int128 r = units % p;
    if (r < 0) r = -r;
    if (r * 2 >= p) q += units < 0 ? -1 : 1;
    return q;
}

/// A whole number of units wider than a decimal holds, for the sums and products
/// worked out on the way: 256 bits, the lowest 32 first.
using Wide = std::array<uint32_t, 8>;

Wide wide_of(__int128 units) {
    auto v = static_cast<unsigned __int128>(units < 0 ? -units : units);
    Wide x{};
    for (int i = 0; i < 4; ++i, v >>= 32) x[i] = static_cast<uint32_t>(v);
    return x;
}

Wide wide_mul(const Wide& a, const Wide& b) {
    Wide x{};
    for (int i = 0; i < 8; ++i) {
        uint64_t carry = 0;
        for (int j = 0; i + j < 8; ++j) {
            uint64_t t = uint64_t(a[i]) * b[j] + x[i + j] + carry;
            x[i + j] = static_cast<uint32_t>(t);
            carry = t >> 32;
        }
    }
    return x;
}

/// a times ten to the n.
Wide wide_scaled(Wide a, int n) {
    for (int i = 0; i < n; ++i) a = wide_mul(a, wide_of(10));
    return a;
}

int wide_compare(const Wide& a, const Wide& b) {
    for (int i = 7; i >= 0; --i)
        if (a[i] != b[i]) return a[i] < b[i] ? -1 : 1;
    return 0;
}

Wide wide_add(Wide a, const Wide& b) {
    uint64_t carry = 0;
    for (int i = 0; i < 8; ++i) {
        uint64_t t = uint64_t(a[i]) + b[i] + carry;
        a[i] = static_cast<uint32_t>(t);
        carry = t >> 32;
    }
    return a;
}

/// a less b, where b is not more than a.
Wide wide_sub(Wide a, const Wide& b) {
    int64_t borrow = 0;
    for (int i = 0; i < 8; ++i) {
        int64_t t = int64_t(a[i]) - b[i] - borrow;
        borrow = t < 0;
        a[i] = static_cast<uint32_t>(t + (borrow << 32));
    }
    return a;
}

/// a without its last digit; the digit.
uint32_t wide_tenth(Wide& a) {
    uint64_t rest = 0;
    for (int i = 7; i >= 0; --i) {
        uint64_t t = (rest << 32) | a[i];
        a[i] = static_cast<uint32_t>(t / 10);
        rest = t % 10;
    }
    return static_cast<uint32_t>(rest);
}

/// a without its last `digits` digits, rounded to the nearest, a tie to the even one.
Wide wide_rounded_off(Wide a, int digits) {
    uint32_t last = 0;
    bool below = false;
    for (int i = 0; i < digits; ++i) {
        below |= last != 0;
        last = wide_tenth(a);
    }
    if (last > 5 || (last == 5 && (below || (a[0] & 1)))) a = wide_add(a, wide_of(1));
    return a;
}

/// Whether the units fit in 96 bits.
bool wide_fits(const Wide& a) { return std::all_of(a.begin() + 3, a.end(), [](uint32_t w) { return w == 0; }); }

}  // namespace

/// An exact result as a decimal keeps it (conformance/overflow.json): rounded to at
/// most max_scale places, and to fewer while its units do not fit in 96 bits; a value
/// error when they do not fit with no places.
Decimal Decimal::fitted(bool negative, const std::array<uint32_t, 8>& exact, int places) {
    Wide units = exact;
    if (places > max_scale) {
        units = wide_rounded_off(units, places - max_scale);
        places = max_scale;
    }
    for (; !wide_fits(units) && places > 0; --places) units = wide_rounded_off(units, 1);
    if (!wide_fits(units)) throw too_large();
    __int128 v = 0;
    for (int i = 2; i >= 0; --i) v = (v << 32) | units[i];
    Decimal d;
    d.units_ = negative ? -v : v;
    d.scale_ = places;
    return d;
}

namespace {

}  // namespace

Decimal Decimal::parse(std::string_view text) {
    Decimal d;
    size_t i = 0;
    bool negative = false;
    if (i < text.size() && (text[i] == '-' || text[i] == '+')) negative = text[i++] == '-';
    bool digits = false, point = false;
    for (; i < text.size(); ++i) {
        char c = text[i];
        if (c == '.' && !point) {
            point = true;
        } else if (c >= '0' && c <= '9') {
            digits = true;
            d.units_ = d.units_ * 10 + (c - '0');
            if (point) ++d.scale_;
            if (d.scale_ > max_scale) throw InvalidError("'" + std::string(text) + "' has more than 28 decimals");
        } else {
            digits = false;
            break;
        }
    }
    if (!digits) throw InvalidError("'" + std::string(text) + "' is no number");
    if (negative) d.units_ = -d.units_;
    return d;
}

std::string Decimal::str() const {
    __int128 v = units_ < 0 ? -units_ : units_;
    std::string digits;
    do {
        digits.insert(digits.begin(), char('0' + int(v % 10)));
        v /= 10;
    } while (v > 0);
    if (scale_ > 0) {
        if (int(digits.size()) <= scale_) digits.insert(0, size_t(scale_ - int(digits.size()) + 1), '0');
        digits.insert(digits.size() - size_t(scale_), ".");
    }
    return units_ < 0 ? "-" + digits : digits;
}

Decimal Decimal::rescaled(int scale) const {
    Decimal d;
    d.scale_ = std::clamp(scale, 0, max_scale);
    d.units_ = shorten(units_, scale_ - d.scale_);
    return d;
}

Decimal Decimal::rounded(int places) const {
    places = std::max(places, 0);
    return scale_ <= places ? *this : rescaled(places);
}

Decimal Decimal::operator-() const {
    Decimal d = *this;
    d.units_ = -d.units_;
    return d;
}

Decimal operator+(const Decimal& a, const Decimal& b) {
    int scale = std::max(a.scale_, b.scale_);
    Wide x = wide_scaled(wide_of(a.units_), scale - a.scale_);
    Wide y = wide_scaled(wide_of(b.units_), scale - b.scale_);
    if ((a.units_ < 0) == (b.units_ < 0)) return Decimal::fitted(a.units_ < 0, wide_add(x, y), scale);
    // Signs that differ: the larger less the smaller, with the larger one's sign.
    return wide_compare(x, y) >= 0 ? Decimal::fitted(a.units_ < 0, wide_sub(x, y), scale) : Decimal::fitted(b.units_ < 0, wide_sub(y, x), scale);
}

Decimal operator-(const Decimal& a, const Decimal& b) { return a + -b; }

Decimal operator*(const Decimal& a, const Decimal& b) {
    return Decimal::fitted((a.units_ < 0) != (b.units_ < 0), wide_mul(wide_of(a.units_), wide_of(b.units_)), a.scale_ + b.scale_);
}

/// `a / b`, as every stack divides decimals (conformance/decimal-division.json): up to
/// max_scale places in 96 bits, the last digit rounded to the nearest (a tie to the even
/// one), worked out a digit at a time so nothing overflows on the way; an exact quotient
/// keeps only the places it needs. Dividing by zero is a value error.
Decimal operator/(const Decimal& a, const Decimal& b) {
    if (b.units_ == 0) throw division_by_zero();
    const __int128 most_units = (__int128(1) << 96) - 1;
    bool negative = (a.units_ < 0) != (b.units_ < 0);
    __int128 top = a.units_ < 0 ? -a.units_ : a.units_;
    __int128 bottom = b.units_ < 0 ? -b.units_ : b.units_;
    // a / b = (top / bottom) / 10^shift; the quotient's places are its digits after the point plus shift.
    int shift = a.scale_ - b.scale_;
    __int128 quotient = top / bottom, rest = top % bottom;
    if (quotient > most_units && shift <= 0) throw too_large();
    int digits = 0;
    while (rest != 0 && digits + shift < Decimal::max_scale && quotient <= (most_units - 9) / 10) {
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
        if (quotient > most_units) {
            quotient /= 10;
            --digits;
        }
    }
    int places = digits + shift;
    for (; places < 0; ++places) quotient *= 10;
    // An exact quotient keeps only the places it needs: 6.0 / 3 is 2.
    for (; exact && places > 0 && quotient % 10 == 0; --places) quotient /= 10;
    Decimal d;
    d.units_ = negative ? -quotient : quotient;
    d.scale_ = places;
    return d;
}

std::strong_ordering operator<=>(const Decimal& a, const Decimal& b) {
    if ((a.units_ < 0) != (b.units_ < 0)) return a.units_ < 0 ? std::strong_ordering::less : std::strong_ordering::greater;
    int scale = std::max(a.scale_, b.scale_);
    int magnitude = wide_compare(wide_scaled(wide_of(a.units_), scale - a.scale_), wide_scaled(wide_of(b.units_), scale - b.scale_));
    if (a.units_ < 0) magnitude = -magnitude;
    return magnitude < 0 ? std::strong_ordering::less : magnitude > 0 ? std::strong_ordering::greater : std::strong_ordering::equal;
}

std::ostream& operator<<(std::ostream& out, const Decimal& value) { return out << value.str(); }

double units(const Decimal& value) { return std::stod(value.str()); }

namespace {

/// An angle function of degrees as a decimal with 10 decimals.
Decimal angle(const Decimal& degrees, double (*of)(double)) {
    double radians = units(degrees) * 3.14159265358979323846 / 180.0;
    return Decimal::of(std::llround(of(radians) * 1e10), 10);
}

}  // namespace

Decimal sin(const Decimal& degrees) { return angle(degrees, [](double x) { return std::sin(x); }); }

Decimal cos(const Decimal& degrees) { return angle(degrees, [](double x) { return std::cos(x); }); }

}  // namespace clarity
