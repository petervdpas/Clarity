// The web target (DESIGN 7.3): Clarity.UI runs in the browser and the
// server serves the page, the library's browser files, the program's files
// and its resources, and answers the library's commands over HTTP as the
// desktop answers them in-process.

#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#include <windows.h>
#else
#include <pthread.h>
#include <unistd.h>
#endif

#include "check.h"
#include "clarity/text.h"
#include "clarity/watch.h"
#include "service/serve.h"
#include "tests.h"

static bool open_card(ClarityDb* db, const char* window, const cJSON* with, ClarityWindowState** out) {
    (void)db;
    if (strcmp(window, "Card") != 0) return clarity_fail(CLARITY_INVALID, "there is no window %s", window);
    *out = clarity_opened(window, clarity_json_object(1, "name", cJSON_CreateString("Ada")), with);
    return true;
}

static const char* handler_of(const char* window, const char* event) {
    (void)window;
    (void)event;
    return NULL;
}

static bool redraw(ClarityDb* db, ClarityReply* reply) {
    (void)db;
    (void)reply;
    return true;
}

/// `plan` runs long as an engine call does: it says how far it is, then waits until it is stopped.
static bool dispatch(ClarityDb* db, const ClarityWindowState* state, const char* handler, ClarityReply** out) {
    (void)db;
    if (strcmp(handler, "plan") != 0) return clarity_fail(CLARITY_INVALID, "no handler %s", handler);
    const ClarityEngineWatch* watch = clarity_current_watch();
    if (!watch) return clarity_fail(CLARITY_ENGINE, "no watch");
    watch->progress(1, 10, watch->context);
    for (long i = 0; i < 2000000000L && !*watch->stop; i++) {
    }
    if (*watch->stop) return clarity_fail(CLARITY_ENGINE, "it was stopped");
    *out = clarity_ui_reply(clarity_ui(state), state->window, state->data);
    return true;
}

static const ClarityWindows cards = {open_card, handler_of, dispatch, NULL, NULL, redraw, NULL, NULL};

static const char page[] = "<!doctype html><canvas id=\"clarity-ui\"></canvas><script src=\"clarity-ui-page.js\"></script>";
static const char files_json[] = "{\"title\":\"Shop\",\"windows\":{}}";
static const unsigned char wasm[] = {0x00, 'a', 's', 'm', 0x01, 0x00};
static const unsigned char logo[] = {0x89, 'P', 'N', 'G'};
static const ClarityCarried browser[] = {
    {"index.html", (const unsigned char*)page, sizeof page - 1},
    {"clarity-ui.wasm", wasm, sizeof wasm},
    {"clarity-ui-page.js", (const unsigned char*)"page();", 7},
    {"clarity/files.json", (const unsigned char*)files_json, sizeof files_json - 1},
    {"clarity/resources/pictures/a logo.png", logo, sizeof logo},
};
static const ClarityCarriedFiles carried = {NULL, 0, NULL, 0, NULL, 0, NULL, 0, browser, 5};

static const char* answered(ClarityServer* s, const ClarityServeOptions* options, const char* request, size_t* length) {
    (void)options;
    *length = 0;
    return clarity_serve_answer(s, request, strlen(request), length);
}

static const char* body_of(const char* reply) {
    const char* end = strstr(reply, "\r\n\r\n");
    return end ? end + 4 : "";
}

static const char* post(const char* body) {
    return clarity_format("POST /clarity/invoke HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: %zu\r\n\r\n%s", strlen(body), body);
}

