// SQLite through its own C library: one file per data source. Decimals and
// dates are stored as text, bools as 0 and 1. One connection holds the
// transaction of a save with its data events.

#include <stdlib.h>
#include <string.h>

#include <sqlite3.h>

#include "clarity/disk.h"
#include "clarity/store.h"

typedef struct {
    ClarityDriver driver;
    sqlite3* db;
} Sqlite;

static sqlite3* db_of(ClarityDriver* driver) { return ((Sqlite*)driver)->db; }

static bool failed(sqlite3* db) {
    const char* what = sqlite3_errmsg(db);
    if (sqlite3_extended_errcode(db) == SQLITE_CONSTRAINT_FOREIGNKEY) return clarity_fail(CLARITY_REFERENCE, "%s", what);
    return clarity_fail(CLARITY_STORAGE, "%s", what);
}

static bool prepare(sqlite3* db, const char* sql, const ClarityValues* params, sqlite3_stmt** out) {
    sqlite3_stmt* stmt = NULL;
    if (sqlite3_prepare_v2(db, sql, -1, &stmt, NULL) != SQLITE_OK) return failed(db);
    for (size_t i = 0; params && i < params->count; ++i) {
        const ClarityValue* v = &params->items[i];
        int at = (int)i + 1;
        int rc;
        switch (v->kind) {
        case CLARITY_INTEGER: rc = sqlite3_bind_int64(stmt, at, v->integer); break;
        case CLARITY_REAL: rc = sqlite3_bind_double(stmt, at, v->real); break;
        case CLARITY_BOOL: rc = sqlite3_bind_int64(stmt, at, v->flag ? 1 : 0); break;
        case CLARITY_TEXT: rc = sqlite3_bind_text(stmt, at, v->text, -1, SQLITE_TRANSIENT); break;
        case CLARITY_BYTES: rc = sqlite3_bind_blob(stmt, at, v->bytes.data, (int)v->bytes.size, SQLITE_TRANSIENT); break;
        case CLARITY_DECIMAL: rc = sqlite3_bind_text(stmt, at, clarity_decimal_text(v->decimal), -1, SQLITE_TRANSIENT); break;
        case CLARITY_DATE: rc = sqlite3_bind_text(stmt, at, clarity_date_text(v->date), -1, SQLITE_TRANSIENT); break;
        case CLARITY_DATE_TIME: rc = sqlite3_bind_text(stmt, at, clarity_date_time_text(v->date_time), -1, SQLITE_TRANSIENT); break;
        default: rc = sqlite3_bind_null(stmt, at); break;
        }
        if (rc != SQLITE_OK) {
            failed(db);
            sqlite3_finalize(stmt);
            return false;
        }
    }
    *out = stmt;
    return true;
}

static ClarityRow row_of(sqlite3_stmt* stmt) {
    int n = sqlite3_column_count(stmt);
    ClarityRow row = {CLARITY_NEW(ClarityValue, n), (size_t)n};
    for (int i = 0; i < n; ++i) {
        switch (sqlite3_column_type(stmt, i)) {
        case SQLITE_INTEGER: row.values[i] = clarity_integer_value(sqlite3_column_int64(stmt, i)); break;
        case SQLITE_FLOAT: row.values[i] = (ClarityValue){.kind = CLARITY_REAL, .real = sqlite3_column_double(stmt, i)}; break;
        case SQLITE_TEXT:
            row.values[i] = clarity_text_value(clarity_copy_n((const char*)sqlite3_column_text(stmt, i), (size_t)sqlite3_column_bytes(stmt, i)));
            break;
        case SQLITE_BLOB: {
            size_t size = (size_t)sqlite3_column_bytes(stmt, i);
            uint8_t* data = clarity_alloc(size);
            if (size) memcpy(data, sqlite3_column_blob(stmt, i), size);
            row.values[i] = clarity_bytes_value((ClarityBytes){data, size});
            break;
        }
        default: row.values[i] = clarity_null();
        }
    }
    return row;
}

