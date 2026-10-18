// The SQL of queries per database, against the cases every stack's runtime
// shares (conformance/queries.json).

#include "check.hpp"
#include "clarity/query.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

Op op(const json& value) {
    std::string name = value.get<std::string>();
    if (name == "Eq") return Op::Eq;
    if (name == "NotEq") return Op::NotEq;
    if (name == "Lt") return Op::Lt;
    if (name == "LtEq") return Op::LtEq;
    if (name == "Gt") return Op::Gt;
    return Op::GtEq;
}

SqlValue sql_value(const json& value) {
    auto [kind, inner] = *value.items().begin();
    if (kind == "text") return inner.get<std::string>();
    if (kind == "integer") return inner.get<int64_t>();
    if (kind == "decimal") return Decimal::parse(inner.get<std::string>());
    if (kind == "bool") return inner.get<bool>();
    if (kind == "date") return *Date::parse(inner.get<std::string>());
    return null();
}

/// A parameter as the cases write it.
json case_value(const SqlValue& value) {
    return std::visit(
        [](const auto& v) -> json {
            using T = std::decay_t<decltype(v)>;
            if constexpr (std::is_same_v<T, std::string>) return {{"text", v}};
            else if constexpr (std::is_same_v<T, int64_t>) return {{"integer", v}};
            else if constexpr (std::is_same_v<T, Decimal>) return {{"decimal", v.str()}};
            else if constexpr (std::is_same_v<T, bool>) return {{"bool", v}};
            else if constexpr (std::is_same_v<T, Date>) return {{"date", v.str()}};
            else return {{"null", nullptr}};
        },
        value);
}

std::vector<SqlValue> values(const json& list) {
    std::vector<SqlValue> out;
    for (const auto& v : list) out.push_back(sql_value(v));
    return out;
}

Cond cond(const json& value) {
    auto [kind, inner] = *value.items().begin();
    if (kind == "compare") return Cond::compare(inner["column"], op(inner["op"]), sql_value(inner["value"]));
    if (kind == "number") return Cond::number(inner["column"], op(inner["op"]), sql_value(inner["value"]));
    if (kind == "contains") return Cond::contains(inner["column"], inner["text"]);
    if (kind == "oneOf") return Cond::one_of(inner["column"], values(inner["values"]));
    if (kind == "oneOfNumbers") return Cond::one_of_numbers(inner["column"], values(inner["values"]));
    if (kind == "isTrue") return Cond::is_true(inner.get<std::string>());
    if (kind == "isNone") return Cond::is_none(inner.get<std::string>());
    if (kind == "and") return cond(inner[0]) && cond(inner[1]);
    if (kind == "or") return cond(inner[0]) || cond(inner[1]);
    if (kind == "not") return !cond(inner);
    return Cond::through(inner["column"], inner["table"], inner["key"], cond(inner["filter"]));
}

Query query(const json& value) {
    Query q;
    if (value.contains("filter")) q = q.filter(cond(value["filter"]));
    for (const auto& key : value.value("order", json::array())) {
        std::vector<Hop> hops;
        for (const auto& h : key.value("through", json::array())) hops.push_back({h["column"], h["table"], h["key"]});
        q = q.order_through(hops, key["column"], key["descending"], key["numeric"]);
    }
    return q;
}

}  // namespace

TEST(every_query_case_gives_its_sql_on_every_database) {
    json cases = conformance("queries.json")["cases"];
    REQUIRE(!cases.empty());
    std::pair<const char*, Dialect> dialects[] = {{"sqlite", Dialect::Sqlite}, {"postgres", Dialect::Postgres}, {"mysql", Dialect::Mysql}, {"mssql", Dialect::Mssql}};
    for (const auto& c : cases) {
        Query q = query(c["query"]);
        for (auto [word, d] : dialects) {
            std::string columns = quote(d, c["key"]);
            for (const auto& column : c["columns"]) columns += ", " + quote(d, column);
            auto [sql, params] = q.sql(d, c["table"], c["key"], columns);
            const json& expect = c["expect"][word];
            CHECK_EQ(sql, expect["sql"].get<std::string>());
            json got = json::array();
            for (const auto& p : params) got.push_back(case_value(p));
            CHECK_EQ(got, expect["params"]);
        }
    }
}
