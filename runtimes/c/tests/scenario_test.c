// The generated migration steps (conformance/migrations.json) applied by the
// C runtime, with records stored between the steps: the same scenario as
// the other runtimes' server tests, on SQLite.

#include <stddef.h>
#include <stdlib.h>

#include "check.h"
#include "clarity/code.h"
#include "tests.h"

/// A customer as the first step stores it.
typedef struct {
    int64_t id;
    const char *name, *email, *kind;
    ClarityDecimal limit;
    bool active;
    ClarityDate since;
    const char* notes;
} Before;

static const char* const before_columns[] = {"name", "email", "kind", "limit", "active", "since", "notes"};

static void before_values(const void* record, ClarityValue* out) {
    const Before* r = record;
    out[0] = clarity_text_value(r->name);
    out[1] = clarity_text_value(r->email);
    out[2] = clarity_text_value(r->kind);
    out[3] = clarity_decimal_value(r->limit);
    out[4] = clarity_bool_value(r->active);
    out[5] = clarity_date_value(r->since);
    out[6] = clarity_text_value(r->notes);
}

static bool before_from_values(int64_t id, const ClarityValue* v, void* record) {
    Before* r = record;
    r->id = id;
    return clarity_get_text("name", v[0], &r->name) && clarity_get_text("email", v[1], &r->email) && clarity_get_text("kind", v[2], &r->kind) &&
           clarity_get_decimal("limit", v[3], &r->limit) && clarity_get_bool("active", v[4], &r->active) && clarity_get_date("since", v[5], &r->since) &&
           clarity_get_text("notes", v[6], &r->notes);
}

static const ClarityEntity before = {
    .name = "Customer", .table = "customer", .key = "id", .source = "", .columns = before_columns, .column_count = 7,
    .size = sizeof(Before), .id_at = offsetof(Before, id), .values = before_values, .from_values = before_from_values,
};

/// An order as the first step stores it.
typedef struct {
    int64_t id;
    const char* number;
    int64_t customer;
    int64_t total;
    ClarityDateTime placed;
    bool paid;
    const char* code;
} Order;

static const char* const order_columns[] = {"number", "customer", "total", "placed", "paid", "code"};

static void order_values(const void* record, ClarityValue* out) {
    const Order* r = record;
    out[0] = clarity_text_value(r->number);
    out[1] = clarity_reference_value(r->customer);
    out[2] = clarity_integer_value(r->total);
    out[3] = clarity_date_time_value(r->placed);
    out[4] = clarity_bool_value(r->paid);
    out[5] = clarity_text_value(r->code);
}

static bool order_from_values(int64_t id, const ClarityValue* v, void* record) {
    Order* r = record;
    r->id = id;
    return clarity_get_text("number", v[0], &r->number) && clarity_get_reference("customer", v[1], &r->customer) &&
           clarity_get_integer("total", v[2], &r->total) && clarity_get_date_time("placed", v[3], &r->placed) && clarity_get_bool("paid", v[4], &r->paid) &&
           clarity_get_text("code", v[5], &r->code);
}

static const ClarityEntity order = {
    .name = "Order", .table = "order", .key = "id", .source = "", .columns = order_columns, .column_count = 6,
    .size = sizeof(Order), .id_at = offsetof(Order, id), .values = order_values, .from_values = order_from_values,
};

/// A customer after the second step.
typedef struct {
    int64_t id;
    const char *name, *email, *kind;
    ClarityDecimal credit_limit;
    bool vip;
    const char *remarks, *phone;
    ClarityPicture* portrait;
} Customer;

static const char* const customer_columns[] = {"name", "email", "kind", "credit_limit", "vip", "remarks", "phone", "portrait"};
static const ClarityPictureColumn customer_pictures[] = {{"portrait", NULL}};

