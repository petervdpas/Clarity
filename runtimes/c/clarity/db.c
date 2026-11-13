#include "clarity/db.h"

#include <stdlib.h>
#include <string.h>

#include "clarity/backup.h"
#include "clarity/db_internal.h"
#include "clarity/disk.h"
#include "clarity/people.h"
#include "clarity/record_file.h"

char* clarity_lasting(const char* text) {
    size_t length = strlen(text);
    char* out = malloc(length + 1);
    if (out) memcpy(out, text, length + 1);
    return out;
}

bool clarity_db_open(const ClaritySource* sources, size_t count, ClarityDb** out) {
    clarity_start();
    ClarityDb* db = calloc(1, sizeof *db);
    if (!db) return clarity_fail(CLARITY_STORAGE, "out of memory");
    db->sources = calloc(count ? count : 1, sizeof *db->sources);
    for (size_t i = 0; i < count; ++i) {
        db->sources[i].name = clarity_lasting(sources[i].name);
        if (!clarity_open_driver(sources[i].dialect, sources[i].at, &db->sources[i].driver)) {
            db->count = i;
            clarity_db_close(db);
            return false;
        }
        db->count = i + 1;
    }
    *out = db;
    return true;
}

bool clarity_db_open_sqlite(const char* path, ClarityDb** out) {
    ClaritySource source = {"", CLARITY_SQLITE, path};
    return clarity_db_open(&source, 1, out);
}

bool clarity_db_memory(ClarityDb** out) { return clarity_db_open_sqlite(":memory:", out); }

static void free_changes(ClarityChanges* changes);

void clarity_db_close(ClarityDb* db) {
    if (!db) return;
    if (db->people && db->forget_people) db->forget_people(db->people);
    free_changes(&db->changes);
    for (size_t i = 0; i < db->step_count; ++i) free_changes(&db->steps[i].changes);
    free(db->steps);
    free(db->held);
    free(db->held_settings);
    for (size_t i = 0; i < db->count; ++i) {
        db->sources[i].driver->close(db->sources[i].driver);
        free(db->sources[i].name);
    }
    for (size_t i = 0; i < db->store_count; ++i) {
        free(db->stores[i].name);
        free(db->stores[i].folder);
        free(db->stores[i].program);
        free(db->stores[i].setting);
        free(db->stores[i].fallback);
    }
    free(db->sources);
    free(db->stores);
    free(db);
}

bool clarity_source(ClarityDb* db, const char* source, ClaritySourceDb** out) {
    bool single = db->count == 1;
    for (size_t i = 0; i < db->count; ++i) {
        if (single || !source || !*source || strcmp(db->sources[i].name, source) == 0) {
            *out = &db->sources[i];
            return true;
        }
    }
    return clarity_fail(CLARITY_STORAGE, "there is no data source '%s'", source);
}

bool clarity_db_batch(ClarityDb* db, const char* source, const char* sql) {
    ClaritySourceDb* s = NULL;
    return clarity_source(db, source, &s) && s->driver->batch(s->driver, sql);
}

bool clarity_db_rows(ClarityDb* db, const char* source, const char* sql, const ClarityValues* params, ClarityRows* out) {
    ClaritySourceDb* s = NULL;
    return clarity_source(db, source, &s) && s->driver->rows(s->driver, sql, params, out);
}

bool clarity_db_dialect(ClarityDb* db, const char* source, ClarityDialect* out) {
    ClaritySourceDb* s = NULL;
    if (!clarity_source(db, source, &s)) return false;
    *out = s->driver->dialect;
    return true;
}

static bool version_of(ClarityDriver* d, int* out) {
    ClarityRows rows = {0};
    if (!d->batch(d, clarity_version_table_sql(d->dialect)) || !d->rows(d, "SELECT MAX(version) FROM clarity_schema", NULL, &rows)) return false;
    *out = rows.count && rows.items[0].values[0].kind == CLARITY_INTEGER ? (int)rows.items[0].values[0].integer : 0;
    return true;
}

bool clarity_schema_version(ClarityDb* db, int* out) {
    ClaritySourceDb* s = NULL;
    return clarity_source(db, "", &s) && version_of(s->driver, out);
}

bool clarity_migrate(ClarityDb* db, const char* source, const ClarityStep* steps, size_t count, int* applied) {
    ClaritySourceDb* s = NULL;
    int done;
    if (!clarity_source(db, source, &s) || !version_of(s->driver, &done)) return false;
    // A database that already has a schema is backed up before it changes (docs/BACKUP-PLAN.md).
    if (done > 0 && done < (int)count && !clarity_backup_before_migration(s->driver, done + 1)) return false;
    for (int i = done; i < (int)count; ++i)
        if (!s->driver->apply(s->driver, i + 1, &steps[i])) return false;
    if (applied) *applied = (int)count > done ? (int)count - done : 0;
    return true;
}

static const ClarityPictureColumn* picture_of(const ClarityEntity* entity, const char* column) {
    for (size_t i = 0; i < entity->picture_count; ++i)
        if (strcmp(entity->pictures[i].column, column) == 0) return &entity->pictures[i];
    return NULL;
}

/// Clarity's own words, by key, with their English for a program without the key.
static ClarityText* keyed(const char* key, const char* english) {
    ClarityText* text = clarity_text(english);
    text->key = clarity_copy(key);
    return text;
}

