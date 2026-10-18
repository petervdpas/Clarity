// The generated migration steps (conformance/migrations.json) applied by the
// C++ runtime, with records stored between the steps: the same scenario as
// the other runtimes' server tests, on SQLite.

#include <algorithm>
#include <filesystem>

#include "check.hpp"
#include "clarity/db.hpp"
#include "clarity/sql.hpp"
#include "conformance.hpp"

using namespace clarity;

namespace {

/// A customer as the first step stores it.
struct Before {
    int64_t id = 0;
    std::string name, email, kind;
    Decimal limit;
    bool active = false;
    std::optional<Date> since;
    std::string notes;
    static constexpr const char* table = "customer";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"name", "email", "kind", "limit", "active", "since", "notes"};
        return names;
    }
    std::vector<SqlValue> values() const {
        return {sql::text(name), sql::text(email), sql::text(kind), sql::decimal(limit), sql::boolean(active), sql::date(since), sql::text(notes)};
    }
    static Before from_values(int64_t id, const Row& v) {
        return {id, sql::get_text("name", v[0]), sql::get_text("email", v[1]), sql::get_text("kind", v[2]), sql::get_decimal("limit", v[3]),
                sql::get_boolean("active", v[4]), sql::get_date("since", v[5]), sql::get_text("notes", v[6])};
    }
};

/// An order as the first step stores it.
struct Order {
    int64_t id = 0;
    std::string number;
    std::optional<int64_t> customer;
    int64_t total = 0;
    std::optional<DateTime> placed;
    bool paid = false;
    std::string code;
    static constexpr const char* table = "order";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"number", "customer", "total", "placed", "paid", "code"};
        return names;
    }
    std::vector<SqlValue> values() const {
        return {sql::text(number), sql::reference(customer), sql::integer(total), sql::date_time(placed), sql::boolean(paid), sql::text(code)};
    }
    static Order from_values(int64_t id, const Row& v) {
        return {id, sql::get_text("number", v[0]), sql::get_reference("customer", v[1]), sql::get_integer("total", v[2]),
                sql::get_date_time("placed", v[3]), sql::get_boolean("paid", v[4]), sql::get_text("code", v[5])};
    }
};

/// A customer after the second step.
struct Customer {
    int64_t id = 0;
    std::string name, email, kind;
    Decimal credit_limit;
    bool vip = false;
    std::string remarks, phone;
    std::optional<Picture> portrait;
    static constexpr const char* table = "customer";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"name", "email", "kind", "credit_limit", "vip", "remarks", "phone", "portrait"};
        return names;
    }
    static std::vector<PictureColumn> pictures() { return {{"portrait", std::nullopt}}; }
    std::vector<SqlValue> values() const {
        return {sql::text(name), sql::text(email), sql::text(kind), sql::decimal(credit_limit), sql::boolean(vip), sql::text(remarks), sql::text(phone), sql::picture(portrait)};
    }
    static Customer from_values(int64_t id, const Row& v) {
        return {id, sql::get_text("name", v[0]), sql::get_text("email", v[1]), sql::get_text("kind", v[2]), sql::get_decimal("credit_limit", v[3]),
                sql::get_boolean("vip", v[4]), sql::get_text("remarks", v[5]), sql::get_text("phone", v[6]), sql::get_picture("portrait", v[7])};
    }
};

/// An order after the second step: another table, and a money total.
struct Purchase {
    int64_t id = 0;
    std::string number;
    std::optional<int64_t> customer;
    Decimal total;
    std::optional<DateTime> placed;
    int64_t paid = 0;
    int64_t code = 0;
    static constexpr const char* table = "purchase";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"number", "customer", "total", "placed", "paid", "code"};
        return names;
    }
    std::vector<SqlValue> values() const {
        return {sql::text(number), sql::reference(customer), sql::decimal(total), sql::date_time(placed), sql::integer(paid), sql::integer(code)};
    }
    static Purchase from_values(int64_t id, const Row& v) {
        return {id, sql::get_text("number", v[0]), sql::get_reference("customer", v[1]), sql::get_decimal("total", v[2]),
                sql::get_date_time("placed", v[3]), sql::get_integer("paid", v[4]), sql::get_integer("code", v[5])};
    }
};

template <typename R, typename F>
std::vector<std::string> names(const std::vector<R>& found, F name) {
    std::vector<std::string> out;
    for (const auto& r : found) out.push_back(name(r));
    return out;
}

using Names = std::vector<std::string>;

}  // namespace

