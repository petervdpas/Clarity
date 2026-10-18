#include "clarity/disk.h"

#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>

#ifdef _WIN32
#include <direct.h>
#define make_folder(path) _mkdir(path)
#else
#define make_folder(path) mkdir(path, 0777)
#endif

bool clarity_make_folders(const char* folder) {
    if (!folder || !*folder) return true;
    char* path = clarity_copy(folder);
    size_t length = strlen(path);
    for (size_t i = 1; i <= length; ++i) {
        if (path[i] != '/' && path[i] != '\\' && path[i] != '\0') continue;
        char kept = path[i];
        path[i] = '\0';
        if (make_folder(path) != 0 && errno != EEXIST) return clarity_fail(CLARITY_STORAGE, "cannot make the folder %s: %s", path, strerror(errno));
        path[i] = kept;
    }
    return true;
}

const char* clarity_parent(const char* path) {
    const char* slash = strrchr(path, '/');
    const char* back = strrchr(path, '\\');
    if (back > slash) slash = back;
    return slash ? clarity_copy_n(path, (size_t)(slash - path)) : "";
}

bool clarity_exists(const char* path) {
    struct stat info;
    return stat(path, &info) == 0;
}

bool clarity_read_file(const char* path, ClarityBytes* out) {
    FILE* in = fopen(path, "rb");
    if (!in) return clarity_fail(CLARITY_STORAGE, "cannot read %s", path);
    ClarityBuffer bytes = {0};
    char chunk[8192];
    size_t got;
    while ((got = fread(chunk, 1, sizeof chunk, in)) > 0) clarity_buffer_add_n(&bytes, chunk, got);
    bool failed = ferror(in);
    fclose(in);
    if (failed) return clarity_fail(CLARITY_STORAGE, "cannot read %s", path);
    *out = (ClarityBytes){(const uint8_t*)clarity_buffer_text(&bytes), bytes.length};
    return true;
}

bool clarity_write_file(const char* path, ClarityBytes bytes) {
    FILE* file = fopen(path, "wb");
    if (!file) return clarity_fail(CLARITY_STORAGE, "cannot write %s", path);
    bool written = fwrite(bytes.data, 1, bytes.size, file) == bytes.size;
    written = fclose(file) == 0 && written;
    return written || clarity_fail(CLARITY_STORAGE, "cannot write %s", path);
}
