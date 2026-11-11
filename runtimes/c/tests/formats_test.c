// The data formats of code on the C runtime (conformance/formats-runtime.json):
// read through Clarity.Formats and written back, the kind and options passed
// and the messages given word for word. A record's own `_from_json` and
// `_json`, which the generator writes, are built and run with every
// generated program that reads or writes a format.

#include <stddef.h>

#include "check.h"
#include "clarity/disk.h"
#include "clarity/json.h"
#include "clarity/sql.h"
#include "formats/file_source.h"
#include "formats/formats.h"
#include "tests.h"

void formats_read_and_write_as_every_stack_does(void) {
    cJSON* file = conformance("formats-runtime.json");
    REQUIRE(file);
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const char* name = cJSON_GetObjectItem(c, "name")->valuestring;
        const char* kind = clarity_json_of(cJSON_GetObjectItem(c, "kind"));
        const cJSON* given = cJSON_GetObjectItem(c, "options");
        const char* options = given ? clarity_json_of(given) : "";
        const char* input = cJSON_GetObjectItem(c, "input")->valuestring;
        const cJSON* named = cJSON_GetObjectItem(c, "format");
        const char* format = named ? named->valuestring : "json";
        cJSON* fitted = NULL;
        bool read = clarity_formats_read(format, input, options, kind, &fitted);
        const cJSON* written = cJSON_GetObjectItem(c, "written");
        if (written) {
            if (!read) {
                check_fail(__FILE__, __LINE__, name);
                continue;
            }
            CHECK_TEXT(clarity_formats_write(format, clarity_json_of(fitted), options, kind), written->valuestring);
        } else {
            CHECK(!read);
            CHECK_TEXT(clarity_error()->message, cJSON_GetObjectItem(c, "error")->valuestring);
        }
    }
}

// A data source that is a file in a format (docs/FORMATS-PLAN.md): a CSV
// read into a table its entity finds, saves and deletes in, each change
// written back to the file with its own column order and the columns no
// entity maps kept.
typedef struct {
    int64_t id;
    const char* site;
    int64_t visits;
    bool paid;
} Login;

static const char* const login_columns[] = {"Site", "visits", "paid"};

static void login_values(const void* record, ClarityValue* out) {
    const Login* r = record;
    out[0] = clarity_text_value(r->site);
    out[1] = clarity_integer_value(r->visits);
    out[2] = clarity_bool_value(r->paid);
}
static bool login_from_values(int64_t id, const ClarityValue* v, void* record) {
    Login* r = record;
    r->id = id;
    return clarity_get_text("site", v[0], &r->site) && clarity_get_integer("visits", v[1], &r->visits) && clarity_get_bool("paid", v[2], &r->paid);
}
static const ClarityEntity login = {
    .name = "Login", .table = "login", .key = "id", .source = "logins", .columns = login_columns, .column_count = 3,
    .size = sizeof(Login), .id_at = offsetof(Login, id), .values = login_values, .from_values = login_from_values,
};

static ClarityFileSource login_source(const char* path) {
    return (ClarityFileSource){
        .name = "logins", .format = "csv", .path = path, .options = "{\"separator\":\";\"}", .table = "login", .key = "id",
        .fields = "[{\"name\":\"Site\",\"kind\":\"text\"},{\"name\":\"visits\",\"kind\":\"integer\"},{\"name\":\"paid\",\"kind\":\"bool\"}]",
    };
}

static bool write_text(const char* path, const char* text) { return clarity_write_file(path, (ClarityBytes){(const uint8_t*)text, strlen(text)}); }

static const char* read_text(const char* path) {
    ClarityBytes bytes = {0};
    return clarity_read_file(path, &bytes) ? clarity_copy_n((const char*)bytes.data, bytes.size) : "(not read)";
}

