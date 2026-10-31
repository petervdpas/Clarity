// Charts of U1c, grouped by the runtime (conformance/charts.json, `groups`):
// a list's rows grouped by a field, each group counted, summed or averaged,
// the groups in the order of their values. The axis and the slices' colors
// are drawing, the renderer's.

#pragma once

#include <string>
#include <vector>

#include <nlohmann/json.hpp>

namespace clarity {

/// A row as a chart reads it: the value it is grouped by and the number of its measure.
struct ChartRow {
    nlohmann::json by;
    nlohmann::json of;
};

/// One group: its value and its number.
struct ChartGroup {
    nlohmann::json key;
    nlohmann::json value;
};

/// The groups of `rows` grouped by a value of kind `by` (text, number,
/// date, enum or bool) with `measure` count, sum or average; `order` is an
/// enum's values in the order it declares them. Rows with no value are left out.
std::vector<ChartGroup> chart_groups(const std::vector<ChartRow>& rows, const std::string& by, const std::string& measure, const nlohmann::json& order);

}  // namespace clarity
