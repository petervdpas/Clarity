#include "clarity/ranges.hpp"

#include <algorithm>
#include <cctype>
#include <initializer_list>

namespace clarity {

namespace {

/// A number as whole units and how many of its digits are decimals.
struct Exact {
    __int128 n = 0;
    int scale = 0;
};

Exact exact(const std::string& value) {
    std::string text = value;
    auto space = [](char c) { return std::isspace(static_cast<unsigned char>(c)) != 0; };
    while (!text.empty() && space(text.back())) text.pop_back();
    size_t i = 0;
    while (i < text.size() && space(text[i])) ++i;
    bool negative = i < text.size() && text[i] == '-';
    if (i < text.size() && (text[i] == '-' || text[i] == '+')) ++i;
    Exact e;
    bool point = false;
    int count = 0;
    for (; i < text.size(); ++i) {
        char c = text[i];
        if (c == '.' && !point) {
            point = true;
            continue;
        }
        if (!std::isdigit(static_cast<unsigned char>(c))) throw InvalidError("\"" + value + "\" is no number");
        // At most 18 digits, so the sums stay within 128 bits at any scale.
        if (++count > 18) throw InvalidError("\"" + value + "\" has more digits than a range keeps");
        e.n = e.n * 10 + (c - '0');
        if (point) ++e.scale;
    }
    bool any = count > 0;
    if (!any) throw InvalidError("\"" + value + "\" is no number");
    if (negative) e.n = -e.n;
    return e;
}

__int128 power(int n) {
    __int128 p = 1;
    for (int i = 0; i < n; ++i) p *= 10;
    return p;
}

/// Whole units at a scale of `scale` decimals.
__int128 at(const std::string& value, int scale) {
    Exact e = exact(value);
    return e.n * power(scale - e.scale);
}

int scale_of(std::initializer_list<const std::optional<std::string>*> values) {
    int scale = 0;
    for (const auto* v : values)
        if (*v) scale = std::max(scale, exact(**v).scale);
    return scale;
}

std::string text(__int128 n, int scale) {
    bool negative = n < 0;
    if (negative) n = -n;
    std::string digits;
    do {
        digits.insert(digits.begin(), char('0' + int(n % 10)));
        n /= 10;
    } while (n > 0);
    if (digits.size() < size_t(scale) + 1) digits.insert(0, size_t(scale) + 1 - digits.size(), '0');
    std::string whole = digits.substr(0, digits.size() - size_t(scale));
    std::string part = digits.substr(digits.size() - size_t(scale));
    while (!part.empty() && part.back() == '0') part.pop_back();
    return (negative ? "-" : "") + whole + (part.empty() ? "" : "." + part);
}

__int128 within(__int128 n, const std::optional<__int128>& low, const std::optional<__int128>& high) {
    if (high && n > *high) n = *high;
    if (low && n < *low) n = *low;
    return n;
}

std::optional<__int128> end_at(const std::optional<std::string>& end, int scale) {
    if (!end) return std::nullopt;
    return at(*end, scale);
}

}  // namespace

std::string keyed(const std::optional<std::string>& value, const NumberRange& range, const std::string& key) {
    std::optional<std::string> now_text = value && !value->empty() ? value : std::nullopt;
    std::optional<std::string> step = range.step;
    int scale = scale_of({&now_text, &range.min, &range.max, &step});
    auto low = end_at(range.min, scale);
    auto high = end_at(range.max, scale);
    __int128 now = now_text ? at(*now_text, scale) : low.value_or(0);
    __int128 by = at(range.step, scale);
    __int128 next;
    if (key == "up") next = now + by;
    else if (key == "down") next = now - by;
    else if (key == "pageUp") next = now + 10 * by;
    else if (key == "pageDown") next = now - 10 * by;
    else if (key == "home") next = low.value_or(now);
    else if (key == "end") next = high.value_or(now);
    else throw InvalidError("there is no range key " + key);
    return text(within(next, low, high), scale);
}

std::string dragged(const std::string& where, const NumberRange& range) {
    std::optional<std::string> at_text = where, step_text = range.step;
    int scale = scale_of({&at_text, &range.min, &range.max, &step_text});
    __int128 low = at(range.min.value_or("0"), scale);
    __int128 step = at(range.step, scale);
    if (step <= 0) throw InvalidError("a range's step is more than 0, not " + range.step);
    __int128 from = at(where, scale) - low;
    // Rounded half up, as a whole number of steps.
    __int128 steps = from >= 0 ? (2 * from + step) / (2 * step) : -((-2 * from + step - 1) / (2 * step));
    std::optional<__int128> most;
    if (range.max) most = (at(*range.max, scale) - low) / step;
    steps = within(steps, __int128(0), most);
    return text(low + steps * step, scale);
}

std::string typed_in(const std::string& value, const NumberRange& range) {
    std::optional<std::string> typed = value;
    int scale = scale_of({&typed, &range.min, &range.max});
    return text(within(at(value, scale), end_at(range.min, scale), end_at(range.max, scale)), scale);
}

nlohmann::json range_value(const std::string& value, bool whole) {
    if (!whole) return value;
    if (value.find('.') == std::string::npos) return int64_t(exact(value).n);
    return std::stod(value);
}

}  // namespace clarity
