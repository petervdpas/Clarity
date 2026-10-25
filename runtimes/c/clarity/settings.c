#include "clarity/settings.h"

#include "clarity/disk.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/// A variable that is set and not empty, or NULL.
static const char* set(ClarityEnv env, void* context, const char* name) {
    const char* value = env(name, context);
    return value && *value ? value : NULL;
}

const char* clarity_settings_file_in(const char* os, ClarityEnv env, void* context, const char* program) {
    const char* folder = NULL;
    if (strcmp(os, "windows") == 0) {
        folder = set(env, context, "APPDATA");
    } else if (strcmp(os, "macos") == 0) {
        const char* home = set(env, context, "HOME");
        if (home) folder = clarity_format("%s/Library/Application Support", home);
    } else {
        folder = set(env, context, "XDG_CONFIG_HOME");
        const char* home = set(env, context, "HOME");
        if (!folder && home) folder = clarity_format("%s/.config", home);
    }
    return folder ? clarity_format("%s/%s/settings.json", folder, program) : NULL;
}

static const char* from_system(const char* name, void* context) {
    (void)context;
    return getenv(name);
}

const char* clarity_settings_file(const char* program) {
#if defined(_WIN32)
    const char* os = "windows";
#elif defined(__APPLE__)
    const char* os = "macos";
#else
    const char* os = "linux";
#endif
    return clarity_settings_file_in(os, from_system, NULL, program);
}

cJSON* clarity_read_settings(const char* program) {
    const char* file = clarity_settings_file(program);
    ClarityBytes bytes = {0};
    cJSON* kept = file && clarity_exists(file) && clarity_read_file(file, &bytes) ? cJSON_ParseWithLength((const char*)bytes.data, bytes.size) : NULL;
    if (cJSON_IsObject(kept)) return kept;
    cJSON_Delete(kept);
    return cJSON_CreateObject();
}

bool clarity_keep_settings(const char* program, const cJSON* values) {
    const char* file = clarity_settings_file(program);
    if (!file) return clarity_fail(CLARITY_INVALID, "this system names no folder for application settings");
    if (!clarity_make_folders(clarity_parent(file))) return false;
    char* text = cJSON_Print(values);
    if (!text) return clarity_fail(CLARITY_INVALID, "the settings cannot be written");
    const char* written = clarity_format("%s.new", file);
    bool ok = clarity_write_file(written, (ClarityBytes){(const uint8_t*)text, strlen(text)});
    cJSON_free(text);
    if (!ok) return false;
#ifdef _WIN32
    remove(file);
#endif
    return rename(written, file) == 0 || clarity_fail(CLARITY_STORAGE, "cannot keep the settings in %s", file);
}

ClaritySecretStore clarity_secret_store = {0};

cJSON* clarity_read_settings_with(const char* program, const cJSON* secrets) {
    cJSON* values = clarity_read_settings(program);
    const cJSON* name;
    cJSON_ArrayForEach(name, secrets) {
        if (!cJSON_IsString(name)) continue;
        cJSON_DeleteItemFromObjectCaseSensitive(values, name->valuestring);
        const char* secret = clarity_secret_store.get ? clarity_secret_store.get(program, name->valuestring) : NULL;
        if (secret) cJSON_AddStringToObject(values, name->valuestring, secret);
    }
    return values;
}

bool clarity_keep_settings_with(const char* program, const cJSON* values, const cJSON* secrets) {
    cJSON* plain = cJSON_Duplicate(values, 1);
    const cJSON* name;
    cJSON_ArrayForEach(name, secrets) {
        if (!cJSON_IsString(name)) continue;
        const cJSON* value = cJSON_GetObjectItemCaseSensitive(plain, name->valuestring);
        if (!value) continue;
        const char* text = cJSON_IsString(value) ? value->valuestring : "";
        if (clarity_secret_store.set && !clarity_secret_store.set(program, name->valuestring, text)) {
            cJSON_Delete(plain);
            return false;
        }
        cJSON_DeleteItemFromObjectCaseSensitive(plain, name->valuestring);
    }
    bool kept = clarity_keep_settings(program, plain);
    cJSON_Delete(plain);
    return kept;
}
