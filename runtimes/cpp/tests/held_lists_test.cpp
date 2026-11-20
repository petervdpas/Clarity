// A stored record's list (order.lines) loads with the record, one level
// deep: find, all and get call load_lists, which reads the lines with
// find_alone (runtimes/rust/tests/held_lists.rs, docs/ledger/runtime-data.md).

#include "check.hpp"
#include "clarity/db.hpp"
#include "clarity/sql.hpp"

using namespace clarity;

namespace {

struct HeldLine {
    int64_t id = 0;
    std::optional<int64_t> order;
    std::string product;
    static constexpr const char* table = "held_line";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"held_order", "product"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::reference(order), sql::text(product)}; }
    static HeldLine from_values(int64_t id, const Row& v) { return {id, sql::get_reference("held_order", v[0]), sql::get_text("product", v[1])}; }
};

struct HeldOrder {
    int64_t id = 0;
    std::string number;
    std::vector<HeldLine> lines;
    static constexpr const char* table = "held_order";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"number"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::text(number)}; }
    static HeldOrder from_values(int64_t id, const Row& v) { return {id, sql::get_text("number", v[0]), {}}; }
    // As generated code writes it.
    void load_lists(Db& db) { lines = db.find_alone<HeldLine>(Query().filter(Cond::compare("held_order", Op::Eq, sql::integer(id))).order("id", false)); }
};

}  // namespace

TEST(a_stored_records_lists_load_with_it) {
    Db db = Db::memory();
    db.migrate({{"CREATE TABLE held_order (id INTEGER PRIMARY KEY, number TEXT NOT NULL)",
                 "CREATE TABLE held_line (id INTEGER PRIMARY KEY, held_order INTEGER, product TEXT NOT NULL)"}});
    HeldOrder first{0, "ORD-1", {}};
    HeldOrder second{0, "ORD-2", {}};
    db.save(first);
    db.save(second);
    for (const char* product : {"pen", "ink"}) {
        HeldLine line{0, first.id, product};
        db.save(line);
    }
    auto found = db.find<HeldOrder>(Query().order("number"));
    REQUIRE(found.size() == 2);
    REQUIRE(found[0].lines.size() == 2);
    CHECK_EQ(found[0].lines[0].product, std::string("pen"));
    CHECK_EQ(found[0].lines[1].product, std::string("ink"));
    CHECK(found[1].lines.empty());
    CHECK_EQ(db.all<HeldOrder>()[0].lines.size(), size_t(2));
    CHECK_EQ(db.get<HeldOrder>(first.id).lines.size(), size_t(2));
    CHECK(db.find_alone<HeldOrder>(Query())[0].lines.empty());
}
