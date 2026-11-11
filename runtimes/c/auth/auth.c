// Who is using the program, through Clarity.Auth: see auth.h.

#include "auth/auth.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "clarity/cli.h"
#include "clarity/db_internal.h"
#include "clarity/dialect.h"
#include "clarity/settings.h"
#include "clarity/sql.h"
#include "clarity/text.h"
#include "clarity_auth.h"

#if defined(_WIN32)
#include <windows.h>
#else
#include <termios.h>
#include <unistd.h>
#endif

typedef struct {
    char* name;
    char** columns;
    size_t column_count;
} Strategy;

/// The project's people as the database handle keeps them, outside the scratch arena.
struct ClarityPeople {
    char* source;
    char* table;
    char* key;
    char* page;
    char** targets;
    size_t target_count;
    Strategy* strategies;
    size_t strategy_count;
    uint64_t session_seconds;
    uint32_t tries;
    uint64_t tries_seconds;
    char* program;
    /// How long a desktop sign-in is remembered; 0 for not at all.
    uint64_t remember_seconds;
    /// Whether only a sign-in whose input `remember` is "true" is remembered.
    bool remember_asks;
    /// The person signed in on the desktop or the command line; 0 for nobody.
    int64_t me;
    /// The command line signs in as `cli` (clarity_signin_cli); else `desktop`.
    bool cli;
};

// -- Clarity.Auth ------------------------------------------------------------

static const ClarityAuthApi* api(void) {
    static const ClarityAuthApi* table;
    if (!table) table = clarity_auth_api();
    return table;
}

/* Runs `op` with `args` (NULL for none): what it gave, parsed in the scratch arena, or its problem kept. */
static bool auth_call(const char* op, const cJSON* args, cJSON** out) {
    const ClarityAuthApi* auth = api();
    if (!auth || !auth->call) return clarity_fail(CLARITY_VALUE, "Clarity.Auth's library is not there");
    const char* text = args ? cJSON_PrintUnformatted(args) : "{}";
    if (!text) return clarity_fail(CLARITY_VALUE, "out of memory");
    uint8_t* made = NULL;
    size_t length = 0;
    int32_t code = auth->call(op, strlen(op), text, strlen(text), &made, &length);
    if (code != CLARITY_AUTH_OK) {
        char why[1024] = {0};
        if (auth->problem) auth->problem(why, sizeof why);
        return clarity_fail(CLARITY_VALUE, "%s", why[0] ? why : "Clarity.Auth failed");
    }
    char* copy = clarity_copy_n((const char*)made, length);
    auth->free(made, length);
    *out = cJSON_Parse(copy);
    return *out != NULL || clarity_fail(CLARITY_VALUE, "Clarity.Auth gave no JSON for %s", op);
}

/* The text `op` gives, or NULL with its problem kept. */
static const char* auth_text(const char* op, const cJSON* args) {
    cJSON* out = NULL;
    if (!auth_call(op, args, &out)) return NULL;
    if (cJSON_IsString(out)) return out->valuestring;
    clarity_fail(CLARITY_VALUE, "Clarity.Auth gave no text for %s", op);
    return NULL;
}

static cJSON* key_args(const char* key) {
    cJSON* args = cJSON_CreateObject();
    cJSON_AddStringToObject(args, "key", key);
    return args;
}

static cJSON* token_args(const char* token) {
    cJSON* args = cJSON_CreateObject();
    cJSON_AddStringToObject(args, "token", token);
    return args;
}

static void session_close(const char* token) {
    cJSON* out = NULL;
    auth_call("sessionClose", token_args(token), &out);
}

static int64_t session_find(const char* token) {
    cJSON* out = NULL;
    if (!auth_call("sessionFind", token_args(token), &out) || !cJSON_IsNumber(out)) return 0;
    return (int64_t)out->valuedouble;
}

