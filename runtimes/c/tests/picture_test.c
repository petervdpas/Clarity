// Pictures in records (X2), against the cases every runtime shares
// (conformance/pictures.json), and kept in SQLite or in a file store.

#include <stddef.h>
#include <stdlib.h>

#include "check.h"
#include "clarity/code.h"
#include "clarity/disk.h"
#include "tests.h"

static ClarityBytes hex(const char* text) {
    size_t n = strlen(text) / 2;
    uint8_t* out = clarity_alloc(n);
    for (size_t i = 0; i < n; ++i) {
        char pair[3] = {text[i * 2], text[i * 2 + 1], 0};
        out[i] = (uint8_t)strtol(pair, NULL, 16);
    }
    return (ClarityBytes){out, n};
}

static const uint8_t png_bytes[] = {0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
static const uint8_t jpg_bytes[] = {0xFF, 0xD8, 0xFF, 0xE0};
static const ClarityBytes png = {png_bytes, sizeof png_bytes};
static const ClarityBytes jpg = {jpg_bytes, sizeof jpg_bytes};

typedef struct {
    int64_t id;
    const char* title;
    ClarityPicture* cover;
    ClarityPicture* photo;
} Album;

static const char* const album_columns[] = {"title", "cover", "photo"};
static const ClarityPictureColumn album_pictures[] = {{"cover", NULL}, {"photo", "photos"}};

static void album_values(const void* record, ClarityValue* out) {
    const Album* r = record;
    out[0] = clarity_text_value(r->title);
    out[1] = clarity_picture_value(r->cover);
    out[2] = clarity_picture_value(r->photo);
}

static bool album_from_values(int64_t id, const ClarityValue* v, void* record) {
    Album* r = record;
    r->id = id;
    return clarity_get_text("title", v[0], &r->title) && clarity_get_picture("cover", v[1], &r->cover) && clarity_get_picture("photo", v[2], &r->photo);
}

static const ClarityEntity album = {
    .name = "Album", .table = "album", .key = "id", .source = "", .columns = album_columns, .column_count = 3,
    .pictures = album_pictures, .picture_count = 2, .size = sizeof(Album), .id_at = offsetof(Album, id),
    .values = album_values, .from_values = album_from_values,
};

static const char* scratch(const char* name) {
    const char* dir = clarity_format("%s/clarity-pictures-c-%s", getenv("TMPDIR") ? getenv("TMPDIR") : "/tmp", name);
    system(clarity_format("rm -rf '%s'", dir));
    return dir;
}

static bool albums(const char* store, ClarityDb** out) {
    static const char* const create[] = {"CREATE TABLE album (id INTEGER PRIMARY KEY, title TEXT NOT NULL, cover BLOB, photo TEXT)"};
    ClarityStep step = {create, 1};
    return clarity_db_memory(out) && clarity_migrate(*out, "", &step, 1, NULL) && clarity_add_store(*out, "photos", store);
}

static bool same(ClarityBytes a, ClarityBytes b) { return a.size == b.size && a.data && memcmp(a.data, b.data, a.size) == 0; }

void a_picture_is_told_by_its_first_bytes_and_named_by_them(void) {
    cJSON* cases = conformance("pictures.json");
    const cJSON* kind;
    cJSON_ArrayForEach(kind, cJSON_GetObjectItem(cases, "kinds")) {
        ClarityPictureKind found;
        REQUIRE(clarity_picture_kind(hex(cJSON_GetObjectItem(kind, "bytes")->valuestring), &found));
        CHECK_TEXT(found.extension, cJSON_GetObjectItem(kind, "extension")->valuestring);
        CHECK_TEXT(found.media_type, cJSON_GetObjectItem(kind, "type")->valuestring);
    }
    const cJSON* other;
    ClarityPictureKind none;
    cJSON_ArrayForEach(other, cJSON_GetObjectItem(cases, "notPictures")) CHECK(!clarity_picture_kind(hex(other->valuestring), &none));
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "fileNames")) {
        const char* name;
        REQUIRE_OK(clarity_picture_file_name(hex(cJSON_GetObjectItem(c, "bytes")->valuestring), &name));
        CHECK_TEXT(name, cJSON_GetObjectItem(c, "name")->valuestring);
    }
    CHECK_TEXT(clarity_sha256_hex((ClarityBytes){NULL, 0}), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    const char* name;
    CHECK_FAILS(CLARITY_INVALID, clarity_picture_file_name((ClarityBytes){(const uint8_t*)"%PDF", 4}, &name));
    const struct {
        const char* word;
        ClarityDialect dialect;
    } dialects[] = {{"sqlite", CLARITY_SQLITE}, {"postgres", CLARITY_POSTGRES}, {"mysql", CLARITY_MYSQL}, {"mssql", CLARITY_MSSQL}};
    for (size_t i = 0; i < 4; ++i)
        CHECK_TEXT(clarity_presence_sql(dialects[i].dialect, "cover"), cJSON_GetObjectItem(cJSON_GetObjectItem(cases, "presence"), dialects[i].word)->valuestring);
    const char* url;
    CHECK_OK(clarity_data_url(png, &url));
    CHECK_TEXT(url, "data:image/png;base64,iVBORw0KGgo=");
}

