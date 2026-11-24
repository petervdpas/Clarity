#include "clarity/store.h"

#include "clarity/settings.h"

#include <stdlib.h>
#include <string.h>

bool clarity_location(const char* text, const char** out) {
    // A secret setting (manual/data.settings.md): setting:<program>/<name>, from the system's protected store.
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
    if (!value || !*value) return clarity_not_set(text + 4);
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

const char* clarity_data_folder(const char* program, const char* (*env)(const char* name)) {
    return clarity_data_folder_on(this_system(), program, env);
}

const char* clarity_data_folder_on(const char* os, const char* program, const char* (*env)(const char* name)) {
    const char* (*get)(const char*) = env ? env : process_env;
    const char* moved = get(CLARITY_DATA_VARIABLE);
    if (moved && *moved) return clarity_copy(moved);
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
    if (!data || !*data) return "";
    return clarity_format("%s/%s", data, program);
}

const char* clarity_in_folder(const char* folder, const char* name) { return folder && *folder ? clarity_format("%s/%s", folder, name) : clarity_copy(name); }

bool clarity_data_folder_store(const char* location, const char* (*env)(const char* name), const char** out) {
    return clarity_data_folder_store_on(this_system(), location, env, out);
}

bool clarity_data_folder_store_on(const char* os, const char* location, const char* (*env)(const char* name), const char** out) {
    const char* (*get)(const char*) = env ? env : process_env;
    if (strncmp(location, "env:", 4) == 0) {
        const char* value = get(location + 4);
        if (!value || !*value) return clarity_not_set(location + 4);
        *out = clarity_copy(value);
        return true;
    }
    if (strncmp(location, "data:", 5) != 0) {
        *out = location;
        return true;
    }
    const char* kept = location + 5;
    const char* slash = strchr(kept, '/');
    const char* program = slash ? clarity_copy_n(kept, (size_t)(slash - kept)) : "";
    *out = clarity_in_folder(clarity_data_folder_on(os, program, env), slash ? slash + 1 : kept);
    return true;
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
    // A database at an unset environment variable says which (conformance/data-folder.json).
    if (error->kind == CLARITY_NOT_SET) return (ClarityText*)error->text;
    // A storage error says what went wrong after its kind.
    const char* problem = error->message;
    if (error->kind == CLARITY_STORAGE && strncmp(problem, "storage: ", 9) == 0) problem += 9;
    return clarity_text_with(clarity_key_text("unconnectedFailed"), "problem", clarity_slot_text(clarity_copy(problem)));
}