void a_csv_file_is_a_data_source_written_back_after_each_change(void) {
    const char* folder = clarity_format("%s/clarity-file-source-c", temp_folder());
    remove_tree(folder);
    REQUIRE(clarity_make_folders(folder));
    const char* file = clarity_format("%s/logins.csv", folder);
    REQUIRE(write_text(file, "Site;Note;visits;paid\r\nexample.com;keep me;3;true\r\nshop.nl;;1;false\r\n"));
    ClarityDb* db = NULL;
    REQUIRE_OK(clarity_db_open(NULL, 0, &db));
    ClarityFileSource source = login_source(file);
    REQUIRE_OK(clarity_db_add_file(db, &source));

    void* items = NULL;
    size_t count = 0;
    REQUIRE_OK(clarity_find(db, &login, clarity_query(), &items, &count));
    REQUIRE(count == 2);
    Login* all = items;
    CHECK_INT(all[0].id, 1);
    CHECK_TEXT(all[0].site, "example.com");
    CHECK_INT(all[0].visits, 3);
    CHECK(all[0].paid);
    CHECK_INT(all[1].id, 2);
    CHECK_TEXT(all[1].site, "shop.nl");
    CHECK_INT(all[1].visits, 1);
    CHECK(!all[1].paid);

    all[0].visits = 4;
    CHECK_OK(clarity_save(db, &login, &all[0]));
    Login added = {0, "a;b", 0, true};
    CHECK_OK(clarity_save(db, &login, &added));
    CHECK_OK(clarity_delete(db, &login, &all[1]));
    CHECK_TEXT(read_text(file), "Site;Note;visits;paid\r\nexample.com;keep me;4;true\r\n\"a;b\";;0;true\r\n");
    // The new file replaced the old one.
    CHECK(!clarity_exists(clarity_format("%s.new", file)));
    clarity_db_close(db);

    // A column the entity maps that the file lacks stops the program as it opens.
    REQUIRE(write_text(file, "Site;paid\r\n"));
    REQUIRE_OK(clarity_db_open(NULL, 0, &db));
    CHECK(!clarity_db_add_file(db, &source));
    CHECK(strstr(clarity_error()->message, "data source logins: the CSV has no column 'visits'") != NULL);
    clarity_db_close(db);

    // A file not there yet starts empty, and the first save makes it and its folder.
    const char* fresh = clarity_format("%s/fresh/logins.csv", folder);
    REQUIRE_OK(clarity_db_open(NULL, 0, &db));
    ClarityFileSource new_source = login_source(fresh);
    REQUIRE_OK(clarity_db_add_file(db, &new_source));
    REQUIRE_OK(clarity_find(db, &login, clarity_query(), &items, &count));
    CHECK(count == 0);
    Login first = {0, "new.nl", 0, true};
    CHECK_OK(clarity_save(db, &login, &first));
    CHECK_TEXT(read_text(fresh), "Site;visits;paid\r\nnew.nl;0;true\r\n");
    // A readonly source needs its file.
    ClarityFileSource needed = login_source(clarity_format("%s/missing.csv", folder));
    needed.readonly = true;
    CHECK(!clarity_db_add_file(db, &needed));
    CHECK(strstr(clarity_error()->message, "data source logins: cannot read") != NULL);
    clarity_db_close(db);
    remove_tree(folder);
}

// A file source whose mapping holds records inside its records (`nested`):
// each entity in its own table, a line linked to its order by the order's key.
typedef struct {
    int64_t id;
    const char* number;
} Order;

static const char* const order_columns[] = {"number"};
static void order_values(const void* record, ClarityValue* out) { out[0] = clarity_text_value(((const Order*)record)->number); }
static bool order_from_values(int64_t id, const ClarityValue* v, void* record) {
    Order* r = record;
    r->id = id;
    return clarity_get_text("number", v[0], &r->number);
}
static const ClarityEntity order = {
    .name = "Order", .table = "order", .key = "id", .source = "feed", .columns = order_columns, .column_count = 1,
    .size = sizeof(Order), .id_at = offsetof(Order, id), .values = order_values, .from_values = order_from_values,
};

