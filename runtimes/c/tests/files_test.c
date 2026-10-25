// The files of a file store (X2), against the cases every runtime shares
// (conformance/files.json), and filtered and sorted as generated code does.

#include <stdlib.h>
#include <time.h>
#ifdef _WIN32
#include <sys/utime.h>
#else
#include <sys/time.h>
#endif

#include "check.h"
#include "clarity/code.h"
#include "clarity/disk.h"
#include "tests.h"

static const char* planted(void) {
    const char* root = clarity_format("%s/clarity-files-c", temp_folder());
    remove_tree(root);
    cJSON* cases = conformance("files.json");
    const cJSON* f;
    cJSON_ArrayForEach(f, cJSON_GetObjectItem(cases, "tree")) {
        const char* path = clarity_format("%s/%s", root, cJSON_GetObjectItem(f, "path")->valuestring);
        const char* text = cJSON_GetObjectItem(f, "text")->valuestring;
        clarity_make_folders(clarity_parent(path));
        clarity_write_file(path, (ClarityBytes){(const uint8_t*)text, strlen(text)});
        ClarityDateTime at;
        clarity_date_time_parse(cJSON_GetObjectItem(f, "modified")->valuestring, &at);
        struct tm when = {.tm_year = at.year - 1900, .tm_mon = at.month - 1, .tm_mday = at.day, .tm_hour = at.hour, .tm_min = at.minute, .tm_sec = at.second};
#ifdef _WIN32
        struct _utimbuf times = {_mkgmtime(&when), _mkgmtime(&when)};
        _utime(path, &times);
#else
        time_t seconds = timegm(&when);
        struct timeval times[2] = {{seconds, 0}, {seconds, 0}};
        utimes(path, times);
#endif
    }
    return root;
}

static const ClarityFile* named(const ClarityFileList* files, const char* name) {
    for (size_t i = 0; i < files->count; ++i)
        if (strcmp(files->items[i].name, name) == 0) return &files->items[i];
    return NULL;
}

void a_store_lists_its_files_as_every_runtime_does(void) {
    const char* root = planted();
    cJSON* cases = conformance("files.json");
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_add_store(db, "photos", root));
    ClarityFileList files;
    REQUIRE_OK(clarity_files(db, "photos", &files));
    cJSON* listed = cJSON_CreateArray();
    for (size_t i = 0; i < files.count; ++i) {
        const ClarityFile* f = &files.items[i];
        cJSON* o = cJSON_CreateObject();
        cJSON_AddStringToObject(o, "path", f->path);
        cJSON_AddStringToObject(o, "name", f->name);
        cJSON_AddStringToObject(o, "extension", f->extension);
        cJSON_AddNumberToObject(o, "size", (double)f->size);
        cJSON_AddStringToObject(o, "modified", clarity_date_time_text(f->modified));
        cJSON_AddItemToArray(listed, o);
    }
    CHECK_JSON(listed, cJSON_PrintUnformatted(cJSON_GetObjectItem(cases, "listing")));
    ClarityFileList none;
    CHECK_FAILS(CLARITY_STORAGE, clarity_files(db, "films", &none));

    // Text is read and written inside the store.
    const ClarityFile* notes = named(&files, "notes.txt");
    REQUIRE(notes);
    const char* text;
    CHECK_OK(clarity_read_text(db, notes, &text));
    CHECK_TEXT(text, "Hello\nworld\n");
    CHECK_OK(clarity_write_text(db, "photos", "exports/titles.txt", "Blue Train\n"));
    ClarityBytes written;
    CHECK_OK(clarity_read_file(clarity_format("%s/exports/titles.txt", root), &written));
    CHECK_TEXT(clarity_copy_n((const char*)written.data, written.size), "Blue Train\n");
    const cJSON* path;
    cJSON_ArrayForEach(path, cJSON_GetObjectItem(cases, "refused")) CHECK_FAILS(CLARITY_INVALID, clarity_write_text(db, "photos", path->valuestring, "x"));

    // A file becomes a picture: linked in its own store, copied elsewhere.
    static const uint8_t png_bytes[] = {0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
    ClarityBytes png = {png_bytes, sizeof png_bytes};
    clarity_make_folders(clarity_format("%s/covers", root));
    clarity_write_file(clarity_format("%s/covers/cover.png", root), png);
    REQUIRE_OK(clarity_files(db, "photos", &files));
    const ClarityFile* cover = named(&files, "cover.png");
    REQUIRE(cover);
    ClarityPicture* picture;
    CHECK_OK(clarity_picture_from(db, cover, "photos", &picture));
    CHECK(clarity_picture_same(picture, clarity_picture_in_store("covers/cover.png")));
    CHECK_OK(clarity_picture_from(db, cover, NULL, &picture));
    CHECK(clarity_picture_same(picture, clarity_picture_new(png)));
    CHECK_FAILS(CLARITY_INVALID, clarity_picture_from(db, named(&files, "notes.txt"), NULL, &picture));
    ClarityBytes bytes;
    CHECK_OK(clarity_store_file(db, "photos", "covers/cover.png", &bytes));
    CHECK(bytes.size == png.size && memcmp(bytes.data, png.data, png.size) == 0);
    CHECK_FAILS(CLARITY_INVALID, clarity_store_file(db, "photos", "../x.png", &bytes));
    clarity_db_close(db);
    remove_tree(root);
}

