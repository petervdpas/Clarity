// Required fields: a save refuses a record whose required field is empty,
// as every runtime does (conformance/required.json).

#include "check.hpp"
#include "clarity/db.hpp"
#include "clarity/sql.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

namespace {

/// The slot `field` of the refusal of run, or what else happened.
std::string refused_field(auto run) {
    try {
        run();
    } catch (const RefusedError& e) {
        if (e.text.key() != std::optional<std::string>("fieldRequired")) return "(another refusal)";
        auto found = e.text.values().find("field");
        return found == e.text.values().end() ? "(no field slot)" : found->second.plain();
    }
    return "(not refused)";
}

struct Member {
    int64_t id = 0;
    std::string name;
    int64_t group = 0;
    static constexpr const char* table = "member";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"name", "group_id"};
        return names;
    }
    static std::vector<Required> required() { return {{"name", "Name"}, {"group_id", "@groupLabel"}}; }
    std::vector<SqlValue> values() const { return {sql::text(name), group ? SqlValue(group) : null()}; }
    static Member from_values(int64_t id, const Row& v) { return {id, sql::get_text("name", v[0]), 0}; }
};

/// A member whose before save fills an empty name.
struct Guest : Member {
    static Guest from_values(int64_t id, const Row& v) { return {Member::from_values(id, v)}; }
    void before_save(Db&) {
        if (name.empty()) name = "Guest";
    }
};

}  // namespace

TEST(required_fields_are_checked_as_every_runtime_does) {
    const auto& file = conformance("required.json");
    CHECK_EQ(file["refusal"]["key"].get<std::string>(), std::string("fieldRequired"));
    for (const auto& c : file["cases"]) {
        std::vector<Required> required;
        for (const auto& r : c["required"]) required.push_back({r["column"], r["label"]});
        std::vector<std::string> columns;
        std::vector<SqlValue> values;
        for (const auto& [column, v] : c["values"].items()) {
            columns.push_back(column);
            if (v.is_string()) values.push_back(v.get<std::string>());
            else if (v.is_boolean()) values.push_back(v.get<bool>());
            else if (v.is_number()) values.push_back(v.get<int64_t>());
            else values.push_back(null());
        }
        std::string want = c["refused"].is_string() ? c["refused"].get<std::string>() : "(not refused)";
        std::string got = refused_field([&] { check_required(required, columns, values); });
        if (got != want) check::fail(__FILE__, __LINE__, c["name"].get<std::string>() + ": " + got + ", want " + want);
    }
}

TEST(a_save_refuses_a_record_with_an_empty_required_field) {
    Db db = Db::memory();
    db.migrate({{"CREATE TABLE member (id INTEGER PRIMARY KEY, name TEXT NOT NULL, group_id INTEGER)"}});
    load_translations(R"({"project":{"translations":{"en":{"groupLabel":"Group"}}}})");
    Member empty{0, "", 1};
    CHECK_EQ(refused_field([&] { db.save(empty); }), std::string("Name"));
    // A label written as a key takes the key's value.
    Member no_group{0, "Ada", 0};
    CHECK_EQ(refused_field([&] { db.save(no_group); }), std::string("Group"));
    // Before save may fill a required field; a refused one keeps a new record new.
    Guest filled{{0, "", 1}};
    db.save(filled);
    CHECK(filled.id != 0);
    Guest unfilled{{0, "", 0}};
    CHECK_EQ(refused_field([&] { db.save(unfilled); }), std::string("Group"));
    CHECK_EQ(unfilled.id, int64_t(0));
    auto stored = db.all<Member>();
    CHECK_EQ(stored.size(), size_t(1));
    if (!stored.empty()) CHECK_EQ(stored[0].name, std::string("Guest"));
    load_translations("{}");
}
