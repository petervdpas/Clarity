// Trees of U1c, as the runtime shows them (conformance/trees.json): the rows
// of a list arranged by their parent (tree_parents in reorder.hpp), a row
// under its parent when that is in the list and at the top otherwise, each
// level in the order of the list; a closed row hides the rows under it.

#pragma once

#include <cstdint>
#include <vector>

#include <nlohmann/json.hpp>

#include "clarity/reorder.hpp"

namespace clarity {

/// A row shown: its place in the list, its depth, and whether rows are under it.
struct ShownRow {
    int64_t place = 0;
    int64_t depth = 0;
    bool children = false;
};

/// The rows shown, top to bottom, with `open` the ids of the rows shown open.
std::vector<ShownRow> shown_tree(const std::vector<ReorderRow>& rows, const std::vector<nlohmann::json>& open);

/// The ids of the rows above the row at `at`, top first: what a chosen row opens.
std::vector<nlohmann::json> rows_above(const std::vector<ReorderRow>& rows, int64_t at);

}  // namespace clarity
