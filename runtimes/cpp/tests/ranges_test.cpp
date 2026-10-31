// The slider and the spin box of U1c, moved by the runtime, against the
// cases every runtime shares (conformance/ranges.json).

#include "check.hpp"
#include "clarity/ranges.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

std::optional<std::string> text_or_none(const json& v) { return v.is_string() ? std::optional(v.get<std::string>()) : std::nullopt; }

NumberRange range_of(const json& c) { return {text_or_none(c["min"]), text_or_none(c["max"]), c.value("step", std::string("1"))}; }

}  // namespace

TEST(a_key_moves_a_value_within_its_range) {
    for (const auto& c : conformance("ranges.json")["keys"]) {
        std::string got = keyed(text_or_none(c["value"]), range_of(c), c["key"]);
        if (json(got) != c["after"]) check::fail(__FILE__, __LINE__, c.dump() + ": " + got);
    }
}

TEST(a_drag_takes_the_nearest_step) {
    for (const auto& c : conformance("ranges.json")["drag"]) {
        std::string got = dragged(c["at"], range_of(c));
        if (json(got) != c["value"]) check::fail(__FILE__, __LINE__, c.dump() + ": " + got);
    }
}

TEST(a_typed_number_is_kept_within_its_range) {
    for (const auto& c : conformance("ranges.json")["typed"]) {
        std::string got = typed_in(c["value"], range_of(c));
        if (json(got) != c["after"]) check::fail(__FILE__, __LINE__, c.dump() + ": " + got);
    }
}

// The window holds a whole number as a number, a decimal as text.
TEST(a_range_value_is_a_number_or_text) {
    CHECK_EQ(range_value("12", true), json(12));
    CHECK_EQ(range_value("12.5", false), json("12.5"));
    CHECK_THROWS(InvalidError, keyed(std::string("ten"), NumberRange{}, "up"));
}
