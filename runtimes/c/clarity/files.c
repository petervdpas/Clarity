// The files of a file store, which code reaches through `files f in
// store`, `read text of f` and `write t to store "path"`: only folders the
// project settings name. A file found can become a record's picture: in the
// field's own store by its path, elsewhere as a copy. The other runtimes do
// the same (conformance/files.json).

#include <ctype.h>
#include <dirent.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>

#include "clarity/db_internal.h"
#include "clarity/disk.h"

static bool add(ClarityDb* db, ClarityStore store) {
    ClarityStore* more = realloc(db->stores, (db->store_count + 1) * sizeof *more);
    if (!more) return clarity_fail(CLARITY_STORAGE, "out of memory");
    db->stores = more;
    db->stores[db->store_count++] = store;
    return true;
}

bool clarity_add_store(ClarityDb* db, const char* name, const char* at) {
    const char* folder = NULL;
    if (!clarity_location(at, &folder) || !clarity_make_folders(folder)) return false;
    return add(db, (ClarityStore){clarity_lasting(name), clarity_lasting(folder), NULL, NULL, NULL});
}

bool clarity_add_store_setting(ClarityDb* db, const char* name, const char* program, const char* setting, const char* fallback) {
    const char* folder = NULL;
    return add(db, (ClarityStore){clarity_lasting(name), NULL, clarity_lasting(program), clarity_lasting(setting), clarity_lasting(fallback)}) && clarity_store_folder(db, name, &folder);
}

bool clarity_store_folder(ClarityDb* db, const char* name, const char** out) {
    // The last one added under a name is the one in use.
    for (size_t i = db->store_count; i-- > 0;) {
        const ClarityStore* store = &db->stores[i];
        if (strcmp(store->name, name) != 0) continue;
        if (store->folder) {
            *out = store->folder;
            return true;
        }
        const char* file = clarity_settings_file(store->program);
        if (!file) return clarity_fail(CLARITY_STORAGE, "this system names no folder for application settings");
        cJSON* kept = clarity_read_settings(store->program);
        *out = clarity_store_folder_in(clarity_settings_os(), clarity_parent(file), name, cJSON_GetObjectItemCaseSensitive(kept, store->setting), store->fallback);
        cJSON_Delete(kept);
        return clarity_make_folders(*out);
    }
    return clarity_fail(CLARITY_STORAGE, "there is no file store '%s'", name);
}

static bool read_in_store(ClarityDb* db, const char* store, const char* path, ClarityBytes* out) {
    const char *folder = NULL, *file = NULL;
    return clarity_store_folder(db, store, &folder) && clarity_in_store(folder, path, &file) && clarity_read_file(file, out);
}

typedef struct {
    ClarityFile* items;
    size_t count;
    size_t capacity;
} Found;

static bool walk(const char* store, const char* folder, const char* prefix, Found* out) {
    DIR* dir = opendir(folder);
    if (!dir) return clarity_fail(CLARITY_STORAGE, "cannot read %s", folder);
    struct dirent* entry;
    bool ok = true;
    while (ok && (entry = readdir(dir))) {
        const char* name = entry->d_name;
        if (name[0] == '.') continue;
        const char* full = clarity_format("%s/%s", folder, name);
        struct stat info;
        if (stat(full, &info) != 0) continue;
        if (S_ISDIR(info.st_mode)) {
            ok = walk(store, full, clarity_format("%s%s/", prefix, name), out);
        } else if (S_ISREG(info.st_mode)) {
            const char* dot = strrchr(name, '.');
            char* extension = clarity_copy(dot ? dot + 1 : "");
            for (char* c = extension; *c; ++c) *c = (char)tolower((unsigned char)*c);
            struct tm at;
#ifdef _WIN32
            gmtime_s(&at, &info.st_mtime);
#else
            gmtime_r(&info.st_mtime, &at);
#endif
            ClarityDateTime modified = {at.tm_year + 1900, at.tm_mon + 1, at.tm_mday, at.tm_hour, at.tm_min, at.tm_sec};
            clarity_grow((void**)&out->items, &out->capacity, out->count, sizeof *out->items);
            out->items[out->count++] = (ClarityFile){clarity_copy(store), clarity_format("%s%s", prefix, name), clarity_copy(name), extension,
                                                     (int64_t)info.st_size, modified};
        }
    }
    closedir(dir);
    return ok;
}

static int by_path(const void* a, const void* b) { return strcmp(((const ClarityFile*)a)->path, ((const ClarityFile*)b)->path); }