/* The catalog's entry of strategy `name`; NULL when it has none. */
static const cJSON* catalog_entry(const char* name) {
    cJSON* catalog = NULL;
    if (!auth_call("catalog", NULL, &catalog)) return NULL;
    const cJSON* entry;
    cJSON_ArrayForEach(entry, catalog) {
        const cJSON* named = cJSON_GetObjectItemCaseSensitive(entry, "name");
        if (cJSON_IsString(named) && strcmp(named->valuestring, name) == 0) return entry;
    }
    return NULL;
}

static bool holds_text(const cJSON* list, const char* text) {
    const cJSON* item;
    cJSON_ArrayForEach(item, list) if (cJSON_IsString(item) && strcmp(item->valuestring, text) == 0) return true;
    return false;
}

// -- Helpers code calls ------------------------------------------------------

bool clarity_allow(bool holds) { return holds || clarity_refuse(clarity_not_allowed()); }

bool clarity_yours(bool holds) { return holds || clarity_refuse(clarity_not_yours()); }

bool clarity_hashed(const char* method, const char* text, int64_t minimum, const char** out) {
    if (minimum > 0) {
        cJSON* fits_args = cJSON_CreateObject();
        cJSON_AddStringToObject(fits_args, "text", text ? text : "");
        cJSON_AddNumberToObject(fits_args, "minimum", (double)minimum);
        cJSON* fits = NULL;
        if (!auth_call("secretFits", fits_args, &fits)) return false;
        if (!cJSON_IsTrue(fits)) return clarity_refuse(clarity_secret_too_short(minimum));
    }
    cJSON* args = cJSON_CreateObject();
    cJSON_AddStringToObject(args, "method", method ? method : "");
    cJSON_AddStringToObject(args, "text", text ? text : "");
    const char* hash = auth_text("hash", args);
    if (!hash) return false;
    *out = hash;
    return true;
}

bool clarity_new_recovery_codes(int64_t count, const char** out) {
    cJSON* args = cJSON_CreateObject();
    cJSON_AddNumberToObject(args, "count", (double)count);
    const char* codes = auth_text("newRecoveryCodes", args);
    if (!codes) return false;
    *out = codes;
    return true;
}

bool clarity_new_token(const char** out) {
    const char* token = auth_text("newToken", NULL);
    if (!token) return false;
    *out = token;
    return true;
}

// -- The people of the program -----------------------------------------------

static void free_list(char** items, size_t count) {
    for (size_t i = 0; items && i < count; i++) free(items[i]);
    free(items);
}

static void forget(struct ClarityPeople* people) {
    if (!people) return;
    free(people->source);
    free(people->table);
    free(people->key);
    free(people->page);
    free(people->program);
    free_list(people->targets, people->target_count);
    for (size_t i = 0; people->strategies && i < people->strategy_count; i++) {
        free(people->strategies[i].name);
        free_list(people->strategies[i].columns, people->strategies[i].column_count);
    }
    free(people->strategies);
    free(people);
}

/* Lasting copies of `count` texts; NULL when memory ran out (or for none). */
static char** copied_list(const char* const* items, size_t count, bool* ok) {
    if (count == 0) return NULL;
    char** out = calloc(count, sizeof *out);
    if (!out) {
        *ok = false;
        return NULL;
    }
    for (size_t i = 0; i < count; i++)
        if (!(out[i] = clarity_lasting(items[i] ? items[i] : ""))) *ok = false;
    return out;
}

static bool on(const struct ClarityPeople* people, const char* target) {
    for (size_t i = 0; i < people->target_count; i++)
        if (strcmp(people->targets[i], target) == 0) return true;
    return false;
}

static const Strategy* used(const struct ClarityPeople* people, const char* name) {
    for (size_t i = 0; i < people->strategy_count; i++)
        if (strcmp(people->strategies[i].name, name) == 0) return &people->strategies[i];
    return NULL;
}

static const char* first_hook(ClarityDb* db) { return clarity_signin_first(db); }
static bool signin_hook(ClarityDb* db, const char* strategy, const cJSON* inputs, int64_t* id) { return clarity_signin(db, strategy, inputs, id); }
static void signout_hook(ClarityDb* db) { clarity_signout(db); }

