// The slider and the spin box of U1c, moved by the runtime
// (conformance/ranges.json): how a key, a drag or a typed number moves a
// value within its range. Numbers come as text and stay exact, so the sums
// are done on whole numbers scaled by ten as often as needed.

#pragma once

#include <optional>
#include <string>

#include <nlohmann/json.hpp>

#include "clarity/errors.hpp"

namespace clarity {

/// A range: its lowest and highest value (none: no end) and its step.
struct NumberRange {
    std::optional<std::string> min;
    std::optional<std::string> max;
    std::string step = "1";
};

/// The value after `key` (up, down, pageUp, pageDown, home, end): a step up
/// or down, ten, or an end; within the range. No value counts as the lowest,
/// or 0. Throws InvalidError for text that is no number or another key.
std::string keyed(const std::optional<std::string>& value, const NumberRange& range, const std::string& key);

/// The value a slider takes where it is dragged to: the nearest step from
/// the lowest value, at most the highest step in the range.
std::string dragged(const std::string& at, const NumberRange& range);

/// A typed number, kept within the range.
std::string typed_in(const std::string& value, const NumberRange& range);

/// A value as the window holds it: a whole number as a number, a decimal as text.
nlohmann::json range_value(const std::string& value, bool whole);

}  // namespace clarity
