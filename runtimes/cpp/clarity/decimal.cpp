#include "clarity/decimal.hpp"

#include <algorithm>
#include <ostream>

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

Decimal Decimal::operator-() const {
    Decimal d = *this;
    d.units_ = -d.units_;
    return d;
}

Decimal operator+(const Decimal& a, const Decimal& b) {
    int scale = std::max(a.scale_, b.scale_);
    Decimal d;
    d.scale_ = scale;
    d.units_ = a.units_ * power(scale - a.scale_) + b.units_ * power(scale - b.scale_);
    return d;
}

Decimal operator-(const Decimal& a, const Decimal& b) { return a + -b; }

Decimal operator*(const Decimal& a, const Decimal& b) {
    Decimal d;
    d.units_ = a.units_ * b.units_;
    d.scale_ = a.scale_ + b.scale_;
    return d.scale_ > Decimal::max_scale ? d.rescaled(Decimal::max_scale) : d;
}

Decimal operator/(const Decimal& a, const Decimal& b) {
    if (b.units_ == 0) throw InvalidError("division by zero");
    // a / b with division_scale + 1 decimals, then rounded to division_scale.
    int extra = Decimal::division_scale + 1 - a.scale_ + b.scale_;
    __int128 top = a.units_ * power(std::max(extra, 0));
    __int128 bottom = b.units_ * power(std::max(-extra, 0));
    Decimal d;
    d.units_ = shorten(top / bottom, 1);
    d.scale_ = Decimal::division_scale;
    while (d.scale_ > 0 && d.units_ % 10 == 0) {
        d.units_ /= 10;
        --d.scale_;
    }
    return d;
}

std::strong_ordering operator<=>(const Decimal& a, const Decimal& b) {
    int scale = std::max(a.scale_, b.scale_);
    __int128 x = a.units_ * power(scale - a.scale_);
    __int128 y = b.units_ * power(scale - b.scale_);
    return x < y ? std::strong_ordering::less : x > y ? std::strong_ordering::greater : std::strong_ordering::equal;
}

std::ostream& operator<<(std::ostream& out, const Decimal& value) { return out << value.str(); }

}  // namespace clarity
