// JSON in code (DESIGN 5.23): `json of value` and `read Order from json
// text`. A service's JSON is not Clarity's own, so what is read is fitted to
// the entity first: names match ignoring case, underscores and hyphens,
// numbers and strings of numbers stand for each other, what is left out
// keeps its default. Every runtime fits alike (conformance/json.json); the
// record's own from_json then reads what was fitted.

#pragma once

#include <memory>
#include <optional>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "clarity/code.hpp"

namespace clarity {

/// What a field of an entity takes from JSON.
struct JsonKind {
    enum Is { Text, Integer, Decimal, Bool, Date, DateTime, Enum, Values, Other } is = Text;
    /// The values of an enum, matched in any case.
    std::vector<std::string> values = {};
    /// What the items of a list of values are.
    std::shared_ptr<const JsonKind> item = nullptr;
};

/// A field of an entity, as JSON is fitted to it.
struct JsonField {
    std::string name;
    JsonKind kind;
};

/// The JSON of a value, as Clarity writes values everywhere.
template <typename T>
std::string json_of(const T& value) {
    return nlohmann::json(value).dump();
}

/// What json_read decodes: `text` as one record of `entity` (or, with
/// `many`, a list of them), starting at `at` (object names with dots
/// between), fitted to `fields` over `base`, a new record. Throws a
/// ValueError that says why it does not fit.
nlohmann::json json_fit(const std::string& text, const std::string& at, bool many, const std::string& entity, const std::vector<JsonField>& fields, const nlohmann::json& base);

/// Reads `text` as a `T` (a record, or a vector of them with `many`) of `entity`.
template <typename T, typename Record>
T json_read(const std::string& text, const std::string& at, bool many, const std::string& entity, const std::vector<JsonField>& fields, const Record& base) {
    nlohmann::json fitted = json_fit(text, at, many, entity, fields, nlohmann::json(base));
    try {
        return fitted.get<T>();
    } catch (const nlohmann::json::exception& e) {
        throw ValueError("the JSON does not fit " + entity + ": " + e.what());
    }
}

}  // namespace clarity
