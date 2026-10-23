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

bool clarity_add_store(ClarityDb* db, const char* name, const char* at) {
    const char* folder = NULL;
    if (!clarity_location(at, &folder) || !clarity_make_folders(folder)) return false;
    ClarityStore* more = realloc(db->stores, (db->store_count + 1) * sizeof *more);
    if (!more) return clarity_fail(CLARITY_STORAGE, "out of memory");
    db->stores = more;
    db->stores[db->store_count++] = (ClarityStore){clarity_lasting(name), clarity_lasting(folder)};
    return true;
}

bool clarity_store_folder(ClarityDb* db, const char* name, const char** out) {
    for (size_t i = 0; i < db->store_count; ++i) {
        if (strcmp(db->stores[i].name, name) == 0) {
            *out = db->stores[i].folder;
            return true;
        }
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
