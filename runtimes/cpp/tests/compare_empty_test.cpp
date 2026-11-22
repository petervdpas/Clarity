// A date compared with an empty one, as every stack compares them
// (conformance/compare-empty.json): generated C++ orders dates through compare.

#include <optional>
#include <string>

#include "check.hpp"
#include "conformance.hpp"
#include "clarity/values.hpp"

using namespace clarity;

TEST(an_empty_date_is_smallest_as_every_stack_compares_it) {
    for (const auto& c : conformance("compare-empty.json")["cases"]) {
        int order;
        if (c["kind"] == "date") {
            auto date = [](const nlohmann::json& v) { return v.is_null() ? std::optional<Date>() : Date::parse(v.get<std::string>()); };
            order = compare(date(c["a"]), date(c["b"]));
        } else {
            auto moment = [](const nlohmann::json& v) { return v.is_null() ? std::optional<DateTime>() : DateTime::parse(v.get<std::string>()); };
            order = compare(moment(c["a"]), moment(c["b"]));
        }
        std::string said = c["a"].dump() + " ? " + c["b"].dump() + " ";
        CHECK_EQ(said + "<" + (order < 0 ? "true" : "false"), said + "<" + (c["<"].get<bool>() ? "true" : "false"));
        CHECK_EQ(said + "<=" + (order <= 0 ? "true" : "false"), said + "<=" + (c["<="].get<bool>() ? "true" : "false"));
        CHECK_EQ(said + ">" + (order > 0 ? "true" : "false"), said + ">" + (c[">"].get<bool>() ? "true" : "false"));
        CHECK_EQ(said + ">=" + (order >= 0 ? "true" : "false"), said + ">=" + (c[">="].get<bool>() ? "true" : "false"));
    }
}
