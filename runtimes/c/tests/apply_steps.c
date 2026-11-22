// A test's database set up as clarity-migrate leaves it (apply_steps.h).

#include "apply_steps.h"

#include "clarity/core.h"
#include "clarity/dialect.h"

bool apply_steps(ClarityDb* db, const char* source, const ClarityStep* steps, size_t count, int* applied) {
    ClarityDialect dialect;
    ClarityRows rows = {0};
    if (!clarity_db_dialect(db, source, &dialect) || !clarity_db_batch(db, source, clarity_version_table_sql(dialect)) ||
        !clarity_db_rows(db, source, "SELECT MAX(version) FROM clarity_schema", NULL, &rows))
        return false;
    int done = rows.count && rows.items[0].values[0].kind == CLARITY_INTEGER ? (int)rows.items[0].values[0].integer : 0;
    bool sqlite = dialect == CLARITY_SQLITE;
    if (sqlite && !clarity_db_batch(db, source, "PRAGMA foreign_keys = OFF")) return false;
    bool ok = true;
    for (int i = done; ok && i < (int)count; ++i) {
        for (size_t n = 0; ok && n < steps[i].count; ++n) ok = clarity_db_batch(db, source, steps[i].statements[n]);
        if (ok) ok = clarity_db_batch(db, source, clarity_format("INSERT INTO clarity_schema (version) VALUES (%d)", i + 1));
    }
    if (sqlite && !clarity_db_batch(db, source, "PRAGMA foreign_keys = ON")) return false;
    if (ok && applied) *applied = (int)count > done ? (int)count - done : 0;
    return ok;
}
