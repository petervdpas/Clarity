// Lists of values as fields (U1c), kept in one column as JSON, against the
// cases every runtime shares (conformance/list-fields.json).

#include "check.hpp"
#include "clarity/list_fields.hpp"
#include "clarity/sql.hpp"
#include "clarity/text.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

template <typename T>
std::string kept(const json& items) {
    return std::get<std::string>(sql::list(items.get<std::vector<T>>()));
}

template <typename T>
json read(const json& column) {
    SqlValue value = column.is_null() ? SqlValue{} : SqlValue{column.get<std::string>()};
    return json(sql::get_list("tags", value).get<std::vector<T>>());
}

}  // namespace

TEST(a_list_is_kept_as_compact_json) {
    for (const auto& c : conformance("list-fields.json")["kept"]) {
        std::string item = c["item"];
        std::string got = item == "integer" ? kept<int64_t>(c["items"])
                          : item == "decimal" ? kept<Decimal>(c["items"])
                          : item == "date"    ? kept<std::optional<Date>>(c["items"])
                                              : kept<std::string>(c["items"]);
        CHECK(got == c["column"].get<std::string>());
    }
}

TEST(a_column_gives_its_items) {
    for (const auto& c : conformance("list-fields.json")["read"]) {
        std::string item = c["item"];
        json got = item == "integer" ? read<int64_t>(c["column"])
                   : item == "decimal" ? read<Decimal>(c["column"])
                   : item == "date"    ? read<std::optional<Date>>(c["column"])
                                       : read<std::string>(c["column"]);
        CHECK(got == c["items"]);
    }
}

TEST(a_typed_item_is_added_as_every_runtime_does) {
    for (const auto& c : conformance("list-fields.json")["added"]) {
        auto got = add_typed(c["item"], c["items"], c["typed"]);
        if (c.value("refused", false)) {
            if (!got.refused) check::fail(__FILE__, __LINE__, c.dump() + ": not refused");
            continue;
        }
        if (got.refused || got.items != c["gives"]) check::fail(__FILE__, __LINE__, c.dump() + ": " + got.items.dump());
    }
}

// A date is a date, an enum's item one of its values.
TEST(a_typed_date_or_enum_item_is_one_of_its_kind) {
    CHECK_EQ(add_typed("date", json::array(), " 2026-10-31 ").items, json::parse(R"(["2026-10-31"])"));
    CHECK(add_typed("date", json::array(), "2026-02-30").refused);
    json genres = json::parse(R"(["Jazz","Rock"])");
    CHECK_EQ(add_typed("enum", json::array(), "Rock", genres).items, json::parse(R"(["Rock"])"));
    CHECK(add_typed("enum", json::array(), "Polka", genres).refused);
    // 1.50 is the item 1.5 the list has.
    CHECK_EQ(add_typed("decimal", json::parse(R"(["1.5"])"), "1.50").items, json::parse(R"(["1.5"])"));
}
