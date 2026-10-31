// A chart's groups, decided by the runtime, against the cases every runtime
// shares (conformance/charts.json, `groups`). The axis and the slices' colors
// are drawing, the renderer's.

#include "check.hpp"
#include "clarity/charts.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

TEST(a_chart_groups_its_rows_as_every_runtime_does) {
    for (const auto& c : conformance("charts.json")["groups"]) {
        std::vector<ChartRow> rows;
        for (const auto& r : c["rows"]) rows.push_back({r["by"], r["of"]});
        json got = json::array();
        for (const auto& g : chart_groups(rows, c["by"], c["measure"], c.value("order", json::array()))) got.push_back({{"key", g.key}, {"value", g.value}});
        if (got != c["gives"]) check::fail(__FILE__, __LINE__, c["name"].get<std::string>() + ": " + got.dump());
    }
}
