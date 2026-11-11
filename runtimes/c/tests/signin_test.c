// Who is using the program on the C runtime (conformance/signin.json):
// each strategy's success and refusal, the first account, failed tries, and
// served calls carrying the session cookie the program set last. Then the
// web server itself: the cookie on the wire, and a call without a session
// reaching only the sign-in window.

#include <stddef.h>
#include <stdlib.h>
#include <string.h>

#include "auth/auth.h"
#include "check.h"
#include "clarity/db_internal.h"
#include "clarity/json.h"
#include "clarity/settings.h"
#include "clarity/sql.h"
#include "clarity/text.h"
#include "service/serve.h"
#include "tests.h"

/* The conformance file's signin block, as the generated program writes it. */
typedef struct {
    const char* targets[3];
    ClaritySigninStrategy strategies[8];
    const char* columns[8][4];
    ClaritySigninSource source;
} Block;

static void block_of(const cJSON* file, Block* b) {
    const cJSON* people = cJSON_GetObjectItem(file, "people");
    const cJSON* signin = cJSON_GetObjectItem(file, "signin");
    memset(b, 0, sizeof *b);
    size_t targets = 0, strategies = 0;
    const cJSON* item;
    cJSON_ArrayForEach(item, cJSON_GetObjectItem(signin, "targets")) if (targets < 3) b->targets[targets++] = item->valuestring;
    cJSON_ArrayForEach(item, cJSON_GetObjectItem(signin, "strategies")) {
        if (strategies >= 8) break;
        size_t columns = 0;
        const cJSON* column;
        cJSON_ArrayForEach(column, cJSON_GetObjectItem(item, "columns")) if (columns < 4) b->columns[strategies][columns++] = column->valuestring;
        b->strategies[strategies] = (ClaritySigninStrategy){cJSON_GetObjectItem(item, "name")->valuestring, b->columns[strategies], columns};
        strategies++;
    }
    b->source = (ClaritySigninSource){
        .source = "",
        .table = cJSON_GetObjectItem(people, "table")->valuestring,
        .key = cJSON_GetObjectItem(people, "key")->valuestring,
        .page = cJSON_GetObjectItem(signin, "page")->valuestring,
        .targets = b->targets,
        .target_count = targets,
        .strategies = b->strategies,
        .strategy_count = strategies,
        .session_seconds = (uint64_t)cJSON_GetObjectItem(signin, "sessionMinutes")->valuedouble * 60,
        .tries = (uint32_t)cJSON_GetObjectItem(signin, "tries")->valuedouble,
        .tries_seconds = (uint64_t)cJSON_GetObjectItem(signin, "triesMinutes")->valuedouble * 60,
        .program = cJSON_GetObjectItem(signin, "program")->valuestring,
    };
}

/* The secret store of a case, in memory: `remember N days` keeps its entry here. */
static char kept_secret[8][2][256];

static const char* memory_get(const char* program, const char* name) {
    const char* key = clarity_format("%s:%s", program, name);
    for (size_t i = 0; i < 8; i++)
        if (strcmp(kept_secret[i][0], key) == 0) return clarity_copy(kept_secret[i][1]);
    return NULL;
}

static bool memory_set(const char* program, const char* name, const char* value) {
    const char* key = clarity_format("%s:%s", program, name);
    for (size_t i = 0; i < 8; i++)
        if (strcmp(kept_secret[i][0], key) == 0 || !kept_secret[i][0][0]) {
            snprintf(kept_secret[i][0], sizeof kept_secret[i][0], "%s", key);
            snprintf(kept_secret[i][1], sizeof kept_secret[i][1], "%s", value);
            return true;
        }
    return false;
}

/* A row of the case into the table: a value {hash, text} stored as that hash of the text. */
static bool insert_row(ClarityDb* db, const char* table, const cJSON* row) {
    ClarityBuffer names = {0}, values = {0};
    const cJSON* field;
    cJSON_ArrayForEach(field, row) {
        const char* text = cJSON_IsString(field) ? field->valuestring : NULL;
        if (cJSON_IsObject(field) && !clarity_hashed(cJSON_GetObjectItem(field, "hash")->valuestring, cJSON_GetObjectItem(field, "text")->valuestring, 0, &text)) return false;
        clarity_buffer_addf(&names, "%s%s", names.length ? ", " : "", field->string);
        clarity_buffer_addf(&values, "%s'%s'", values.length ? ", " : "", text);
    }
    return clarity_db_batch(db, "", clarity_format("INSERT INTO %s (%s) VALUES (%s)", table, clarity_buffer_text(&names), clarity_buffer_text(&values)));
}

