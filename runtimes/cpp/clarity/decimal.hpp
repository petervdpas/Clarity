// An exact decimal number for `decimal` and `money` values: a whole number
// of units and how many of its digits are decimals (its scale). C++ has no
// decimal type of its own; this one keeps what it is given, so 1250.50
// stays 1250.50 when it is stored and shown.

#pragma once

#include <compare>
#include <cstdint>
#include <string>
#include <string_view>

namespace clarity {

class Decimal {
public:
    /// The most decimals a value keeps; more are rounded off.
    static constexpr int max_scale = 28;
    /// The decimals a division gives, before trailing zeros go.
    static constexpr int division_scale = 16;

    Decimal() = default;
    Decimal(int64_t whole) : units_(whole) {}  // NOLINT: an integer widens to a decimal
    Decimal(int whole) : units_(whole) {}      // NOLINT

    /// Text such as `-1250.50`; throws InvalidError for anything else.
    static Decimal parse(std::string_view text);

    /// The value as text, with its decimals: `1250.50`.
    std::string str() const;
    int scale() const { return scale_; }

    /// The value with `scale` decimals, rounded half away from zero.
    Decimal rescaled(int scale) const;
    /// `round(value, places)` in code: at most `places` decimals, rounded
    /// half away from zero; a value with fewer stays as it is
    /// (conformance/rounding.json).
    Decimal rounded(int places) const;
    /// `units` with `scale` decimals: of(125050, 2) is 1250.50.
    static Decimal of(int64_t units, int scale) {
        Decimal d;
        d.units_ = units;
        d.scale_ = scale;
        return d;
    }

    Decimal operator-() const;
    friend Decimal operator+(const Decimal& a, const Decimal& b);
    friend Decimal operator-(const Decimal& a, const Decimal& b);
    friend Decimal operator*(const Decimal& a, const Decimal& b);
    /// Throws InvalidError when dividing by zero.
    friend Decimal operator/(const Decimal& a, const Decimal& b);
    Decimal& operator+=(const Decimal& other) { return *this = *this + other; }
    Decimal& operator-=(const Decimal& other) { return *this = *this - other; }

    /// By value: 1.50 equals 1.5.
    friend std::strong_ordering operator<=>(const Decimal& a, const Decimal& b);
    friend bool operator==(const Decimal& a, const Decimal& b) { return (a <=> b) == 0; }

private:
    __int128 units_ = 0;
    int scale_ = 0;
};

std::ostream& operator<<(std::ostream& out, const Decimal& value);

/// `sin(degrees)` and `cos(degrees)` in code, rounded to 10 decimals
/// (conformance/drawing.json).
Decimal sin(const Decimal& degrees);
Decimal cos(const Decimal& degrees);
/// A value of code as a number a canvas draws with.
double units(const Decimal& value);

}  // namespace clarity
