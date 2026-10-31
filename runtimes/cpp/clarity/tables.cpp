#include "clarity/tables.hpp"

#include <algorithm>
#include <cmath>
#include <cstdlib>

#include "clarity/text.hpp"

namespace clarity {

using nlohmann::json;

namespace {

/// What a value sorts by: a number or text, or nothing when it is empty.
struct SortKey {
    bool empty = true;
    bool number = false;
    double n = 0;
    std::string text;
};

std::string text_of_value(const json& v) { return v.is_string() ? v.get<std::string>() : v.dump(); }

SortKey key_of(const TableColumn& column, const json& v) {
    SortKey k;
    if (v.is_null() || (v.is_string() && v.get<std::string>().empty())) return k;
    if (column.kind == "choice") {
        std::string label = text_of_value(v);
        if (column.choices.is_array())
            for (const auto& choice : column.choices)
                if (choice.is_array() && choice.size() == 2 && choice[0] == v) label = text_of_value(choice[1]);
        if (label.empty()) return k;
        k.empty = false;
        k.text = label;
        return k;
    }
    if (column.kind == "number") {
        if (v.is_number()) {
            k.n = v.get<double>();
        } else if (v.is_boolean()) {
            k.n = v.get<bool>() ? 1 : 0;
        } else {
            std::string t = trimmed(text_of_value(v));
            char* end = nullptr;
            k.n = std::strtod(t.c_str(), &end);
            if (t.empty() || end != t.c_str() + t.size() || std::isnan(k.n)) return k;
        }
        k.empty = false;
        k.number = true;
        return k;
    }
    k.empty = false;
    if (column.kind == "bool") {
        k.number = true;
        k.n = v == json(true) ? 1 : 0;
        return k;
    }
    k.text = text_of_value(v);
    return k;
}

}  // namespace

std::optional<TableOrder> next_order(const std::optional<TableOrder>& order, const std::string& field) {
    if (order && order->field == field) return TableOrder{field, !order->desc};
    return TableOrder{field, false};
}

std::vector<int64_t> shown_order(const json& rows, const std::vector<TableColumn>& columns, const std::optional<TableOrder>& order) {
    std::vector<int64_t> places;
    size_t count = rows.is_array() ? rows.size() : 0;
    for (size_t i = 0; i < count; ++i) places.push_back(int64_t(i));
    if (!order) return places;
    auto column = std::find_if(columns.begin(), columns.end(), [&](const TableColumn& c) { return c.field == order->field; });
    if (column == columns.end()) return places;
    std::vector<SortKey> keys;
    for (const auto& row : rows) keys.push_back(key_of(*column, row.is_object() && row.contains(column->field) ? row[column->field] : json(nullptr)));
    // A stable sort keeps equal rows in the order of the data.
    std::stable_sort(places.begin(), places.end(), [&](int64_t a, int64_t b) {
        const auto& ka = keys[size_t(a)];
        const auto& kb = keys[size_t(b)];
        if (ka.empty || kb.empty) return !ka.empty && kb.empty;
        int by;
        if (ka.number && kb.number) {
            by = ka.n < kb.n ? -1 : ka.n > kb.n ? 1 : 0;
        } else {
            std::string la = lower_case(ka.number ? json(ka.n).dump() : ka.text), lb = lower_case(kb.number ? json(kb.n).dump() : kb.text);
            by = la < lb ? -1 : la > lb ? 1 : 0;
        }
        return order->desc ? by > 0 : by < 0;
    });
    return places;
}

TablePage page_of(int64_t count, int64_t size, int64_t page) {
    if (size < 1) size = 1;
    if (count <= 0) return {1, 1, 0, 0, 0, "0 / 0"};
    int64_t pages = (count + size - 1) / size;
    int64_t shown = std::min(std::max(page, int64_t(1)), pages);
    int64_t from = (shown - 1) * size + 1;
    int64_t to = std::min(shown * size, count);
    return {shown, pages, from, to, count, std::to_string(from) + "-" + std::to_string(to) + " / " + std::to_string(count)};
}

int64_t page_with(int64_t size, int64_t at) { return size < 1 ? 1 : at / size + 1; }

}  // namespace clarity
