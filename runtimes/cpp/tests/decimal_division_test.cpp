// Dividing decimals as every stack divides them (conformance/decimal-division.json):
// generated C++ divides decimals with `/`, a value error for dividing by zero.

#include <string>

#include "check.hpp"
#include "conformance.hpp"
#include "clarity/decimal.hpp"
#include "clarity/code.hpp"
#include "clarity/errors.hpp"

using namespace clarity;

TEST(decimals_divide_as_every_stack_divides_them) {
    for (const auto& c : conformance("decimal-division.json")["cases"]) {
        std::string a = c["a"], b = c["b"];
        std::string got;
        try {
            got = (Decimal::parse(a) / Decimal::parse(b)).str();
        } catch (const ValueError&) {
            got = "a value error";
        } catch (const std::exception&) {
            got = "another error";
        }
        std::string want = c.value("error", false) ? "a value error" : c["quotient"].get<std::string>();
        CHECK_EQ(a + " / " + b + " = " + got, a + " / " + b + " = " + want);
    }
}