ClarityText* clarity_saved_since(void) { return keyed("savedSince", "Someone else changed this record since you opened it. Reload it to see their change."); }
ClarityText* clarity_undo_changed_since(void) { return keyed("undoChangedSince", "This was changed since, so it cannot be taken back"); }
ClarityText* clarity_undo_too_old(void) { return keyed("undoTooOld", "This was done too long ago to take back"); }

/// The key, the stored columns and the version of a record, for SELECT.
static const char* select_columns(ClarityDialect d, const ClarityEntity* entity) {
    ClarityBuffer out = {0};
    clarity_buffer_add(&out, clarity_quote(d, entity->key));
    for (size_t i = 0; i < entity->column_count; ++i) {
        const char* column = entity->columns[i];
        const ClarityPictureColumn* p = picture_of(entity, column);
        clarity_buffer_add(&out, ", ");
        // Of a picture in the database: whether there is one, not the picture.
        clarity_buffer_add(&out, p && !p->store ? clarity_presence_sql(d, column) : clarity_quote(d, column));
    }
    if (entity->version) clarity_buffer_addf(&out, ", %s", clarity_quote(d, entity->version));
    return clarity_buffer_text(&out);
}

static int64_t* id_of(const ClarityEntity* entity, void* record) { return (int64_t*)((char*)record + entity->id_at); }

/// The version a record holds; NULL for an entity without one.
static int64_t* record_version(const ClarityEntity* entity, void* record) {
    return entity->version ? (int64_t*)((char*)record + entity->version_at) : NULL;
}

/// The refusal of a save or delete of record `id` someone saved since: in
/// the entity's own words about the record as stored now, or Clarity's.
static bool refused_since(ClarityDb* db, const ClarityEntity* entity, int64_t id) {
    if (entity->saved_since) {
        void* current = clarity_alloc(entity->size);
        if (clarity_get(db, entity, id, current)) return entity->saved_since(db, current);
    }
    return clarity_refuse(clarity_saved_since());
}

/// The version record `id` is stored with now.
static bool stored_version(ClarityDriver* d, const ClarityEntity* entity, int64_t id, int64_t* out) {
    ClarityValues params = {0};
    clarity_values_add(&params, clarity_integer_value(id));
    const char* sql = clarity_format("SELECT %s FROM %s WHERE %s = %s", clarity_quote(d->dialect, entity->version), clarity_quote(d->dialect, entity->table),
                                     clarity_quote(d->dialect, entity->key), clarity_mark(d->dialect, 1));
    ClarityRows rows = {0};
    if (!d->rows(d, sql, &params, &rows)) return false;
    *out = rows.count ? rows.items[0].values[0].integer : 0;
    return true;
}

/// A failed save: a reference to a record that is not there reads better than the database's own words.
static bool unfit(void) {
    if (clarity_error()->kind == CLARITY_REFERENCE) return clarity_fail(CLARITY_INVALID, "a reference does not fit: %s", clarity_error()->message);
    return false;
}

static bool exists(ClarityDriver* d, const ClarityEntity* entity, int64_t id, bool* out) {
    ClarityValues params = {0};
    clarity_values_add(&params, clarity_integer_value(id));
    const char* sql = clarity_format("SELECT %s FROM %s WHERE %s = %s", clarity_quote(d->dialect, entity->key), clarity_quote(d->dialect, entity->table),
                                     clarity_quote(d->dialect, entity->key), clarity_mark(d->dialect, 1));
    ClarityRows rows = {0};
    if (!d->rows(d, sql, &params, &rows)) return false;
    *out = rows.count > 0;
    return true;
}

