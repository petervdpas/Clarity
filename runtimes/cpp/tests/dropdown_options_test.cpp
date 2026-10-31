// A dropdown's options (U1c), found by the runtime, against the cases every
// runtime shares (conformance/dropdown-options.json).

#include "check.hpp"
#include "clarity/dropdown_options.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

TEST(a_typed_dropdown_suggests_the_values_its_field_has) {
    for (const auto& c : conformance("dropdown-options.json")["suggestions"]) {
        auto got = suggestions(c["values"].get<std::vector<std::string>>(), c["typed"]);
        if (json(got) != c["gives"]) check::fail(__FILE__, __LINE__, c.dump() + ": " + json(got).dump());
    }
}

TEST(a_search_dropdown_offers_the_records_with_every_typed_word) {
    auto file = conformance("dropdown-options.json")["search"];
    std::vector<RefOption> options;
    for (const auto& o : file["options"]) options.push_back({o["id"], o["text"], o["fields"]});
    for (const auto& c : file["cases"]) {
        json ids = json::array();
        for (const auto& o : searched(options, c["typed"], {"name", "city"})) ids.push_back(o.id);
        if (ids != c["gives"]) check::fail(__FILE__, __LINE__, c.dump() + ": " + ids.dump());
    }
    // Only the fields it shows count.
    CHECK_EQ(searched(options, "utrecht", {"name"}).size(), size_t(0));
    CHECK_EQ(searched(options, "utrecht", {}).size(), size_t(2));
}
