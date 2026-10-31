// A tree's shown rows, decided by the runtime, against the cases every
// runtime shares (conformance/trees.json).

#include "check.hpp"
#include "clarity/trees.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

std::vector<ReorderRow> rows_of(const json& rows) {
    std::vector<ReorderRow> out;
    for (const auto& r : rows) out.push_back({r["id"], r["parent"]});
    return out;
}

}  // namespace

TEST(a_tree_shows_its_open_rows_as_every_runtime_does) {
    for (const auto& c : conformance("trees.json")["cases"]) {
        json shown = json::array();
        for (const auto& s : shown_tree(rows_of(c["rows"]), c["open"].get<std::vector<json>>())) shown.push_back({s.place, s.depth, s.children});
        if (shown != c["shows"]) check::fail(__FILE__, __LINE__, c["name"].get<std::string>() + ": " + shown.dump());
    }
}

TEST(a_chosen_row_opens_the_rows_above_it) {
    for (const auto& c : conformance("trees.json")["reveal"]) CHECK_EQ(json(rows_above(rows_of(c["rows"]), c["at"])), c["opens"]);
}
