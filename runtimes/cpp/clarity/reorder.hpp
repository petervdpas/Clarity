// Reordering (DESIGN 5.30), decided by the runtime (conformance/reorder.json):
// a row of a list, table, tree or tree table moved by a drag or by Alt and
// an arrow key. The result is the list's new order, each row's parent (in a
// tree) and each row's number at its level, which the part writes into the
// field of `reorder by`. A move that changes no parent or number is refused.

#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

namespace clarity {

/// A row as a tree places it: its id, and the id of its parent or null.
struct ReorderRow {
    nlohmann::json id;
    nlohmann::json parent;
};

/// A drag onto a row (`where` before, after or into `target`), or a key on
/// the moved row (`key` up, down, left or right); places count from 0.
struct RowMove {
    int64_t row = -1;
    std::string where;
    int64_t target = -1;
    std::string key;
};

/// A move as the screen sends it: `{row, where, target}` or `{row, key}`.
RowMove move_of(const nlohmann::json& move);

/// The list after a move: the old places in the new order, each row's parent and number in that order.
struct Reordered {
    std::vector<int64_t> order;
    std::vector<nlohmann::json> parents;
    std::vector<int64_t> numbers;
};

/// The place of each row's parent in the list, or -1 for a row at the top;
/// a loop of parents is broken at its row that comes first (conformance/trees.json).
std::vector<int64_t> tree_parents(const std::vector<ReorderRow>& rows);

/// The rows after `move`, or none when it changes nothing. In a list that is no tree, rows have no parent.
std::optional<Reordered> reorder(const std::vector<ReorderRow>& rows, const RowMove& move, bool tree);

}  // namespace clarity