static void customer_values(const void* record, ClarityValue* out) {
    const Customer* r = record;
    out[0] = clarity_text_value(r->name);
    out[1] = clarity_text_value(r->email);
    out[2] = clarity_text_value(r->kind);
    out[3] = clarity_decimal_value(r->credit_limit);
    out[4] = clarity_bool_value(r->vip);
    out[5] = clarity_text_value(r->remarks);
    out[6] = clarity_text_value(r->phone);
    out[7] = clarity_picture_value(r->portrait);
}

static bool customer_from_values(int64_t id, const ClarityValue* v, void* record) {
    Customer* r = record;
    r->id = id;
    return clarity_get_text("name", v[0], &r->name) && clarity_get_text("email", v[1], &r->email) && clarity_get_text("kind", v[2], &r->kind) &&
           clarity_get_decimal("credit_limit", v[3], &r->credit_limit) && clarity_get_bool("vip", v[4], &r->vip) &&
           clarity_get_text("remarks", v[5], &r->remarks) && clarity_get_text("phone", v[6], &r->phone) && clarity_get_picture("portrait", v[7], &r->portrait);
}

static const ClarityEntity customer = {
    .name = "Customer", .table = "customer", .key = "id", .source = "", .columns = customer_columns, .column_count = 8,
    .pictures = customer_pictures, .picture_count = 1, .size = sizeof(Customer), .id_at = offsetof(Customer, id),
    .values = customer_values, .from_values = customer_from_values,
};

/// An order after the second step: another table, and a money total.
typedef struct {
    int64_t id;
    const char* number;
    int64_t customer;
    ClarityDecimal total;
    ClarityDateTime placed;
    int64_t paid;
    int64_t code;
} Purchase;

static void purchase_values(const void* record, ClarityValue* out) {
    const Purchase* r = record;
    out[0] = clarity_text_value(r->number);
    out[1] = clarity_reference_value(r->customer);
    out[2] = clarity_decimal_value(r->total);
    out[3] = clarity_date_time_value(r->placed);
    out[4] = clarity_integer_value(r->paid);
    out[5] = clarity_integer_value(r->code);
}

static bool purchase_from_values(int64_t id, const ClarityValue* v, void* record) {
    Purchase* r = record;
    r->id = id;
    return clarity_get_text("number", v[0], &r->number) && clarity_get_reference("customer", v[1], &r->customer) &&
           clarity_get_decimal("total", v[2], &r->total) && clarity_get_date_time("placed", v[3], &r->placed) &&
           clarity_get_integer("paid", v[4], &r->paid) && clarity_get_integer("code", v[5], &r->code);
}

static const ClarityEntity purchase = {
    .name = "Purchase", .table = "purchase", .key = "id", .source = "", .columns = order_columns, .column_count = 6,
    .size = sizeof(Purchase), .id_at = offsetof(Purchase, id), .values = purchase_values, .from_values = purchase_from_values,
};

/// The names of what a query finds, one after another.
static const char* found(ClarityDb* db, const ClarityEntity* entity, ClarityQuery* query, size_t name_at) {
    void* items;
    size_t count;
    if (!clarity_find(db, entity, query, &items, &count)) return clarity_error()->message;
    ClarityBuffer out = {0};
    for (size_t i = 0; i < count; ++i) {
        const char* name = *(const char**)((char*)items + i * entity->size + name_at);
        clarity_buffer_addf(&out, "%s%s", i ? ", " : "", name);
    }
    return clarity_buffer_text(&out);
}

static ClarityStep step_of(const cJSON* statements) {
    size_t count = (size_t)cJSON_GetArraySize(statements);
    const char** list = CLARITY_NEW(const char*, count);
    size_t i = 0;
    const cJSON* s;
    cJSON_ArrayForEach(s, statements) list[i++] = s->valuestring;
    return (ClarityStep){list, count};
}

static ClarityDecimal dec(const char* text) {
    ClarityDecimal d = {0, 0};
    clarity_decimal_parse(text, &d);
    return d;
}