/// Steps through a prepared statement to its end, keeping the rows when `out` is set.
static bool run(sqlite3* db, sqlite3_stmt* stmt, ClarityRows* out) {
    int rc;
    while ((rc = sqlite3_step(stmt)) == SQLITE_ROW) {
        if (!out) continue;
        clarity_grow((void**)&out->items, &out->capacity, out->count, sizeof *out->items);
        out->items[out->count++] = row_of(stmt);
    }
    bool ok = rc == SQLITE_DONE || failed(db);
    sqlite3_finalize(stmt);
    return ok;
}

static bool execute(ClarityDriver* driver, const char* sql, const ClarityValues* params, int64_t* changed) {
    sqlite3* db = db_of(driver);
    sqlite3_stmt* stmt;
    if (!prepare(db, sql, params, &stmt) || !run(db, stmt, NULL)) return false;
    if (changed) *changed = sqlite3_changes64(db);
    return true;
}

static bool insert(ClarityDriver* driver, const char* sql, const ClarityValues* params, int64_t* key) {
    if (!execute(driver, sql, params, NULL)) return false;
    *key = sqlite3_last_insert_rowid(db_of(driver));
    return true;
}

static bool rows(ClarityDriver* driver, const char* sql, const ClarityValues* params, ClarityRows* out) {
    sqlite3* db = db_of(driver);
    sqlite3_stmt* stmt;
    *out = (ClarityRows){0};
    return prepare(db, sql, params, &stmt) && run(db, stmt, out);
}

static bool batch(ClarityDriver* driver, const char* sql) {
    sqlite3* db = db_of(driver);
    return sqlite3_exec(db, sql, NULL, NULL, NULL) == SQLITE_OK || failed(db);
}

// A step runs with references unchecked, since it may build a table again
// that others refer to; they are checked before the step is kept.
static bool steps(ClarityDriver* driver, int version, const ClarityStep* step) {
    if (!batch(driver, "BEGIN")) return false;
    for (size_t i = 0; i < step->count; ++i)
        if (!batch(driver, step->statements[i])) return false;
    ClarityRows broken = {0};
    if (!rows(driver, "SELECT 1 FROM pragma_foreign_key_check", NULL, &broken)) return false;
    if (broken.count) return clarity_fail(CLARITY_STORAGE, "migration step leaves references to records that do not exist");
    return batch(driver, clarity_applied_sql(version)) && batch(driver, "COMMIT");
}

static bool apply(ClarityDriver* driver, int version, const ClarityStep* step) {
    sqlite3* db = db_of(driver);
    if (!batch(driver, "PRAGMA foreign_keys = OFF")) return false;
    if (!steps(driver, version, step)) {
        // Straight to SQLite, so the error that stopped the step stays the one reported.
        sqlite3_exec(db, "ROLLBACK", NULL, NULL, NULL);
        sqlite3_exec(db, "PRAGMA foreign_keys = ON", NULL, NULL, NULL);
        return false;
    }
    return batch(driver, "PRAGMA foreign_keys = ON");
}

static void close_driver(ClarityDriver* driver) {
    sqlite3_close(db_of(driver));
    free(driver);
}

bool clarity_open_sqlite(const char* path, ClarityDriver** out) {
    if (strcmp(path, ":memory:") != 0 && !clarity_make_folders(clarity_parent(path))) return false;
    sqlite3* db = NULL;
    if (sqlite3_open(path, &db) != SQLITE_OK) {
        bool result = clarity_fail(CLARITY_STORAGE, "cannot open %s: %s", path, db ? sqlite3_errmsg(db) : "out of memory");
        sqlite3_close(db);
        return result;
    }
    sqlite3_extended_result_codes(db, 1);
    Sqlite* s = calloc(1, sizeof *s);
    if (!s) {
        sqlite3_close(db);
        return clarity_fail(CLARITY_STORAGE, "out of memory");
    }
    s->driver = (ClarityDriver){CLARITY_SQLITE, execute, insert, rows, batch, apply, close_driver};
    s->db = db;
    if (!batch(&s->driver, "PRAGMA foreign_keys = ON")) {
        close_driver(&s->driver);
        return false;
    }
    *out = &s->driver;
    return true;
}