static int64_t count_of(ClarityDb* db, const char* table) {
    ClaritySourceDb* source = NULL;
    ClarityRows rows = {0};
    int64_t count = -1;
    if (clarity_source(db, "", &source) && source->driver->rows(source->driver, clarity_format("SELECT COUNT(*) FROM %s", table), NULL, &rows) && rows.count == 1)
        clarity_get_integer("count", rows.items[0].values[0], &count);
    return count;
}

/* The tests after these run without people, as a program that names none. */
static void no_people(void) {
    clarity_people_set(NULL);
    clarity_people_set_web_page(NULL);
}

/* The key of the refusal the last call left, "" when it is none. */
static const char* refusal(void) {
    const ClarityError* e = clarity_error();
    return e->kind == CLARITY_REFUSED && e->text && e->text->key ? e->text->key : "";
}

/* The table of people as an entity: every column of the file's CREATE TABLE but its key, each value kept as it is. */
typedef struct {
    int64_t id;
    ClarityValue values[16];
} Person;

static size_t person_column_count;

static void person_values(const void* record, ClarityValue* out) { memcpy(out, ((const Person*)record)->values, person_column_count * sizeof *out); }

static bool person_from_values(int64_t id, const ClarityValue* values, void* record) {
    Person* p = record;
    p->id = id;
    memcpy(p->values, values, person_column_count * sizeof *values);
    return true;
}

static ClarityEntity people_entity(const cJSON* file) {
    const cJSON* people = cJSON_GetObjectItem(file, "people");
    const char* create = strchr(cJSON_GetObjectItem(people, "create")->valuestring, '(');
    const char* key = cJSON_GetObjectItem(people, "key")->valuestring;
    const char** columns = CLARITY_NEW(const char*, 16);
    size_t count = 0;
    for (const char* part = create ? create + 1 : ""; *part && count < 16;) {
        while (*part == ' ') part++;
        size_t length = strcspn(part, " ,)");
        const char* column = clarity_format("%.*s", (int)length, part);
        if (strcmp(column, key) != 0) columns[count++] = column;
        const char* comma = strchr(part, ',');
        if (!comma) break;
        part = comma + 1;
    }
    person_column_count = count;
    return (ClarityEntity){.name = "Member", .table = cJSON_GetObjectItem(people, "table")->valuestring, .key = key, .source = "", .columns = columns, .column_count = count,
                           .size = sizeof(Person), .id_at = offsetof(Person, id), .values = person_values, .from_values = person_from_values};
}

/* A text of the file, or {hash, text} kept as that hash; NULL when hashing failed. */
static const char* stored_text(const cJSON* value) {
    if (cJSON_IsString(value)) return value->valuestring;
    const char* hash = NULL;
    return clarity_hashed(cJSON_GetObjectItem(value, "hash")->valuestring, cJSON_GetObjectItem(value, "text")->valuestring, 0, &hash) ? hash : NULL;
}

/* Saves the person `save` names with the values it gives. */
static bool saved(ClarityDb* db, const ClarityEntity* entity, const cJSON* save) {
    Person p;
    if (!clarity_get(db, entity, (int64_t)cJSON_GetObjectItem(save, "id")->valuedouble, &p)) return false;
    const cJSON* field;
    cJSON_ArrayForEach(field, save) {
        if (strcmp(field->string, "id") == 0) continue;
        const char* text = stored_text(field);
        if (!text) return false;
        for (size_t i = 0; i < entity->column_count; i++)
            if (strcmp(entity->columns[i], field->string) == 0) p.values[i] = clarity_text_value(text);
    }
    return clarity_save(db, entity, &p);
}

/* Whether the last call was refused with `key`, in the words the file gives (its slots filled from `values`). */
static void check_refused(const cJSON* file, const char* where, bool done, const char* key) {
    if (done || strcmp(refusal(), key) != 0) {
        check_fail(__FILE__, __LINE__, clarity_format("%s: refused %s, got '%s' (%s)", where, key, done ? "done" : refusal(), done ? "" : clarity_error()->message));
        return;
    }
    const cJSON* english = cJSON_GetObjectItem(clarity_text_json(clarity_error()->text), "text");
    const cJSON* words = cJSON_GetObjectItem(cJSON_GetObjectItem(file, "refusals"), key);
    if (!cJSON_IsString(english) || !cJSON_IsString(words) || strcmp(english->valuestring, words->valuestring) != 0) check_fail(__FILE__, __LINE__, clarity_format("%s: the words of %s", where, key));
}

