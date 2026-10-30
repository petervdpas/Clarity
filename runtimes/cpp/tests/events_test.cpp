// Data events: a record whose entity has them runs them around its save and
// delete, in one transaction with them, as on the other stacks.

#include "check.hpp"
#include "clarity/db.hpp"
#include "clarity/sql.hpp"

using namespace clarity;

namespace {

struct LogLine {
    int64_t id = 0;
    std::string text;
    static constexpr const char* table = "log";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"text"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::text(text)}; }
    static LogLine from_values(int64_t id, const Row& v) { return {id, sql::get_text("text", v[0])}; }
};

struct Account {
    int64_t id = 0;
    std::string name;
    int64_t balance = 0;
    static constexpr const char* table = "account";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"name", "balance"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::text(name), sql::integer(balance)}; }
    static Account from_values(int64_t id, const Row& v) { return {id, sql::get_text("name", v[0]), sql::get_integer("balance", v[1])}; }

    void before_save(Db&) {
        auto start = name.find_first_not_of(' ');
        if (start == std::string::npos) refuse(Text::plain("An account needs a name"));
        name = name.substr(start, name.find_last_not_of(' ') - start + 1);
    }
    void after_save(Db& db) {
        LogLine line{0, "saved " + name + " with " + std::to_string(balance)};
        db.save(line);
        if (balance < 0) refuse(Text::plain("An account cannot be in debt"));
    }
    void before_delete(Db&) {
        if (balance != 0) refuse(Text("{name} still holds {balance}").with("name", slot_text(name)).with("balance", slot_integer(balance)));
    }
    void after_delete(Db& db) {
        LogLine line{0, "deleted " + name};
        db.save(line);
    }
};

Db accounts() {
    Db db = Db::memory();
    db.migrate({{"CREATE TABLE account (id INTEGER PRIMARY KEY, name TEXT NOT NULL, balance INTEGER NOT NULL)",
                 "CREATE TABLE log (id INTEGER PRIMARY KEY, text TEXT NOT NULL)"}});
    return db;
}

std::vector<std::string> logged(Db& db) {
    std::vector<std::string> out;
    for (const auto& l : db.all<LogLine>()) out.push_back(l.text);
    return out;
}

std::string refusal(auto run) {
    try {
        run();
    } catch (const RefusedError& e) {
        return e.text.show();
    }
    return "(not refused)";
}

// Cascading deletes: a record whose records refer to it with `cascade`
// deletes them first, each with its own cascades and data events, all in the
// transaction of its delete (runtimes/rust/tests/cascade.rs).
std::vector<std::string> deleted_tasks;

struct PlanTask {
    int64_t id = 0;
    std::optional<int64_t> plan;
    std::string title;
    static constexpr const char* table = "plan_task";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"plan", "title"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::reference(plan), sql::text(title)}; }
    static PlanTask from_values(int64_t id, const Row& v) { return {id, sql::get_reference("plan", v[0]), sql::get_text("title", v[1])}; }
    void before_delete(Db&) {
        if (title == "locked") refuse(Text::plain("This task is locked"));
        deleted_tasks.push_back(title);
    }
};

struct Plan {
    int64_t id = 0;
    std::string name;
    static constexpr const char* table = "plan";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"name"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::text(name)}; }
    static Plan from_values(int64_t id, const Row& v) { return {id, sql::get_text("name", v[0])}; }
    // As generated code writes it: the tasks of the plan, each deleted as any record is.
    void cascade(Db& db) const {
        for (const auto& task : db.find<PlanTask>(Query().filter(Cond::compare("plan", Op::Eq, sql::integer(id))))) db.remove(task);
    }
};

std::pair<Db, Plan> planned() {
    Db db = Db::memory();
    db.migrate({{"CREATE TABLE plan (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
                 "CREATE TABLE plan_task (id INTEGER PRIMARY KEY, plan INTEGER REFERENCES plan(id), title TEXT NOT NULL)"}});
    Plan plan{0, "Model"};
    db.save(plan);
    for (std::string title : {"Draw", "Write"}) {
        PlanTask task{0, plan.id, title};
        db.save(task);
    }
    return {std::move(db), plan};
}

}  // namespace

TEST(events_run_around_a_save) {
    Db db = accounts();
    Account ada{0, "  Ada ", 10};
    db.save(ada);
    // Before save changed the record that was stored.
    CHECK_EQ(ada.name, "Ada");
    CHECK_EQ(db.get<Account>(ada.id).name, "Ada");
    CHECK(logged(db) == std::vector<std::string>{"saved Ada with 10"});
    Account nameless;
    CHECK_EQ(refusal([&] { db.save(nameless); }), "An account needs a name");
    CHECK_EQ(db.all<Account>().size(), size_t(1));
}

TEST(a_failing_event_keeps_nothing) {
    Db db = accounts();
    Account debtor{0, "Grace", -5};
    CHECK_EQ(refusal([&] { db.save(debtor); }), "An account cannot be in debt");
    // The record, and what after save wrote, are gone; the record is new again.
    CHECK_EQ(debtor.id, int64_t(0));
    CHECK(db.all<Account>().empty());
    CHECK(logged(db).empty());
    // The database is usable afterwards.
    debtor.balance = 0;
    db.save(debtor);
    CHECK(logged(db) == std::vector<std::string>{"saved Grace with 0"});
}

TEST(events_run_around_a_delete) {
    Db db = accounts();
    Account ada{0, "Ada", 10};
    db.save(ada);
    CHECK_EQ(refusal([&] { db.remove(ada); }), "Ada still holds 10");
    ada.balance = 0;
    db.save(ada);
    db.remove(ada);
    CHECK(db.all<Account>().empty());
    CHECK(logged(db) == (std::vector<std::string>{"saved Ada with 10", "saved Ada with 0", "deleted Ada"}));
}

TEST(deleting_a_record_deletes_what_cascades_from_it_first) {
    auto [db, plan] = planned();
    deleted_tasks.clear();
    db.remove(plan);
    CHECK(db.all<Plan>().empty());
    CHECK(db.all<PlanTask>().empty());
    CHECK((deleted_tasks == std::vector<std::string>{"Draw", "Write"}));
}

TEST(a_refused_child_keeps_all_of_it) {
    auto [db, plan] = planned();
    PlanTask locked{0, plan.id, "locked"};
    db.save(locked);
    CHECK_EQ(refusal([&] { db.remove(plan); }), std::string("This task is locked"));
    CHECK_EQ(db.all<Plan>().size(), size_t(1));
    CHECK_EQ(db.all<PlanTask>().size(), size_t(3));
}
