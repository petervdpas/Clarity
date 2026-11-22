// A stored record's list (order.lines) loads with the record, one level
// deep: find and all call load_lists_of for every record found, which reads
// each list once for all of them (find_by_owners); get loads one record's
// the same way (runtimes/rust/tests/held_lists.rs, docs/ledger/runtime-data.md).

#include "apply_steps.hpp"
#include "check.hpp"
#include "conformance.hpp"
#include "clarity/db.hpp"
#include "clarity/sql.hpp"

using namespace clarity;

namespace {

/// How many times the lines were read: once per list for a whole find.
size_t line_reads = 0;

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
    // As generated code writes them.
    void load_lists(Db& db) {
        std::vector<HeldOrder> all{*this};
        load_lists_of(db, all);
        *this = std::move(all[0]);
    }
    static void load_lists_of(Db& db, std::vector<HeldOrder>& records) {
        std::vector<int64_t> owners;
        for (const auto& record : records) owners.push_back(record.id);
        line_reads += (owners.size() + owners_per_read - 1) / owners_per_read;
        auto found = by_owner(db.find_by_owners<HeldLine>("held_order", owners), [](const HeldLine& line) { return line.order; });
        for (auto& record : records) record.lines = std::move(found[record.id]);
    }
};

}  // namespace

TEST(a_stored_records_lists_load_with_it) {
    Db db = Db::memory();
    apply_steps(db, {{"CREATE TABLE held_order (id INTEGER PRIMARY KEY, number TEXT NOT NULL)",
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

// Every case of conformance/held-lists.json: a find reads each list once per
// ownersPerRead orders and gives each order its own lines in order.
TEST(a_find_reads_each_list_once_for_all_the_records_it_finds) {
    const auto& file = conformance("held-lists.json");
    CHECK_EQ(file["ownersPerRead"].get<size_t>(), owners_per_read);
    for (const auto& c : file["cases"]) {
        std::string name = c["name"];
        Db db = Db::memory();
        apply_steps(db, {{"CREATE TABLE held_order (id INTEGER PRIMARY KEY, number TEXT NOT NULL)",
                     "CREATE TABLE held_line (id INTEGER PRIMARY KEY, held_order INTEGER, product TEXT NOT NULL)"}});
        for (int n = 1; n <= c["orders"].get<int>(); ++n) {
            char number[16];
            std::snprintf(number, sizeof number, "ORD-%05d", n);
            HeldOrder order{0, number, {}};
            db.save(order);
        }
        for (const auto& l : c["lines"]) {
            HeldLine line{0, l[0].get<int64_t>(), l[1].get<std::string>()};
            db.save(line);
        }
        line_reads = 0;
        auto found = db.find<HeldOrder>(Query().order("number"));
        CHECK_EQ(line_reads, c["reads"].get<size_t>());
        for (const auto& [id, want] : c["expect"].items()) {
            const HeldOrder* order = nullptr;
            for (const auto& o : found)
                if (std::to_string(o.id) == id) order = &o;
            REQUIRE(order != nullptr);
            std::string got = name + ": order " + id + ":", expected = got;
            for (const auto& l : order->lines) got += " " + l.product;
            for (const auto& p : want) expected += " " + p.get<std::string>();
            CHECK_EQ(got, expected);
            CHECK_EQ(db.get<HeldOrder>(order->id).lines.size(), want.size());
        }
    }
}