void signing_in_is_as_every_runtime_does(void) {
    cJSON* file = conformance("signin.json");
    REQUIRE(file);
    Block block;
    block_of(file, &block);
    const char* table = block.source.table;
    const cJSON* c;
    cJSON_ArrayForEach(c, cJSON_GetObjectItem(file, "cases")) {
        const char* name = cJSON_GetObjectItem(c, "name")->valuestring;
        bool served = cJSON_IsTrue(cJSON_GetObjectItem(c, "served"));
        const cJSON* at = cJSON_GetObjectItem(c, "address");
        // remember N days keeps its entry in the secret store, here in memory, empty per case.
        ClaritySigninSource source = block.source;
        const cJSON* days = cJSON_GetObjectItem(c, "rememberDays");
        source.remember_seconds = cJSON_IsNumber(days) ? (uint64_t)days->valuedouble * 24 * 3600 : 0;
        // remember N days when asked keeps it only when the input remember is "true".
        source.remember_asks = cJSON_IsTrue(cJSON_GetObjectItem(c, "rememberAsks"));
        memset(kept_secret, 0, sizeof kept_secret);
        clarity_secret_store = (ClaritySecretStore){memory_get, memory_set};
        ClarityDb* db = NULL;
        if (!clarity_db_memory(&db) || !clarity_db_batch(db, "", cJSON_GetObjectItem(cJSON_GetObjectItem(file, "people"), "create")->valuestring) ||
            !clarity_db_add_signin(db, &source)) {
            check_fail(__FILE__, __LINE__, clarity_format("%s: %s", name, clarity_error()->message));
            clarity_db_close(db);
            continue;
        }
        const cJSON* row;
        cJSON_ArrayForEach(row, cJSON_GetObjectItem(c, "rows")) if (!insert_row(db, table, row)) check_fail(__FILE__, __LINE__, clarity_format("%s: row: %s", name, clarity_error()->message));
        ClarityEntity entity = people_entity(file);
        // The cookie each browser carries: the session the program set it last.
        char cookies[8][256] = {{0}};
        int step_at = 0;
        const cJSON* step;
        cJSON_ArrayForEach(step, cJSON_GetObjectItem(c, "steps")) {
            step_at++;
            const char* where = clarity_format("%s, step %d", name, step_at);
            ClarityCall call;
            const cJSON* browser = cJSON_GetObjectItem(step, "browser");
            int which = cJSON_IsString(browser) ? (browser->valuestring[0] - 'a') & 7 : 0;
            char* cookie = cookies[which];
            if (served) {
                clarity_call_open(&call, cookie, cJSON_IsString(at) ? at->valuestring : "");
                clarity_call_enter(&call);
            }
            bool failed = false;
            const cJSON* strategy = cJSON_GetObjectItem(step, "signin");
            if (cJSON_IsString(strategy)) {
                int64_t id = 0;
                bool in = clarity_signin(db, strategy->valuestring, cJSON_GetObjectItem(step, "inputs"), &id);
                failed = !in;
                const cJSON* refused = cJSON_GetObjectItem(step, "refused");
                if (refused) {
                    check_refused(file, where, in, refused->valuestring);
                } else if (!in || id != (int64_t)cJSON_GetObjectItem(step, "me")->valuedouble) {
                    check_fail(__FILE__, __LINE__, clarity_format("%s: signed in %lld, %s", where, (long long)id, in ? "" : clarity_error()->message));
                }
            }
            if (cJSON_IsTrue(cJSON_GetObjectItem(step, "signout"))) clarity_signout(db);
            const cJSON* hashed = cJSON_GetObjectItem(step, "hashed");
            if (hashed) {
                const cJSON* minimum = cJSON_GetObjectItem(hashed, "minimum");
                const char* text = cJSON_GetObjectItem(hashed, "text")->valuestring;
                const char* hash = NULL;
                bool kept = clarity_hashed(cJSON_GetObjectItem(hashed, "method")->valuestring, text, minimum ? (int64_t)minimum->valuedouble : 0, &hash);
                const cJSON* refused = cJSON_GetObjectItem(step, "refused");
                if (refused) {
                    check_refused(file, where, kept, refused->valuestring);
                    const cJSON* slot = cJSON_GetObjectItem(step, "minimum");
                    const char* json = kept ? "" : clarity_json_of(clarity_text_json(clarity_error()->text));
                    if (slot && !strstr(json, clarity_format("\"value\":%d", (int)slot->valuedouble))) check_fail(__FILE__, __LINE__, clarity_format("%s: the slot minimum in %s", where, json));
                } else if (!kept || strcmp(hash, text) == 0) {
                    check_fail(__FILE__, __LINE__, clarity_format("%s: not hashed: %s", where, kept ? hash : clarity_error()->message));
                }
            }
            const cJSON* save = cJSON_GetObjectItem(step, "save");
            if (save && !saved(db, &entity, save)) check_fail(__FILE__, __LINE__, clarity_format("%s: save: %s", where, clarity_error()->message));
            const cJSON* me = cJSON_GetObjectItem(step, "me");
            if (me && !cJSON_IsString(strategy)) {
                int64_t want = cJSON_IsNumber(me) ? (int64_t)me->valuedouble : 0;
                if (clarity_me_id(db) != want) check_fail(__FILE__, __LINE__, clarity_format("%s: me %lld, not %lld", where, (long long)clarity_me_id(db), (long long)want));
            }
            if (cJSON_GetObjectItem(step, "restart")) {
                // The program starts anew on the same database and secret store.
                const cJSON* page = cJSON_GetObjectItem(step, "page");
                const char* first = clarity_db_add_signin(db, &source) ? clarity_signin_first(db) : "(not named)";
                bool same = cJSON_IsString(page) ? first && strcmp(first, page->valuestring) == 0 : first == NULL;
                if (!same) check_fail(__FILE__, __LINE__, clarity_format("%s: starts with %s", where, first ? first : "nothing"));
            }
            const cJSON* remembered = cJSON_GetObjectItem(step, "remembered");
            if (remembered) {
                const char* entry = memory_get(source.program, CLARITY_REMEMBERED);
                bool held = entry && *entry;
                if (held != cJSON_IsTrue(remembered)) check_fail(__FILE__, __LINE__, clarity_format("%s: remembered %d", where, held));
            }
            const cJSON* count = cJSON_GetObjectItem(step, "count");
            if (count && count_of(db, table) != (int64_t)count->valuedouble) check_fail(__FILE__, __LINE__, clarity_format("%s: count %lld", where, (long long)count_of(db, table)));
            if (served) {
                clarity_call_enter(NULL);
                const char* set = clarity_call_cookie(&call, failed);
                if (set) {
                    const char* end = strchr(set, ';');
                    snprintf(cookie, sizeof cookies[0], "%.*s", (int)(end ? end - set : (long)strlen(set)), set);
                }
            }
        }
        clarity_db_close(db);
    }
    clarity_secret_store = (ClaritySecretStore){0};
    no_people();
}

