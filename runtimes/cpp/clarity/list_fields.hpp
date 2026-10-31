// A typed item of a list field (U1c), added by the runtime
// (conformance/list-fields.json, `added`): trimmed; empty text and an item
// the list has add nothing; text that is no value of the list's kind is
// refused, and the screen keeps it as typed.

#pragma once

#include <string>

#include <nlohmann/json.hpp>

namespace clarity {

/// What a typed item does to a list: the list it gives (the same list when
/// nothing is added), or `refused` with the list as it was.
struct AddedItem {
    nlohmann::json items = nlohmann::json::array();
    bool refused = false;
};

/// Whether two items of a list of `kind` are the same value: 1.50 and 1.5 are one decimal.
bool same_item(const std::string& kind, const nlohmann::json& a, const nlohmann::json& b);

/// `typed` added to `items`, a list of `kind` (text, integer, decimal, date,
/// or enum with its `values`): a whole number as a number, a decimal as
/// text, a date as 2026-10-31, an enum value by name.
AddedItem add_typed(const std::string& kind, const nlohmann::json& items, const std::string& typed, const nlohmann::json& values = nlohmann::json::array());

}  // namespace clarity
