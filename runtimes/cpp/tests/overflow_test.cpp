// Whole numbers and decimals that do not fit, as every stack treats them
// (conformance/overflow.json): generated C++ works whole numbers out through
// whole_add and the others and decimals with their operators, a value error,
// never a wrapped number.

#include <string>

#include "check.hpp"
#include "conformance.hpp"
#include "clarity/decimal.hpp"
#include "clarity/code.hpp"
#include "clarity/errors.hpp"

using namespace clarity;

namespace {

std::string whole(const std::string& op, int64_t a, int64_t b) {
    if (op == "+") return std::to_string(whole_add(a, b));
    if (op == "-") return std::to_string(whole_sub(a, b));
    if (op == "*") return std::to_string(whole_mul(a, b));
    if (op == "div") return std::to_string(whole_div(a, b));
    if (op == "mod") return std::to_string(whole_mod(a, b));
    return std::to_string(whole_neg(a));
}

std::string decimal(const std::string& op, const Decimal& a, const Decimal& b) {
    if (op == "+") return (a + b).str();
    if (op == "-") return (a - b).str();
    if (op == "*") return (a * b).str();
    return (-a).str();
}

}  // namespace

TEST(numbers_that_do_not_fit_are_a_value_error) {
    for (const auto& c : conformance("overflow.json")["cases"]) {
        std::string kind = c["kind"], op = c["op"], a = c["a"], b = c.value("b", std::string("0"));
        std::string got;
        try {
            got = kind == "whole" ? whole(op, std::stoll(a), std::stoll(b)) : decimal(op, Decimal::parse(a), Decimal::parse(b));
        } catch (const ValueError&) {
            got = "a value error";
        } catch (const std::exception&) {
            got = "another error";
        }
        std::string want = c.value("error", false) ? "a value error" : c["result"].get<std::string>();
        std::string said = kind + " " + a + " " + op + " " + b + " = ";
        CHECK_EQ(said + got, said + want);
    }
}