void the_server_serves_the_page_and_answers_its_commands(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityServeOptions options = {{"shop", "Shop", "Card", "{\"project\":{}}", db, &cards}, &carried, {"{\"windows\":{\"Card\":{\"name\":\"text\"}}}", NULL}, false};
    ClarityServer* s = clarity_server_new(&options);
    size_t length = 0;
    // The page at /, as it is: its script finds the server by itself.
    const char* got = answered(s, &options, "GET / HTTP/1.1\r\nHost: x\r\n\r\n", &length);
    CHECK(strncmp(got, "HTTP/1.1 200 OK\r\n", 17) == 0);
    CHECK(strstr(got, "Content-Type: text/html; charset=utf-8\r\n") != NULL);
    CHECK_TEXT(body_of(got), page);
    // The browser files by their names and types; the wasm byte for byte, its zeros too.
    got = answered(s, &options, "GET /clarity-ui.wasm HTTP/1.1\r\nHost: x\r\n\r\n", &length);
    CHECK(strstr(got, "Content-Type: application/wasm\r\n") != NULL);
    CHECK(strstr(got, "Content-Length: 6\r\n") != NULL);
    CHECK(length == (size_t)(body_of(got) - got) + sizeof wasm && memcmp(body_of(got), wasm, sizeof wasm) == 0);
    got = answered(s, &options, "GET /clarity-ui-page.js?v=1 HTTP/1.1\r\nHost: x\r\n\r\n", &length);
    CHECK(strstr(got, "Content-Type: text/javascript\r\n") != NULL);
    got = answered(s, &options, "GET /clarity/files.json HTTP/1.1\r\nHost: x\r\n\r\n", &length);
    CHECK(strstr(got, "Content-Type: application/json\r\n") != NULL);
    CHECK_TEXT(body_of(got), files_json);
    // A resource by its name, spaces written as %20.
    got = answered(s, &options, "GET /clarity/resources/pictures/a%20logo.png HTTP/1.1\r\nHost: x\r\n\r\n", &length);
    CHECK(strstr(got, "Content-Type: image/png\r\n") != NULL);
    CHECK(memcmp(body_of(got), logo, sizeof logo) == 0);
    // Clarity.UI's commands, answered as the desktop answers them.
    got = answered(s, &options, post("{\"command\": \"started\", \"args\": {}}"), &length);
    CHECK(strncmp(got, "HTTP/1.1 200 OK\r\n", 17) == 0);
    CHECK_TEXT(body_of(got), "{\"value\":{\"main\":\"Card\",\"unconnected\":null}}");
    got = answered(s, &options, post("{\"command\": \"open_window\", \"args\": {\"window\": \"Card\"}}"), &length);
    CHECK(strstr(body_of(got), "\"name\":\"Ada\"") != NULL);
    got = answered(s, &options, post("{\"command\": \"facts\", \"args\": {}}"), &length);
    CHECK_TEXT(body_of(got), "{\"value\":{\"windows\":{\"Card\":{\"name\":\"text\"}}}}");
    // What the program does not answer is an error, which the library takes as its default.
    got = answered(s, &options, post("{\"command\": \"nonsense\", \"args\": {}}"), &length);
    CHECK_TEXT(body_of(got), "{\"error\":\"there is no command nonsense\"}");
    got = answered(s, &options, post("not json"), &length);
    CHECK(strncmp(body_of(got), "{\"error\":", 9) == 0);
    got = answered(s, &options, "GET /clarity/invoke HTTP/1.1\r\nHost: x\r\n\r\n", &length);
    CHECK(strncmp(got, "HTTP/1.1 405 Method Not Allowed\r\n", 33) == 0);
    // Nothing else is there.
    got = answered(s, &options, "GET /secret HTTP/1.1\r\nHost: x\r\n\r\n", &length);
    CHECK(strncmp(got, "HTTP/1.1 404 Not Found\r\n", 24) == 0);
    got = answered(s, &options, "GET /clarity/resources/none.png HTTP/1.1\r\nHost: x\r\n\r\n", &length);
    CHECK(strncmp(got, "HTTP/1.1 404 Not Found\r\n", 24) == 0);
    // Without facts, facts is none.
    options.wire.facts = NULL;
    got = answered(s, &options, post("{\"command\": \"facts\", \"args\": {}}"), &length);
    CHECK(strncmp(body_of(got), "{\"error\":", 9) == 0);
    clarity_server_free(s);
    clarity_db_close(db);
}

/// The handler's request of a long call, sent from a thread of its own.
static ClarityServer* planning;
static const char* planned;

