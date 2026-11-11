// A save never overwrites someone else's change (conformance/versions.json):
// the version read with every record, the checked save and delete, a save
// anyway, reload, undo by the version, and a file source changed on disk.
// The steps of one edit are undone all or none, a handler is one
// transaction, and only the last CLARITY_UNDO_STEPS are kept.

#include <stddef.h>
#include <stdlib.h>

#include "check.h"
#include "clarity/db.h"
#include "clarity/disk.h"
#include "clarity/sql.h"
#include "clarity/text.h"
#include "formats/file_source.h"
#include "tests.h"

typedef struct {
    int64_t id;
    const char* name;
    int64_t version;
} Item;

static const char* const item_columns[] = {"name"};

static void item_values(const void* record, ClarityValue* out) { out[0] = clarity_text_value(((const Item*)record)->name); }
static bool item_from_values(int64_t id, const ClarityValue* v, void* record) {
    Item* r = record;
    r->id = id;
    return clarity_get_text("name", v[0], &r->name);
}

static const ClarityEntity item = {
    .name = "Item", .table = "item", .key = "id", .source = "", .columns = item_columns, .column_count = 1,
    .size = sizeof(Item), .id_at = offsetof(Item, id), .values = item_values, .from_values = item_from_values,
    .version = "row_version", .version_at = offsetof(Item, version),
};

/// Whether `ok` is what the step expects: ok, refused with `refusal`'s words, or not found.
static void expect(const char* name, const cJSON* step, bool ok, const cJSON* refusal) {
    const cJSON* want = cJSON_GetObjectItem(step, "expect");
    const char* expected = cJSON_IsString(want) ? want->valuestring : "ok";
    const char* what = clarity_format("%s: %s", name, cJSON_PrintUnformatted(step));
    if (strcmp(expected, "ok") == 0) {
        if (!ok) check_fail(__FILE__, __LINE__, clarity_format("%s failed: %s", what, clarity_error()->message));
        return;
    }
    if (ok) {
        check_fail(__FILE__, __LINE__, clarity_format("%s went, %s expected", what, expected));
        return;
    }
    if (strcmp(expected, "notFound") == 0) {
        if (clarity_error()->kind != CLARITY_NOT_FOUND) check_fail(__FILE__, __LINE__, clarity_format("%s: %s", what, clarity_error()->message));
        return;
    }
    const ClarityText* text = clarity_error()->text;
    if (clarity_error()->kind != CLARITY_REFUSED || !text || !text->key) {
        check_fail(__FILE__, __LINE__, clarity_format("%s not refused: %s", what, clarity_error()->message));
        return;
    }
    CHECK_TEXT(text->key, cJSON_GetObjectItem(refusal, "key")->valuestring);
    CHECK_TEXT(text->text, cJSON_GetObjectItem(refusal, "english")->valuestring);
}

typedef struct {
    ClarityDb* db;
    Item* item;
} Saving;

static bool save_it(void* context) {
    Saving* s = context;
    return clarity_save(s->db, &item, s->item);
}

static const char* text_of(const cJSON* step, const char* field) {
    const cJSON* v = cJSON_GetObjectItem(step, field);
    return cJSON_IsString(v) ? v->valuestring : "";
}

