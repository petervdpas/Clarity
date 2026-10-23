// The conversions between field values and stored values, used by the
// records Clarity generates: `values()` gives each field as a stored value,
// `from_values` reads each back. Each driver stores a value its own way:
// SQLite keeps decimals and dates as text, so no precision is lost.

#pragma once

#include <optional>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "clarity/picture.hpp"
#include "clarity/value.hpp"

namespace clarity::sql {

inline SqlValue text(const std::string& value) { return value; }
/// A list of values as its column keeps it: compact JSON, text as it is
/// (conformance/list-fields.json).
template <typename T>
SqlValue list(const std::vector<T>& items) { return nlohmann::json(items).dump(); }
inline SqlValue integer(int64_t value) { return value; }
inline SqlValue decimal(const Decimal& value) { return value; }
inline SqlValue boolean(bool value) { return value; }
inline SqlValue date(const std::optional<Date>& value) { return value ? SqlValue(*value) : null(); }
inline SqlValue date_time(const std::optional<DateTime>& value) { return value ? SqlValue(*value) : null(); }
inline SqlValue reference(const std::optional<int64_t>& value) { return value ? SqlValue(*value) : null(); }
/// A picture: none, the one stored already (kept), a new one, or a file of its store.
SqlValue picture(const std::optional<Picture>& value);

std::string get_text(const std::string& column, const SqlValue& value);
/// The items of a list column as JSON, read as the field's list with
/// `.get<std::vector<T>>()`; none when it is null or empty.
nlohmann::json get_list(const std::string& column, const SqlValue& value);
int64_t get_integer(const std::string& column, const SqlValue& value);
Decimal get_decimal(const std::string& column, const SqlValue& value);
bool get_boolean(const std::string& column, const SqlValue& value);
std::optional<Date> get_date(const std::string& column, const SqlValue& value);
std::optional<DateTime> get_date_time(const std::string& column, const SqlValue& value);
std::optional<int64_t> get_reference(const std::string& column, const SqlValue& value);
/// Whether a record has a picture: what the select list reads of one kept
/// in the database (1 or 0), or the path of its file in a file store.
std::optional<Picture> get_picture(const std::string& column, const SqlValue& value);

}  // namespace clarity::sql