#if defined(_WIN32)
static DWORD WINAPI plan_elsewhere(void* arg) {
#else
static void* plan_elsewhere(void* arg) {
#endif
    (void)arg;
    size_t length = 0;
    const char* request = post("{\"command\":\"dispatch_event\",\"args\":{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"plan\",\"watch\":5}}");
    const char* body = body_of(clarity_serve_answer(planning, request, strlen(request), &length));
    char* kept = malloc(strlen(body) + 1);
    if (kept) strcpy(kept, body);
    planned = kept;
    clarity_scratch_release();
    return 0;
}

static const char* asked(ClarityServer* s, const char* command) {
    size_t length = 0;
    return body_of(clarity_serve_answer(s, post(command), strlen(post(command)), &length));
}

// A long call over the wire (DESIGN 5.12): its handler's request names its
// watch and stays open while the page asks how far it is and stops it from
// requests of their own; a stop sent before its handler started holds.
void a_long_call_over_the_wire_says_how_far_it_is_and_stops(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityServeOptions options = {{"shop", "Shop", "Card", "{\"project\":{}}", db, &cards}, &carried, {NULL, NULL}, true};
    ClarityServer* s = clarity_server_new(&options);
    CHECK_TEXT(asked(s, "{\"command\":\"watches\",\"args\":{}}"), "{\"value\":true}");
    CHECK_TEXT(asked(s, "{\"command\":\"watch_progress\",\"args\":{\"id\":5}}"), "{\"value\":null}");
    planning = s;
#if defined(_WIN32)
    HANDLE thread = CreateThread(NULL, 0, plan_elsewhere, NULL, 0, NULL);
#else
    pthread_t thread;
    pthread_create(&thread, NULL, plan_elsewhere, NULL);
#endif
    // How far it is, while its request is open.
    const char* progress = "";
    for (int i = 0; i < 5000 && strcmp(progress, "{\"value\":[1,10]}") != 0; i++) {
        progress = asked(s, "{\"command\":\"watch_progress\",\"args\":{\"id\":5}}");
#if defined(_WIN32)
        Sleep(1);
#else
        usleep(1000);
#endif
    }
    CHECK_TEXT(progress, "{\"value\":[1,10]}");
    CHECK_TEXT(asked(s, "{\"command\":\"stop_watch\",\"args\":{\"id\":5}}"), "{\"value\":null}");
#if defined(_WIN32)
    WaitForSingleObject(thread, INFINITE);
    CloseHandle(thread);
#else
    pthread_join(thread, NULL);
#endif
    CHECK(planned && strstr(planned, "it was stopped") != NULL);
    // It keeps how far it got until end_watch, which may come twice.
    CHECK_TEXT(asked(s, "{\"command\":\"watch_progress\",\"args\":{\"id\":5}}"), "{\"value\":[1,10]}");
    CHECK_TEXT(asked(s, "{\"command\":\"end_watch\",\"args\":{\"id\":5}}"), "{\"value\":null}");
    CHECK_TEXT(asked(s, "{\"command\":\"end_watch\",\"args\":{\"id\":5}}"), "{\"value\":null}");
    CHECK_TEXT(asked(s, "{\"command\":\"watch_progress\",\"args\":{\"id\":5}}"), "{\"value\":null}");
    // A stop before its handler started stops it when it starts.
    CHECK_TEXT(asked(s, "{\"command\":\"stop_watch\",\"args\":{\"id\":6}}"), "{\"value\":null}");
    const char* stopped = asked(s, "{\"command\":\"dispatch_event\",\"args\":{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"plan\",\"watch\":6}}");
    CHECK(strstr(stopped, "it was stopped") != NULL);
    asked(s, "{\"command\":\"end_watch\",\"args\":{\"id\":6}}");
    free((void*)planned);
    clarity_server_free(s);
    clarity_db_close(db);
}

void the_server_listens_where_it_is_told(void) {
    char* none[] = {"app"};
    CHECK(strcmp(clarity_serve_address(1, none, "127.0.0.1:9090"), "127.0.0.1:9090") == 0 || getenv("CLARITY_ADDRESS"));
    char* told[] = {"app", "--address", "0.0.0.0:9000"};
    CHECK(strcmp(clarity_serve_address(3, told, "127.0.0.1:9090"), "0.0.0.0:9000") == 0);
}

static bool open_page_card(ClarityDb* db, const char* window, const cJSON* with, ClarityWindowState** out) {
    (void)db;
    cJSON* settings = clarity_json_object(3, "id", cJSON_CreateNumber(0), "showDone", cJSON_CreateTrue(), "theme", cJSON_CreateString(""));
    *out = clarity_opened(window, clarity_json_object(1, "settings", settings), with);
    return true;
}

static const char* page_handler_of(const char* window, const char* event) {
    (void)window;
    return strcmp(event, "open") == 0 ? "opened" : NULL;
}

/// `opened` writes what it saw of the settings; `flip` changes one and quits.
static bool page_dispatch(ClarityDb* db, const ClarityWindowState* state, const char* handler, ClarityReply** out) {
    (void)db;
    ClarityUi* ui = clarity_ui(state);
    cJSON* data = cJSON_Duplicate(state->data, true);
    cJSON* settings = cJSON_GetObjectItemCaseSensitive(data, "settings");
    bool shows = cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(settings, "showDone"));
    if (strcmp(handler, "opened") == 0) cJSON_AddBoolToObject(data, "saw", shows);
    if (strcmp(handler, "flip") == 0) {
        cJSON_ReplaceItemInObjectCaseSensitive(settings, "showDone", cJSON_CreateBool(!shows));
        clarity_ui_quit(ui);
    }
    cJSON_DeleteItemFromObjectCaseSensitive(data, "said");
    cJSON_AddStringToObject(data, "said", clarity_text_of("hi"));
    *out = clarity_ui_reply(ui, state->window, data);
    return true;
}

