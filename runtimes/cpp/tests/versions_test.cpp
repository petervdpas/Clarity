// A save never overwrites someone else's change (conformance/versions.json):
// the version read with every record, the checked save and delete, a save
// anyway, reload, undo by the version, and a file source changed on disk.
// The steps of one edit are undone all or none, a handler is one
// transaction, and only the last undo_steps are kept.

#include <chrono>
#include <filesystem>
#include <fstream>
#include <map>
#include <thread>

#include "check.hpp"
#include "clarity/db.hpp"
#include "clarity/sql.hpp"
#include "conformance.hpp"
#include "formats/file_source.hpp"

using namespace clarity;
namespace fs = std::filesystem;
using nlohmann::json;

namespace {

struct Item {
    int64_t id = 0;
    std::string name;
    int64_t version = 0;
    static constexpr const char* table = "item";
    static constexpr const char* version_column = "row_version";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"name"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::text(name)}; }
    static Item from_values(int64_t id, const Row& v) { return {id, sql::get_text("name", v[0]), 0}; }
};

struct Line {
    int64_t id = 0;
    std::string name;
    static constexpr const char* source = "lines";
    static constexpr const char* table = "line";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"name"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::text(name)}; }
    static Line from_values(int64_t id, const Row& v) { return {id, sql::get_text("name", v[0])}; }
};

/// Whether `run` does what the step expects: goes, is refused with `refusal`'s words, or finds nothing.
template <typename F>
void expect(const std::string& name, const json& step, const json& refusal, F run) {
    std::string expected = step.value("expect", "ok");
    std::string what = name + ": " + step.dump();
    try {
        run();
        if (expected != "ok") check::fail(__FILE__, __LINE__, what + " went, " + expected + " expected");
    } catch (const RefusedError& e) {
        if (expected != "refused") check::fail(__FILE__, __LINE__, what + " refused: " + e.what());
        CHECK_EQ(e.text.key().value_or(""), refusal["key"].get<std::string>());
        CHECK_EQ(e.text.text(), refusal["english"].get<std::string>());
    } catch (const NotFoundError& e) {
        if (expected != "notFound") check::fail(__FILE__, __LINE__, what + " not found: " + e.what());
    }
}

const std::string undo_table = "CREATE TABLE item (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, row_version INTEGER NOT NULL DEFAULT 1)";

}  // namespace

TEST(versions_are_checked_as_every_runtime_does) {
    const json& file = conformance("versions.json");
    for (const auto& c : file["cases"]) {
        std::string name = c["name"];
        Db db = Db::memory();
        db.batch("", file["table"].get<std::string>());
        std::map<std::string, Item> copies;
        int64_t last = 0;
        for (const auto& step : c["steps"]) {
            std::string doing = step["do"];
            std::string copy = step.value("copy", "");
            if (doing == "insert") {
                Item item{0, step["name"].get<std::string>(), 0};
                expect(name, step, file["refusal"], [&] { db.save(item); });
                copies[copy] = item;
            } else if (doing == "read") {
                copies[copy] = db.get<Item>(copies[step["of"].get<std::string>()].id);
            } else if (doing == "set") {
                copies[copy].name = step["name"].get<std::string>();
            } else if (doing == "save") {
                expect(name, step, file["refusal"], [&] { db.save(copies[copy]); });
            } else if (doing == "saveAnyway") {
                expect(name, step, file["refusal"], [&] { db.save_anyway(copies[copy]); });
            } else if (doing == "delete") {
                expect(name, step, file["refusal"], [&] { db.remove(copies[copy]); });
            } else if (doing == "reload") {
                expect(name, step, file["refusal"], [&] { db.reload(copies[copy]); });
            } else if (doing == "version") {
                CHECK_EQ(copies[copy].version, step["is"].get<int64_t>());
            } else if (doing == "stored") {
                auto rows = db.rows("", "SELECT name, row_version FROM item WHERE id = ?", {copies[copy].id});
                if (step.value("gone", false)) {
                    CHECK(rows.empty());
                } else {
                    REQUIRE(rows.size() == 1);
                    CHECK_EQ(std::get<std::string>(rows[0][0]), step["name"].get<std::string>());
                    CHECK_EQ(std::get<int64_t>(rows[0][1]), step["version"].get<int64_t>());
                }
            } else if (doing == "stepSave") {
                expect(name, step, file["refusal"], [&] { db.step([&] { db.save(copies[copy]); }); });
                last = db.last_step();
            } else if (doing == "undo") {
                expect(name, step, file["undoRefusal"], [&] { db.undo(last); });
            } else if (doing == "redo") {
                expect(name, step, file["undoRefusal"], [&] { db.redo(last); });
            } else {
                check::fail(__FILE__, __LINE__, name + ": no step " + doing);
            }
        }
    }
}

