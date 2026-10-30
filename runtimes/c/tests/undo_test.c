// The database side of undo (DESIGN 5.29), as on the other stacks:
// everything one handler run changed is one step, with an id the screen's
// edit keeps. Undo puts the rows back as they were, with their own ids,
// cascades included; redo does it again; a handler that fails leaves
// nothing behind; an undo whose rows changed since is refused.

#include <stddef.h>

#include "check.h"
#include "clarity/db.h"
#include "clarity/sql.h"
#include "tests.h"

typedef struct {
    int64_t id;
    const char* name;
    int64_t x;
} Shape;

typedef struct {
    int64_t id;
    int64_t shape;
    const char* text;
} Note;

static const char* const shape_columns[] = {"name", "x"};
static const char* const note_columns[] = {"shape", "text"};

static void shape_values(const void* record, ClarityValue* out) {
    const Shape* r = record;
    out[0] = clarity_text_value(r->name);
    out[1] = clarity_integer_value(r->x);
}
static bool shape_from_values(int64_t id, const ClarityValue* v, void* record) {
    Shape* r = record;
    r->id = id;
    return clarity_get_text("name", v[0], &r->name) && clarity_get_integer("x", v[1], &r->x);
}
static void note_values(const void* record, ClarityValue* out) {
    const Note* r = record;
    out[0] = clarity_reference_value(r->shape);
    out[1] = clarity_text_value(r->text);
}
static bool note_from_values(int64_t id, const ClarityValue* v, void* record) {
    Note* r = record;
    r->id = id;
    return clarity_get_reference("shape", v[0], &r->shape) && clarity_get_text("text", v[1], &r->text);
}
static const ClarityEntity note = {
    .name = "Note", .table = "note", .key = "id", .source = "", .columns = note_columns, .column_count = 2,
    .size = sizeof(Note), .id_at = offsetof(Note, id), .values = note_values, .from_values = note_from_values,
};
// As generated code writes it: the notes of the shape, each deleted as any record is.
static bool shape_cascade(ClarityDb* db, int64_t id) {
    void* items;
    size_t count;
    if (!clarity_find(db, &note, clarity_where(clarity_query(), clarity_compare("shape", CLARITY_EQ, clarity_integer_value(id))), &items, &count)) return false;
    for (size_t i = 0; i < count; ++i)
        if (!clarity_delete(db, &note, (char*)items + i * note.size)) return false;
    return true;
}
static const ClarityEntity shape = {
    .name = "Shape", .table = "shape", .key = "id", .source = "", .columns = shape_columns, .column_count = 2,
    .size = sizeof(Shape), .id_at = offsetof(Shape, id), .values = shape_values, .from_values = shape_from_values, .cascade = shape_cascade,
};

static bool drawn(ClarityDb** db) {
    static const char* const create[] = {"CREATE TABLE shape (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, x INTEGER NOT NULL)",
                                         "CREATE TABLE note (id INTEGER PRIMARY KEY AUTOINCREMENT, shape INTEGER REFERENCES shape(id), text TEXT NOT NULL)"};
    ClarityStep step = {create, 2};
    return clarity_db_memory(db) && clarity_migrate(*db, "", &step, 1, NULL);
}

/// The shapes as "id name x", one after another.
static const char* shapes(ClarityDb* db) {
    void* items;
    size_t count = 0;
    if (!clarity_find(db, &shape, clarity_query(), &items, &count)) return clarity_error()->message;
    ClarityBuffer out = {0};
    for (size_t i = 0; i < count; ++i) {
        const Shape* s = (const Shape*)items + i;
        clarity_buffer_addf(&out, "%s%lld %s %lld", i ? ", " : "", (long long)s->id, s->name, (long long)s->x);
    }
    return clarity_buffer_text(&out);
}

typedef struct {
    ClarityDb* db;
    Shape* a;
    Shape* b;
} Drawing;

static bool make_a(void* context) {
    Drawing* d = context;
    *d->a = (Shape){0, "A", 10};
    return clarity_save(d->db, &shape, d->a);
}

static bool move_a_and_make_b(void* context) {
    Drawing* d = context;
    d->a->x = 300;
    *d->b = (Shape){0, "B", 20};
    return clarity_save(d->db, &shape, d->a) && clarity_save(d->db, &shape, d->b);
}

static bool delete_a(void* context) {
    Drawing* d = context;
    return clarity_delete(d->db, &shape, d->a);
}

static bool make_a_and_fail(void* context) {
    return make_a(context) && clarity_fail(CLARITY_STORAGE, "no");
}