static const ClarityWindows page_cards = {open_page_card, page_handler_of, page_dispatch, NULL, NULL, redraw, NULL, NULL};

static cJSON* asked_json(ClarityServer* s, const char* body) { return cJSON_Parse(asked(s, body)); }

static const cJSON* data_of(const cJSON* answer) {
    return cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(answer, "value"), "state"), "data");
}

static const char* said(const cJSON* answer) {
    const cJSON* text = cJSON_GetObjectItemCaseSensitive(data_of(answer), "said");
    return cJSON_IsString(text) ? text->valuestring : "";
}

// Each browser apart (DESIGN 7.9 B2): a request names its page and brings
// the person's settings as the page keeps them. A handler sees that page's
// settings and language, a page that sends none keeps what it had, settings
// a command changed and quit go back to that page alone, and a page unasked
// for longer than the quiet time is forgotten.
void each_browser_has_its_own_settings_and_language(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    const char* model = "{\"project\":{\"locale\":{\"language\":\"en\"},\"translations\":{\"en\":{\"hi\":\"Hello\"},\"nl\":{\"hi\":\"Hallo\"}}}}";
    ClarityServeOptions options = {{"shop", "Shop", "Card", model, db, &page_cards}, &carried, {NULL, NULL}, false};
    ClarityServer* s = clarity_server_new(&options);
    clarity_server_quiet(s, 300);
    CHECK_TEXT(asked(s, "{\"command\":\"set_language\",\"args\":{\"language\":\"nl\"},\"browser\":\"a\",\"settings\":{\"showDone\":false}}"), "{\"value\":null}");
    cJSON* a = asked_json(s, "{\"command\":\"open_window\",\"args\":{\"window\":\"Card\"},\"browser\":\"a\",\"settings\":{\"showDone\":false}}");
    CHECK(cJSON_IsFalse(cJSON_GetObjectItemCaseSensitive(data_of(a), "saw")));
    CHECK_TEXT(said(a), "Hallo");
    CHECK(cJSON_GetObjectItemCaseSensitive(a, "settings") == NULL);
    cJSON* b = asked_json(s, "{\"command\":\"open_window\",\"args\":{\"window\":\"Card\"},\"browser\":\"b\"}");
    CHECK(cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(data_of(b), "saw")));
    CHECK_TEXT(said(b), "Hello");
    // a's handler changes its settings: they come back to a alone, quit with them.
    const char* state = cJSON_PrintUnformatted(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(a, "value"), "state"));
    cJSON* flipped = asked_json(s, clarity_format("{\"command\":\"dispatch_event\",\"args\":{\"state\":%s,\"handler\":\"flip\"},\"browser\":\"a\"}", state));
    CHECK_TEXT(cJSON_PrintUnformatted(cJSON_GetObjectItemCaseSensitive(flipped, "settings")), "{\"showDone\":true,\"theme\":\"\"}");
    CHECK(cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(flipped, "value"), "quit")));
    CHECK_TEXT(said(flipped), "Hallo");
    b = asked_json(s, "{\"command\":\"open_window\",\"args\":{\"window\":\"Card\"},\"browser\":\"b\"}");
    CHECK(cJSON_GetObjectItemCaseSensitive(b, "settings") == NULL);
    CHECK(cJSON_GetObjectItemCaseSensitive(cJSON_GetObjectItemCaseSensitive(b, "value"), "quit") == NULL);
    CHECK_TEXT(said(b), "Hello");
    // a sends no settings: it keeps those it had, the changed ones.
    a = asked_json(s, "{\"command\":\"open_window\",\"args\":{\"window\":\"Card\"},\"browser\":\"a\"}");
    CHECK(cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(data_of(a), "saw")));
    // Unasked for longer than the quiet time, a page is forgotten: its language too.
#if defined(_WIN32)
    Sleep(400);
#else
    usleep(400000);
#endif
    asked(s, "{\"command\":\"open_window\",\"args\":{\"window\":\"Card\"},\"browser\":\"b\"}");
    a = asked_json(s, "{\"command\":\"open_window\",\"args\":{\"window\":\"Card\"},\"browser\":\"a\"}");
    CHECK_TEXT(said(a), "Hello");
    clarity_set_language(NULL);
    clarity_server_free(s);
    clarity_db_close(db);
}