bool clarity_files(ClarityDb* db, const char* store, ClarityFileList* out) {
    const char* folder = NULL;
    Found found = {0};
    if (!clarity_store_folder(db, store, &folder) || !walk(store, folder, "", &found)) return false;
    if (found.count) qsort(found.items, found.count, sizeof *found.items, by_path);
    *out = (ClarityFileList){found.items, found.count};
    return true;
}

/// Whether text is UTF-8.
static bool utf8(ClarityBytes b) {
    for (size_t i = 0; i < b.size;) {
        uint8_t c = b.data[i];
        size_t n = c < 0x80 ? 0 : (c >> 5) == 6 ? 1 : (c >> 4) == 14 ? 2 : (c >> 3) == 30 ? 3 : 9;
        if (n == 9 || i + n >= b.size) return false;
        for (size_t k = 1; k <= n; ++k)
            if ((b.data[i + k] >> 6) != 2) return false;
        i += n + 1;
    }
    return true;
}

bool clarity_read_text(ClarityDb* db, const ClarityFile* file, const char** out) {
    ClarityBytes bytes = {0};
    if (!read_in_store(db, file->store, file->path, &bytes)) return false;
    if (!utf8(bytes) || memchr(bytes.data, 0, bytes.size)) return clarity_fail(CLARITY_INVALID, "%s is not text", file->path);
    *out = clarity_copy_n((const char*)bytes.data, bytes.size);
    return true;
}

bool clarity_write_text(ClarityDb* db, const char* store, const char* path, const char* text) {
    return clarity_write_bytes(db, store, path, (ClarityBytes){(const uint8_t*)text, strlen(text)});
}

bool clarity_read_bytes(ClarityDb* db, const ClarityFile* file, ClarityBytes* out) { return read_in_store(db, file->store, file->path, out); }

bool clarity_write_bytes(ClarityDb* db, const char* store, const char* path, ClarityBytes bytes) {
    const char *folder = NULL, *file = NULL;
    if (!clarity_store_folder(db, store, &folder) || !clarity_in_store(folder, path, &file)) return false;
    return clarity_make_folders(clarity_parent(file)) && clarity_write_file(file, bytes);
}

bool clarity_picture_from(ClarityDb* db, const ClarityFile* file, const char* field_store, ClarityPicture** out) {
    ClarityBytes bytes = {0};
    ClarityPictureKind kind = {0};
    if (!read_in_store(db, file->store, file->path, &bytes) || !clarity_picture_checked(bytes, &kind)) return false;
    *out = field_store && strcmp(field_store, file->store) == 0 ? clarity_picture_in_store(file->path) : clarity_picture_new(bytes);
    return true;
}

bool clarity_record_file_from(ClarityDb* db, const ClarityFile* file, const char* field_store, ClarityRecordFile** out) {
    if (field_store && strcmp(field_store, file->store) == 0) {
        *out = clarity_record_file(file->name, clarity_picture_in_store(file->path));
        return true;
    }
    ClarityBytes bytes = {0};
    if (!read_in_store(db, file->store, file->path, &bytes)) return false;
    *out = clarity_record_file(file->name, clarity_picture_new(bytes));
    return true;
}

bool clarity_store_file(ClarityDb* db, const char* store, const char* path, ClarityBytes* out) { return read_in_store(db, store, path, out); }

bool clarity_copied_picture(ClarityDb* db, const ClarityEntity* entity, const ClarityPicture* value, int64_t id, const char* column, const char* from, const char* to, ClarityPicture** out) {
    *out = NULL;
    if (!value) return true;
    ClarityBytes bytes = {0};
    switch (value->state) {
    case CLARITY_PICTURE_NEW:
        *out = clarity_picture_new(value->bytes);
        return true;
    case CLARITY_PICTURE_IN_STORE:
        if (!from) return clarity_fail(CLARITY_STORAGE, "%s is kept in the database, not in a file store", column);
        if (to && strcmp(from, to) == 0) {
            *out = clarity_picture_in_store(value->path);
            return true;
        }
        if (!read_in_store(db, from, value->path, &bytes)) return false;
        *out = clarity_picture_new(bytes);
        return true;
    case CLARITY_PICTURE_STORED:
        if (!clarity_picture(db, entity, id, column, &bytes)) return false;
        if (bytes.data) *out = clarity_picture_new(bytes);
        return true;
    }
    return true;
}

bool clarity_copied_file(ClarityDb* db, const ClarityEntity* entity, const ClarityRecordFile* value, int64_t id, const char* column, const char* from, const char* to, ClarityRecordFile** out) {
    *out = NULL;
    if (!value) return true;
    ClarityPicture* content = NULL;
    if (!clarity_copied_picture(db, entity, value->content, id, column, from, to, &content)) return false;
    if (content) *out = clarity_record_file(value->name, content);
    return true;
}
