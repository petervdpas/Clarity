// Reordering (DESIGN 5.30), decided by the runtime, against the cases every
// runtime shares (conformance/reorder.json).

#include "check.hpp"
#include "clarity/reorder.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

void moves_as_every_runtime_does(const char* kind, bool tree) {
    for (const auto& c : conformance("reorder.json")[kind]) {
        std::string name = c["name"];
        std::vector<ReorderRow> rows;
        for (const auto& r : c["rows"]) rows.push_back({r["id"], r.value("parent", json(nullptr))});
        auto got = reorder(rows, move_of(c["move"]), tree);
        if (c.value("refused", false)) {
            if (got) check::fail(__FILE__, __LINE__, name + ": not refused");
            continue;
        }
        if (!got) {
            check::fail(__FILE__, __LINE__, name + ": refused");
            continue;
        }
        if (json(got->order) != c["order"]) check::fail(__FILE__, __LINE__, name + ": order " + json(got->order).dump());
        if (json(got->numbers) != c["numbers"]) check::fail(__FILE__, __LINE__, name + ": numbers " + json(got->numbers).dump());
        if (tree && json(got->parents) != c["parents"]) check::fail(__FILE__, __LINE__, name + ": parents " + json(got->parents).dump());
    }
}

}  // namespace

TEST(a_list_is_reordered_as_every_runtime_does) { moves_as_every_runtime_does("lists", false); }

TEST(a_tree_is_reordered_as_every_runtime_does) { moves_as_every_runtime_does("trees", true); }
