#include "clarity/store.h"

#include "clarity/settings.h"

#include <stdlib.h>
#include <string.h>

const char* clarity_applied_sql(int version) { return clarity_format("INSERT INTO clarity_schema (version) VALUES (%d)", version); }

bool clarity_location(const char* text, const char** out) {
    // A secret setting (DESIGN 5.27): setting:<program>/<name>, from the system's protected store.
    if (strncmp(text, "setting:", 8) == 0) {
        const char* at = text + 8;
        const char* slash = strchr(at, '/');
        const char* name = slash ? slash + 1 : at;
        const char* value = slash && clarity_secret_store.get ? clarity_secret_store.get(clarity_copy_n(at, (size_t)(slash - at)), name) : NULL;
        if (!value || !*value) return clarity_fail(CLARITY_NOT_KEPT, "the secret setting %s is not kept yet: the settings name the connection", name);
        *out = value;
        return true;
    }
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

ClarityText* clarity_unconnected_reason(const ClarityError* error) {
    if (error->kind == CLARITY_NOT_KEPT) return clarity_key_text("unconnectedNotKept");
    // A storage error says what went wrong after its kind.
    const char* problem = error->message;
    if (error->kind == CLARITY_STORAGE && strncmp(problem, "storage: ", 9) == 0) problem += 9;
    return clarity_text_with(clarity_key_text("unconnectedFailed"), "problem", clarity_slot_text(clarity_copy(problem)));
}
