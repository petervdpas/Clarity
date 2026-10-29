// Files in records (U1c), against the cases every runtime shares
// (conformance/record-files.json), and kept in SQLite or in a file store.

#include <stddef.h>
#include <stdlib.h>

#include "check.h"
#include "clarity/code.h"
#include "clarity/disk.h"
#include "clarity/record_file.h"
#include "tests.h"

static const char* from_case(const char* name, void* context) {
    const cJSON* v = cJSON_GetObjectItemCaseSensitive((const cJSON*)context, name);
    return cJSON_IsString(v) ? v->valuestring : NULL;
}

static bool taken_in(const char* name, void* context) {
    const cJSON* t;
    cJSON_ArrayForEach(t, (const cJSON*)context) if (strcmp(t->valuestring, name) == 0) return true;
    return false;
}

void a_file_in_a_store_is_kept_under_its_name_in_a_folder_of_its_contents(void) {
    cJSON* cases = conformance("record-files.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "storePaths")) {
        const char* text = cJSON_GetObjectItem(c, "text")->valuestring;
        CHECK_TEXT(clarity_store_path((ClarityBytes){(const uint8_t*)text, strlen(text)}, cJSON_GetObjectItem(c, "chosen")->valuestring), cJSON_GetObjectItem(c, "path")->valuestring);
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "names")) CHECK_TEXT(clarity_name_of(cJSON_GetObjectItem(c, "path")->valuestring), cJSON_GetObjectItem(c, "name")->valuestring);
    cJSON_Delete(cases);
}

void a_copy_goes_to_the_downloads_folder_under_a_free_name(void) {
    cJSON* cases = conformance("record-files.json");
    REQUIRE(cases);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "downloads")) {
        const char* found = clarity_downloads_in(cJSON_GetObjectItem(c, "os")->valuestring, from_case, cJSON_GetObjectItem(c, "env"));
        const cJSON* folder = cJSON_GetObjectItem(c, "folder");
        if (cJSON_IsNull(folder)) CHECK(found == NULL);
        else CHECK_TEXT(found, folder->valuestring);
    }
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(cases, "copies")) {
        CHECK_TEXT(clarity_free_name(cJSON_GetObjectItem(c, "name")->valuestring, taken_in, cJSON_GetObjectItem(c, "taken")), cJSON_GetObjectItem(c, "saved")->valuestring);
    }
    cJSON_Delete(cases);
}

typedef struct {
    int64_t id;
    ClarityRecordFile* manual;
    ClarityRecordFile* sheet;
} Product;

static const char* const product_columns[] = {"manual", "manual_name", "sheet", "sheet_name"};
static const ClarityPictureColumn product_files[] = {{"manual", NULL, "manual_name"}, {"sheet", "documents", "sheet_name"}};

static void product_values(const void* record, ClarityValue* out) {
    const Product* r = record;
    out[0] = clarity_file_content_value(r->manual);
    out[1] = clarity_file_name_value(r->manual);
    out[2] = clarity_file_content_value(r->sheet);
    out[3] = clarity_file_name_value(r->sheet);
}

static bool product_from_values(int64_t id, const ClarityValue* v, void* record) {
    Product* r = record;
    r->id = id;
    return clarity_get_file("manual", v[0], v[1], &r->manual) && clarity_get_file("sheet", v[2], v[3], &r->sheet);
}

static const ClarityEntity product = {
    .name = "Product", .table = "product", .key = "id", .source = "", .columns = product_columns, .column_count = 4,
    .pictures = product_files, .picture_count = 2, .size = sizeof(Product), .id_at = offsetof(Product, id),
    .values = product_values, .from_values = product_from_values,
};

void a_file_is_kept_with_its_name_in_the_database_or_its_store(void) {
    const char* store = clarity_format("clarity-record-files-%d", rand());
    REQUIRE(clarity_make_folders(store));
    ClarityDb* db;
    static const char* const create[] = {"CREATE TABLE product (id INTEGER PRIMARY KEY, manual BLOB, manual_name TEXT NOT NULL DEFAULT '', sheet TEXT, sheet_name TEXT NOT NULL DEFAULT '')"};
    ClarityStep step = {create, 1};
    REQUIRE_OK(clarity_db_memory(&db) && clarity_migrate(db, "", &step, 1, NULL) && clarity_add_store(db, "documents", store));
    const char* text = "%PDF-1.7 not a picture";
    ClarityBytes pdf = {(const uint8_t*)text, strlen(text)};
    Product p = {0, clarity_record_file("manual.pdf", clarity_picture_new(pdf)), clarity_record_file("sheet.pdf", clarity_picture_new(pdf))};
    CHECK_OK(clarity_save(db, &product, &p));
    Product read;
    CHECK_OK(clarity_get(db, &product, p.id, &read));
    CHECK_TEXT(clarity_record_file_name(read.manual), "manual.pdf");
    CHECK_TEXT(clarity_record_file_name(read.sheet), "sheet.pdf");
    CHECK_TEXT(clarity_record_file_extension(read.sheet), "pdf");
    ClarityBytes got;
    CHECK_OK(clarity_picture(db, &product, p.id, "manual", &got));
    CHECK(got.size == pdf.size && memcmp(got.data, pdf.data, pdf.size) == 0);
    ClarityBytes kept;
    CHECK_OK(clarity_read_file(clarity_format("%s/%s", store, clarity_store_path(pdf, "sheet.pdf")), &kept));
    CHECK(kept.size == pdf.size);
    read.manual = NULL;
    CHECK_OK(clarity_save(db, &product, &read));
    Product again;
    CHECK_OK(clarity_get(db, &product, p.id, &again));
    CHECK(!again.manual);
    // Copied to another record (product.manual = other.sheet): with its name, the bytes read when stored.
    ClarityRecordFile* copied = NULL;
    CHECK_OK(clarity_copied_file(db, &product, again.sheet, again.id, "sheet", "documents", NULL, &copied));
    REQUIRE(copied);
    CHECK_TEXT(copied->name, "sheet.pdf");
    CHECK(clarity_picture_same(copied->content, clarity_picture_new(pdf)));
    ClarityRecordFile* none = NULL;
    CHECK_OK(clarity_copied_file(db, &product, again.manual, again.id, "manual", NULL, NULL, &none));
    CHECK(!none);
    Product other = {0, copied, NULL};
    CHECK_OK(clarity_save(db, &product, &other));
    CHECK_OK(clarity_picture(db, &product, other.id, "manual", &got));
    CHECK(got.size == pdf.size && memcmp(got.data, pdf.data, pdf.size) == 0);
    CHECK_JSON(clarity_file_json(clarity_record_file("C:\\docs\\a.pdf", clarity_picture_stored())), "{\"stored\":true,\"name\":\"a.pdf\"}");
    clarity_db_close(db);
    remove_tree(store);
}
