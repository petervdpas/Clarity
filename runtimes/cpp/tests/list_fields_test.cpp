// Lists of values as fields (U1c), kept in one column as JSON, against the
// cases every runtime shares (conformance/list-fields.json).

#include "check.hpp"
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