void versions_are_checked_as_every_runtime_does(void) {
    cJSON* file = conformance("versions.json");
    REQUIRE(file);
    const cJSON* refusal = cJSON_GetObjectItem(file, "refusal");
    const cJSON* undo_refusal = cJSON_GetObjectItem(file, "undoRefusal");
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const char* name = text_of(c, "name");
        ClarityDb* db;
        REQUIRE_OK(clarity_db_memory(&db));
        REQUIRE_OK(clarity_db_batch(db, "", text_of(file, "table")));
        // The copies, by their one-letter names.
        Item copies[26] = {0};
        int64_t last = 0;
        const cJSON* step;
        cJSON_ArrayForEach(step, cJSON_GetObjectItem(c, "steps")) {
            const char* doing = text_of(step, "do");
            Item* copy = &copies[(text_of(step, "copy")[0] ? text_of(step, "copy")[0] : 'a') - 'a'];
            if (strcmp(doing, "insert") == 0) {
                *copy = (Item){0, clarity_copy(text_of(step, "name")), 0};
                expect(name, step, clarity_save(db, &item, copy), refusal);
            } else if (strcmp(doing, "read") == 0) {
                CHECK_OK(clarity_get(db, &item, copies[text_of(step, "of")[0] - 'a'].id, copy));
            } else if (strcmp(doing, "set") == 0) {
                copy->name = clarity_copy(text_of(step, "name"));
            } else if (strcmp(doing, "save") == 0) {
                expect(name, step, clarity_save(db, &item, copy), refusal);
            } else if (strcmp(doing, "saveAnyway") == 0) {
                expect(name, step, clarity_save_anyway(db, &item, copy), refusal);
            } else if (strcmp(doing, "delete") == 0) {
                expect(name, step, clarity_delete(db, &item, copy), refusal);
            } else if (strcmp(doing, "reload") == 0) {
                expect(name, step, clarity_reload(db, &item, copy), refusal);
            } else if (strcmp(doing, "version") == 0) {
                CHECK(copy->version == (int64_t)cJSON_GetObjectItem(step, "is")->valuedouble);
            } else if (strcmp(doing, "stored") == 0) {
                ClarityValues params = {0};
                clarity_values_add(&params, clarity_integer_value(copy->id));
                ClarityRows rows = {0};
                REQUIRE_OK(clarity_db_rows(db, "", "SELECT name, row_version FROM item WHERE id = ?", &params, &rows));
                if (cJSON_IsTrue(cJSON_GetObjectItem(step, "gone"))) {
                    CHECK(rows.count == 0);
                } else {
                    REQUIRE(rows.count == 1);
                    CHECK_TEXT(rows.items[0].values[0].text, text_of(step, "name"));
                    CHECK(rows.items[0].values[1].integer == (int64_t)cJSON_GetObjectItem(step, "version")->valuedouble);
                }
            } else if (strcmp(doing, "stepSave") == 0) {
                Saving saving = {db, copy};
                expect(name, step, clarity_step(db, save_it, &saving), refusal);
                last = clarity_last_step(db);
            } else if (strcmp(doing, "undo") == 0) {
                expect(name, step, clarity_undo(db, last, NULL), undo_refusal);
            } else if (strcmp(doing, "redo") == 0) {
                expect(name, step, clarity_redo(db, last, NULL), undo_refusal);
            } else {
                check_fail(__FILE__, __LINE__, clarity_format("%s: no step %s", name, doing));
            }
        }
        clarity_db_close(db);
    }
    cJSON_Delete(file);
}

typedef struct {
    int64_t id;
    const char* name;
} Line;

static const char* const line_columns[] = {"name"};
static void line_values(const void* record, ClarityValue* out) { out[0] = clarity_text_value(((const Line*)record)->name); }
static bool line_from_values(int64_t id, const ClarityValue* v, void* record) {
    Line* r = record;
    r->id = id;
    return clarity_get_text("name", v[0], &r->name);
}

static const ClarityEntity line = {
    .name = "Line", .table = "line", .key = "id", .source = "lines", .columns = line_columns, .column_count = 1,
    .size = sizeof(Line), .id_at = offsetof(Line, id), .values = line_values, .from_values = line_from_values,
};

static void write_text(const char* path, const char* text) { clarity_write_file(path, (ClarityBytes){(const uint8_t*)text, strlen(text)}); }

void a_file_changed_on_disk_refuses_the_save(void) {
    cJSON* file = conformance("versions.json");
    REQUIRE(file);
    const cJSON* refusal = cJSON_GetObjectItem(file, "refusal");
    const cJSON* source_case = cJSON_GetObjectItem(file, "file");
    const char* path = clarity_format("%s/clarity_versions_lines.csv", temp_folder());
    write_text(path, text_of(source_case, "text"));
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityFileSource source = {"lines", text_of(source_case, "format"), path, "", "line", "id", cJSON_PrintUnformatted(cJSON_GetObjectItem(source_case, "fields")),
                                false};
    REQUIRE_OK(clarity_db_add_file(db, &source));
    void* items;
    size_t count = 0;
    REQUIRE_OK(clarity_find(db, &line, clarity_query(), &items, &count));
    REQUIRE(count > 0);
    Line first = *(Line*)items;
    const cJSON* step;
    cJSON_ArrayForEach(step, cJSON_GetObjectItem(source_case, "steps")) {
        const char* doing = text_of(step, "do");
        if (strcmp(doing, "save") == 0) {
            first.name = clarity_copy(text_of(step, "name"));
            expect("file", step, clarity_save(db, &line, &first), refusal);
        } else if (strcmp(doing, "rewrite") == 0) {
            ClarityBytes bytes = {0};
            CHECK_OK(clarity_read_file(path, &bytes));
            clarity_write_file(path, bytes);
        } else if (strcmp(doing, "write") == 0) {
            write_text(path, text_of(step, "text"));
        } else if (strcmp(doing, "contents") == 0) {
            ClarityBytes bytes = {0};
            CHECK_OK(clarity_read_file(path, &bytes));
            CHECK_TEXT(clarity_copy_n((const char*)bytes.data, bytes.size), text_of(step, "text"));
        } else {
            check_fail(__FILE__, __LINE__, clarity_format("file: no step %s", doing));
        }
    }
    clarity_db_close(db);
    remove(path);
    cJSON_Delete(file);
}

static bool fail_after_two(void* context) {
    Saving* s = context;
    s->item->name = "changed";
    Item other = {0, "B", 0};
    return clarity_save(s->db, &item, s->item) && clarity_save(s->db, &item, &other) && clarity_fail(CLARITY_STORAGE, "no");
}