void allow_readable_and_hashed_say_clarity_words(void) {
    CHECK(clarity_allow(true));
    CHECK(!clarity_allow(false));
    CHECK_TEXT(refusal(), "notAllowed");
    CHECK(clarity_yours(true));
    CHECK(!clarity_yours(false));
    CHECK_TEXT(refusal(), "notYours");
    const char* hash = NULL;
    REQUIRE(clarity_hashed("sha256", "abc", 0, &hash));
    CHECK(strlen(hash) > 0 && strcmp(hash, "abc") != 0);
    CHECK(!clarity_hashed("rot13", "abc", 0, &hash));
    const char *one = NULL, *two = NULL;
    REQUIRE(clarity_new_token(&one) && clarity_new_token(&two));
    CHECK(strlen(one) >= 32 && strcmp(one, two) != 0);
    // Nobody signed in: `me` is refused, and a program without people has nobody.
    ClarityDb* db = NULL;
    REQUIRE(clarity_db_memory(&db));
    CHECK_INT(clarity_me_id(db), 0);
    CHECK(clarity_signin_first(db) == NULL);
    int64_t id = 0;
    CHECK(!clarity_signin(db, "password", cJSON_CreateObject(), &id));
    clarity_db_close(db);
    // Bad input never crashes: no inputs, an unknown strategy, a strategy the project does not name.
    static const char* const targets[] = {"desktop"};
    static const char* const columns[] = {"email", "password_hash"};
    static const ClaritySigninStrategy strategies[] = {{"password", columns, 2}};
    ClaritySigninSource source = {.table = "members", .page = "SignIn", .targets = targets, .target_count = 1, .strategies = strategies, .strategy_count = 1};
    REQUIRE(clarity_db_memory(&db));
    REQUIRE(clarity_db_batch(db, "", "CREATE TABLE members (id INTEGER PRIMARY KEY, email TEXT, password_hash TEXT)"));
    REQUIRE(clarity_db_add_signin(db, &source));
    CHECK(!clarity_signin(db, "password", NULL, &id));
    CHECK_TEXT(refusal(), "signinRefused");
    CHECK(!clarity_signin(db, "nonsense", cJSON_CreateObject(), &id));
    CHECK_TEXT(refusal(), "signinRefused");
    CHECK(!clarity_signin(db, "token", clarity_json_object(1, "token", cJSON_CreateString("x")), &id));
    CHECK_TEXT(refusal(), "signinRefused");
    // The desktop starts with the sign-in window while nobody is signed in.
    CHECK_TEXT(clarity_signin_first(db), "SignIn");
    clarity_db_close(db);
    // With `account` the desktop signs the computer's user in by itself at the start.
    static const char* const login[] = {"login"};
    static const ClaritySigninStrategy by_account[] = {{"account", login, 1}};
    source.strategies = by_account;
    REQUIRE(clarity_db_memory(&db));
    REQUIRE(clarity_db_batch(db, "", "CREATE TABLE members (id INTEGER PRIMARY KEY, login TEXT)"));
    REQUIRE(clarity_db_add_signin(db, &source));
    CHECK(clarity_signin_first(db) == NULL);
    CHECK_INT(clarity_me_id(db), 1);
    clarity_signout(db);
    CHECK(clarity_signin_first(db) == NULL);
    CHECK_INT(clarity_me_id(db), 1);
    CHECK_INT(count_of(db, "members"), 1);
    clarity_db_close(db);
    no_people();
}

