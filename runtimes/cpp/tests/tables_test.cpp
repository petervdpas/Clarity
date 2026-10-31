// A table's shown order and its pages, decided by the runtime, against the
// cases every runtime shares (conformance/table-order.json, table-pages.json).

#include "check.hpp"
#include "clarity/tables.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

std::optional<TableOrder> order_of(const json& order) {
    if (!order.is_object()) return std::nullopt;
    return TableOrder{order["field"], order.value("desc", false)};
}

}  // namespace

TEST(a_table_shows_its_rows_in_order_as_every_runtime_does) {
    auto file = conformance("table-order.json");
    std::vector<TableColumn> columns;
    for (const auto& c : file["columns"]) columns.push_back({c["field"], c["kind"], c.value("choices", json::array())});
    for (const auto& c : file["cases"]) {
        std::string name = c["name"];
        auto order = order_of(c["order"]);
        json events = json::array();
        std::optional<int64_t> chosen;
        for (const auto& step : c["steps"]) {
            if (step[0] == "sort") {
                if (c["sortable"].get<bool>()) order = next_order(order, step[1]);
                continue;
            }
            int64_t place = shown_order(file["rows"], columns, order)[step[1].get<size_t>()];
            events.push_back("select " + std::to_string(place));
            chosen = place;
        }
        const json& expect = c["expect"];
        json shown = shown_order(file["rows"], columns, order);
        if (shown != expect["shown"]) check::fail(__FILE__, __LINE__, name + ": shown " + shown.dump());
        if (events != expect["events"]) check::fail(__FILE__, __LINE__, name + ": events " + events.dump());
        if (expect.contains("chosen") && chosen != expect["chosen"].get<int64_t>()) check::fail(__FILE__, __LINE__, name + ": chosen");
    }
}

TEST(a_table_shows_a_page_as_every_runtime_does) {
    auto file = conformance("table-pages.json");
    for (const auto& c : file["pages"]) {
        auto got = page_of(c["count"], c["size"], c["page"]);
        json shown = {{"shows", got.page}, {"pages", got.pages}, {"from", got.from}, {"to", got.to}, {"text", got.text}};
        json want = {{"shows", c["shows"]}, {"pages", c["pages"]}, {"from", c["from"]}, {"to", c["to"]}, {"text", c["text"]}};
        if (shown != want) check::fail(__FILE__, __LINE__, c.dump() + ": " + shown.dump());
    }
    for (const auto& c : file["selected"]) CHECK_EQ(page_with(c["size"], c["at"]), c["page"].get<int64_t>());
}