TEST(the_server_scenario_on_sqlite) {
    auto file = std::filesystem::temp_directory_path() / "clarity-scenario-cpp.db";
    std::filesystem::remove(file);
    nlohmann::json cases = conformance("migrations.json");
    std::vector<Step> all = cases["steps"]["sqlite"].get<std::vector<Step>>();
    {
        Db db({{"main", Dialect::Sqlite, file.string()}});
        CHECK_EQ(db.migrate({all[0]}), 1);
        auto since = Date::parse("2026-01-15");
        Before ada{0, "Ada Lovelace", "ada@example.com", "Business", Decimal::parse("1250.50"), true, since, "first's"};
        db.save(ada);
        Before grace{0, "Grace Hopper", "", "Private", Decimal::parse("99.5"), false, std::nullopt, ""};
        db.save(grace);
        Before got = db.get<Before>(ada.id);
        CHECK(got.name == ada.name && got.limit == ada.limit && got.since == since && got.notes == ada.notes && got.active);
        // Saving what is stored already is no missing record.
        db.save(grace);

        auto placed = DateTime::parse("2026-10-17T09:30:15");
        Order order{0, "ORD-1", ada.id, 99, placed, true, "42"};
        db.save(order);
        Order blank{0, "ORD-0", std::nullopt, 0, std::nullopt, false, ""};
        db.save(blank);
        db.save(order);
        // Through a reference: the customer's name, in a condition and an order.
        auto by_number = [](const Order& o) { return o.number; };
        CHECK(names(db.find<Order>(Query().filter(Cond::through("customer", "customer", "id", Cond::contains("name", "LOVE")))), by_number) == Names{"ORD-1"});
        Order grace_order{0, "ORD-G", grace.id, 0, std::nullopt, false, ""};
        db.save(grace_order);
        auto by_name = Query().filter(!Cond::is_none("customer")).order_through({{"customer", "customer", "id"}}, "name", true, false);
        CHECK(names(db.find<Order>(by_name), by_number) == (Names{"ORD-G", "ORD-1"}));
        db.remove(grace_order);
        try {
            db.remove(ada);
            CHECK(false);
        } catch (const InUseError& e) {
            CHECK_EQ(e.table, "customer");
            CHECK_EQ(e.id, ada.id);
        }

        // Queries: contains ignores case and wildcards, decimals compare as numbers.
        auto name_of = [](const Before& c) { return c.name; };
        CHECK(names(db.find<Before>(Query().filter(Cond::contains("name", "LOVE"))), name_of) == Names{"Ada Lovelace"});
        CHECK(db.find<Before>(Query().filter(Cond::contains("name", "%"))).empty());
        CHECK(names(db.find<Before>(Query().filter(Cond::number("limit", Op::Gt, int64_t(900)) && Cond::is_true("active"))), name_of) == Names{"Ada Lovelace"});
        CHECK(names(db.find<Before>(Query().order_number("limit")), name_of) == (Names{"Grace Hopper", "Ada Lovelace"}));
        CHECK_EQ(db.find<Before>(Query().filter(Cond::compare("since", Op::Eq, *since))).size(), size_t(1));
        CHECK(names(db.find<Before>(Query().filter(Cond::one_of("kind", {std::string("Private")}))), name_of) == Names{"Grace Hopper"});
        CHECK(names(db.find<Before>(Query().filter(Cond::is_none("since"))), name_of) == Names{"Grace Hopper"});

        CHECK_EQ(db.migrate(all), 1);
        CHECK_EQ(db.migrate(all), 0);
        CHECK_EQ(db.schema_version(), 2);

        Customer c = db.get<Customer>(ada.id);
        CHECK(c.credit_limit == Decimal::parse("1250.50") && c.vip && c.remarks == "first's" && c.phone.empty());
        Purchase p = db.get<Purchase>(order.id);
        CHECK(p.total == Decimal(99) && p.customer == ada.id && p.placed == placed && p.paid == 1 && p.code == 42);
        // Empty text is no number: it becomes the default.
        CHECK_EQ(db.get<Purchase>(blank.id).code, int64_t(0));

        // A picture in a binary column: saved, kept by a save that leaves it, emptied.
        Bytes png{0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
        Customer pictured = db.get<Customer>(ada.id);
        CHECK(!pictured.portrait);
        pictured.portrait = Picture::of(png);
        db.save(pictured);
        pictured = db.get<Customer>(ada.id);
        CHECK(pictured.portrait == Picture::stored());
        pictured.phone = "555";
        db.save(pictured);
        CHECK(db.picture<Customer>(ada.id, "portrait") == png);
        pictured.portrait.reset();
        pictured.phone = "";
        db.save(pictured);
        CHECK(!db.picture<Customer>(ada.id, "portrait"));

        // Unique email: empty many times, a filled one once.
        Customer linus{0, "Linus", "", "", Decimal(), false, "", "", std::nullopt};
        db.save(linus);
        Customer copy{0, "Copy", "ada@example.com", "", Decimal(), false, "", "", std::nullopt};
        CHECK_THROWS(Error, db.save(copy));
        // The reference is kept through the renamed table.
        Customer ada_now = db.get<Customer>(ada.id);
        CHECK_THROWS(InUseError, db.remove(ada_now));
        Purchase orphan{0, "ORD-2", 999, Decimal(), std::nullopt, 0, 0};
        CHECK_THROWS(InvalidError, db.save(orphan));
        db.remove(p);
        db.remove(ada_now);
        CHECK_EQ(db.all<Customer>().size(), size_t(2));
    }
    std::filesystem::remove(file);
}