void the_server_scenario_on_sqlite(void) {
    const char* file = clarity_format("%s/clarity-scenario-c.db", temp_folder());
    remove(file);
    cJSON* cases = conformance("migrations.json");
    const cJSON* sqlite = cJSON_GetObjectItem(cJSON_GetObjectItem(cases, "steps"), "sqlite");
    ClarityStep all[2] = {step_of(cJSON_GetArrayItem(sqlite, 0)), step_of(cJSON_GetArrayItem(sqlite, 1))};
    ClaritySource source = {"main", CLARITY_SQLITE, file};
    ClarityDb* db;
    REQUIRE_OK(clarity_db_open(&source, 1, &db));
    int applied;
    CHECK_OK(clarity_migrate(db, "", all, 1, &applied));
    CHECK_INT(applied, 1);
    ClarityDate since;
    clarity_date_parse("2026-01-15", &since);
    Before ada = {0, "Ada Lovelace", "ada@example.com", "Business", dec("1250.50"), true, since, "first's"};
    CHECK_OK(clarity_save(db, &before, &ada));
    Before grace = {0, "Grace Hopper", "", "Private", dec("99.5"), false, {0}, ""};
    CHECK_OK(clarity_save(db, &before, &grace));
    Before got;
    CHECK_OK(clarity_get(db, &before, ada.id, &got));
    CHECK(strcmp(got.name, ada.name) == 0 && clarity_decimal_compare(got.limit, ada.limit) == 0 && clarity_date_compare(got.since, since) == 0 &&
          strcmp(got.notes, ada.notes) == 0 && got.active);
    // Saving what is stored already is no missing record.
    CHECK_OK(clarity_save(db, &before, &grace));

    ClarityDateTime placed;
    clarity_date_time_parse("2026-10-17T09:30:15", &placed);
    Order o = {0, "ORD-1", ada.id, 99, placed, true, "42"};
    CHECK_OK(clarity_save(db, &order, &o));
    Order blank = {0, "ORD-0", 0, 0, {0}, false, ""};
    CHECK_OK(clarity_save(db, &order, &blank));
    CHECK_OK(clarity_save(db, &order, &o));
    // Through a reference: the customer's name, in a condition and an order.
    size_t number_at = offsetof(Order, number);
    CHECK_TEXT(found(db, &order, clarity_where(clarity_query(), clarity_through("customer", "customer", "id", clarity_contains("name", "LOVE"))), number_at), "ORD-1");
    Order grace_order = {0, "ORD-G", grace.id, 0, {0}, false, ""};
    CHECK_OK(clarity_save(db, &order, &grace_order));
    ClarityHop hop = {"customer", "customer", "id"};
    ClarityQuery* by_name = clarity_order_through(clarity_where(clarity_query(), clarity_not(clarity_is_none("customer"))), 1, &hop, "name", true, false);
    CHECK_TEXT(found(db, &order, by_name, number_at), "ORD-G, ORD-1");
    CHECK_OK(clarity_delete(db, &order, &grace_order));
    CHECK_FAILS(CLARITY_IN_USE, clarity_delete(db, &before, &ada));
    CHECK_TEXT(clarity_error()->table, "customer");
    CHECK_INT(clarity_error()->id, ada.id);

    // Queries: contains ignores case and wildcards, decimals compare as numbers.
    size_t name_at = offsetof(Before, name);
    CHECK_TEXT(found(db, &before, clarity_where(clarity_query(), clarity_contains("name", "LOVE")), name_at), "Ada Lovelace");
    CHECK_TEXT(found(db, &before, clarity_where(clarity_query(), clarity_contains("name", "%")), name_at), "");
    ClarityCond* rich = clarity_and(clarity_number("limit", CLARITY_GT, clarity_integer_value(900)), clarity_is_true("active"));
    CHECK_TEXT(found(db, &before, clarity_where(clarity_query(), rich), name_at), "Ada Lovelace");
    CHECK_TEXT(found(db, &before, clarity_order_by_number(clarity_query(), "limit", false), name_at), "Grace Hopper, Ada Lovelace");
    CHECK_TEXT(found(db, &before, clarity_where(clarity_query(), clarity_compare("since", CLARITY_EQ, clarity_date_value(since))), name_at), "Ada Lovelace");
    ClarityValue kinds[] = {clarity_text_value("Private")};
    CHECK_TEXT(found(db, &before, clarity_where(clarity_query(), clarity_one_of("kind", 1, kinds)), name_at), "Grace Hopper");
    CHECK_TEXT(found(db, &before, clarity_where(clarity_query(), clarity_is_none("since")), name_at), "Grace Hopper");

    CHECK_OK(clarity_migrate(db, "", all, 2, &applied));
    CHECK_INT(applied, 1);
    CHECK_OK(clarity_migrate(db, "", all, 2, &applied));
    CHECK_INT(applied, 0);
    int version;
    CHECK_OK(clarity_schema_version(db, &version));
    CHECK_INT(version, 2);

    Customer c;
    CHECK_OK(clarity_get(db, &customer, ada.id, &c));
    CHECK(clarity_decimal_compare(c.credit_limit, dec("1250.50")) == 0 && c.vip && strcmp(c.remarks, "first's") == 0 && !*c.phone);
    Purchase p;
    CHECK_OK(clarity_get(db, &purchase, o.id, &p));
    CHECK(clarity_decimal_compare(p.total, clarity_decimal_of(99)) == 0 && p.customer == ada.id && clarity_date_time_compare(p.placed, placed) == 0 &&
          p.paid == 1 && p.code == 42);
    // Empty text is no number: it becomes the default.
    Purchase empty;
    CHECK_OK(clarity_get(db, &purchase, blank.id, &empty));
    CHECK_INT(empty.code, 0);

    // A picture in a binary column: saved, kept by a save that leaves it, emptied.
    static const uint8_t png_bytes[] = {0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
    ClarityBytes png = {png_bytes, sizeof png_bytes};
    Customer pictured;
    CHECK_OK(clarity_get(db, &customer, ada.id, &pictured));
    CHECK(!pictured.portrait);
    pictured.portrait = clarity_picture_new(png);
    CHECK_OK(clarity_save(db, &customer, &pictured));
    CHECK_OK(clarity_get(db, &customer, ada.id, &pictured));
    CHECK(clarity_picture_same(pictured.portrait, clarity_picture_stored()));
    pictured.phone = "555";
    CHECK_OK(clarity_save(db, &customer, &pictured));
    ClarityBytes bytes;
    CHECK_OK(clarity_picture(db, &customer, ada.id, "portrait", &bytes));
    CHECK(bytes.size == png.size && memcmp(bytes.data, png.data, png.size) == 0);
    pictured.portrait = NULL;
    pictured.phone = "";
    CHECK_OK(clarity_save(db, &customer, &pictured));
    CHECK_OK(clarity_picture(db, &customer, ada.id, "portrait", &bytes));
    CHECK(!bytes.data);

    // Unique email: empty many times, a filled one once.
    Customer linus = {0, "Linus", "", "", {0, 0}, false, "", "", NULL};
    CHECK_OK(clarity_save(db, &customer, &linus));
    Customer copy = {0, "Copy", "ada@example.com", "", {0, 0}, false, "", "", NULL};
    CHECK(!clarity_save(db, &customer, &copy));
    // The reference is kept through the renamed table.
    Customer ada_now;
    CHECK_OK(clarity_get(db, &customer, ada.id, &ada_now));
    CHECK_FAILS(CLARITY_IN_USE, clarity_delete(db, &customer, &ada_now));
    Purchase orphan = {0, "ORD-2", 999, {0, 0}, {0}, 0, 0};
    CHECK_FAILS(CLARITY_INVALID, clarity_save(db, &purchase, &orphan));
    CHECK_OK(clarity_delete(db, &purchase, &p));
    CHECK_OK(clarity_delete(db, &customer, &ada_now));
    void* items;
    size_t count;
    CHECK_OK(clarity_find(db, &customer, clarity_query(), &items, &count));
    CHECK_INT(count, 2);
    clarity_db_close(db);
    remove(file);
}