static const char* typed_password(const char* prompt) {
    (void)prompt;
    return "cli secret";
}

static const char* no_password(const char* prompt) {
    (void)prompt;
    return NULL;
}

void a_command_line_signs_in_before_its_command(void) {
    char* argv[] = {"tool", "list", "--user", "cli@signin.test", "--token", "the-cli-token"};
    ClarityCliSignin given = clarity_cli_signin_args(6, argv);
    CHECK_TEXT(given.user, "cli@signin.test");
    CHECK_TEXT(given.token, "the-cli-token");
    static const char* const targets[] = {"cli"};
    static const char* const password_columns[] = {"email", "password_hash"};
    static const char* const token_columns[] = {"api_token"};
    static const ClaritySigninStrategy strategies[] = {{"password", password_columns, 2}, {"token", token_columns, 1}};
    ClaritySigninSource source = {.table = "members", .page = "SignIn", .targets = targets, .target_count = 1, .strategies = strategies, .strategy_count = 2};
    ClarityDb* db = NULL;
    REQUIRE(clarity_db_memory(&db));
    REQUIRE(clarity_db_batch(db, "", "CREATE TABLE members (id INTEGER PRIMARY KEY, email TEXT, password_hash TEXT, api_token TEXT)"));
    REQUIRE(clarity_db_add_signin(db, &source));
    const char *hash = NULL, *token = NULL;
    REQUIRE(clarity_hashed("argon2id", "cli secret", 0, &hash) && clarity_hashed("sha256", "the-cli-token", 0, &token));
    REQUIRE(clarity_db_batch(db, "", clarity_format("INSERT INTO members (email, password_hash, api_token) VALUES ('cli@signin.test', '%s', '%s')", hash, token)));
    int64_t id = 0;
    bool signed_in = false;
    // The token first, when given.
    CHECK(clarity_signin_cli(db, &given, typed_password, "Password: ", &id, &signed_in));
    CHECK(signed_in);
    CHECK_INT(id, 1);
    clarity_signout(db);
    // --user with the password typed.
    ClarityCliSignin user = {"cli@signin.test", NULL};
    CHECK(clarity_signin_cli(db, &user, typed_password, "Password: ", &id, &signed_in));
    CHECK_INT(clarity_me_id(db), 1);
    clarity_signout(db);
    // No password typed, nothing given: refused.
    CHECK(!clarity_signin_cli(db, &user, no_password, "Password: ", &id, &signed_in));
    CHECK_TEXT(refusal(), "signinRefused");
    ClarityCliSignin nothing = {NULL, NULL};
    CHECK(!clarity_signin_cli(db, &nothing, no_password, "Password: ", &id, &signed_in));
    CHECK_TEXT(refusal(), "signinRefused");
    clarity_db_close(db);
    // People who do not sign in on the command line: nothing to do.
    static const char* const desktop[] = {"desktop"};
    source.targets = desktop;
    REQUIRE(clarity_db_memory(&db));
    REQUIRE(clarity_db_add_signin(db, &source));
    CHECK(clarity_signin_cli(db, &nothing, no_password, "Password: ", &id, &signed_in));
    CHECK(!signed_in);
    clarity_db_close(db);
    no_people();
}

