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
    /// Key and value in turn, option_count pairs.
    char** options;
    size_t option_count;
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

/* The text `name` of an object, NULL when it has none (or null). */
static const char* text_of(const cJSON* object, const char* name) {
    const cJSON* item = cJSON_GetObjectItemCaseSensitive(object, name);
    return cJSON_IsString(item) ? item->valuestring : NULL;
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
        free_list(people->strategies[i].options, people->strategies[i].option_count * 2);
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
                people->strategies[i].options = copied_list(given->options, given->option_count * 2, &ok);
                people->strategies[i].option_count = people->strategies[i].options ? given->option_count : 0;
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

/* The project's `signin` block as Clarity.Auth's sign-in calls take it. */
cJSON* clarity_signin_options(const char* program, const ClaritySigninStrategy* strategy) {
    cJSON* options = cJSON_CreateObject();
    for (size_t i = 0; strategy && i < strategy->option_count; i++) {
        const char* key = strategy->options[2 * i];
        const char* value = strategy->options[2 * i + 1];
        if (strcmp(key, "secret") == 0 && strncmp(value, "setting:", 8) == 0) {
            const char* kept = clarity_secret_store.get ? clarity_secret_store.get(program ? program : "", value + 8) : NULL;
            value = kept ? kept : "";
        }
        cJSON_AddStringToObject(options, key, value);
    }
    return options;
}

static cJSON* block_of(const struct ClarityPeople* people) {
    cJSON* block = cJSON_CreateObject();
    cJSON* strategies = cJSON_AddArrayToObject(block, "strategies");
    for (size_t i = 0; i < people->strategy_count; i++) {
        cJSON* strategy = cJSON_CreateObject();
        cJSON_AddStringToObject(strategy, "name", people->strategies[i].name);
        cJSON* columns = cJSON_AddArrayToObject(strategy, "columns");
        for (size_t c = 0; c < people->strategies[i].column_count; c++) cJSON_AddItemToArray(columns, cJSON_CreateString(people->strategies[i].columns[c]));
        const Strategy* kept = &people->strategies[i];
        ClaritySigninStrategy given = {kept->name, (const char* const*)kept->columns, kept->column_count, (const char* const*)kept->options, kept->option_count};
        cJSON_AddItemToObject(strategy, "options", clarity_signin_options(people->program, &given));
        cJSON_AddItemToArray(strategies, strategy);
    }
    cJSON_AddNumberToObject(block, "tries", people->tries);
    cJSON_AddNumberToObject(block, "triesSeconds", (double)people->tries_seconds);
    return block;
}

/* Clarity's refusal by the key Clarity.Auth gave. */
static bool refused_by(const char* key) {
    if (key && strcmp(key, "signinTooManyTries") == 0) return clarity_refuse(clarity_signin_too_many_tries());
    return refused();
}

/* Writes `store`, values by column, to the person's record `id`, in one UPDATE. */
static bool store_columns(ClarityDb* db, const struct ClarityPeople* people, const cJSON* store, int64_t id) {
    ClaritySourceDb* source = NULL;
    if (!clarity_source(db, people->source, &source)) return false;
    ClarityDialect d = source->driver->dialect;
    ClarityValues params = {0};
    const char* sets = NULL;
    const cJSON* value;
    cJSON_ArrayForEach(value, store) {
        if (!cJSON_IsString(value)) continue;
        clarity_values_add(&params, clarity_text_value(value->valuestring));
        const char* set = clarity_format("%s = %s", clarity_quote(d, value->string), clarity_mark(d, (int)params.count));
        sets = sets ? clarity_format("%s, %s", sets, set) : set;
    }
    if (!sets) return true;
    clarity_values_add(&params, clarity_integer_value(id));
    const char* sql = clarity_format("UPDATE %s SET %s WHERE %s = %s", clarity_quote(d, people->table), sets, clarity_quote(d, people->key), clarity_mark(d, (int)params.count));
    return source->driver->execute(source->driver, sql, &params, NULL);
}

/* Inserts a person with only `columns` (values by column); the other columns take their defaults. */
static bool make_person(ClarityDb* db, const struct ClarityPeople* people, const cJSON* columns, int64_t* id) {
    const char** names = CLARITY_NEW(const char*, (size_t)cJSON_GetArraySize(columns) + 1);
    ClarityValues values = {0};
    size_t count = 0;
    const cJSON* value;
    cJSON_ArrayForEach(value, columns) {
        names[count++] = value->string;
        clarity_values_add(&values, clarity_text_value(cJSON_IsString(value) ? value->valuestring : ""));
    }
    ClaritySourceDb* source = NULL;
    if (!clarity_source(db, people->source, &source)) return false;
    const char* sql = clarity_insert_sql(source->driver->dialect, people->table, people->key, names, count);
    return source->driver->insert(source->driver, sql, &values, id);
}

/* One sign-in attempt: Clarity.Auth decides (signinBegin, signinDecide);
 * this looks the person up and stores what it is told. An attempt not
 * `counted` (the computer's user signed in at the start) counts no tries. */
static bool attempt(ClarityDb* db, const char* name, const cJSON* given, bool counted, int64_t* id) {
    struct ClarityPeople* people = db ? db->people : NULL;
    if (!people) return clarity_fail(CLARITY_STORAGE, "the program names no people who sign in");
    if (!name) return refused();
    const cJSON* inputs = cJSON_IsObject(given) ? given : cJSON_CreateObject();
    const char* where = target(people);
    cJSON* args = cJSON_CreateObject();
    cJSON_AddItemToObject(args, "block", block_of(people));
    cJSON_AddStringToObject(args, "strategy", name);
    cJSON_AddItemToObject(args, "inputs", cJSON_Duplicate(inputs, true));
    cJSON_AddStringToObject(args, "target", where);
    ClarityCall* call = clarity_call();
    if (call && call->address[0]) cJSON_AddStringToObject(args, "address", call->address);
    cJSON* begun = NULL;
    if (!auth_call("signinBegin", args, &begun)) return false;
    if (cJSON_GetObjectItemCaseSensitive(begun, "refused")) return refused_by(text_of(begun, "refused"));
    const char* column = text_of(begun, "column");
    const char* secret = text_of(begun, "secret");
    const char* equals = text_of(begun, "equals");
    if (!column || !equals) return refused();
    bool there = false;
    int64_t person = 0;
    const char* stored = "";
    if (!person_by(db, people, column, secret, equals, &there, &person, &stored)) return false;
    bool none = false;
    if (!there && !no_people(db, people, &none)) return false;
    cJSON* decide = cJSON_CreateObject();
    cJSON_AddItemToObject(decide, "block", block_of(people));
    cJSON_AddStringToObject(decide, "strategy", name);
    cJSON_AddItemToObject(decide, "inputs", cJSON_Duplicate(inputs, true));
    const cJSON* keys = cJSON_GetObjectItemCaseSensitive(begun, "keys");
    cJSON_AddItemToObject(decide, "keys", counted && cJSON_IsArray(keys) ? cJSON_Duplicate(keys, true) : cJSON_CreateArray());
    if (there) cJSON_AddStringToObject(cJSON_AddObjectToObject(decide, "found"), "stored", stored);
    else cJSON_AddNullToObject(decide, "found");
    cJSON_AddBoolToObject(decide, "noPeople", none);
    cJSON* decided = NULL;
    if (!auth_call("signinDecide", decide, &decided)) return false;
    const cJSON* make = cJSON_GetObjectItemCaseSensitive(decided, "make");
    if (cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(decided, "signedIn"))) {
        if (!store_columns(db, people, cJSON_GetObjectItemCaseSensitive(decided, "store"), person)) return false;
    } else if (cJSON_IsObject(make)) {
        // A first account the table cannot take (a required column without a default) is an ordinary refusal.
        if (!make_person(db, people, make, &person)) return refused();
    } else {
        return refused_by(text_of(decided, "refused"));
    }
    signed_in(db, person);
    if (strcmp(where, "desktop") == 0 && remembers(people, inputs)) remember(db, people, secret, person);
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
