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

static const char* process_env(const char* name) { return getenv(name); }

/* The system this runtime was built for, as conformance/data-folder.json names it. */
static const char* this_system(void) {
#if defined(_WIN32)
    return "windows";
#elif defined(__APPLE__)
    return "macos";
#else
    return "linux";
#endif
}

const char* clarity_data_folder_sqlite(const char* location, const char* (*env)(const char* name)) {
    return clarity_data_folder_sqlite_on(this_system(), location, env);
}

const char* clarity_data_folder_sqlite_on(const char* os, const char* location, const char* (*env)(const char* name)) {
    if (strncmp(location, "env:", 4) == 0 || strncmp(location, "setting:", 8) == 0 || strcmp(location, ":memory:") == 0) return location;
    const char* (*get)(const char*) = env ? env : process_env;
    // A database the project keeps beside it: data:<program>/<file>.
    const char* kept = strncmp(location, "data:", 5) == 0 ? location + 5 : NULL;
    const char* name = kept ? kept : location;
    for (const char* c = name; *c; ++c)
        if (*c == '/' || *c == '\\') name = c + 1;
    const char* folder = get(CLARITY_DATA_VARIABLE);
    if (folder && *folder && *name && strcmp(name, ".") != 0 && strcmp(name, "..") != 0) {
        size_t length = strlen(folder);
        bool slashed = folder[length - 1] == '/' || folder[length - 1] == '\\';
        return clarity_format("%s%s%s", folder, slashed ? "" : "/", name);
    }
    if (!kept) return location;
    /* The person's folder for application data, under the program's name. */
    const char* home = get("HOME");
    const char* data = NULL;
    if (strcmp(os, "windows") == 0) {
        data = get("APPDATA");
    } else if (strcmp(os, "macos") == 0) {
        if (home && *home) data = clarity_format("%s/Library/Application Support", home);
    } else {
        data = get("XDG_DATA_HOME");
        if ((!data || !*data) && home && *home) data = clarity_format("%s/.local/share", home);
    }
    if (!data || !*data) return clarity_copy(name);
    return clarity_format("%s/%s", data, kept);
}

bool clarity_open_driver(ClarityDialect dialect, const char* at, ClarityDriver** out) {
    // An SQLite file opens in the folder CLARITY_DATA names, when it is set.
    if (dialect == CLARITY_SQLITE) at = clarity_data_folder_sqlite(at, NULL);
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
