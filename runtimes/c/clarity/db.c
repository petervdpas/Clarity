#include "clarity/db.h"

#include <stdlib.h>
#include <string.h>

#include "clarity/db_internal.h"
#include "clarity/disk.h"
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

void clarity_db_close(ClarityDb* db) {
    if (!db) return;
    for (size_t i = 0; i < db->count; ++i) {
        db->sources[i].driver->close(db->sources[i].driver);
        free(db->sources[i].name);
    }
    for (size_t i = 0; i < db->store_count; ++i) {
        free(db->stores[i].name);
        free(db->stores[i].folder);
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
    return clarity_buffer_text(&out);
}

static int64_t* id_of(const ClarityEntity* entity, void* record) { return (int64_t*)((char*)record + entity->id_at); }

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

/// Writes a record: an INSERT for a new one, an UPDATE for a stored one.
static bool store_record(ClarityDb* db, const ClarityEntity* entity, void* record) {
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
    ClarityValues params = {0};
    if (*id == 0) {
        // An empty or kept picture is left to the column's NULL, so no NULL
        // is bound to a binary column.
        const char** names = CLARITY_NEW(const char*, n);
        size_t count = 0;
        for (size_t i = 0; i < n; ++i) {
            bool none = values[i].kind == CLARITY_NULL || values[i].kind == CLARITY_KEPT;
            if (picture_of(entity, entity->columns[i]) && none) continue;
            names[count++] = entity->columns[i];
            clarity_values_add(&params, values[i]);
        }
        const char* sql = clarity_insert_sql(dialect, entity->table, entity->key, names, count);
        return d->insert(d, sql, &params, id) || unfit();
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
    bool there;
    if (sets.length == 0) {
        if (!exists(d, entity, *id, &there)) return false;
        return there || clarity_not_found(entity->table, *id);
    }
    clarity_values_add(&params, clarity_integer_value(*id));
    const char* sql = clarity_format("UPDATE %s SET %s WHERE %s = %s", clarity_quote(dialect, entity->table), clarity_buffer_text(&sets),
                                     clarity_quote(dialect, entity->key), clarity_mark(dialect, (int)params.count));
    int64_t changed;
    if (!d->execute(d, sql, &params, &changed)) return unfit();
    // MySQL counts only the rows whose values changed, so an update that
    // changes nothing is told apart from a missing record.
    if (changed == 0) {
        if (!exists(d, entity, *id, &there)) return false;
        if (!there) return clarity_not_found(entity->table, *id);
    }
    return true;
}

static bool remove_record(ClarityDb* db, const ClarityEntity* entity, int64_t id) {
    ClaritySourceDb* s = NULL;
    if (!clarity_source(db, entity->source, &s)) return false;
    ClarityDriver* d = s->driver;
    ClarityValues params = {0};
    clarity_values_add(&params, clarity_integer_value(id));
    const char* sql = clarity_format("DELETE FROM %s WHERE %s = %s", clarity_quote(d->dialect, entity->table), clarity_quote(d->dialect, entity->key),
                                     clarity_mark(d->dialect, 1));
    if (d->execute(d, sql, &params, NULL)) return true;
    return clarity_error()->kind == CLARITY_REFERENCE ? clarity_in_use(entity->table, id) : false;
}

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

/// Ends what begin started: the outermost keeps it all, or nothing when `ok` is false.
static bool end(ClaritySourceDb* s, bool ok) {
    if (--s->depth > 0) return ok;
    ClarityDriver* d = s->driver;
    if (ok) return d->batch(d, clarity_commit_sql(d->dialect));
    // The error that made it roll back is the one to report.
    ClarityError kept = *clarity_error();
    d->batch(d, clarity_rollback_sql(d->dialect));
    clarity_set_error(&kept);
    return false;
}

static bool has_events(const ClarityEntity* entity) {
    return entity->before_save || entity->after_save || entity->before_delete || entity->after_delete;
}

bool clarity_save(ClarityDb* db, const ClarityEntity* entity, void* record) {
    if (!has_events(entity)) return store_record(db, entity, record);
    int64_t* id = id_of(entity, record);
    int64_t was = *id;
    ClaritySourceDb* s = NULL;
    if (!begin(db, entity->source, &s)) return false;
    bool ok = (!entity->before_save || entity->before_save(db, record)) && store_record(db, entity, record) &&
              (!entity->after_save || entity->after_save(db, record));
    if (end(s, ok)) return true;
    // Nothing was kept, so a new record is still new.
    *id = was;
    return false;
}

bool clarity_delete(ClarityDb* db, const ClarityEntity* entity, const void* record) {
    int64_t id = *id_of(entity, (void*)record);
    if (!has_events(entity)) return remove_record(db, entity, id);
    // The events work on a copy, as the record itself is not changed by deleting it.
    void* copy = clarity_alloc(entity->size);
    memcpy(copy, record, entity->size);
    ClaritySourceDb* s = NULL;
    if (!begin(db, entity->source, &s)) return false;
    bool ok = (!entity->before_delete || entity->before_delete(db, copy)) && remove_record(db, entity, id) &&
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
        if (!entity->from_values(row->values[0].integer, row->values + 1, out + i * entity->size)) return false;
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
