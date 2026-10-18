#include "clarity/store.h"

#include <stdlib.h>
#include <string.h>

const char* clarity_applied_sql(int version) { return clarity_format("INSERT INTO clarity_schema (version) VALUES (%d)", version); }

bool clarity_location(const char* text, const char** out) {
    if (strncmp(text, "env:", 4) != 0) {
        *out = text;
        return true;
    }
    const char* value = getenv(text + 4);
    if (!value) return clarity_fail(CLARITY_STORAGE, "the environment variable %s is not set", text + 4);
    *out = clarity_copy(value);
    return true;
}

bool clarity_open_driver(ClarityDialect dialect, const char* at, ClarityDriver** out) {
    const char* where = NULL;
    if (!clarity_location(at, &where)) return false;
    if (dialect == CLARITY_SQLITE) return clarity_open_sqlite(where, out);
    return clarity_fail(CLARITY_STORAGE, "this application was built without %s", clarity_dialect_label(dialect));
}