// -- The web server ----------------------------------------------------------

static bool open_window(ClarityDb* db, const char* window, const cJSON* with, ClarityWindowState** out) {
    (void)db;
    *out = clarity_opened(window, cJSON_CreateObject(), with);
    return true;
}

static const char* no_handler(const char* window, const char* event) {
    (void)window;
    (void)event;
    return NULL;
}

static bool no_dispatch(ClarityDb* db, const ClarityWindowState* state, const char* handler, ClarityReply** out) {
    (void)db;
    (void)state;
    (void)out;
    return clarity_fail(CLARITY_INVALID, "no handler %s", handler);
}

static bool no_redraw(ClarityDb* db, ClarityReply* reply) {
    (void)db;
    (void)reply;
    return true;
}

static const ClarityWindows windows = {open_window, no_handler, no_dispatch, NULL, NULL, no_redraw, NULL, NULL};

static const char* posted(ClarityServer* s, const char* cookie, const char* body) {
    const char* request = clarity_format("POST /clarity/invoke HTTP/1.1\r\nHost: x\r\n%s%s%sContent-Length: %zu\r\n\r\n%s", cookie ? "Cookie: " : "", cookie ? cookie : "", cookie ? "\r\n" : "",
                                         strlen(body), body);
    size_t length = 0;
    return clarity_serve_answer_from(s, request, strlen(request), "198.51.100.7", &length);
}

static const char* body_of(const char* reply) {
    const char* end = strstr(reply, "\r\n\r\n");
    return end ? end + 4 : "";
}

/* The value of the reply's Set-Cookie header up to its first ';', "" for none. */
static const char* cookie_set(const char* reply) {
    const char* header = strstr(reply, "\r\nSet-Cookie: ");
    const char* end = strstr(reply, "\r\n\r\n");
    if (!header || header > end) return "";
    header += 14;
    return clarity_format("%.*s", (int)(strcspn(header, ";\r")), header);
}

