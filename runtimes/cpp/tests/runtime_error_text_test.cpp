// What a window says when code fails at a value or a record: Clarity's own
// words by a literal key (translated where the project has them), while the
// error's message stays the English of the shared cases.

#include <climits>
#include <string>

#include "check.hpp"
#include "clarity/code.hpp"
#include "clarity/decimal.hpp"
#include "clarity/errors.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

/// The message of what `run` throws, and the words a window shows for it ("none" for none of Clarity's).
template <typename Run>
std::pair<std::string, std::string> thrown(Run run) {
    try {
        run();
    } catch (const std::exception& e) {
        auto words = said(e);
        return {e.what(), words ? json(*words).dump() : "none"};
    }
    return {"nothing thrown", ""};
}

}  // namespace

TEST(runtime_errors_are_said_by_their_keys) {
    std::vector<int64_t> one{1};
    auto [outside, outside_said] = thrown([&] { (void)at(one, 10); });
    CHECK_EQ(outside, std::string("there is no item 10 in a list of 1"));
    CHECK_EQ(json::parse(outside_said), json::parse(R"({"key":"itemOutsideList","text":"There is no item {position} in a list of {count}","values":{"position":{"kind":"integer","value":10},"count":{"kind":"integer","value":1}}})"));

    auto [division, division_said] = thrown([] { (void)whole_div(1, 0); });
    CHECK_EQ(division, std::string("division by zero"));
    CHECK_EQ(json::parse(division_said), json::parse(R"({"key":"divisionByZero","text":"Division by zero"})"));
    CHECK_EQ(json::parse(thrown([] { (void)(Decimal::parse("1") / Decimal::parse("0")); }).second), json::parse(R"({"key":"divisionByZero","text":"Division by zero"})"));

    auto [large, large_said] = thrown([] { (void)whole_add(INT64_MAX, 1); });
    CHECK_EQ(large, std::string("a number too large"));
    CHECK_EQ(json::parse(large_said), json::parse(R"({"key":"numberTooLarge","text":"The number is too large"})"));
    CHECK_EQ(json::parse(thrown([] { (void)(Decimal::parse("79228162514264337593543950335") * Decimal::parse("2")); }).second), json::parse(R"({"key":"numberTooLarge","text":"The number is too large"})"));

    CHECK_EQ(json::parse(thrown([] { throw NotFoundError("Customer", 3); }).second),
             json::parse(R"({"key":"recordNotThere","text":"There is no {table} with id {id}","values":{"table":{"kind":"text","value":"Customer"},"id":{"kind":"integer","value":3}}})"));
    CHECK_EQ(json::parse(thrown([] { throw InUseError("Customer", 3); }).second),
             json::parse(R"({"key":"recordStillUsed","text":"This {table} is still used by other records","values":{"table":{"kind":"text","value":"Customer"}}})"));

    // A refusal is said in its own words; an error of its own has none of Clarity's.
    CHECK_EQ(json::parse(thrown([] { refuse(Text::key("tooLate")); }).second), json::parse(R"({"key":"tooLate"})"));
    CHECK_EQ(thrown([] { throw StorageError("locked"); }).second, std::string("none"));
}
