// The values records hold beside C++'s own: a calendar date and a date with
// a time of day, without a time zone, as Clarity's date and datetime. They
// read and write text as the renderer and the other stacks do: 2026-10-17
// and 2026-10-17T09:30:00.

#pragma once

#include <compare>
#include <optional>
#include <ostream>
#include <string>
#include <string_view>

namespace clarity {

struct Date {
    int year = 1970;
    int month = 1;
    int day = 1;

    /// `2026-10-17`; nothing for text that is no date.
    static std::optional<Date> parse(std::string_view text);
    std::string str() const;
    friend auto operator<=>(const Date&, const Date&) = default;
};

/// A date with a time of day, to the second.
struct DateTime {
    Date date;
    int hour = 0;
    int minute = 0;
    int second = 0;

    /// `2026-10-17T09:30:00`, also with a space or without seconds.
    static std::optional<DateTime> parse(std::string_view text);
    std::string str() const;
    friend auto operator<=>(const DateTime&, const DateTime&) = default;
};

/// The date where the application runs: Clarity's `today()`.
Date today();
/// The date and time where the application runs: Clarity's `now()`.
DateTime now();

/// Empty dates sort first, as in the other runtimes.
int compare(const std::optional<Date>& a, const std::optional<Date>& b);
int compare(const std::optional<DateTime>& a, const std::optional<DateTime>& b);

std::ostream& operator<<(std::ostream& out, const Date& value);
std::ostream& operator<<(std::ostream& out, const DateTime& value);

}  // namespace clarity
