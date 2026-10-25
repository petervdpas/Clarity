// Reading JSON into records (DESIGN 5.23) against the cases every runtime
// shares (conformance/json.json): what is read is fitted to the entity.

#include "check.hpp"
#include "clarity/json_read.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

std::vector<JsonField> fields_of(const json& listed);

JsonKind kind_of(const json& field, const std::string& word) {
    JsonKind kind;
    static const std::vector<std::pair<std::string, JsonKind::Is>> words = {
        {"text", JsonKind::Text}, {"integer", JsonKind::Integer}, {"decimal", JsonKind::Decimal}, {"bool", JsonKind::Bool}, {"date", JsonKind::Date},
        {"datetime", JsonKind::DateTime}, {"enum", JsonKind::Enum}, {"values", JsonKind::Values}, {"other", JsonKind::Other},
        {"record", JsonKind::Record}, {"records", JsonKind::Records}};
    for (const auto& [w, is] : words) {
        if (w == word) kind.is = is;
    }
    if (field.contains("values")) kind.values = field["values"].get<std::vector<std::string>>();
    if (kind.is == JsonKind::Values) kind.item = std::make_shared<JsonKind>(kind_of(field, field["item"].get<std::string>()));
    if (kind.is == JsonKind::Record || kind.is == JsonKind::Records) {
        kind.entity = field["entity"].get<std::string>();
        kind.fields = std::make_shared<std::vector<JsonField>>(fields_of(field["fields"]));
        kind.base = field["base"];
    }
    return kind;
}

std::vector<JsonField> fields_of(const json& listed) {
    std::vector<JsonField> fields;
    for (const auto& f : listed) fields.push_back({f["name"].get<std::string>(), kind_of(f, f["kind"].get<std::string>())});
    return fields;
}

}  // namespace

TEST(json_is_fitted_as_every_runtime_fits_it) {
    json cases = conformance("json.json");
    for (const auto& c : cases["cases"]) {
        std::vector<JsonField> fields = fields_of(c["fields"]);
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
