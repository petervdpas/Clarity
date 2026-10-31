// A table's rows as they show, decided by the runtime
// (conformance/table-order.json, table-pages.json): text ignoring case,
// numbers as numbers, a choice by the label it shows, an empty value last
// either way, and equal rows in the order of the data. Rows keep their place
// in the data; only the order they show in changes. A table shown a page at
// a time cuts that order into pages.

#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

namespace clarity {

/// A column as it sorts: its field, its kind (text, number, bool, choice,
/// date and so on) and, for a choice, its `[value, label]` pairs.
struct TableColumn {
    std::string field;
    std::string kind;
    nlohmann::json choices = nlohmann::json::array();
};

/// The order of a table: by a field, the other way when `desc`.
struct TableOrder {
    std::string field;
    bool desc = false;
};

/// What a click on the header of `field` makes of the order: by it, then the other way.
std::optional<TableOrder> next_order(const std::optional<TableOrder>& order, const std::string& field);

/// The places in the data of `rows`, in the order they show.
std::vector<int64_t> shown_order(const nlohmann::json& rows, const std::vector<TableColumn>& columns, const std::optional<TableOrder>& order);

/// A page of a table: the page shown, how many there are, the first and
/// last row shown counted from 1 (0 and 0 for none), and the rows in all.
struct TablePage {
    int64_t page = 1;
    int64_t pages = 1;
    int64_t from = 0;
    int64_t to = 0;
    int64_t count = 0;
    /// `from-to / count`, or `0 / 0`.
    std::string text;
};

/// Page `page` of `count` rows, `size` to a page, kept within the pages there are.
TablePage page_of(int64_t count, int64_t size, int64_t page);

/// The page that shows the row at place `at` of the shown order, counted from 0.
int64_t page_with(int64_t size, int64_t at);

}  // namespace clarity