void a_picture_travels_to_the_renderer_as_every_runtime_sends_it(void) {
    cJSON* cases = conformance("pictures.json");
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "wire")) {
        const char* kind = cJSON_GetObjectItem(c, "value")->valuestring;
        ClarityPicture* value = NULL;
        if (strcmp(kind, "stored") == 0) value = clarity_picture_stored();
        if (strcmp(kind, "new") == 0) value = clarity_picture_new(hex(cJSON_GetObjectItem(c, "bytes")->valuestring));
        const cJSON* wire = cJSON_GetObjectItem(c, "json");
        CHECK_JSON(clarity_picture_json(value), cJSON_PrintUnformatted(wire));
        CHECK(clarity_picture_same(clarity_picture_from_json(wire), value));
    }
    CHECK_JSON(clarity_picture_json(clarity_picture_in_store("2024/a.png")), "{\"file\":\"2024/a.png\"}");
}

void a_picture_is_kept_in_the_database_and_fetched_on_its_own(void) {
    const char* store = scratch("db");
    ClarityDb* db;
    REQUIRE_OK(albums(store, &db));
    Album a = {0, "Blue Train", clarity_picture_new(png), NULL};
    CHECK_OK(clarity_save(db, &album, &a));
    Album read;
    CHECK_OK(clarity_get(db, &album, a.id, &read));
    CHECK(clarity_picture_same(read.cover, clarity_picture_stored()));
    CHECK(!read.photo);
    ClarityBytes got;
    CHECK_OK(clarity_picture(db, &album, a.id, "cover", &got));
    CHECK(same(got, png));
    // Saving the record again keeps the picture it has.
    read.title = "Blue Train (mono)";
    CHECK_OK(clarity_save(db, &album, &read));
    CHECK_OK(clarity_picture(db, &album, a.id, "cover", &got));
    CHECK(same(got, png));
    // A new picture replaces it; none empties it.
    read.cover = clarity_picture_new(jpg);
    CHECK_OK(clarity_save(db, &album, &read));
    CHECK_OK(clarity_picture(db, &album, a.id, "cover", &got));
    CHECK(same(got, jpg));
    read.cover = NULL;
    CHECK_OK(clarity_save(db, &album, &read));
    CHECK_OK(clarity_picture(db, &album, a.id, "cover", &got));
    CHECK(!got.data);
    // Only a picture is kept.
    read.cover = clarity_picture_new((ClarityBytes){(const uint8_t*)"%PDF", 4});
    CHECK_FAILS(CLARITY_INVALID, clarity_save(db, &album, &read));
    clarity_db_close(db);
    system(clarity_format("rm -rf '%s'", store));
}

void a_picture_in_a_file_store_is_a_file_named_by_its_bytes(void) {
    const char* store = scratch("store");
    ClarityDb* db;
    REQUIRE_OK(albums(store, &db));
    Album a = {0, "Kind of Blue", NULL, clarity_picture_new(jpg)};
    CHECK_OK(clarity_save(db, &album, &a));
    const char* name;
    REQUIRE_OK(clarity_picture_file_name(jpg, &name));
    ClarityBytes written;
    CHECK_OK(clarity_read_file(clarity_format("%s/%s", store, name), &written));
    CHECK(same(written, jpg));
    ClarityBytes got;
    CHECK_OK(clarity_picture(db, &album, a.id, "photo", &got));
    CHECK(same(got, jpg));
    // A path that leaves the store is refused.
    CHECK_OK(clarity_db_batch(db, "", clarity_format("UPDATE album SET photo = '../secret.png' WHERE id = %lld", (long long)a.id)));
    CHECK_FAILS(CLARITY_INVALID, clarity_picture(db, &album, a.id, "photo", &got));
    ClarityDb* other;
    REQUIRE_OK(clarity_db_memory(&other));
    CHECK_FAILS(CLARITY_STORAGE, clarity_add_store(other, "photos", "env:CLARITY_NO_SUCH_STORE"));
    clarity_db_close(other);
    clarity_db_close(db);
    system(clarity_format("rm -rf '%s'", store));
}
