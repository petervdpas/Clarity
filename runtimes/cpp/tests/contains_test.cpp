// `contains` outside a query, as every stack does it (conformance/contains.json).

#include <string>

#include "check.hpp"
#include "conformance.hpp"
#include "clarity/query.hpp"

using namespace clarity;

TEST(text_contains_a_part_case_ignored_as_every_stack_does) {
    for (const auto& c : conformance("contains.json")["cases"]) {
        std::string text = c["text"], part = c["part"];
        std::string said = "'" + text + "' contains '" + part + "': ";
        CHECK_EQ(said + (text_contains(text, part) ? "yes" : "no"), said + (c["contains"].get<bool>() ? "yes" : "no"));
    }
}