/// Writes a record: an INSERT for a new one, an UPDATE for a stored one,
/// checked by its version unless `anyway`.
static bool store_row(ClarityDb* db, const ClarityEntity* entity, void* record, bool anyway) {
    size_t n = entity->column_count;
    ClarityValue* values = CLARITY_NEW(ClarityValue, n);
    entity->values(record, values);
    // A new picture is checked, and written to its file store when it has
    // one, its column then holding the file's name.
    for (size_t i = 0; i < n; ++i) {
        const ClarityPictureColumn* p = picture_of(entity, entity->columns[i]);
        if (!p || values[i].kind != CLARITY_BYTES) continue;
        // A file of a file field is kept under its own name, in a folder named by its contents.
        if (p->name) {
            if (!p->store) continue;
            const char* named = "";
            for (size_t j = 0; j < n; ++j)
                if (strcmp(entity->columns[j], p->name) == 0 && values[j].kind == CLARITY_TEXT) named = values[j].text;
            const char* folder = NULL;
            if (!clarity_store_folder(db, p->store, &folder)) return false;
            const char* path = clarity_store_path(values[i].bytes, named);
            const char* file = NULL;
            if (!clarity_in_store(folder, path, &file)) return false;
            if (!clarity_exists(file) && !(clarity_make_folders(clarity_parent(file)) && clarity_write_file(file, values[i].bytes))) return false;
            values[i] = clarity_text_value(path);
            continue;
        }
        const char* name = NULL;
        if (!clarity_picture_file_name(values[i].bytes, &name)) return false;
        if (!p->store) continue;
        const char* folder = NULL;
        if (!clarity_store_folder(db, p->store, &folder)) return false;
        const char* file = clarity_format("%s/%s", folder, name);
        if (!clarity_exists(file) && !clarity_write_file(file, values[i].bytes)) return false;
        values[i] = clarity_text_value(name);
    }
    ClaritySourceDb* s = NULL;
    if (!clarity_source(db, entity->source, &s)) return false;
    ClarityDriver* d = s->driver;
    ClarityDialect dialect = d->dialect;
    int64_t* id = id_of(entity, record);
    int64_t* version = record_version(entity, record);
    ClarityValues params = {0};
    if (*id == 0) {
        // An empty or kept picture is left to the column's NULL, so no NULL
        // is bound to a binary column.
        const char** names = CLARITY_NEW(const char*, n + 1);
        size_t count = 0;
        for (size_t i = 0; i < n; ++i) {
            bool none = values[i].kind == CLARITY_NULL || values[i].kind == CLARITY_KEPT;
            if (picture_of(entity, entity->columns[i]) && none) continue;
            names[count++] = entity->columns[i];
            clarity_values_add(&params, values[i]);
        }
        // A new record starts at version 1.
        if (version) {
            names[count++] = entity->version;
            clarity_values_add(&params, clarity_integer_value(1));
        }
        const char* sql = clarity_insert_sql(dialect, entity->table, entity->key, names, count);
        if (!d->insert(d, sql, &params, id)) return unfit();
        if (version) *version = 1;
        return true;
    }
    // A kept picture stays as it is; an emptied one is set to NULL as text.
    ClarityBuffer sets = {0};
    for (size_t i = 0; i < n; ++i) {
        if (values[i].kind == CLARITY_KEPT) continue;
        if (sets.length) clarity_buffer_add(&sets, ", ");
        const char* column = clarity_quote(dialect, entity->columns[i]);
        if (picture_of(entity, entity->columns[i]) && values[i].kind == CLARITY_NULL) {
            clarity_buffer_addf(&sets, "%s = NULL", column);
            continue;
        }
        clarity_values_add(&params, values[i]);
        clarity_buffer_addf(&sets, "%s = %s", column, clarity_mark(dialect, (int)params.count));
    }
    // Each save adds one to the version; a checked one writes only over the version it read.
    if (version) {
        const char* column = clarity_quote(dialect, entity->version);
        clarity_buffer_addf(&sets, "%s%s = %s + 1", sets.length ? ", " : "", column, column);
    }
    bool there;
    if (sets.length == 0) {
        if (!exists(d, entity, *id, &there)) return false;
        return there || clarity_not_found(entity->table, *id);
    }
    clarity_values_add(&params, clarity_integer_value(*id));
    const char* sql = clarity_format("UPDATE %s SET %s WHERE %s = %s", clarity_quote(dialect, entity->table), clarity_buffer_text(&sets),
                                     clarity_quote(dialect, entity->key), clarity_mark(dialect, (int)params.count));
    bool checked = version && !anyway;
    if (checked) {
        clarity_values_add(&params, clarity_integer_value(*version));
        sql = clarity_format("%s AND %s = %s", sql, clarity_quote(dialect, entity->version), clarity_mark(dialect, (int)params.count));
    }
    int64_t changed;
    if (!d->execute(d, sql, &params, &changed)) return unfit();
    // MySQL counts only the rows whose values changed, so an update that
    // changes nothing is told apart from a missing record (one with a
    // version always changes).
    if (changed == 0) {
        if (!exists(d, entity, *id, &there)) return false;
        if (!there) return clarity_not_found(entity->table, *id);
        if (checked) return refused_since(db, entity, *id);
    }
    if (version && anyway) return stored_version(d, entity, *id, version);
    if (version) ++*version;
    return true;
}

/// Deletes a row; one of a versioned entity only at `version`, the one read.
static bool remove_row(ClarityDb* db, const ClarityEntity* entity, int64_t id, int64_t version) {
    ClaritySourceDb* s = NULL;
    if (!clarity_source(db, entity->source, &s)) return false;
    ClarityDriver* d = s->driver;
    ClarityValues params = {0};
    clarity_values_add(&params, clarity_integer_value(id));
    const char* sql = clarity_format("DELETE FROM %s WHERE %s = %s", clarity_quote(d->dialect, entity->table), clarity_quote(d->dialect, entity->key),
                                     clarity_mark(d->dialect, 1));
    // A record someone saved since it was read is not deleted.
    if (entity->version) {
        clarity_values_add(&params, clarity_integer_value(version));
        sql = clarity_format("%s AND %s = %s", sql, clarity_quote(d->dialect, entity->version), clarity_mark(d->dialect, 2));
    }
    int64_t changed = 0;
    if (!d->execute(d, sql, &params, &changed)) return clarity_error()->kind == CLARITY_REFERENCE ? clarity_in_use(entity->table, id) : false;
    bool there = false;
    if (changed == 0 && entity->version) {
        if (!exists(d, entity, id, &there)) return false;
        if (there) return refused_since(db, entity, id);
    }
    return true;
}

// Undo (DESIGN 5.29): while a step runs, every row a save or delete
// changes is kept as it was before and as it is after.

/// A copy of values that outlives the scratch arena, to free with free_values.
static ClarityValue* lasting_values(const ClarityValue* values, size_t count) {
    ClarityValue* out = calloc(count ? count : 1, sizeof *out);
    if (!out) return NULL;
    for (size_t i = 0; i < count; ++i) {
        out[i] = values[i];
        if (values[i].kind == CLARITY_TEXT) out[i].text = clarity_lasting(values[i].text ? values[i].text : "");
        if (values[i].kind == CLARITY_BYTES) {
            uint8_t* bytes = malloc(values[i].bytes.size ? values[i].bytes.size : 1);
            if (bytes && values[i].bytes.size) memcpy(bytes, values[i].bytes.data, values[i].bytes.size);
            out[i].bytes = (ClarityBytes){bytes, values[i].bytes.size};
        }
    }
    return out;
}

