#include "clarity/views.hpp"

#include <algorithm>
#include <cmath>
#include <cstdlib>
#include <optional>

#include "clarity/charts.hpp"
#include "clarity/paths.hpp"
#include "clarity/tables.hpp"
#include "clarity/text.hpp"
#include "clarity/trees.hpp"

namespace clarity {

using nlohmann::json;

namespace {

std::string text_in(const json& view, const char* key, const std::string& otherwise = "") {
    return view.contains(key) && view[key].is_string() ? view[key].get<std::string>() : otherwise;
}

json field_of(const json& row, const std::string& field) { return row.is_object() && row.contains(field) ? row[field] : json(nullptr); }

json rows_of(const WindowState& state, const json& view) {
    std::string list = text_in(view, "list");
    json rows = list.empty() ? json(nullptr) : value_at(state.data, list);
    return rows.is_array() ? rows : json::array();
}

std::optional<TableOrder> order_in(const json& view) {
    if (!view.contains("order") || !view["order"].is_object()) return std::nullopt;
    const json& order = view["order"];
    return TableOrder{text_in(order, "field"), order.value("desc", false)};
}

std::vector<ReorderRow> tree_rows(const json& rows, const json& view) {
    std::string key = text_in(view, "key", "id"), parent = text_in(view, "parent", "parent");
    std::vector<ReorderRow> out;
    for (const auto& row : rows) out.push_back({field_of(row, key), field_of(row, parent)});
    return out;
}

json open_in(const json& view) { return view.contains("open") && view["open"].is_array() ? view["open"] : json::array(); }

void fill_table(json& view, const json& rows, std::optional<int> chosen) {
    std::vector<TableColumn> columns;
    if (view.contains("columns") && view["columns"].is_array())
        for (const auto& c : view["columns"])
            if (c.is_object()) columns.push_back({text_in(c, "field"), text_in(c, "kind", "text"), c.value("choices", json::array())});
    auto shown = shown_order(rows, columns, order_in(view));
    bool paged = view.contains("page") && view["page"].is_number_integer() && view["page"].get<int64_t>() > 0;
    if (!paged) {
        view["shown"] = shown;
        view.erase("pages");
        return;
    }
    int64_t size = view["page"];
    int64_t at = view.contains("at") && view["at"].is_number_integer() ? view["at"].get<int64_t>() : 1;
    if (chosen) {
        auto found = std::find(shown.begin(), shown.end(), int64_t(*chosen));
        if (found != shown.end()) at = page_with(size, found - shown.begin());
    }
    auto page = page_of(int64_t(shown.size()), size, at);
    view["at"] = page.page;
    view["pages"] = {{"page", page.page}, {"pages", page.pages}, {"from", page.from}, {"to", page.to}, {"count", page.count}};
    json places = json::array();
    for (int64_t i = page.from; i >= 1 && i <= page.to; ++i) places.push_back(shown[size_t(i - 1)]);
    view["shown"] = places;
}

void fill_tree(json& view, const json& rows, std::optional<int> chosen) {
    auto tree = tree_rows(rows, view);
    json open = open_in(view);
    if (chosen)
        for (const auto& id : rows_above(tree, *chosen))
            if (std::find(open.begin(), open.end(), id) == open.end()) open.push_back(id);
    view["open"] = open;
    json shown = json::array();
    for (const auto& s : shown_tree(tree, open.get<std::vector<json>>())) shown.push_back({s.place, s.depth, s.children});
    view["shown"] = shown;
}

void fill_chart(json& view, const json& rows) {
    std::string by = text_in(view, "by"), of = text_in(view, "of");
    std::vector<ChartRow> chart;
    for (const auto& row : rows) chart.push_back({field_of(row, by), of.empty() ? json(nullptr) : field_of(row, of)});
    json groups = json::array();
    for (const auto& g : chart_groups(chart, text_in(view, "byKind", "text"), text_in(view, "measure", "count"), view.value("values", json::array())))
        groups.push_back({{"key", g.key}, {"value", g.value}});
    view["groups"] = groups;
}

/// A value as a number: a number, or text that is not blank and reads as a finite one.
std::optional<double> number_in(const json& v) {
    if (v.is_number()) return v.get<double>();
    if (!v.is_string()) return std::nullopt;
    std::string t = trimmed(v.get<std::string>());
    if (t.empty()) return std::nullopt;
    char* end = nullptr;
    double n = std::strtod(t.c_str(), &end);
    if (end != t.c_str() + t.size() || !std::isfinite(n)) return std::nullopt;
    return n;
}

/// The last `points` numbers of the list a chart of numbers draws; all of them without `points`.
void fill_numbers(json& view, const json& rows) {
    json numbers = json::array();
    for (const auto& v : rows)
        if (auto n = number_in(v)) numbers.push_back(*n == std::floor(*n) && std::fabs(*n) < 9e15 ? json(int64_t(*n)) : json(*n));
    int64_t points = view.contains("points") && view["points"].is_number_integer() ? view["points"].get<int64_t>() : 0;
    if (points > 0 && int64_t(numbers.size()) > points) numbers.erase(numbers.begin(), numbers.end() - points);
    view["numbers"] = numbers;
}

}  // namespace

void fill_views(WindowState& state, const std::map<std::string, int>* sent) {
    if (!state.views.is_object()) {
        state.views = json::object();
        return;
    }
    for (auto it = state.views.begin(); it != state.views.end(); ++it) {
        const std::string& part = it.key();
        json& view = it.value();
        if (!view.is_object()) continue;
        // The chosen row; after the person's own view action, only one chosen since.
        std::optional<int> chosen;
        auto now = state.selected.find(part);
        if (now != state.selected.end()) {
            auto was = sent ? sent->find(part) : std::map<std::string, int>::const_iterator{};
            if (!sent || was == sent->end() || was->second != now->second) chosen = now->second;
        }
        json rows = rows_of(state, view);
        std::string kind = text_in(view, "kind");
        if (kind == "table") fill_table(view, rows, chosen);
        else if (kind == "tree") fill_tree(view, rows, chosen);
        else if (kind == "chart") fill_chart(view, rows);
        else if (kind == "numbers") fill_numbers(view, rows);
    }
}

void view_action(WindowState& state, const std::string& part, const json& action) {
    // Nothing to do: the reply shows a view the part declared anew.
    if (!action.is_object() || action.empty()) return;
    if (!state.views.is_object() || !state.views.contains(part) || !state.views[part].is_object()) throw InvalidError(part + " shows no list");
    json& view = state.views[part];
    // Only a sortable table sorts by a header.
    if (action.contains("sort") && action["sort"].is_string() && view.value("sortable", json(false)) == json(true)) {
        auto order = next_order(order_in(view), action["sort"]);
        view["order"] = {{"field", order->field}, {"desc", order->desc}};
        // A new order starts on the first page.
        view["at"] = 1;
    }
    if (action.contains("page") && action["page"].is_number_integer()) view["at"] = action["page"];
    json open = open_in(view);
    if (action.contains("open") && std::find(open.begin(), open.end(), action["open"]) == open.end()) open.push_back(action["open"]);
    if (action.contains("close")) open.erase(std::remove(open.begin(), open.end(), action["close"]), open.end());
    if (action.contains("open") || action.contains("close")) view["open"] = open;
}

}  // namespace clarity