static bool watches_hook(ClarityDb* db, const ClarityEntity* entity);
static void changed_hook(ClarityDb* db, const ClarityEntity* entity, int64_t id, const char* const* columns, size_t count);

static const ClarityPeopleHooks hooks = {first_hook, signin_hook, signout_hook, session_find, session_close, watches_hook, changed_hook};

bool clarity_db_add_signin(ClarityDb* db, const ClaritySigninSource* source) {
    if (!db || !source || !source->table) return clarity_fail(CLARITY_STORAGE, "the people who sign in need a table");
    struct ClarityPeople* people = calloc(1, sizeof *people);
    if (!people) return clarity_fail(CLARITY_STORAGE, "out of memory");
    bool ok = true;
    people->source = clarity_lasting(source->source ? source->source : "");
    people->table = clarity_lasting(source->table);
    people->key = clarity_lasting(source->key && *source->key ? source->key : "id");
    people->page = clarity_lasting(source->page ? source->page : "");
    people->program = clarity_lasting(source->program ? source->program : "");
    people->remember_seconds = source->remember_seconds;
    people->remember_asks = source->remember_asks;
    ok = ok && people->source && people->table && people->key && people->page && people->program;
    people->targets = copied_list(source->targets, source->target_count, &ok);
    people->target_count = people->targets ? source->target_count : 0;
    if (source->strategy_count) {
        people->strategies = calloc(source->strategy_count, sizeof *people->strategies);
        if (people->strategies) {
            people->strategy_count = source->strategy_count;
            for (size_t i = 0; i < source->strategy_count; i++) {
                const ClaritySigninStrategy* given = &source->strategies[i];
                people->strategies[i].name = clarity_lasting(given->name ? given->name : "");
                people->strategies[i].columns = copied_list(given->columns, given->column_count, &ok);
                people->strategies[i].column_count = people->strategies[i].columns ? given->column_count : 0;
                ok = ok && people->strategies[i].name;
            }
        } else {
            ok = false;
        }
    }
    people->session_seconds = source->session_seconds ? source->session_seconds : 8 * 3600;
    people->tries = source->tries ? source->tries : 5;
    people->tries_seconds = source->tries_seconds ? source->tries_seconds : 15 * 60;
    if (!ok) {
        forget(people);
        return clarity_fail(CLARITY_STORAGE, "out of memory");
    }
    if (db->people && db->forget_people) db->forget_people(db->people);
    db->people = people;
    db->forget_people = forget;
    if (on(people, "web")) clarity_people_set_web_page(people->page);
    clarity_people_set(&hooks);
    return true;
}

static const char* target(const struct ClarityPeople* people) {
    if (clarity_call()) return "web";
    return people->cli ? "cli" : "desktop";
}

int64_t clarity_me_id(ClarityDb* db) {
    ClarityCall* call = clarity_call();
    if (call) return call->me;
    return db && db->people ? db->people->me : 0;
}

bool clarity_me(ClarityDb* db, const ClarityEntity* entity, void* record) {
    int64_t id = clarity_me_id(db);
    if (id <= 0) return clarity_refuse(clarity_not_allowed());
    return clarity_get(db, entity, id, record);
}

/* The column the strategy names for the field of `role`; NULL when it names none. */
static const char* column_of(const cJSON* info, const Strategy* strategy, const char* role) {
    size_t at = 0;
    const cJSON* field;
    cJSON_ArrayForEach(field, cJSON_GetObjectItemCaseSensitive(info, "fields")) {
        const cJSON* named = cJSON_GetObjectItemCaseSensitive(field, "role");
        if (cJSON_IsString(named) && strcmp(named->valuestring, role) == 0) return at < strategy->column_count ? strategy->columns[at] : NULL;
        at++;
    }
    return NULL;
}