static void free_values(ClarityValue* values, size_t count) {
    if (!values) return;
    for (size_t i = 0; i < count; ++i) {
        if (values[i].kind == CLARITY_TEXT) free((char*)values[i].text);
        if (values[i].kind == CLARITY_BYTES) free((uint8_t*)values[i].bytes.data);
    }
    free(values);
}

/// Frees changes, and forgets them.
static void free_changes(ClarityChanges* changes) {
    for (size_t i = 0; i < changes->count; ++i) {
        ClarityChange* c = &changes->items[i];
        free_values(c->before, c->entity->column_count);
        free_values(c->after, c->entity->column_count);
    }
    free(changes->items);
    *changes = (ClarityChanges){0};
}

static bool same_value(const ClarityValue* a, const ClarityValue* b) {
    if (a->kind != b->kind) return false;
    switch (a->kind) {
    case CLARITY_NULL:
    case CLARITY_KEPT:
        return true;
    case CLARITY_INTEGER:
        return a->integer == b->integer;
    case CLARITY_REAL:
        return a->real == b->real;
    case CLARITY_TEXT:
        return strcmp(a->text, b->text) == 0;
    case CLARITY_BYTES:
        return a->bytes.size == b->bytes.size && (a->bytes.size == 0 || memcmp(a->bytes.data, b->bytes.data, a->bytes.size) == 0);
    case CLARITY_DECIMAL:
        return a->decimal.units == b->decimal.units && a->decimal.scale == b->decimal.scale;
    case CLARITY_BOOL:
        return a->flag == b->flag;
    case CLARITY_DATE:
        return a->date.year == b->date.year && a->date.month == b->date.month && a->date.day == b->date.day;
    case CLARITY_DATE_TIME:
        return memcmp(&a->date_time, &b->date_time, sizeof a->date_time) == 0;
    }
    return false;
}

/// A row's raw values (its columns), in the scratch arena, and its version
/// (`version`, may be NULL; 0 without one); `*there` is false when it is not there.
static bool read_row(ClarityDb* db, const ClarityEntity* entity, int64_t id, bool* there, ClarityValue** out, int64_t* version) {
    ClaritySourceDb* s = NULL;
    if (!clarity_source(db, entity->source, &s)) return false;
    ClarityDialect d = s->driver->dialect;
    ClarityBuffer names = {0};
    clarity_buffer_add(&names, clarity_quote(d, entity->key));
    for (size_t i = 0; i < entity->column_count; ++i) clarity_buffer_addf(&names, ", %s", clarity_quote(d, entity->columns[i]));
    if (entity->version) clarity_buffer_addf(&names, ", %s", clarity_quote(d, entity->version));
    ClarityValues params = {0};
    clarity_values_add(&params, clarity_integer_value(id));
    const char* sql = clarity_format("SELECT %s FROM %s WHERE %s = %s", clarity_buffer_text(&names), clarity_quote(d, entity->table), clarity_quote(d, entity->key),
                                     clarity_mark(d, 1));
    ClarityRows rows = {0};
    if (!s->driver->rows(s->driver, sql, &params, &rows)) return false;
    *there = rows.count > 0;
    *out = rows.count ? rows.items[0].values + 1 : NULL;
    if (version) *version = rows.count && entity->version ? rows.items[0].values[entity->column_count + 1].integer : 0;
    return true;
}

/// Keeps a change of the step that runs.
static bool record_change(ClarityDb* db, const ClarityEntity* entity, int64_t id, bool had, const ClarityValue* before, bool has, const ClarityValue* after,
                          int64_t before_version, int64_t after_version) {
    ClarityChanges* changes = &db->changes;
    if (changes->count == changes->capacity) {
        size_t capacity = changes->capacity ? changes->capacity * 2 : 8;
        ClarityChange* items = realloc(changes->items, capacity * sizeof *items);
        if (!items) return clarity_fail(CLARITY_STORAGE, "out of memory");
        changes->items = items;
        changes->capacity = capacity;
    }
    size_t n = entity->column_count;
    changes->items[changes->count++] =
        (ClarityChange){entity, id, had, has, had ? lasting_values(before, n) : NULL, has ? lasting_values(after, n) : NULL, before_version, after_version};
    return true;
}

static bool begin(ClarityDb* db, const char* source, ClaritySourceDb** out);
static bool end(ClaritySourceDb* s, bool ok);

/// Makes the running step hold a transaction in `source`, from its first
/// change there; outside a step, nothing.
static bool enlist(ClarityDb* db, const char* source) {
    if (!db->recording) return true;
    ClaritySourceDb* s = NULL;
    if (!clarity_source(db, source, &s)) return false;
    size_t at = (size_t)(s - db->sources);
    for (size_t i = 0; i < db->held_count; ++i)
        if (db->held[i] == at) return true;
    size_t* held = realloc(db->held, (db->held_count + 1) * sizeof *held);
    if (!held) return clarity_fail(CLARITY_STORAGE, "out of memory");
    db->held = held;
    if (!begin(db, source, &s)) return false;
    db->held[db->held_count++] = at;
    return true;
}