static bool read_only(void* context) {
    Drawing* d = context;
    return *shapes(d->db) || true;
}

static bool move_a_to_50(void* context) {
    Drawing* d = context;
    d->a->x = 50;
    return clarity_save(d->db, &shape, d->a);
}

void a_step_of_saves_is_undone_and_done_again_with_the_same_ids(void) {
    ClarityDb* db;
    REQUIRE_OK(drawn(&db));
    Shape a, b;
    Drawing d = {db, &a, &b};
    REQUIRE_OK(clarity_step(db, make_a, &d));
    int64_t made = clarity_last_step(db);
    REQUIRE_OK(clarity_step(db, move_a_and_make_b, &d));
    int64_t moved = clarity_last_step(db);
    CHECK(made != 0 && moved != made);
    bool done = false;
    CHECK_OK(clarity_undo(db, moved, &done));
    CHECK(done);
    CHECK_TEXT(shapes(db), clarity_format("%lld A 10", (long long)a.id));
    CHECK_OK(clarity_undo(db, moved, &done));
    CHECK(!done);
    CHECK_OK(clarity_redo(db, moved, &done));
    CHECK(done);
    CHECK_TEXT(shapes(db), clarity_format("%lld A 300, %lld B 20", (long long)a.id, (long long)b.id));
    CHECK_OK(clarity_undo(db, moved, NULL));
    CHECK_OK(clarity_undo(db, made, NULL));
    CHECK_TEXT(shapes(db), "");
    clarity_db_close(db);
}

void a_delete_with_its_cascade_comes_back_whole(void) {
    ClarityDb* db;
    REQUIRE_OK(drawn(&db));
    Shape a = {0, "A", 10};
    REQUIRE_OK(clarity_save(db, &shape, &a));
    Note n = {0, a.id, "keep"};
    REQUIRE_OK(clarity_save(db, &note, &n));
    Drawing d = {db, &a, NULL};
    REQUIRE_OK(clarity_step(db, delete_a, &d));
    CHECK_TEXT(shapes(db), "");
    bool done = false;
    CHECK_OK(clarity_undo(db, clarity_last_step(db), &done));
    CHECK(done);
    CHECK_TEXT(shapes(db), clarity_format("%lld A 10", (long long)a.id));
    void* items;
    size_t count = 0;
    REQUIRE_OK(clarity_find(db, &note, clarity_query(), &items, &count));
    REQUIRE(count == 1);
    CHECK_INT(((Note*)items)->id, n.id);
    CHECK_INT(((Note*)items)->shape, a.id);
    clarity_db_close(db);
}

void a_handler_that_fails_leaves_nothing_and_is_no_step(void) {
    ClarityDb* db;
    REQUIRE_OK(drawn(&db));
    Shape a;
    Drawing d = {db, &a, NULL};
    CHECK_FAILS(CLARITY_STORAGE, clarity_step(db, make_a_and_fail, &d));
    CHECK_TEXT(clarity_error()->message, "storage: no");
    CHECK_TEXT(shapes(db), "");
    CHECK_INT(clarity_last_step(db), 0);
    // A step that changed nothing is no step.
    CHECK_OK(clarity_step(db, read_only, &d));
    CHECK_INT(clarity_last_step(db), 0);
    bool done = true;
    CHECK_OK(clarity_undo(db, 1, &done));
    CHECK(!done);
    clarity_db_close(db);
    // Fail safe: without a database a step just runs, and there is nothing to undo.
    CHECK_INT(clarity_last_step(NULL), 0);
    CHECK_OK(clarity_undo(NULL, 1, &done));
    CHECK(!done);
}

void an_undo_whose_rows_changed_since_is_refused(void) {
    ClarityDb* db;
    REQUIRE_OK(drawn(&db));
    Shape a = {0, "A", 10};
    REQUIRE_OK(clarity_save(db, &shape, &a));
    Drawing d = {db, &a, NULL};
    REQUIRE_OK(clarity_step(db, move_a_to_50, &d));
    REQUIRE_OK(clarity_db_batch(db, "", "UPDATE shape SET x = 99"));
    CHECK_FAILS(CLARITY_REFUSED, clarity_undo(db, clarity_last_step(db), NULL));
    CHECK(clarity_error()->text && clarity_error()->text->key && strcmp(clarity_error()->text->key, "undoChangedSince") == 0);
    CHECK_TEXT(shapes(db), clarity_format("%lld A 99", (long long)a.id));
    clarity_db_close(db);
}
