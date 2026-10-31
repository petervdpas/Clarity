#include "clarity/reorder.hpp"

#include <algorithm>
#include <map>

namespace clarity {

using nlohmann::json;

namespace {

int64_t number_in(const json& move, const char* key) { return move.contains(key) && move[key].is_number_integer() ? move[key].get<int64_t>() : -1; }

std::string text_in(const json& move, const char* key) { return move.contains(key) && move[key].is_string() ? move[key].get<std::string>() : ""; }

/// Each row's place at its level counted from 1, in the order of the list.
std::vector<int64_t> numbered(const std::vector<int64_t>& levels, const std::vector<int64_t>& order) {
    std::map<int64_t, int64_t> counts;
    std::vector<int64_t> numbers;
    for (int64_t place : order) numbers.push_back(++counts[levels[size_t(place)]]);
    return numbers;
}

}  // namespace

RowMove move_of(const json& move) {
    if (!move.is_object()) return {};
    RowMove m;
    m.row = number_in(move, "row");
    m.key = text_in(move, "key");
    if (m.key.empty()) {
        m.where = text_in(move, "where");
        m.target = number_in(move, "target");
    }
    return m;
}

std::vector<int64_t> tree_parents(const std::vector<ReorderRow>& rows) {
    std::map<json, int64_t> at;
    for (size_t i = 0; i < rows.size(); ++i) at.emplace(rows[i].id, int64_t(i));
    std::vector<int64_t> parent;
    for (const auto& r : rows) {
        auto found = r.parent.is_null() ? at.end() : at.find(r.parent);
        parent.push_back(found == at.end() ? -1 : found->second);
    }
    // Break each loop at its row that comes first in the list.
    for (size_t i = 0; i < rows.size(); ++i) {
        std::vector<int64_t> seen;
        int64_t p = int64_t(i);
        while (p != -1 && std::find(seen.begin(), seen.end(), p) == seen.end()) {
            seen.push_back(p);
            p = parent[size_t(p)];
        }
        if (p != -1) {
            auto loop = std::find(seen.begin(), seen.end(), p);
            parent[size_t(*std::min_element(loop, seen.end()))] = -1;
        }
    }
    return parent;
}

std::optional<Reordered> reorder(const std::vector<ReorderRow>& given, const RowMove& move, bool tree) {
    const int64_t count = int64_t(given.size());
    const int64_t r = move.row;
    if (r < 0 || r >= count) return std::nullopt;
    std::vector<ReorderRow> rows;
    for (const auto& row : given) rows.push_back({row.id, tree ? row.parent : json(nullptr)});
    auto levels = tree_parents(rows);
    auto row_at = [&](int64_t place) -> const ReorderRow& { return rows[size_t(place)]; };
    auto level = [&](int64_t place) {
        std::vector<int64_t> places;
        for (size_t i = 0; i < levels.size(); ++i)
            if (levels[i] == place) places.push_back(int64_t(i));
        return places;
    };
    auto under = [&](int64_t place) {
        for (int64_t p = place; p != -1; p = levels[size_t(p)])
            if (p == r) return true;
        return false;
    };
    // The last of a row and the rows right under it, leaving out the moved row.
    auto last_of = [&](int64_t place) {
        int64_t last = place;
        for (int64_t i : level(place))
            if (i != r) last = std::max(last, i);
        return last;
    };
    json parent = row_at(r).parent;
    int64_t anchor = 0;
    bool after = false;
    if (!move.key.empty()) {
        auto siblings = level(levels[size_t(r)]);
        int64_t k = std::find(siblings.begin(), siblings.end(), r) - siblings.begin();
        if (move.key == "up" || move.key == "down") {
            int64_t next = move.key == "up" ? k - 1 : k + 1;
            if (next < 0 || next >= int64_t(siblings.size())) return std::nullopt;
            anchor = siblings[size_t(next)];
            after = move.key == "down";
        } else if (!tree) {
            return std::nullopt;
        } else if (move.key == "left") {
            if (levels[size_t(r)] == -1) return std::nullopt;
            anchor = levels[size_t(r)];
            after = true;
            parent = row_at(anchor).parent;
        } else if (move.key == "right") {
            if (k == 0) return std::nullopt;
            int64_t before = siblings[size_t(k - 1)];
            anchor = last_of(before);
            after = true;
            parent = row_at(before).id;
        } else {
            return std::nullopt;
        }
    } else {
        int64_t t = move.target;
        if (t < 0 || t >= count || t == r || (tree && under(t))) return std::nullopt;
        if (move.where == "into") {
            if (!tree) return std::nullopt;
            anchor = last_of(t);
            after = true;
            parent = row_at(t).id;
        } else if (move.where == "before" || move.where == "after") {
            anchor = t;
            after = move.where == "after";
            parent = tree ? row_at(t).parent : json(nullptr);
        } else {
            return std::nullopt;
        }
    }
    std::vector<int64_t> order;
    for (int64_t i = 0; i < count; ++i)
        if (i != r) order.push_back(i);
    order.insert(std::find(order.begin(), order.end(), anchor) + (after ? 1 : 0), r);
    auto moved = rows;
    moved[size_t(r)].parent = parent;
    std::vector<ReorderRow> in_order;
    for (int64_t place : order) in_order.push_back(moved[size_t(place)]);
    auto new_levels = tree_parents(in_order);
    // Levels by old place, so both lists are numbered alike.
    std::vector<int64_t> by_place(static_cast<size_t>(count), -1);
    for (size_t i = 0; i < order.size(); ++i) by_place[size_t(order[i])] = new_levels[i] == -1 ? -1 : order[size_t(new_levels[i])];
    auto numbers = numbered(by_place, order);
    std::vector<int64_t> all(static_cast<size_t>(count));
    for (int64_t i = 0; i < count; ++i) all[size_t(i)] = i;
    auto before = numbered(levels, all);
    bool same = true;
    for (size_t i = 0; i < order.size(); ++i) {
        size_t place = size_t(order[i]);
        if (numbers[i] != before[place] || moved[place].parent != rows[place].parent) same = false;
    }
    if (same) return std::nullopt;
    Reordered result{order, {}, numbers};
    for (int64_t place : order) result.parents.push_back(moved[size_t(place)].parent);
    return result;
}

}  // namespace clarity