void a_handler_is_one_transaction_kept_at_its_end(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_db_batch(db, "", "CREATE TABLE item (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, row_version INTEGER NOT NULL DEFAULT 1)"));
    Item a = {0, "A", 0};
    REQUIRE_OK(clarity_save(db, &item, &a));
    Saving saving = {db, &a};
    CHECK(!clarity_step(db, fail_after_two, &saving));
    ClarityRows rows = {0};
    REQUIRE_OK(clarity_db_rows(db, "", "SELECT name, row_version FROM item", &(ClarityValues){0}, &rows));
    REQUIRE(rows.count == 1);
    CHECK_TEXT(rows.items[0].values[0].text, "A");
    CHECK(rows.items[0].values[1].integer == 1);
    clarity_db_close(db);
}

static bool set_name(void* context) {
    Saving* s = context;
    return clarity_save(s->db, &item, s->item);
}

void the_steps_of_one_edit_are_undone_all_or_none(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_db_batch(db, "", "CREATE TABLE item (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, row_version INTEGER NOT NULL DEFAULT 1)"));
    Item a = {0, "A", 0}, b = {0, "B", 0};
    REQUIRE_OK(clarity_save(db, &item, &a));
    REQUIRE_OK(clarity_save(db, &item, &b));
    a.name = "A2";
    Saving sa = {db, &a};
    REQUIRE_OK(clarity_step(db, set_name, &sa));
    int64_t first = clarity_last_step(db);
    b.name = "B2";
    Saving sb = {db, &b};
    REQUIRE_OK(clarity_step(db, set_name, &sb));
    int64_t second = clarity_last_step(db);
    // B changed since the edit, so neither step is taken back.
    Item other;
    REQUIRE_OK(clarity_get(db, &item, b.id, &other));
    REQUIRE_OK(clarity_save(db, &item, &other));
    int64_t steps[] = {second, first};
    CHECK(!clarity_replay_steps(db, steps, 2, true, NULL));
    CHECK(clarity_error()->text && strcmp(clarity_error()->text->key, "undoChangedSince") == 0);
    Item now;
    REQUIRE_OK(clarity_get(db, &item, a.id, &now));
    CHECK_TEXT(now.name, "A2");
    clarity_db_close(db);
}

void only_the_last_steps_are_kept(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_db_batch(db, "", "CREATE TABLE item (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, row_version INTEGER NOT NULL DEFAULT 1)"));
    Item a = {0, "A", 0};
    REQUIRE_OK(clarity_save(db, &item, &a));
    Saving saving = {db, &a};
    for (int i = 0; i <= CLARITY_UNDO_STEPS; ++i) {
        a.name = clarity_format("A%d", i);
        REQUIRE_OK(clarity_step(db, set_name, &saving));
    }
    CHECK(!clarity_undo(db, 1, NULL));
    CHECK(clarity_error()->text && strcmp(clarity_error()->text->key, "undoTooOld") == 0);
    bool done = false;
    CHECK_OK(clarity_undo(db, clarity_last_step(db), &done));
    CHECK(done);
    clarity_db_close(db);
}

/// An item whose entity words the refusal itself, about the record as
/// stored now and what the database holds (as `{current.changedBy.name}` needs).
static bool item_saved_since(ClarityDb* db, const void* current);

static const ClarityEntity worded = {
    .name = "Item", .table = "item", .key = "id", .source = "", .columns = item_columns, .column_count = 1,
    .size = sizeof(Item), .id_at = offsetof(Item, id), .values = item_values, .from_values = item_from_values,
    .version = "row_version", .version_at = offsetof(Item, version), .saved_since = item_saved_since,
};

static bool item_saved_since(ClarityDb* db, const void* current) {
    void* items;
    size_t count = 0;
    if (!clarity_find(db, &worded, clarity_query(), &items, &count)) return false;
    return clarity_refuse(clarity_plain_text(clarity_format("%s saved it; %zu records", ((const Item*)current)->name, count)));
}

void an_entity_words_the_refusal_with_the_database_at_hand(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_db_batch(db, "", "CREATE TABLE item (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, row_version INTEGER NOT NULL DEFAULT 1)"));
    Item first = {0, "first", 0}, second;
    REQUIRE_OK(clarity_save(db, &worded, &first));
    REQUIRE_OK(clarity_get(db, &worded, first.id, &second));
    first.name = "Anna";
    REQUIRE_OK(clarity_save(db, &worded, &first));
    CHECK(!clarity_save(db, &worded, &second));
    CHECK(clarity_error()->text && strcmp(clarity_text_show(clarity_error()->text), "Anna saved it; 1 records") == 0);
    CHECK(!clarity_delete(db, &worded, &second));
    CHECK(clarity_error()->text && strcmp(clarity_text_show(clarity_error()->text), "Anna saved it; 1 records") == 0);
    clarity_db_close(db);
}