void a_served_program_signs_in_with_a_session_cookie(void) {
    static const char* const targets[] = {"web"};
    static const char* const columns[] = {"email", "password_hash"};
    static const ClaritySigninStrategy strategies[] = {{"password", columns, 2}};
    ClaritySigninSource source = {.table = "members", .page = "SignIn", .targets = targets, .target_count = 1, .strategies = strategies, .strategy_count = 1, .session_seconds = 600};
    ClarityDb* db = NULL;
    REQUIRE(clarity_db_memory(&db));
    REQUIRE(clarity_db_batch(db, "", "CREATE TABLE members (id INTEGER PRIMARY KEY, email TEXT, password_hash TEXT)"));
    const char* hash = NULL;
    REQUIRE(clarity_hashed("argon2id", "plum", 0, &hash));
    REQUIRE(clarity_db_batch(db, "", clarity_format("INSERT INTO members (email, password_hash) VALUES ('web@signin.test', '%s')", hash)));
    REQUIRE(clarity_db_add_signin(db, &source));
    ClarityServeOptions options = {{"shop", "Shop", "Card", "{\"project\":{}}", db, &windows}, NULL, {NULL, NULL}, false};
    ClarityServer* s = clarity_server_new(&options);
    REQUIRE(s);
    // The start says the sign-in window comes first.
    const char* got = posted(s, NULL, "{\"command\":\"started\",\"args\":{}}");
    CHECK_TEXT(body_of(got), "{\"value\":{\"main\":\"Card\",\"unconnected\":null,\"signin\":\"SignIn\"}}");
    // Without a session only the sign-in window opens.
    got = posted(s, NULL, "{\"command\":\"open_window\",\"args\":{\"window\":\"Card\"}}");
    CHECK(strstr(body_of(got), "\"error\":{\"key\":\"notAllowed\",\"text\":\"You are not allowed to do this.\"}") != NULL);
    got = posted(s, NULL, "{\"command\":\"open_window\",\"args\":{\"window\":\"SignIn\"}}");
    CHECK(strstr(body_of(got), "\"value\"") != NULL);
    // A refused sign-in sets no cookie; one that works sets it, HttpOnly and SameSite=Strict, for the session's time.
    got = posted(s, NULL, "{\"command\":\"signin\",\"args\":{\"strategy\":\"password\",\"inputs\":{\"name\":\"web@signin.test\",\"password\":\"pear\"}}}");
    CHECK_TEXT(body_of(got), "{\"error\":{\"key\":\"signinRefused\",\"text\":\"Signing in did not work. Check what you typed and try again.\"}}");
    CHECK_TEXT(cookie_set(got), "");
    got = posted(s, NULL, "{\"command\":\"signin\",\"args\":{\"strategy\":\"password\",\"inputs\":{\"name\":\"web@signin.test\",\"password\":\"plum\"}}}");
    CHECK_TEXT(body_of(got), "{\"value\":null}");
    CHECK(strstr(got, "; HttpOnly; SameSite=Strict; Path=/; Max-Age=600\r\n") != NULL);
    const char* cookie = cookie_set(got);
    CHECK(strncmp(cookie, "clarity_session=", 16) == 0 && strlen(cookie) > 30);
    // With it the program's windows open, and the start asks for no sign-in.
    got = posted(s, cookie, "{\"command\":\"open_window\",\"args\":{\"window\":\"Card\"}}");
    CHECK(strstr(body_of(got), "\"value\"") != NULL);
    got = posted(s, clarity_format("theme=dark; %s", cookie), "{\"command\":\"started\",\"args\":{}}");
    CHECK_TEXT(body_of(got), "{\"value\":{\"main\":\"Card\",\"unconnected\":null}}");
    // Signing out closes the session and clears the cookie.
    got = posted(s, cookie, "{\"command\":\"signout\",\"args\":{}}");
    CHECK_TEXT(body_of(got), "{\"value\":null}");
    CHECK(strstr(got, "\r\nSet-Cookie: clarity_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0\r\n") != NULL);
    got = posted(s, cookie, "{\"command\":\"open_window\",\"args\":{\"window\":\"Card\"}}");
    CHECK(strstr(body_of(got), "\"error\":{\"key\":\"notAllowed\",\"text\":\"You are not allowed to do this.\"}") != NULL);
    // A cookie the program never set names nobody.
    got = posted(s, "clarity_session=made-up", "{\"command\":\"open_window\",\"args\":{\"window\":\"Card\"}}");
    CHECK(strstr(body_of(got), "\"error\":{\"key\":\"notAllowed\",\"text\":\"You are not allowed to do this.\"}") != NULL);
    clarity_server_free(s);
    clarity_db_close(db);
    no_people();
}

void a_chosen_secret_has_its_minimum_and_a_recovery_code_is_used_once(void) {
    const char* hash = NULL;
    CHECK(!clarity_hashed("argon2id", "short", 8, &hash));
    CHECK_TEXT(refusal(), "secretTooShort");
    CHECK(strstr(clarity_json_of(clarity_text_json(clarity_error()->text)), "\"minimum\"") != NULL);
    CHECK(clarity_hashed("argon2id", "long enough", 8, &hash));
    const char* codes = NULL;
    REQUIRE(clarity_new_recovery_codes(3, &codes));
    const char* stored = NULL;
    REQUIRE(clarity_hashed("sha256-lines", codes, 0, &stored));
    const char* second_line = strchr(codes, '\n');
    REQUIRE(second_line);
    const char* first = clarity_format("%.*s", (int)(second_line - codes), codes);
    const char* second = clarity_format("%.*s", (int)strcspn(second_line + 1, "\n"), second_line + 1);
    static const char* const targets[] = {"desktop"};
    static const char* const columns[] = {"email", "recovery_codes"};
    static const ClaritySigninStrategy strategies[] = {{"recovery", columns, 2}};
    ClaritySigninSource source = {.table = "members", .page = "SignIn", .targets = targets, .target_count = 1, .strategies = strategies, .strategy_count = 1};
    ClarityDb* db = NULL;
    REQUIRE(clarity_db_memory(&db));
    REQUIRE(clarity_db_batch(db, "", "CREATE TABLE members (id INTEGER PRIMARY KEY, email TEXT, recovery_codes TEXT)"));
    REQUIRE(clarity_db_batch(db, "", clarity_format("INSERT INTO members (email, recovery_codes) VALUES ('rec@signin.test', '%s')", stored)));
    REQUIRE(clarity_db_add_signin(db, &source));
    int64_t id = 0;
    cJSON* inputs = clarity_json_object(2, "name", cJSON_CreateString("rec@signin.test"), "code", cJSON_CreateString(first));
    CHECK(clarity_signin(db, "recovery", inputs, &id));
    CHECK_INT(id, 1);
    // The code is used up; the others still work.
    CHECK(!clarity_signin(db, "recovery", inputs, &id));
    CHECK_TEXT(refusal(), "signinRefused");
    CHECK(clarity_signin(db, "recovery", clarity_json_object(2, "name", cJSON_CreateString("rec@signin.test"), "code", cJSON_CreateString(second)), &id));
    clarity_db_close(db);
    no_people();
}

