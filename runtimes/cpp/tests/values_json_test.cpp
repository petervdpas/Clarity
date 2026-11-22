// A value written into JSON as every stack writes it for parts, reports, the
// wire and `json of` (conformance/values-json.json): nlohmann's JSON of the
// types a generated record holds, dumped as a program dumps it.

#include <optional>
#include <string>
#include <vector>

#include "check.hpp"
#include "conformance.hpp"
#include "clarity/text.hpp"
#include "clarity/values.hpp"

using namespace clarity;

namespace {

/// The JSON a generated record writes for a field of `kind` holding `value`.
nlohmann::json written(const std::string& kind, const nlohmann::json& value) {
    std::string text = value.is_string() ? value.get<std::string>() : "";
    if (kind == "decimal" || kind == "money") return Decimal::parse(text);
    if (kind == "integer") return std::stoll(text);
    // None is null, as generated records write an empty date, date and time or reference.
    if (value.is_null()) return nullptr;
    if (kind == "date") return *Date::parse(text);
    if (kind == "datetime") return *DateTime::parse(text);
    if (kind == "ref") return std::stoll(text);
    if (kind == "bool") return value.get<bool>();
    if (kind == "text") return text;
    if (kind == "list") return std::vector<int64_t>();
    return nullptr;
}

}  // namespace

TEST(every_value_is_written_as_every_stack_writes_it) {
    for (const auto& c : conformance("values-json.json")["cases"]) {
        std::string name = c["name"];
        CHECK_EQ(name + ": " + written(c["kind"], c["value"]).dump(), name + ": " + c["json"].get<std::string>());
    }
}