/// Ends the transactions a step held: kept (`keep`) or rolled back. When one
/// cannot be kept, the ones not kept yet are rolled back too.
static bool release(ClarityDb* db, bool keep) {
    for (size_t i = 0; i < db->held_count; ++i) keep = end(&db->sources[db->held[i]], keep) && keep;
    db->held_count = 0;
    return keep;
}

/// Saves a row, keeping it as it was and as it is now while a step records.
/// Tells the people who sign in (clarity/people.h) which columns of a stored person's row a save changed.
static void tell_people(ClarityDb* db, const ClarityEntity* entity, int64_t id, const ClarityValue* before, const ClarityValue* after) {
    const char** changed = CLARITY_NEW(const char*, entity->column_count + 1);
    size_t count = 0;
    for (size_t i = 0; i < entity->column_count; ++i)
        if (!same_value(&before[i], &after[i])) changed[count++] = entity->columns[i];
    if (count) clarity_people()->changed(db, entity, id, changed, count);
}

static bool store_record(ClarityDb* db, const ClarityEntity* entity, void* record, bool anyway) {
    if (!enlist(db, entity->source)) return false;
    int64_t* id = id_of(entity, record);
    // A save of a stored person: signing in may care what changed (their secret).
    const ClarityPeopleHooks* people = clarity_people();
    bool watched = *id != 0 && db->people && people && people->watches && people->changed && people->watches(db, entity);
    if (!db->recording && !watched) return store_row(db, entity, record, anyway);
    bool had = false, has = false;
    ClarityValue *before = NULL, *after = NULL;
    int64_t before_version = 0, after_version = 0;
    if (*id != 0 && !read_row(db, entity, *id, &had, &before, &before_version)) return false;
    if (!store_row(db, entity, record, anyway) || !read_row(db, entity, *id, &has, &after, &after_version)) return false;
    if (watched && had && has) tell_people(db, entity, *id, before, after);
    return !db->recording || record_change(db, entity, *id, had, before, has, after, before_version, after_version);
}

/// Deletes a row (at `version`, the one read), keeping it as it was while a step records.
static bool remove_record(ClarityDb* db, const ClarityEntity* entity, int64_t id, int64_t version) {
    if (!enlist(db, entity->source)) return false;
    if (!db->recording) return remove_row(db, entity, id, version);
    bool had = false;
    ClarityValue* before = NULL;
    int64_t before_version = 0;
    return read_row(db, entity, id, &had, &before, &before_version) && remove_row(db, entity, id, version) &&
           record_change(db, entity, id, had, before, false, NULL, before_version, 0);
}

/// Makes a row so: gone (`there` false), or with these values and its own
/// id. A versioned row put back gets a version past both its current one
/// and `version`, the one of the state it returns to, so a copy read before
/// is refused.
static bool put_row(ClarityDb* db, const ClarityChange* c, bool there, const ClarityValue* values, int64_t version) {
    const ClarityEntity* entity = c->entity;
    bool now = false;
    ClarityValue* current = NULL;
    int64_t current_version = 0;
    ClaritySourceDb* s = NULL;
    if (!read_row(db, entity, c->id, &now, &current, &current_version) || !clarity_source(db, entity->source, &s)) return false;
    int64_t next = (current_version > version ? current_version : version) + 1;
    ClarityDriver* d = s->driver;
    ClarityDialect dialect = d->dialect;
    const char* table = clarity_quote(dialect, entity->table);
    const char* key = clarity_quote(dialect, entity->key);
    size_t n = entity->column_count;
    ClarityValues params = {0};
    if (!there) {
        if (!now) return true;
        clarity_values_add(&params, clarity_integer_value(c->id));
        return d->execute(d, clarity_format("DELETE FROM %s WHERE %s = %s", table, key, clarity_mark(dialect, 1)), &params, NULL);
    }
    if (now) {
        ClarityBuffer sets = {0};
        for (size_t i = 0; i < n; ++i) {
            clarity_buffer_addf(&sets, "%s%s = %s", i ? ", " : "", clarity_quote(dialect, entity->columns[i]), clarity_mark(dialect, (int)i + 1));
            clarity_values_add(&params, values[i]);
        }
        if (entity->version) {
            clarity_values_add(&params, clarity_integer_value(next));
            clarity_buffer_addf(&sets, "%s%s = %s", n ? ", " : "", clarity_quote(dialect, entity->version), clarity_mark(dialect, (int)params.count));
        }
        if (sets.length == 0) return true;
        clarity_values_add(&params, clarity_integer_value(c->id));
        return d->execute(d, clarity_format("UPDATE %s SET %s WHERE %s = %s", table, clarity_buffer_text(&sets), key, clarity_mark(dialect, (int)params.count)), &params,
                          NULL);
    }
    ClarityBuffer names = {0}, marks = {0};
    clarity_buffer_add(&names, key);
    clarity_buffer_add(&marks, clarity_mark(dialect, 1));
    clarity_values_add(&params, clarity_integer_value(c->id));
    for (size_t i = 0; i < n; ++i) {
        clarity_buffer_addf(&names, ", %s", clarity_quote(dialect, entity->columns[i]));
        clarity_buffer_addf(&marks, ", %s", clarity_mark(dialect, (int)i + 2));
        clarity_values_add(&params, values[i]);
    }
    if (entity->version) {
        clarity_buffer_addf(&names, ", %s", clarity_quote(dialect, entity->version));
        clarity_values_add(&params, clarity_integer_value(next));
        clarity_buffer_addf(&marks, ", %s", clarity_mark(dialect, (int)params.count));
    }
    const char* insert = clarity_format("INSERT INTO %s (%s) VALUES (%s)", table, clarity_buffer_text(&names), clarity_buffer_text(&marks));
    if (dialect != CLARITY_MSSQL) return d->execute(d, insert, &params, NULL);
    // SQL Server takes a key of its own counting only when asked to.
    if (!d->batch(d, clarity_format("SET IDENTITY_INSERT %s ON", table))) return false;
    bool ok = d->execute(d, insert, &params, NULL);
    ClarityError kept = *clarity_error();
    d->batch(d, clarity_format("SET IDENTITY_INSERT %s OFF", table));
    if (!ok) clarity_set_error(&kept);
    return ok;
}