/* The key and stored secret ("" without one) of the record whose `column` equals `equals`; `*found` false for none. */
static bool person_by(ClarityDb* db, const struct ClarityPeople* people, const char* column, const char* secret, const char* equals, bool* found, int64_t* id, const char** stored) {
    ClaritySourceDb* source = NULL;
    if (!clarity_source(db, people->source, &source)) return false;
    ClarityDialect d = source->driver->dialect;
    const char* wanted = secret ? clarity_format("%s, %s", clarity_quote(d, people->key), clarity_quote(d, secret)) : clarity_quote(d, people->key);
    const char* sql = clarity_format("SELECT %s FROM %s WHERE %s = %s", wanted, clarity_quote(d, people->table), clarity_quote(d, column), clarity_mark(d, 1));
    ClarityValues params = {0};
    clarity_values_add(&params, clarity_text_value(equals));
    ClarityRows rows = {0};
    if (!source->driver->rows(source->driver, sql, &params, &rows)) return false;
    *found = rows.count > 0 && rows.items[0].count > 0;
    if (!*found) return true;
    if (!clarity_get_integer(people->key, rows.items[0].values[0], id)) return false;
    *stored = rows.items[0].count > 1 && rows.items[0].values[1].kind == CLARITY_TEXT ? rows.items[0].values[1].text : "";
    return true;
}

static bool no_people(ClarityDb* db, const struct ClarityPeople* people, bool* none) {
    ClaritySourceDb* source = NULL;
    if (!clarity_source(db, people->source, &source)) return false;
    ClarityRows rows = {0};
    if (!source->driver->rows(source->driver, clarity_format("SELECT COUNT(*) FROM %s", clarity_quote(source->driver->dialect, people->table)), NULL, &rows)) return false;
    int64_t count = 0;
    if (rows.count > 0 && rows.items[0].count > 0 && !clarity_get_integer("count", rows.items[0].values[0], &count)) return false;
    *none = count == 0;
    return true;
}

/* Inserts the first person, with only the fields the strategy names. */
static bool first_account(ClarityDb* db, const struct ClarityPeople* people, const cJSON* info, const Strategy* strategy, const char* name, const cJSON* inputs, int64_t* id) {
    cJSON* args = cJSON_CreateObject();
    cJSON_AddStringToObject(args, "strategy", name);
    cJSON_AddItemToObject(args, "inputs", cJSON_Duplicate(inputs, true));
    cJSON* made = NULL;
    if (!auth_call("firstAccount", args, &made)) return false;
    const char** columns = CLARITY_NEW(const char*, strategy->column_count + 1);
    ClarityValues values = {0};
    size_t count = 0, at = 0;
    const cJSON* field;
    cJSON_ArrayForEach(field, cJSON_GetObjectItemCaseSensitive(info, "fields")) {
        const cJSON* role = cJSON_GetObjectItemCaseSensitive(field, "role");
        const cJSON* value = cJSON_IsString(role) ? cJSON_GetObjectItemCaseSensitive(made, role->valuestring) : NULL;
        if (at < strategy->column_count && cJSON_IsString(value)) {
            columns[count++] = strategy->columns[at];
            clarity_values_add(&values, clarity_text_value(value->valuestring));
        }
        at++;
    }
    ClaritySourceDb* source = NULL;
    if (!clarity_source(db, people->source, &source)) return false;
    const char* sql = clarity_insert_sql(source->driver->dialect, people->table, people->key, columns, count);
    return source->driver->insert(source->driver, sql, &values, id);
}