/* The people of the session test, as the generated program describes them. */
typedef struct {
    int64_t id;
    const char* email;
    const char* password_hash;
} Member;

static const char* const member_columns[] = {"email", "password_hash"};

static void member_values(const void* record, ClarityValue* out) {
    const Member* m = record;
    out[0] = clarity_text_value(m->email);
    out[1] = clarity_text_value(m->password_hash);
}

static bool member_from_values(int64_t id, const ClarityValue* v, void* record) {
    Member* m = record;
    m->id = id;
    return clarity_get_text("email", v[0], &m->email) && clarity_get_text("password_hash", v[1], &m->password_hash);
}

static const ClarityEntity member = {
    .name = "Member", .table = "members", .key = "id", .source = "", .columns = member_columns, .column_count = 2,
    .size = sizeof(Member), .id_at = offsetof(Member, id), .values = member_values, .from_values = member_from_values,
};

/* A served sign-in of the member: the session token it opened. */
static const char* session_of_signin(ClarityDb* db, const char* address) {
    ClarityCall call;
    clarity_call_open(&call, NULL, address);
    clarity_call_enter(&call);
    int64_t id = 0;
    bool in = clarity_signin(db, "password", clarity_json_object(2, "name", cJSON_CreateString("ses@signin.test"), "password", cJSON_CreateString("first secret")), &id);
    clarity_call_enter(NULL);
    return in ? clarity_copy(call.opened) : "";
}

static int64_t session_person(const char* token) { return clarity_people()->session(token); }

void a_changed_secret_signs_the_person_out_elsewhere(void) {
    static const char* const targets[] = {"web"};
    static const char* const columns[] = {"email", "password_hash"};
    static const ClaritySigninStrategy strategies[] = {{"password", columns, 2}};
    ClaritySigninSource source = {.table = "members", .page = "SignIn", .targets = targets, .target_count = 1, .strategies = strategies, .strategy_count = 1};
    ClarityDb* db = NULL;
    REQUIRE(clarity_db_memory(&db));
    REQUIRE(clarity_db_batch(db, "", "CREATE TABLE members (id INTEGER PRIMARY KEY, email TEXT, password_hash TEXT)"));
    const char* hash = NULL;
    REQUIRE(clarity_hashed("argon2id", "first secret", 0, &hash));
    REQUIRE(clarity_db_batch(db, "", clarity_format("INSERT INTO members (email, password_hash) VALUES ('ses@signin.test', '%s')", hash)));
    REQUIRE(clarity_db_add_signin(db, &source));
    const char* here = session_of_signin(db, "203.0.113.1");
    const char* there = session_of_signin(db, "203.0.113.2");
    REQUIRE(*here && *there);
    // The person changes their own password: their other session ends, this one stays.
    ClarityCall call;
    clarity_call_open(&call, clarity_format("clarity_session=%s", here), "203.0.113.1");
    CHECK_INT(call.me, 1);
    clarity_call_enter(&call);
    Member m;
    REQUIRE(clarity_get(db, &member, 1, &m));
    REQUIRE(clarity_hashed("argon2id", "second secret", 0, &m.password_hash));
    CHECK(clarity_save(db, &member, &m));
    clarity_call_enter(NULL);
    CHECK_INT(session_person(here), 1);
    CHECK_INT(session_person(there), 0);
    // A save that leaves the secret as it was ends nothing.
    clarity_call_open(&call, NULL, "203.0.113.3");
    clarity_call_enter(&call);
    m.email = "ses@signin.test";
    CHECK(clarity_save(db, &member, &m));
    CHECK_INT(session_person(here), 1);
    // Someone else changes it: every session of the person ends.
    REQUIRE(clarity_hashed("argon2id", "third secret", 0, &m.password_hash));
    CHECK(clarity_save(db, &member, &m));
    clarity_call_enter(NULL);
    CHECK_INT(session_person(here), 0);
    clarity_db_close(db);
    no_people();
}
