// Required fields: a save refuses a record whose required field is empty,
// as every runtime does (conformance/required.json).

#include <stddef.h>

#include "check.h"
#include "clarity/code.h"
#include "tests.h"

/// The slot `field` of the refusal a failed check set, or "(not refused)".
static const char* refused_field(bool kept) {
    if (kept) return "(not refused)";
    const ClarityError* e = clarity_error();
    if (e->kind != CLARITY_REFUSED || !e->text->key || strcmp(e->text->key, "fieldRequired") != 0) return "(another error)";
    for (size_t i = 0; i < e->text->count; ++i)
        if (strcmp(e->text->paths[i], "field") == 0) return clarity_slot_plain(&e->text->slots[i]);
    return "(no field slot)";
}

void required_fields_are_checked_as_every_runtime_does(void) {
    cJSON* file = conformance("required.json");
    REQUIRE(file);
    CHECK_TEXT(cJSON_GetObjectItem(cJSON_GetObjectItem(file, "refusal"), "key")->valuestring, "fieldRequired");
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const char* name = cJSON_GetObjectItem(c, "name")->valuestring;
        const cJSON* given = cJSON_GetObjectItem(c, "required");
        size_t required_count = (size_t)cJSON_GetArraySize(given);
        ClarityRequired* required = CLARITY_NEW(ClarityRequired, required_count);
        size_t at = 0;
        const cJSON* r;
        cJSON_ArrayForEach(r, given) {
            required[at++] = (ClarityRequired){cJSON_GetObjectItem(r, "column")->valuestring, cJSON_GetObjectItem(r, "label")->valuestring};
        }
        const cJSON* values = cJSON_GetObjectItem(c, "values");
        size_t count = (size_t)cJSON_GetArraySize(values);
        const char** columns = CLARITY_NEW(const char*, count);
        ClarityValue* stored = CLARITY_NEW(ClarityValue, count);
        at = 0;
        const cJSON* v;
        cJSON_ArrayForEach(v, values) {
            columns[at] = v->string;
            stored[at++] = cJSON_IsString(v) ? clarity_text_value(v->valuestring)
                           : cJSON_IsBool(v) ? clarity_bool_value(cJSON_IsTrue(v))
                           : cJSON_IsNumber(v) ? clarity_integer_value((int64_t)v->valuedouble)
                                               : clarity_null();
        }
        const char* got = refused_field(clarity_check_required(required, required_count, columns, stored, count));
        const cJSON* refused = cJSON_GetObjectItem(c, "refused");
        const char* want = cJSON_IsString(refused) ? refused->valuestring : "(not refused)";
        if (strcmp(got, want) != 0) check_fail(__FILE__, __LINE__, clarity_format("%s: %s, want %s", name, got, want));
    }
}

typedef struct {
    int64_t id;
    const char* name;
    int64_t group;
} Member;

static const char* const member_columns[] = {"name", "group_id"};
static const ClarityRequired member_required[] = {{"name", "Name"}, {"group_id", "@groupLabel"}};

static void member_values(const void* record, ClarityValue* out) {
    const Member* r = record;
    out[0] = clarity_text_value(r->name);
    out[1] = clarity_reference_value(r->group);
}

static bool member_from_values(int64_t id, const ClarityValue* v, void* record) {
    Member* r = record;
    r->id = id;
    r->group = v[1].kind == CLARITY_INTEGER ? v[1].integer : 0;
    return clarity_get_text("name", v[0], &r->name);
}

/// Before save fills an empty name.
static bool fill_name(ClarityDb* db, void* record) {
    (void)db;
    Member* m = record;
    if (!m->name || !*m->name) m->name = "Guest";
    return true;
}

static const ClarityEntity member = {
    .name = "Member", .table = "member", .key = "id", .source = "", .columns = member_columns, .column_count = 2,
    .size = sizeof(Member), .id_at = offsetof(Member, id), .values = member_values, .from_values = member_from_values,
    .required = member_required, .required_count = 2,
};

static const ClarityEntity guest = {
    .name = "Member", .table = "member", .key = "id", .source = "", .columns = member_columns, .column_count = 2,
    .size = sizeof(Member), .id_at = offsetof(Member, id), .values = member_values, .from_values = member_from_values,
    .before_save = fill_name, .required = member_required, .required_count = 2,
};

void a_save_refuses_a_record_with_an_empty_required_field(void) {
    static const char* const create[] = {"CREATE TABLE member (id INTEGER PRIMARY KEY, name TEXT NOT NULL, group_id INTEGER)"};
    ClarityStep step = {create, 1};
    ClarityDb* db = NULL;
    REQUIRE(clarity_db_memory(&db) && clarity_migrate(db, "", &step, 1, NULL));
    clarity_load_translations("{\"project\":{\"translations\":{\"en\":{\"groupLabel\":\"Group\"}}}}");
    Member empty = {0, "", 1};
    CHECK_TEXT(refused_field(clarity_save(db, &member, &empty)), "Name");
    // A label written as a key takes the key's value.
    Member no_group = {0, "Ada", 0};
    CHECK_TEXT(refused_field(clarity_save(db, &member, &no_group)), "Group");
    // Before save may fill a required field; a refused one keeps a new record new.
    Member filled = {0, "", 1};
    CHECK_OK(clarity_save(db, &guest, &filled));
    CHECK(filled.id != 0);
    Member unfilled = {0, "", 0};
    CHECK_TEXT(refused_field(clarity_save(db, &guest, &unfilled)), "Group");
    CHECK_INT(unfilled.id, 0);
    void* items = NULL;
    size_t count = 0;
    REQUIRE_OK(clarity_find(db, &member, clarity_query(), &items, &count));
    CHECK_INT(count, 1);
    if (count) CHECK_TEXT(((Member*)items)[0].name, "Guest");
    clarity_load_translations("{}");
    clarity_db_close(db);
}