static const char* names(const ClarityFileList* files) {
    ClarityBuffer out = {0};
    for (size_t i = 0; i < files->count; ++i) clarity_buffer_addf(&out, "%s%s", i ? " " : "", files->items[i].name);
    return clarity_buffer_text(&out);
}

void files_are_filtered_and_sorted_by_their_details(void) {
    ClarityDateTime day1, day2, day3;
    clarity_date_time_parse("2026-01-01T00:00:00", &day1);
    clarity_date_time_parse("2026-01-02T00:00:00", &day2);
    clarity_date_time_parse("2026-01-03T00:00:00", &day3);
    ClarityFile items[] = {{"s", "x/b.jpg", "b.jpg", "jpg", 2, day3}, {"s", "a.png", "a.png", "png", 9, day1}, {"s", "c.jpg", "c.jpg", "jpg", 2, day2}};
    ClarityFileList files = {items, 3};
    // A filter keeps what holds, as generated code writes it.
    ClarityFileList kept = clarity_file_list(files.count);
    for (size_t i = 0; i < files.count; ++i)
        if (clarity_same_text(files.items[i].extension, "jpg")) kept.items[kept.count++] = files.items[i];
    CHECK_INT(kept.count, 2);
    clarity_sort_files(&files, 2, (ClarityFileOrder[]){clarity_desc("size"), clarity_asc("path")});
    CHECK_TEXT(names(&files), "a.png c.jpg b.jpg");
    clarity_sort_files(&files, 1, (ClarityFileOrder[]){clarity_asc("modified")});
    CHECK_TEXT(names(&files), "a.png c.jpg b.jpg");
    clarity_sort_files(&files, 1, (ClarityFileOrder[]){clarity_desc("name")});
    CHECK_TEXT(names(&files), "c.jpg b.jpg a.png");
}

void bytes_are_read_and_written_as_they_are(void) {
    const char* root = planted();
    cJSON* cases = conformance("files.json");
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_add_store(db, "photos", root));
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "bytes")) {
        const char* path = cJSON_GetObjectItem(c, "path")->valuestring;
        ClarityBytes bytes = clarity_from_base64(cJSON_GetObjectItem(c, "base64")->valuestring);
        CHECK_OK(clarity_write_bytes(db, "photos", path, bytes));
        ClarityFileList files;
        REQUIRE_OK(clarity_files(db, "photos", &files));
        const ClarityFile* file = NULL;
        for (size_t i = 0; i < files.count; ++i)
            if (strcmp(files.items[i].path, path) == 0) file = &files.items[i];
        REQUIRE(file);
        CHECK(file->size == (int64_t)cJSON_GetObjectItem(c, "size")->valuedouble);
        ClarityBytes read;
        CHECK_OK(clarity_read_bytes(db, file, &read));
        CHECK(read.size == bytes.size && memcmp(read.data, bytes.data, bytes.size) == 0);
        // Bytes cross to an engine as base64 text, and back.
        ClarityBytes back;
        cJSON* box = clarity_json_object(1, "value", clarity_json_bytes(bytes));
        CHECK_OK(clarity_json_get_bytes(box, "value", &back));
        CHECK(back.size == bytes.size && memcmp(back.data, bytes.data, bytes.size) == 0);
    }
    clarity_db_close(db);
}