bool clarity_step(ClarityDb* db, bool (*run)(void* context), void* context) {
    // An application without a database has nothing to undo.
    if (!db || db->recording) return run(context);
    db->recording = true;
    bool ok = run(context);
    db->recording = false;
    ClarityChanges changes = db->changes;
    db->changes = (ClarityChanges){0};
    // Kept whole or left whole; the handler's error is the one to report.
    ok = release(db, ok);
    if (!ok || changes.count == 0) {
        free_changes(&changes);
        return ok;
    }
    ClarityDone* steps = realloc(db->steps, (db->step_count + 1) * sizeof *steps);
    if (!steps) {
        free_changes(&changes);
        return true;
    }
    db->steps = steps;
    db->steps[db->step_count++] = (ClarityDone){changes, false, false};
    // The oldest step goes when there are more than are kept.
    if (db->step_count > CLARITY_UNDO_STEPS) {
        ClarityDone* old = &db->steps[db->step_count - 1 - CLARITY_UNDO_STEPS];
        free_changes(&old->changes);
        old->forgotten = true;
    }
    return true;
}

int64_t clarity_last_step(const ClarityDb* db) { return db ? (int64_t)db->step_count : 0; }

static bool same_row(const ClarityChange* a, const ClarityChange* b) {
    return a->id == b->id && strcmp(a->entity->table, b->entity->table) == 0 && strcmp(a->entity->source, b->entity->source) == 0;
}

/// Puts the rows of steps back (`back`) or forward, after checking that each
/// row is still as the steps left it (by its version, or the whole row
/// where there is none), all in one transaction per source, checks
/// included. Each row's new version is noted where the next replay checks it.
static bool replay(ClarityDb* db, ClarityDone** steps, size_t step_count, bool back) {
    size_t n = 0;
    for (size_t s = 0; s < step_count; ++s) n += steps[s]->changes.count;
    ClarityChange** order = CLARITY_NEW(ClarityChange*, n ? n : 1);
    size_t at = 0;
    for (size_t s = 0; s < step_count; ++s) {
        ClarityChanges* changes = &steps[s]->changes;
        for (size_t i = 0; i < changes->count; ++i) order[at++] = &changes->items[back ? changes->count - 1 - i : i];
    }
    const char** sources = CLARITY_NEW(const char*, n ? n : 1);
    size_t source_count = 0;
    for (size_t i = 0; i < n; ++i) {
        bool listed = false;
        for (size_t j = 0; j < source_count && !listed; ++j) listed = strcmp(sources[j], order[i]->entity->source) == 0;
        if (!listed) sources[source_count++] = order[i]->entity->source;
    }
    // Within a transaction of each source, the checks included.
    ClaritySourceDb** begun = CLARITY_NEW(ClaritySourceDb*, source_count ? source_count : 1);
    size_t begun_count = 0;
    bool ok = true;
    for (size_t i = 0; ok && i < source_count; ++i) {
        ok = begin(db, sources[i], &begun[begun_count]);
        if (ok) ++begun_count;
    }
    // Each row as the steps left it (its last change undone, its first redone) must still be so.
    for (size_t i = 0; ok && i < n; ++i) {
        const ClarityChange* c = order[i];
        bool seen = false;
        for (size_t j = 0; j < i && !seen; ++j) seen = same_row(order[j], c);
        if (seen) continue;
        bool there = false;
        ClarityValue* now = NULL;
        int64_t version = 0;
        if (!(ok = read_row(db, c->entity, c->id, &there, &now, &version))) break;
        bool same = there == (back ? c->has : c->had);
        if (same && there && c->entity->version) {
            same = version == (back ? c->after_version : c->before_version);
        } else {
            const ClarityValue* expected = back ? c->after : c->before;
            for (size_t k = 0; same && there && k < c->entity->column_count; ++k) same = same_value(&now[k], &expected[k]);
        }
        if (!same) ok = clarity_refuse(clarity_undo_changed_since());
    }
    for (size_t i = 0; ok && i < n; ++i)
        ok = put_row(db, order[i], back ? order[i]->had : order[i]->has, back ? order[i]->before : order[i]->after,
                     back ? order[i]->before_version : order[i]->after_version);
    // The version each row has now is what the next replay finds: noted on its last change in this order.
    for (size_t i = n; ok && i-- > 0;) {
        ClarityChange* c = order[i];
        if (!c->entity->version) continue;
        bool later = false;
        for (size_t j = i + 1; j < n && !later; ++j) later = same_row(order[j], c);
        if (later) continue;
        bool there = false;
        ClarityValue* now = NULL;
        int64_t version = 0;
        if (!(ok = read_row(db, c->entity, c->id, &there, &now, &version))) break;
        if (back) c->before_version = version;
        else c->after_version = version;
    }
    for (size_t i = begun_count; i-- > 0;) ok = end(begun[i], ok) && ok;
    return ok;
}