/* Writes back what Clarity.Auth says to store after a sign-in (a recovery code used up), by role. */
static bool stored_after(ClarityDb* db, const struct ClarityPeople* people, const cJSON* info, const Strategy* strategy, const char* name, const cJSON* inputs, const char* stored, int64_t id) {
    cJSON* args = cJSON_CreateObject();
    cJSON_AddStringToObject(args, "strategy", name);
    cJSON_AddItemToObject(args, "inputs", cJSON_Duplicate(inputs, true));
    cJSON_AddItemToObject(cJSON_AddObjectToObject(args, "stored"), "secret", cJSON_CreateString(stored));
    cJSON* after = NULL;
    if (!auth_call("afterSignin", args, &after)) return false;
    ClaritySourceDb* source = NULL;
    if (!clarity_source(db, people->source, &source)) return false;
    ClarityDialect d = source->driver->dialect;
    const cJSON* value;
    cJSON_ArrayForEach(value, after) {
        const char* column = column_of(info, strategy, value->string);
        if (!column || !cJSON_IsString(value)) continue;
        ClarityValues params = {0};
        clarity_values_add(&params, clarity_text_value(value->valuestring));
        clarity_values_add(&params, clarity_integer_value(id));
        const char* sql = clarity_format("UPDATE %s SET %s = %s WHERE %s = %s", clarity_quote(d, people->table), clarity_quote(d, column), clarity_mark(d, 1), clarity_quote(d, people->key), clarity_mark(d, 2));
        if (!source->driver->execute(source->driver, sql, &params, NULL)) return false;
    }
    return true;
}

static void signed_in(ClarityDb* db, int64_t id) {
    ClarityCall* call = clarity_call();
    if (!call) {
        db->people->me = id;
        return;
    }
    // A new session for the new sign-in; the one it came with is closed.
    if (call->token[0]) session_close(call->token);
    call->token[0] = '\0';
    cJSON* args = cJSON_CreateObject();
    cJSON_AddNumberToObject(args, "record", (double)id);
    cJSON_AddNumberToObject(args, "seconds", (double)db->people->session_seconds);
    const char* token = auth_text("sessionOpen", args);
    if (token && strlen(token) < sizeof call->opened) {
        snprintf(call->opened, sizeof call->opened, "%s", token);
        snprintf(call->token, sizeof call->token, "%s", token);
        call->opened_seconds = db->people->session_seconds;
    }
    call->me = id;
}

static bool refused(void) { return clarity_refuse(clarity_signin_refused()); }

/* The stored secret in `column` of the person `id`, in the scratch arena; NULL when the record is gone, the column empty or unreadable. */
static const char* stored_secret(ClarityDb* db, const struct ClarityPeople* people, const char* column, int64_t id) {
    ClaritySourceDb* source = NULL;
    if (!clarity_source(db, people->source, &source)) return NULL;
    ClarityDialect d = source->driver->dialect;
    const char* sql = clarity_format("SELECT %s FROM %s WHERE %s = %s", clarity_quote(d, column), clarity_quote(d, people->table), clarity_quote(d, people->key), clarity_mark(d, 1));
    ClarityValues params = {0};
    clarity_values_add(&params, clarity_integer_value(id));
    ClarityRows rows = {0};
    if (!source->driver->rows(source->driver, sql, &params, &rows) || rows.count == 0 || rows.items[0].count == 0) return NULL;
    ClarityValue value = rows.items[0].values[0];
    return value.kind == CLARITY_TEXT && value.text && *value.text ? value.text : NULL;
}

/* After a sign-in on the desktop with `remember N days`: an entry signed
 * with the person's stored secret in `column` (of the strategy used), kept
 * in the system's protected store. A strategy without a secret (`account`)
 * remembers nothing; a store that cannot keep it leaves the sign-in as it is. */
/// Whether a sign-in with these inputs is remembered: always, unless the
/// sign-in part asks (`when asked`) and its box's answer is not "true".
static bool remembers(const struct ClarityPeople* people, const cJSON* inputs) {
    if (!people->remember_asks) return true;
    const cJSON* answer = cJSON_GetObjectItemCaseSensitive(inputs, "remember");
    return cJSON_IsString(answer) && strcmp(answer->valuestring, "true") == 0;
}

