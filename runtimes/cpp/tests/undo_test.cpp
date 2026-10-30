// The database side of undo (DESIGN 5.29), as on the other stacks:
// everything one handler run changed is one step, with an id the screen's
// edit keeps. Undo puts the rows back as they were, with their own ids,
// cascades included; redo does it again; a handler that fails leaves
// nothing behind; an undo whose rows changed since is refused.

#include "check.hpp"
#include "clarity/db.hpp"
#include "clarity/sql.hpp"

using namespace clarity;

namespace {

struct Note {
    int64_t id = 0;
    std::optional<int64_t> shape;
    std::string text;
    static constexpr const char* table = "note";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"shape", "text"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::reference(shape), sql::text(text)}; }
    static Note from_values(int64_t id, const Row& v) { return {id, sql::get_reference("shape", v[0]), sql::get_text("text", v[1])}; }
};

struct Shape {
    int64_t id = 0;
    std::string name;
    int64_t x = 0;
    static constexpr const char* table = "shape";
    static const std::vector<std::string>& columns() {
        static const std::vector<std::string> names{"name", "x"};
        return names;
    }
    std::vector<SqlValue> values() const { return {sql::text(name), sql::integer(x)}; }
    static Shape from_values(int64_t id, const Row& v) { return {id, sql::get_text("name", v[0]), sql::get_integer("x", v[1])}; }
    void cascade(Db& db) const {
        for (const auto& note : db.find<Note>(Query().filter(Cond::compare("shape", Op::Eq, sql::integer(id))))) db.remove(note);
    }
    friend bool operator==(const Shape&, const Shape&) = default;
};

Db drawn() {
    Db db = Db::memory();
    db.migrate({{"CREATE TABLE shape (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, x INTEGER NOT NULL)",
                 "CREATE TABLE note (id INTEGER PRIMARY KEY AUTOINCREMENT, shape INTEGER REFERENCES shape(id), text TEXT NOT NULL)"}});
    return db;
}

Shape new_shape(Db& db, const std::string& name, int64_t x) {
    Shape s{0, name, x};
    db.save(s);
    return s;
}

}  // namespace

TEST(a_step_of_saves_is_undone_and_done_again_with_the_same_ids) {
    Db db = drawn();
    Shape a, b;
    db.step([&] { a = new_shape(db, "A", 10); });
    int64_t made = db.last_step();
    db.step([&] {
        a.x = 300;
        db.save(a);
        b = new_shape(db, "B", 20);
    });
    int64_t moved = db.last_step();
    CHECK(made != 0 && moved != made);
    CHECK(db.undo(moved));
    CHECK((db.all<Shape>() == std::vector<Shape>{{a.id, "A", 10}}));
    CHECK(!db.undo(moved));
    CHECK(db.redo(moved));
    CHECK((db.all<Shape>() == std::vector<Shape>{a, b}));
    db.undo(moved);
    db.undo(made);
    CHECK(db.all<Shape>().empty());
}

TEST(a_delete_with_its_cascade_comes_back_whole) {
    Db db = drawn();
    Shape a = new_shape(db, "A", 10);
    Note n{0, a.id, "keep"};
    db.save(n);
    db.step([&] { db.remove(a); });
    CHECK(db.all<Shape>().empty() && db.all<Note>().empty());
    CHECK(db.undo(db.last_step()));
    auto notes = db.all<Note>();
    CHECK((db.all<Shape>() == std::vector<Shape>{a}));
    REQUIRE(notes.size() == 1);
    CHECK_EQ(notes[0].id, n.id);
    CHECK(notes[0].shape == std::optional<int64_t>(a.id));
}

TEST(a_handler_that_fails_leaves_nothing_and_is_no_step) {
    Db db = drawn();
    CHECK_THROWS(StorageError, db.step([&] {
        new_shape(db, "A", 10);
        throw StorageError("no");
    }));
    CHECK(db.all<Shape>().empty());
    CHECK_EQ(db.last_step(), int64_t(0));
    db.step([&] { db.all<Shape>(); });
    // A step that changed nothing is no step.
    CHECK_EQ(db.last_step(), int64_t(0));
    CHECK(!db.undo(1));
}

TEST(an_undo_whose_rows_changed_since_is_refused) {
    Db db = drawn();
    Shape a = new_shape(db, "A", 10);
    db.step([&] {
        a.x = 50;
        db.save(a);
    });
    db.batch("", "UPDATE shape SET x = 99");
    std::string refused;
    try {
        db.undo(db.last_step());
    } catch (const RefusedError& e) {
        refused = e.text.key().value_or("");
    }
    CHECK_EQ(refused, std::string("undoChangedSince"));
    CHECK_EQ(db.all<Shape>()[0].x, int64_t(99));
}