bool clarity_replay_steps(ClarityDb* db, const int64_t* ids, size_t count, bool back, bool* done) {
    if (done) *done = false;
    if (!db) return true;
    ClarityDone** taken = CLARITY_NEW(ClarityDone*, count ? count : 1);
    size_t taken_count = 0;
    for (size_t i = 0; i < count; ++i) {
        // A step never made is nothing to replay; one made and no longer kept is refused.
        if (ids[i] <= 0 || (uint64_t)ids[i] > db->step_count) continue;
        if (db->steps[ids[i] - 1].forgotten) return clarity_refuse(clarity_undo_too_old());
        ClarityDone* step = &db->steps[ids[i] - 1];
        if (step->undone != back) taken[taken_count++] = step;
    }
    if (taken_count == 0) return true;
    if (!replay(db, taken, taken_count, back)) return false;
    for (size_t i = 0; i < taken_count; ++i) taken[i]->undone = back;
    if (done) *done = true;
    return true;
}

bool clarity_undo(ClarityDb* db, int64_t id, bool* done) { return clarity_replay_steps(db, &id, 1, true, done); }
bool clarity_redo(ClarityDb* db, int64_t id, bool* done) { return clarity_replay_steps(db, &id, 1, false, done); }

/// Starts the transaction of a save or delete with data events in `source`,
/// or joins the one that is open already: a save that an event of another
/// save makes is part of it.
static bool begin(ClarityDb* db, const char* source, ClaritySourceDb** out) {
    if (!clarity_source(db, source, out)) return false;
    ClaritySourceDb* s = *out;
    if (s->depth++ > 0) return true;
    if (s->driver->batch(s->driver, clarity_begin_sql(s->driver->dialect))) return true;
    --s->depth;
    return false;
}

/// Ends what begin started: the outermost keeps it all, or nothing when `ok`
/// is false; a commit that fails leaves nothing half kept.
static bool end(ClaritySourceDb* s, bool ok) {
    if (--s->depth > 0) return ok;
    ClarityDriver* d = s->driver;
    if (ok) {
        if (d->batch(d, clarity_commit_sql(d->dialect))) return true;
        ClarityError failed = *clarity_error();
        d->batch(d, clarity_rollback_sql(d->dialect));
        clarity_set_error(&failed);
        return false;
    }
    // The error that made it roll back is the one to report.
    ClarityError kept = *clarity_error();
    d->batch(d, clarity_rollback_sql(d->dialect));
    clarity_set_error(&kept);
    return false;
}

static bool has_events(const ClarityEntity* entity) {
    return entity->before_save || entity->after_save || entity->before_delete || entity->after_delete;
}

/// A label as people read it: one written as @key takes the key's value; a plain one is itself.
static const char* label_text(const char* label) { return label[0] == '@' ? clarity_text_of(label + 1) : label; }

static bool empty_value(const ClarityValue* v) {
    return v->kind == CLARITY_NULL || (v->kind == CLARITY_TEXT && (!v->text || !v->text[0]));
}

bool clarity_check_required(const ClarityRequired* required, size_t required_count, const char* const* columns, const ClarityValue* values,
                            size_t column_count) {
    for (size_t r = 0; r < required_count; ++r) {
        for (size_t c = 0; c < column_count; ++c) {
            if (strcmp(columns[c], required[r].column) != 0 || !empty_value(&values[c])) continue;
            return clarity_refuse(clarity_text_with(clarity_key_text("fieldRequired"), "field", clarity_slot_text(label_text(required[r].label))));
        }
    }
    return true;
}

/// Whether the record has a value in each of its entity's required fields; refused when not.
static bool required_kept(const ClarityEntity* entity, const void* record) {
    if (!entity->required_count) return true;
    ClarityValue* values = CLARITY_NEW(ClarityValue, entity->column_count);
    entity->values(record, values);
    return clarity_check_required(entity->required, entity->required_count, entity->columns, values, entity->column_count);
}

static bool save_with(ClarityDb* db, const ClarityEntity* entity, void* record, bool anyway) {
    if (!has_events(entity)) return required_kept(entity, record) && store_record(db, entity, record, anyway);
    int64_t* id = id_of(entity, record);
    int64_t* version = record_version(entity, record);
    int64_t was = *id, was_version = version ? *version : 0;
    ClaritySourceDb* s = NULL;
    if (!begin(db, entity->source, &s)) return false;
    // A required field is checked after before save, which may fill it.
    bool ok = (!entity->before_save || entity->before_save(db, record)) && required_kept(entity, record) && store_record(db, entity, record, anyway) &&
              (!entity->after_save || entity->after_save(db, record));
    if (end(s, ok)) return true;
    // Nothing was kept, so a new record is still new, at the version it was read with.
    *id = was;
    if (version) *version = was_version;
    return false;
}