TEST(a_file_changed_on_disk_refuses_the_save) {
    const json& file = conformance("versions.json");
    const json& source_case = file["file"];
    fs::path folder = fs::temp_directory_path() / "clarity-versions-cpp";
    fs::create_directories(folder);
    fs::path path = folder / "lines.csv";
    auto write = [&](const std::string& text) { std::ofstream(path, std::ios::binary | std::ios::trunc) << text; };
    auto read = [&] {
        std::ifstream in(path, std::ios::binary);
        return std::string(std::istreambuf_iterator<char>(in), {});
    };
    write(source_case["text"].get<std::string>());
    Db db = Db::memory();
    db.add_file({"lines", source_case["format"], path.string(), "", "line", "id", source_case["fields"].dump(), false});
    Line first = db.all<Line>().at(0);
    for (const auto& step : source_case["steps"]) {
        std::string doing = step["do"];
        if (doing == "save") {
            first.name = step["name"].get<std::string>();
            expect("file", step, file["refusal"], [&] { db.save(first); });
        } else if (doing == "rewrite") {
            std::string bytes = read();
            // A later modification time, as a program writing the same bytes gives.
            std::this_thread::sleep_for(std::chrono::milliseconds(20));
            write(bytes);
        } else if (doing == "write") {
            write(step["text"].get<std::string>());
        } else if (doing == "contents") {
            CHECK_EQ(read(), step["text"].get<std::string>());
        } else {
            check::fail(__FILE__, __LINE__, "file: no step " + doing);
        }
    }
    fs::remove_all(folder);
}

TEST(a_handler_is_one_transaction_kept_at_its_end) {
    Db db = Db::memory();
    db.batch("", undo_table);
    Item a{0, "A", 0};
    db.save(a);
    // A failure after two saves rolls both back, with the database's own rollback.
    CHECK_THROWS(StorageError, db.step([&] {
        a.name = "changed";
        db.save(a);
        Item b{0, "B", 0};
        db.save(b);
        throw StorageError("no");
    }));
    auto rows = db.rows("", "SELECT name, row_version FROM item", {});
    REQUIRE(rows.size() == 1);
    CHECK_EQ(std::get<std::string>(rows[0][0]), std::string("A"));
    CHECK_EQ(std::get<int64_t>(rows[0][1]), int64_t{1});
}

TEST(the_steps_of_one_edit_are_undone_all_or_none) {
    Db db = Db::memory();
    db.batch("", undo_table);
    Item a{0, "A", 0}, b{0, "B", 0};
    db.save(a);
    db.save(b);
    db.step([&] {
        a.name = "A2";
        db.save(a);
    });
    int64_t first = db.last_step();
    db.step([&] {
        b.name = "B2";
        db.save(b);
    });
    int64_t second = db.last_step();
    // B changed since the edit, so neither step is taken back.
    Item other = db.get<Item>(b.id);
    db.save(other);
    try {
        db.replay_steps({second, first}, true);
        check::fail(__FILE__, __LINE__, "not refused");
    } catch (const RefusedError& e) {
        CHECK_EQ(e.text.key().value_or(""), std::string("undoChangedSince"));
    }
    CHECK_EQ(db.get<Item>(a.id).name, std::string("A2"));
}

TEST(only_the_last_steps_are_kept) {
    Db db = Db::memory();
    db.batch("", undo_table);
    Item a{0, "A", 0};
    db.save(a);
    for (size_t i = 0; i <= undo_steps; ++i) {
        db.step([&] {
            a.name = "A" + std::to_string(i);
            db.save(a);
        });
    }
    try {
        db.undo(1);
        check::fail(__FILE__, __LINE__, "not refused");
    } catch (const RefusedError& e) {
        CHECK_EQ(e.text.key().value_or(""), std::string("undoTooOld"));
    }
    CHECK(db.undo(db.last_step()));
}

namespace {

/// An item whose entity words the refusal itself, about the record as stored
/// now and what the database holds (as `{current.changedBy.name}` needs).
struct Worded : Item {
    static Worded from_values(int64_t id, const Row& v) { return {Item::from_values(id, v)}; }
    static Text saved_since(Db& db, const Worded& current) { return Text::plain(current.name + " saved it; " + std::to_string(db.all<Worded>().size()) + " records"); }
};

}  // namespace

TEST(an_entity_words_the_refusal_with_the_database_at_hand) {
    Db db = Db::memory();
    db.batch("", undo_table);
    Worded first{{0, "first", 0}};
    db.save(first);
    Worded second = db.get<Worded>(first.id);
    first.name = "Anna";
    db.save(first);
    try {
        db.save(second);
        check::fail(__FILE__, __LINE__, "the save was not refused");
    } catch (const RefusedError& e) {
        CHECK_EQ(e.text.show(), std::string("Anna saved it; 1 records"));
    }
    try {
        db.remove(second);
        check::fail(__FILE__, __LINE__, "the delete was not refused");
    } catch (const RefusedError& e) {
        CHECK_EQ(e.text.show(), std::string("Anna saved it; 1 records"));
    }
}
