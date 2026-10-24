// Reading JSON into records (DESIGN 5.23) against the cases every runtime
// shares (conformance/json.json): what is read is fitted to the entity.

#include "check.hpp"
#include "clarity/json_read.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

JsonKind kind_of(const json& field, const std::string& word) {
    JsonKind kind;
    static const std::vector<std::pair<std::string, JsonKind::Is>> words = {
        {"text", JsonKind::Text}, {"integer", JsonKind::Integer}, {"decimal", JsonKind::Decimal}, {"bool", JsonKind::Bool}, {"date", JsonKind::Date},
        {"datetime", JsonKind::DateTime}, {"enum", JsonKind::Enum}, {"values", JsonKind::Values}, {"other", JsonKind::Other}};
    for (const auto& [w, is] : words) {
        if (w == word) kind.is = is;
    }
    if (field.contains("values")) kind.values = field["values"].get<std::vector<std::string>>();
    if (kind.is == JsonKind::Values) kind.item = std::make_shared<JsonKind>(kind_of(field, field["item"].get<std::string>()));
    return kind;
}

}  // namespace

TEST(json_is_fitted_as_every_runtime_fits_it) {
    json cases = conformance("json.json");
    for (const auto& c : cases["cases"]) {
        std::vector<JsonField> fields;
        for (const auto& f : c["fields"]) fields.push_back({f["name"].get<std::string>(), kind_of(f, f["kind"].get<std::string>())});
        std::string at = c.contains("at") ? c["at"].get<std::string>() : "";
        bool many = c.contains("many") && c["many"].get<bool>();
        try {
            json fitted = json_fit(c["json"].get<std::string>(), at, many, c["entity"].get<std::string>(), fields, c["base"]);
            CHECK(!c.contains("error"));
            if (c.contains("fitted")) CHECK_EQ(fitted.dump(), c["fitted"].dump());
        } catch (const ValueError& e) {
            CHECK(c.contains("error"));
            if (c.contains("error")) CHECK_EQ(std::string(e.what()), c["error"].get<std::string>());
        }
    }
}
