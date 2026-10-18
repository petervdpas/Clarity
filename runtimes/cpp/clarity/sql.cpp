#include "clarity/sql.hpp"

#include <cmath>

#include "clarity/errors.hpp"

namespace clarity::sql {

namespace {

[[noreturn]] void unfit(const std::string& column, const std::string& what) {
    throw StorageError(column + ": the stored value is no " + what);
}

}  // namespace

SqlValue picture(const std::optional<Picture>& value) {
    if (!value) return null();
    switch (value->state) {
    case Picture::State::Stored: return Kept{};
    case Picture::State::New: return value->bytes;
    default: return value->path;
    }
}

std::string get_text(const std::string& column, const SqlValue& value) {
    if (auto s = std::get_if<std::string>(&value)) return *s;
    if (std::holds_alternative<std::monostate>(value)) return "";
    if (auto n = std::get_if<int64_t>(&value)) return std::to_string(*n);
    if (auto b = std::get_if<Bytes>(&value)) return std::string(b->begin(), b->end());
    unfit(column, "text");
}

int64_t get_integer(const std::string& column, const SqlValue& value) {
    if (auto n = std::get_if<int64_t>(&value)) return *n;
    if (std::holds_alternative<std::monostate>(value)) return 0;
    if (auto b = std::get_if<bool>(&value)) return *b;
    if (auto r = std::get_if<double>(&value)) return int64_t(std::llround(*r));
    if (auto s = std::get_if<std::string>(&value)) {
        try {
            size_t used = 0;
            int64_t n = std::stoll(*s, &used);
            if (used == s->size()) return n;
        } catch (const std::exception&) {
        }
        // Text that is no number counts as none, as a migration step leaves it.
        if (s->empty()) return 0;
    }
    unfit(column, "whole number");
}

Decimal get_decimal(const std::string& column, const SqlValue& value) {
    if (auto d = std::get_if<Decimal>(&value)) return *d;
    if (auto s = std::get_if<std::string>(&value)) return s->empty() ? Decimal() : Decimal::parse(*s);
    if (auto n = std::get_if<int64_t>(&value)) return Decimal(*n);
    if (auto r = std::get_if<double>(&value)) {
        char text[64];
        std::snprintf(text, sizeof text, "%.10g", *r);
        return Decimal::parse(text);
    }
    if (std::holds_alternative<std::monostate>(value)) return Decimal();
    unfit(column, "number");
}

bool get_boolean(const std::string& column, const SqlValue& value) {
    if (auto b = std::get_if<bool>(&value)) return *b;
    return get_integer(column, value) != 0;
}

std::optional<Date> get_date(const std::string& column, const SqlValue& value) {
    if (std::holds_alternative<std::monostate>(value)) return std::nullopt;
    if (auto d = std::get_if<Date>(&value)) return *d;
    if (auto s = std::get_if<std::string>(&value)) {
        // A date can be written with a time of day after it.
        if (auto d = Date::parse(s->substr(0, 10))) return d;
    }
    unfit(column, "date");
}

std::optional<DateTime> get_date_time(const std::string& column, const SqlValue& value) {
    if (std::holds_alternative<std::monostate>(value)) return std::nullopt;
    if (auto t = std::get_if<DateTime>(&value)) return *t;
    if (auto s = std::get_if<std::string>(&value)) {
        if (auto t = DateTime::parse(*s)) return t;
    }
    unfit(column, "date and time");
}

std::optional<int64_t> get_reference(const std::string& column, const SqlValue& value) {
    if (std::holds_alternative<std::monostate>(value)) return std::nullopt;
    return get_integer(column, value);
}

std::optional<Picture> get_picture(const std::string& column, const SqlValue& value) {
    bool there = false;
    if (auto n = std::get_if<int64_t>(&value)) there = *n != 0;
    else if (auto s = std::get_if<std::string>(&value)) there = !s->empty();
    else if (auto b = std::get_if<Bytes>(&value)) there = !b->empty();
    else if (auto f = std::get_if<bool>(&value)) there = *f;
    else if (!std::holds_alternative<std::monostate>(value)) unfit(column, "picture");
    return there ? std::optional<Picture>(Picture::stored()) : std::nullopt;
}

}  // namespace clarity::sql
