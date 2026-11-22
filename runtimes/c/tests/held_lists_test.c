// A stored record's list (order.lines) loads with the record, one level
// deep: generated code reads each list once for all the records a find
// gives (clarity_find_by_owners) and shares it out by the reference back
// (clarity_owned) (runtimes/rust/tests/held_lists.rs, docs/ledger/runtime-data.md).

#include <stddef.h>
#include <stdlib.h>

#include "check.h"
#include "clarity/code.h"
#include "apply_steps.h"
#include "tests.h"

typedef struct {
    int64_t id;
    int64_t order;
    const char* product;
} HeldLine;

static const char* const held_line_columns[] = {"held_order", "product"};

static void held_line_values(const void* record, ClarityValue* out) {
    const HeldLine* r = record;
    out[0] = clarity_reference_value(r->order);
    out[1] = clarity_text_value(r->product);
}

static bool held_line_from_values(int64_t id, const ClarityValue* v, void* record) {
    HeldLine* r = record;
    r->id = id;
    return clarity_get_reference("held_order", v[0], &r->order) && clarity_get_text("product", v[1], &r->product);
}

static const ClarityEntity held_line = {
    .name = "HeldLine", .table = "held_line", .key = "id", .source = "", .columns = held_line_columns, .column_count = 2,
    .size = sizeof(HeldLine), .id_at = offsetof(HeldLine, id), .values = held_line_values, .from_values = held_line_from_values,
};

/// The products of `owner`'s lines among `items`, joined by spaces.
static const char* products_of(const void* items, size_t count, int64_t owner) {
    void* mine = NULL;
    size_t n = clarity_owned(items, count, sizeof(HeldLine), offsetof(HeldLine, order), owner, &mine);
    const char* text = "";
    for (size_t i = 0; i < n; i++) text = clarity_format("%s%s%s", text, i ? " " : "", ((HeldLine*)mine)[i].product);
    return text;
}

// Every case of conformance/held-lists.json: each list is read for all the
// orders asked for, ownersPerRead at a time, and each order takes its own lines in order.
void a_list_is_read_once_for_all_its_owners(void) {
    cJSON* file = conformance("held-lists.json");
    REQUIRE(file);
    CHECK_INT(cJSON_GetObjectItem(file, "ownersPerRead")->valueint, CLARITY_OWNERS_PER_READ);
    cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        ClarityDb* db;
        static const char* const create[] = {"CREATE TABLE held_line (id INTEGER PRIMARY KEY, held_order INTEGER, product TEXT NOT NULL)"};
        ClarityStep step = {create, 1};
        REQUIRE_OK(clarity_db_memory(&db) && apply_steps(db, "", &step, 1, NULL));
        size_t orders = (size_t)cJSON_GetObjectItem(c, "orders")->valueint;
        int64_t* owners = CLARITY_NEW(int64_t, orders);
        for (size_t i = 0; i < orders; i++) owners[i] = (int64_t)i + 1;
        cJSON* l;
        cJSON_ArrayForEach(l, cJSON_GetObjectItem(c, "lines")) {
            HeldLine line = {0, (int64_t)cJSON_GetArrayItem(l, 0)->valuedouble, clarity_copy(cJSON_GetArrayItem(l, 1)->valuestring)};
            REQUIRE_OK(clarity_save(db, &held_line, &line));
        }
        void* items = NULL;
        size_t count = 0;
        REQUIRE_OK(clarity_find_by_owners(db, &held_line, "held_order", orders, owners, &items, &count));
        cJSON* expect;
        cJSON_ArrayForEach(expect, cJSON_GetObjectItem(c, "expect")) {
            const char* want = clarity_format("%s: order %s:", cJSON_GetObjectItem(c, "name")->valuestring, expect->string);
            const char* got = clarity_format("%s%s", want, *products_of(items, count, (int64_t)atoll(expect->string)) ? " " : "");
            got = clarity_format("%s%s", got, products_of(items, count, (int64_t)atoll(expect->string)));
            cJSON* p;
            cJSON_ArrayForEach(p, expect) want = clarity_format("%s %s", want, p->valuestring);
            CHECK_TEXT(got, want);
        }
        clarity_db_close(db);
    }
}
