#include "clarity/trees.hpp"

#include <algorithm>
#include <functional>

namespace clarity {

std::vector<ShownRow> shown_tree(const std::vector<ReorderRow>& rows, const std::vector<nlohmann::json>& open) {
    auto parent = tree_parents(rows);
    std::vector<std::vector<int64_t>> under(rows.size());
    std::vector<int64_t> tops;
    for (size_t i = 0; i < parent.size(); ++i) (parent[i] == -1 ? tops : under[size_t(parent[i])]).push_back(int64_t(i));
    std::vector<ShownRow> out;
    std::function<void(int64_t, int64_t)> walk = [&](int64_t place, int64_t depth) {
        const auto& children = under[size_t(place)];
        out.push_back({place, depth, !children.empty()});
        if (std::find(open.begin(), open.end(), rows[size_t(place)].id) == open.end()) return;
        for (int64_t child : children) walk(child, depth + 1);
    };
    for (int64_t top : tops) walk(top, 0);
    return out;
}

std::vector<nlohmann::json> rows_above(const std::vector<ReorderRow>& rows, int64_t at) {
    std::vector<nlohmann::json> ids;
    if (at < 0 || at >= int64_t(rows.size())) return ids;
    auto parent = tree_parents(rows);
    for (int64_t p = parent[size_t(at)]; p != -1; p = parent[size_t(p)]) ids.insert(ids.begin(), rows[size_t(p)].id);
    return ids;
}

}  // namespace clarity