static void remember(ClarityDb* db, const struct ClarityPeople* people, const char* column, int64_t id) {
    if (!people->remember_seconds || !column || !clarity_secret_store.set) return;
    // Read after signing in: what signing in changed (a recovery code used up) is what holds.
    const char* stored = stored_secret(db, people, column, id);
    if (!stored) return;
    cJSON* args = cJSON_CreateObject();
    cJSON_AddNumberToObject(args, "record", (double)id);
    cJSON_AddNumberToObject(args, "seconds", (double)people->remember_seconds);
    cJSON_AddStringToObject(args, "stored", stored);
    const char* entry = auth_text("rememberMake", args);
    if (entry) clarity_secret_store.set(people->program, CLARITY_REMEMBERED, entry);
}

/* The person of the remembered sign-in when it still holds: the entry names
 * a record whose stored secret of some strategy it was signed with, and it
 * has not expired. 0 for anything else (no entry, no store, the record
 * gone, the secret changed). */
static int64_t remembered(ClarityDb* db, const struct ClarityPeople* people) {
    if (!people->remember_seconds || !clarity_secret_store.get) return 0;
    const char* entry = clarity_secret_store.get(people->program, CLARITY_REMEMBERED);
    if (!entry || !*entry) return 0;
    cJSON* args = cJSON_CreateObject();
    cJSON_AddStringToObject(args, "entry", entry);
    cJSON* out = NULL;
    if (!auth_call("rememberRecord", args, &out) || !cJSON_IsNumber(out) || out->valuedouble <= 0) return 0;
    int64_t record = (int64_t)out->valuedouble;
    for (size_t i = 0; i < people->strategy_count; i++) {
        const cJSON* info = catalog_entry(people->strategies[i].name);
        const char* column = info ? column_of(info, &people->strategies[i], "secret") : NULL;
        const char* stored = column ? stored_secret(db, people, column, record) : NULL;
        if (!stored) continue;
        cJSON* check = cJSON_CreateObject();
        cJSON_AddStringToObject(check, "entry", entry);
        cJSON_AddStringToObject(check, "stored", stored);
        cJSON* holds = NULL;
        if (auth_call("rememberCheck", check, &holds) && cJSON_IsNumber(holds) && (int64_t)holds->valuedouble == record) return record;
    }
    return 0;
}

static void count_failed(const char* const* keys, size_t count) {
    for (size_t i = 0; i < count; i++) {
        cJSON* out = NULL;
        auth_call("triesFailed", key_args(keys[i]), &out);
    }
}

