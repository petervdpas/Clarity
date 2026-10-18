#include "clarity/values.hpp"

#include <chrono>
#include <cstdio>
#include <ctime>

namespace clarity {

namespace {

bool number(std::string_view text, size_t at, size_t length, int& out) {
    if (at + length > text.size()) return false;
    out = 0;
    for (size_t i = at; i < at + length; ++i) {
        if (text[i] < '0' || text[i] > '9') return false;
        out = out * 10 + (text[i] - '0');
    }
    return true;
}

bool real(const Date& d) {
    if (d.month < 1 || d.month > 12 || d.day < 1) return false;
    return std::chrono::year_month_day{std::chrono::year{d.year}, std::chrono::month{unsigned(d.month)}, std::chrono::day{unsigned(d.day)}}.ok();
}

std::tm local_now() {
    std::time_t t = std::time(nullptr);
    std::tm out{};
#ifdef _WIN32
    localtime_s(&out, &t);
#else
    localtime_r(&t, &out);
#endif
    return out;
}

}  // namespace

std::optional<Date> Date::parse(std::string_view text) {
    Date d;
    if (text.size() != 10 || text[4] != '-' || text[7] != '-') return std::nullopt;
    if (!number(text, 0, 4, d.year) || !number(text, 5, 2, d.month) || !number(text, 8, 2, d.day)) return std::nullopt;
    if (!real(d)) return std::nullopt;
    return d;
}

std::string Date::str() const {
    char out[16];
    std::snprintf(out, sizeof out, "%04d-%02d-%02d", year, month, day);
    return out;
}

std::optional<DateTime> DateTime::parse(std::string_view text) {
    if (text.size() < 16 || (text[10] != 'T' && text[10] != ' ') || text[13] != ':') return std::nullopt;
    auto date = Date::parse(text.substr(0, 10));
    if (!date) return std::nullopt;
    DateTime t{*date};
    if (!number(text, 11, 2, t.hour) || !number(text, 14, 2, t.minute)) return std::nullopt;
    if (text.size() >= 19 && text[16] == ':') {
        if (!number(text, 17, 2, t.second)) return std::nullopt;
    } else if (text.size() != 16) {
        return std::nullopt;
    }
    if (t.hour > 23 || t.minute > 59 || t.second > 59) return std::nullopt;
    return t;
}

std::string DateTime::str() const {
    char out[16];
    std::snprintf(out, sizeof out, "T%02d:%02d:%02d", hour, minute, second);
    return date.str() + out;
}

Date today() {
    std::tm t = local_now();
    return Date{t.tm_year + 1900, t.tm_mon + 1, t.tm_mday};
}

DateTime now() {
    std::tm t = local_now();
    return DateTime{Date{t.tm_year + 1900, t.tm_mon + 1, t.tm_mday}, t.tm_hour, t.tm_min, t.tm_sec};
}

template <typename T>
static int compare_optional(const std::optional<T>& a, const std::optional<T>& b) {
    if (!a || !b) return int(a.has_value()) - int(b.has_value());
    return *a < *b ? -1 : *a > *b ? 1 : 0;
}

int compare(const std::optional<Date>& a, const std::optional<Date>& b) { return compare_optional(a, b); }
int compare(const std::optional<DateTime>& a, const std::optional<DateTime>& b) { return compare_optional(a, b); }

std::ostream& operator<<(std::ostream& out, const Date& value) { return out << value.str(); }
std::ostream& operator<<(std::ostream& out, const DateTime& value) { return out << value.str(); }

}  // namespace clarity
