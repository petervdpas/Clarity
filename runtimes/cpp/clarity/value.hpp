// A stored value, as records are written and read: what a record gives to
// save and what a row of the database holds. SQLite keeps decimals and
// dates as text, so no precision is lost.

#pragma once

#include <cstdint>
#include <string>
#include <variant>
#include <vector>

#include "clarity/decimal.hpp"
#include "clarity/values.hpp"

namespace clarity {

/// A picture the record has stored already: its column is left as it is
/// when the record is saved. It never reaches the database.
struct Kept {
    friend bool operator==(const Kept&, const Kept&) = default;
};

using Bytes = std::vector<uint8_t>;

using SqlValue = std::variant<std::monostate, int64_t, double, std::string, Bytes, Decimal, bool, Date, DateTime, Kept>;

/// No value: NULL.
inline SqlValue null() { return std::monostate{}; }

}  // namespace clarity