static bool attempt(ClarityDb* db, const char* name, const cJSON* given, bool counted, int64_t* id) {
    struct ClarityPeople* people = db ? db->people : NULL;
    if (!people) return clarity_fail(CLARITY_STORAGE, "the program names no people who sign in");
    if (!name) return refused();
    const cJSON* inputs = cJSON_IsObject(given) ? given : cJSON_CreateObject();
    // Only a strategy the project names, on a target the catalog lets it work on.
    const Strategy* strategy = used(people, name);
    const cJSON* info = strategy ? catalog_entry(name) : NULL;
    if (!info || !holds_text(cJSON_GetObjectItemCaseSensitive(info, "targets"), target(people))) return refused();
    cJSON* args = cJSON_CreateObject();
    cJSON_AddStringToObject(args, "strategy", name);
    cJSON_AddItemToObject(args, "inputs", cJSON_Duplicate(inputs, true));
    cJSON* found = NULL;
    if (!auth_call("find", args, &found)) return refused();
    const cJSON* role = cJSON_GetObjectItemCaseSensitive(found, "role");
    const cJSON* equals = cJSON_GetObjectItemCaseSensitive(found, "equals");
    if (!cJSON_IsString(role) || !cJSON_IsString(equals)) return refused();
    const char* keys[2] = {clarity_format("account:%s", equals->valuestring), NULL};
    size_t key_count = 1;
    ClarityCall* call = clarity_call();
    if (call && call->address[0]) keys[key_count++] = clarity_format("address:%s", call->address);
    if (counted) {
        for (size_t i = 0; i < key_count; i++) {
            cJSON* allowed_args = key_args(keys[i]);
            cJSON_AddNumberToObject(allowed_args, "tries", people->tries);
            cJSON_AddNumberToObject(allowed_args, "window", (double)people->tries_seconds);
            cJSON* allowed = NULL;
            if (!auth_call("triesAllowed", allowed_args, &allowed) || !cJSON_IsTrue(allowed)) return clarity_refuse(clarity_signin_too_many_tries());
        }
    }
    const char* by = column_of(info, strategy, role->valuestring);
    const char* secret = column_of(info, strategy, "secret");
    if (!by) {
        if (counted) count_failed(keys, key_count);
        return refused();
    }
    bool there = false;
    int64_t person = 0;
    const char* stored = "";
    if (!person_by(db, people, by, secret, equals->valuestring, &there, &person, &stored)) return false;
    bool ok = false;
    if (there) {
        cJSON* check = cJSON_CreateObject();
        cJSON_AddStringToObject(check, "strategy", name);
        cJSON_AddItemToObject(check, "inputs", cJSON_Duplicate(inputs, true));
        cJSON_AddItemToObject(cJSON_AddObjectToObject(check, "stored"), "secret", cJSON_CreateString(stored));
        cJSON* holds = NULL;
        ok = auth_call("check", check, &holds) && cJSON_IsTrue(holds) && stored_after(db, people, info, strategy, name, inputs, stored, person);
    } else {
        bool none = false;
        if (!no_people(db, people, &none)) return false;
        ok = none && cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(info, "firstAccount")) && first_account(db, people, info, strategy, name, inputs, &person);
    }
    if (!ok) {
        if (counted) count_failed(keys, key_count);
        return refused();
    }
    if (counted) {
        for (size_t i = 0; i < key_count; i++) {
            cJSON* out = NULL;
            auth_call("triesCleared", key_args(keys[i]), &out);
        }
    }
    signed_in(db, person);
    if (strcmp(target(people), "desktop") == 0 && remembers(people, inputs)) remember(db, people, secret, person);
    *id = person;
    return true;
}

static bool watches_hook(ClarityDb* db, const ClarityEntity* entity) {
    const struct ClarityPeople* people = db->people;
    const char* source = entity->source ? entity->source : "";
    return people && strcmp(entity->table, people->table) == 0 && strcmp(source, people->source) == 0;
}

/* Whether `column` holds a secret of one of the project's strategies. */
static bool secret_column(const struct ClarityPeople* people, const char* column) {
    for (size_t i = 0; i < people->strategy_count; i++) {
        const cJSON* info = catalog_entry(people->strategies[i].name);
        const char* secret = info ? column_of(info, &people->strategies[i], "secret") : NULL;
        if (secret && strcmp(secret, column) == 0) return true;
    }
    return false;
}

/* A save changed a person's secret: served, the person is signed out
 * everywhere else (their own change) or everywhere (someone else's). The
 * desktop and the command line have no other sessions. */
static void changed_hook(ClarityDb* db, const ClarityEntity* entity, int64_t id, const char* const* columns, size_t count) {
    (void)entity;
    ClarityCall* call = clarity_call();
    if (!call || !db->people) return;
    bool secret = false;
    for (size_t i = 0; i < count && !secret; i++) secret = secret_column(db->people, columns[i]);
    if (!secret) return;
    cJSON* args = cJSON_CreateObject();
    cJSON_AddNumberToObject(args, "record", (double)id);
    cJSON* out = NULL;
    if (call->me == id && call->token[0]) {
        cJSON_AddStringToObject(args, "token", call->token);
        auth_call("sessionCloseOthers", args, &out);
    } else {
        auth_call("sessionCloseAll", args, &out);
    }
}

bool clarity_signin(ClarityDb* db, const char* strategy, const cJSON* inputs, int64_t* id) {
    int64_t person = 0;
    if (!attempt(db, strategy, inputs, true, &person)) return false;
    if (id) *id = person;
    return true;
}