typedef struct {
    int64_t id;
    int64_t order;
    const char* product;
} Line;

static const char* const line_columns[] = {"order", "product"};
static void line_values(const void* record, ClarityValue* out) {
    const Line* r = record;
    out[0] = clarity_integer_value(r->order);
    out[1] = clarity_text_value(r->product);
}
static bool line_from_values(int64_t id, const ClarityValue* v, void* record) {
    Line* r = record;
    r->id = id;
    return clarity_get_integer("order", v[0], &r->order) && clarity_get_text("product", v[1], &r->product);
}
static const ClarityEntity line = {
    .name = "Line", .table = "line", .key = "id", .source = "feed", .columns = line_columns, .column_count = 2,
    .size = sizeof(Line), .id_at = offsetof(Line, id), .values = line_values, .from_values = line_from_values,
};

void a_file_source_holds_the_records_inside_its_records_in_their_own_table(void) {
    const char* folder = clarity_format("%s/clarity-file-source-nested-c", temp_folder());
    remove_tree(folder);
    REQUIRE(clarity_make_folders(folder));
    const char* file = clarity_format("%s/orders.xml", folder);
    REQUIRE(write_text(file,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><lines><line product=\"Tea\"/><line product=\"Coffee\"/></lines></order>\n"
        "    <order number=\"A-2\"><lines><line product=\"Milk\"/></lines></order>\n  </orders>\n</shop>\n"));
    ClarityDb* db = NULL;
    REQUIRE_OK(clarity_db_open(NULL, 0, &db));
    ClarityFileSource source = {
        .name = "feed", .format = "xml", .path = file, .table = "order", .key = "id",
        .options = "{\"at\":\"shop/orders/order\",\"fields\":{\"number\":\"@number\"},\"nested\":{\"lines\":{\"at\":\"lines/line\",\"fields\":{\"product\":\"@product\"},\"parent\":\"order\"}}}",
        .fields = "[{\"name\":\"number\",\"kind\":\"text\"},{\"name\":\"lines\",\"kind\":\"records\",\"entity\":\"Line\",\"fields\":[{\"name\":\"product\",\"kind\":\"text\"}],\"base\":{},\"table\":\"line\",\"key\":\"id\"}]",
    };
    REQUIRE_OK(clarity_db_add_file(db, &source));

    void* items = NULL;
    size_t count = 0;
    REQUIRE_OK(clarity_find(db, &order, clarity_query(), &items, &count));
    REQUIRE(count == 2);
    Order* orders = items;
    CHECK_INT(orders[0].id, 1);
    CHECK_TEXT(orders[0].number, "A-1");
    CHECK_INT(orders[1].id, 2);
    CHECK_TEXT(orders[1].number, "A-2");
    REQUIRE_OK(clarity_find(db, &line, clarity_query(), &items, &count));
    REQUIRE(count == 3);
    Line* lines = items;
    CHECK_INT(lines[0].order, 1);
    CHECK_TEXT(lines[0].product, "Tea");
    CHECK_INT(lines[1].order, 1);
    CHECK_TEXT(lines[1].product, "Coffee");
    CHECK_INT(lines[2].id, 3);
    CHECK_INT(lines[2].order, 2);
    CHECK_TEXT(lines[2].product, "Milk");

    Line added = {0, 2, "Juice"};
    CHECK_OK(clarity_save(db, &line, &added));
    CHECK_OK(clarity_delete(db, &line, &lines[0]));
    CHECK_TEXT(read_text(file),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<shop>\n  <orders>\n    <order number=\"A-1\"><lines><line product=\"Coffee\"/></lines></order>\n"
        "    <order number=\"A-2\"><lines><line product=\"Milk\"/><line product=\"Juice\"/></lines></order>\n  </orders>\n</shop>\n");
    clarity_db_close(db);
    remove_tree(folder);
}
