#include "clarity/record_file.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "clarity/disk.h"
#include "clarity/json.h"
#include "clarity/sql.h"

ClarityRecordFile* clarity_record_file(const char* name, ClarityPicture* content) {
    ClarityRecordFile* file = CLARITY_NEW(ClarityRecordFile, 1);
    file->name = clarity_clean_name(name);
    file->content = content;
    return file;
}

/// The last part of a path, after any `/` or `\`.
static const char* last_part(const char* path) {
    const char* last = path;
    for (const char* c = path; *c; ++c)
        if (*c == '/' || *c == '\\') last = c + 1;
    return last;
}

const char* clarity_clean_name(const char* chosen) {
    const char* last = last_part(chosen ? chosen : "");
    while (isspace((unsigned char)*last)) ++last;
    size_t length = strlen(last);
    while (length > 0 && isspace((unsigned char)last[length - 1])) --length;
    return length == 0 ? "file" : clarity_copy_n(last, length);
}

const char* clarity_store_path(ClarityBytes bytes, const char* name) {
    return clarity_format("%.32s/%s", clarity_sha256_hex(bytes), clarity_clean_name(name));
}

const char* clarity_name_of(const char* path) { return last_part(path); }

const char* clarity_extension_of(const char* name) {
    const char* dot = strrchr(name, '.');
    if (!dot || dot == name) return "";
    char* extension = clarity_copy(dot + 1);
    for (char* c = extension; *c; ++c) *c = (char)tolower((unsigned char)*c);
    return extension;
}

const char* clarity_record_file_name(const ClarityRecordFile* file) { return file ? file->name : ""; }

const char* clarity_record_file_extension(const ClarityRecordFile* file) { return clarity_extension_of(clarity_record_file_name(file)); }

const char* clarity_downloads_in(const char* os, ClarityEnv env, void* context) {
    const char* home = env(strcmp(os, "windows") == 0 ? "USERPROFILE" : "HOME", context);
    return home && *home ? clarity_format("%s/Downloads", home) : NULL;
}

const char* clarity_free_name(const char* name, bool (*taken)(const char* name, void* context), void* context) {
    if (!taken(name, context)) return name;
    // A leading dot starts a name, not an extension.
    const char* dot = strrchr(name, '.');
    size_t stem = dot && dot != name ? (size_t)(dot - name) : strlen(name);
    const char* extension = dot && dot != name ? dot : "";
    for (int n = 2;; ++n) {
        const char* candidate = clarity_format("%.*s (%d)%s", (int)stem, name, n, extension);
        if (!taken(candidate, context)) return candidate;
    }
}

static const char* from_system(const char* name, void* context) {
    (void)context;
    return getenv(name);
}

static bool in_folder(const char* name, void* folder) { return clarity_exists(clarity_format("%s/%s", (const char*)folder, name)); }

bool clarity_save_copy(const char* name, ClarityBytes bytes, const char** out) {
#if defined(_WIN32)
    const char* os = "windows";
#elif defined(__APPLE__)
    const char* os = "macos";
#else
    const char* os = "linux";
#endif
    const char* folder = clarity_downloads_in(os, from_system, NULL);
    if (!folder) return clarity_fail(CLARITY_INVALID, "this system names no Downloads folder");
    if (!clarity_make_folders(folder)) return false;
    const char* free = clarity_free_name(clarity_clean_name(name), in_folder, (void*)folder);
    const char* file = clarity_format("%s/%s", folder, free);
    if (!clarity_write_file(file, bytes)) return false;
    *out = file;
    return true;
}

bool clarity_temporary_copy(const char* name, ClarityBytes bytes, const char** out) {
    const char* temp = getenv("TMPDIR");
    if (!temp || !*temp) temp = getenv("TEMP");
    if (!temp || !*temp) temp = "/tmp";
    const char* folder = clarity_format("%s/clarity-open-%.32s", temp, clarity_sha256_hex(bytes));
    if (!clarity_make_folders(folder)) return false;
    const char* file = clarity_format("%s/%s", folder, clarity_clean_name(name));
    if (!clarity_write_file(file, bytes)) return false;
    *out = file;
    return true;
}

ClarityValue clarity_file_content_value(const ClarityRecordFile* file) { return clarity_picture_value(file ? file->content : NULL); }

ClarityValue clarity_file_name_value(const ClarityRecordFile* file) { return clarity_text_value(file ? file->name : ""); }

bool clarity_get_file(const char* column, ClarityValue content, ClarityValue name, ClarityRecordFile** out) {
    ClarityPicture* picture = NULL;
    if (!clarity_get_picture(column, content, &picture)) return false;
    *out = picture ? clarity_record_file(name.kind == CLARITY_TEXT ? name.text : "", picture) : NULL;
    return true;
}

cJSON* clarity_file_json(const ClarityRecordFile* file) {
    if (!file) return cJSON_CreateNull();
    cJSON* out = clarity_picture_json(file->content);
    cJSON_AddStringToObject(out, "name", file->name);
    return out;
}

bool clarity_json_get_file(const cJSON* in, const char* key, ClarityRecordFile** out) {
    // Missing or null keeps the default, as for a picture.
    const cJSON* v = clarity_json_given(in, key);
    if (!v) return true;
    if (!cJSON_IsObject(v)) return clarity_fail(CLARITY_INVALID, "%s is no file", key);
    const cJSON* name = cJSON_GetObjectItemCaseSensitive(v, "name");
    *out = clarity_record_file(cJSON_IsString(name) ? name->valuestring : "", clarity_picture_from_json(v));
    return true;
}