bool clarity_save(ClarityDb* db, const ClarityEntity* entity, void* record) { return save_with(db, entity, record, false); }
bool clarity_save_anyway(ClarityDb* db, const ClarityEntity* entity, void* record) { return save_with(db, entity, record, true); }
bool clarity_reload(ClarityDb* db, const ClarityEntity* entity, void* record) { return clarity_get(db, entity, *id_of(entity, record), record); }

bool clarity_delete(ClarityDb* db, const ClarityEntity* entity, const void* record) {
    int64_t id = *id_of(entity, (void*)record);
    int64_t* held = record_version(entity, (void*)record);
    int64_t version = held ? *held : 0;
    if (!has_events(entity) && !entity->cascade) return remove_record(db, entity, id, version);
    // The events work on a copy, as the record itself is not changed by deleting it.
    void* copy = clarity_alloc(entity->size);
    memcpy(copy, record, entity->size);
    ClaritySourceDb* s = NULL;
    if (!begin(db, entity->source, &s)) return false;
    // What cascades from it goes first, so no reference is left behind.
    bool ok = (!entity->before_delete || entity->before_delete(db, copy)) && (!entity->cascade || entity->cascade(db, id)) && remove_record(db, entity, id, version) &&
              (!entity->after_delete || entity->after_delete(db, copy));
    return end(s, ok);
}

static bool records(ClarityDb* db, const ClarityEntity* entity, const char* sql, const ClarityValues* params, void** items, size_t* count) {
    ClaritySourceDb* s = NULL;
    ClarityRows rows = {0};
    if (!clarity_source(db, entity->source, &s) || !s->driver->rows(s->driver, sql, params, &rows)) return false;
    char* out = clarity_alloc(entity->size * (rows.count ? rows.count : 1));
    for (size_t i = 0; i < rows.count; ++i) {
        const ClarityRow* row = &rows.items[i];
        void* record = out + i * entity->size;
        if (!entity->from_values(row->values[0].integer, row->values + 1, record)) return false;
        // The version, read last.
        if (entity->version) *record_version(entity, record) = row->values[entity->column_count + 1].integer;
    }
    *items = out;
    *count = rows.count;
    return true;
}

bool clarity_find(ClarityDb* db, const ClarityEntity* entity, const ClarityQuery* query, void** items, size_t* count) {
    ClaritySourceDb* s = NULL;
    if (!clarity_source(db, entity->source, &s)) return false;
    ClarityDialect d = s->driver->dialect;
    ClarityValues params = {0};
    const char* sql = clarity_query_sql(query, d, entity->table, entity->key, select_columns(d, entity), &params);
    return records(db, entity, sql, &params, items, count);
}

bool clarity_get(ClarityDb* db, const ClarityEntity* entity, int64_t id, void* record) {
    ClaritySourceDb* s = NULL;
    if (!clarity_source(db, entity->source, &s)) return false;
    ClarityDialect d = s->driver->dialect;
    ClarityValues params = {0};
    clarity_values_add(&params, clarity_integer_value(id));
    const char* sql = clarity_format("SELECT %s FROM %s WHERE %s = %s", select_columns(d, entity), clarity_quote(d, entity->table),
                                     clarity_quote(d, entity->key), clarity_mark(d, 1));
    void* items;
    size_t count;
    if (!records(db, entity, sql, &params, &items, &count)) return false;
    if (count == 0) return clarity_not_found(entity->table, id);
    memcpy(record, (char*)items + (count - 1) * entity->size, entity->size);
    return true;
}

bool clarity_load(ClarityDb* db, const ClarityEntity* entity, int64_t reference, void* record) {
    if (reference == 0) return clarity_not_found(entity->table, 0);
    return clarity_get(db, entity, reference, record);
}

bool clarity_picture(ClarityDb* db, const ClarityEntity* entity, int64_t id, const char* column, ClarityBytes* out) {
    const ClarityPictureColumn* p = picture_of(entity, column);
    if (!p) return clarity_fail(CLARITY_STORAGE, "%s has no picture '%s'", entity->table, column);
    ClaritySourceDb* s = NULL;
    if (!clarity_source(db, entity->source, &s)) return false;
    ClarityDriver* d = s->driver;
    ClarityValues params = {0};
    clarity_values_add(&params, clarity_integer_value(id));
    const char* sql = clarity_format("SELECT %s FROM %s WHERE %s = %s", clarity_quote(d->dialect, column), clarity_quote(d->dialect, entity->table),
                                     clarity_quote(d->dialect, entity->key), clarity_mark(d->dialect, 1));
    ClarityRows rows = {0};
    if (!d->rows(d, sql, &params, &rows)) return false;
    if (rows.count == 0) return clarity_not_found(entity->table, id);
    ClarityValue value = rows.items[0].values[0];
    *out = (ClarityBytes){NULL, 0};
    if (value.kind == CLARITY_NULL) return true;
    if (!p->store) {
        if (value.kind != CLARITY_BYTES) return clarity_fail(CLARITY_STORAGE, "%s holds no picture", column);
        *out = value.bytes;
        return true;
    }
    const char* path = value.kind == CLARITY_BYTES ? clarity_copy_n((const char*)value.bytes.data, value.bytes.size) : value.text;
    if (value.kind != CLARITY_BYTES && value.kind != CLARITY_TEXT) return clarity_fail(CLARITY_STORAGE, "%s holds no picture", column);
    if (!*path) return true;
    return clarity_store_file(db, p->store, path, out);
}