void clarity_signout(ClarityDb* db) {
    ClarityCall* call = clarity_call();
    if (!call) {
        if (db && db->people) {
            db->people->me = 0;
            if (db->people->remember_seconds && clarity_secret_store.set) clarity_secret_store.set(db->people->program, CLARITY_REMEMBERED, "");
        }
        return;
    }
    if (call->token[0]) session_close(call->token);
    call->token[0] = '\0';
    call->opened[0] = '\0';
    call->me = 0;
    call->closed = true;
}

const char* clarity_signin_first(ClarityDb* db) {
    struct ClarityPeople* people = db ? db->people : NULL;
    if (!people) return NULL;
    const char* where = target(people);
    if (!on(people, where) || clarity_me_id(db) > 0) return NULL;
    int64_t id = 0;
    if (strcmp(where, "desktop") == 0 && (id = remembered(db, people)) > 0) {
        people->me = id;
        return NULL;
    }
    if (strcmp(where, "desktop") == 0 && used(people, "account") && attempt(db, "account", NULL, false, &id)) return NULL;
    return clarity_copy(people->page);
}

// -- The command line --------------------------------------------------------

ClarityCliSignin clarity_cli_signin_args(int argc, char** argv) {
    ClarityCliSignin given = {NULL, NULL};
    for (int i = 1; argv && i + 1 < argc; i++) {
        if (strcmp(argv[i], "--user") == 0 && !given.user) given.user = clarity_copy(argv[i + 1]);
        if (strcmp(argv[i], "--token") == 0 && !given.token) given.token = clarity_copy(argv[i + 1]);
    }
    if (given.token && !*given.token) given.token = NULL;
    if (!given.token) {
        const char* told = getenv("CLARITY_TOKEN");
        if (told && *told) given.token = clarity_copy(told);
    }
    return given;
}

bool clarity_signin_cli(ClarityDb* db, const ClarityCliSignin* given, const char* (*password)(const char* prompt), const char* prompt, int64_t* id, bool* signed_in) {
    *signed_in = false;
    struct ClarityPeople* people = db ? db->people : NULL;
    if (!people || !on(people, "cli")) return true;
    people->cli = true;
    *signed_in = true;
    cJSON* inputs = cJSON_CreateObject();
    if (given && given->token && used(people, "token")) {
        cJSON_AddStringToObject(inputs, "token", given->token);
        return clarity_signin(db, "token", inputs, id);
    }
    if (given && given->user && used(people, "password")) {
        const char* typed = password ? password(prompt ? prompt : "") : NULL;
        if (!typed) return refused();
        cJSON_AddStringToObject(inputs, "name", given->user);
        cJSON_AddStringToObject(inputs, "password", typed);
        return clarity_signin(db, "password", inputs, id);
    }
    if (used(people, "account")) return clarity_signin(db, "account", inputs, id);
    return refused();
}

const char* clarity_read_password(const char* prompt) {
    ClarityCliTerminal console = clarity_cli_console();
    if (!console.is_terminal(console.context)) return NULL;
    console.write(console.context, prompt ? prompt : "");
#if defined(_WIN32)
    HANDLE input = GetStdHandle(STD_INPUT_HANDLE);
    DWORD mode = 0;
    bool quiet = GetConsoleMode(input, &mode) && SetConsoleMode(input, mode & ~(DWORD)ENABLE_ECHO_INPUT);
    const char* line = console.read_line(console.context);
    if (quiet) SetConsoleMode(input, mode);
#else
    struct termios was;
    bool quiet = tcgetattr(STDIN_FILENO, &was) == 0;
    if (quiet) {
        struct termios silent = was;
        silent.c_lflag &= ~(tcflag_t)ECHO;
        quiet = tcsetattr(STDIN_FILENO, TCSAFLUSH, &silent) == 0;
    }
    const char* line = console.read_line(console.context);
    if (quiet) tcsetattr(STDIN_FILENO, TCSAFLUSH, &was);
#endif
    console.write(console.context, "\n");
    return line;
}
