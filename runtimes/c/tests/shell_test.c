// The shell's commands, run without a window: the same commands and JSON as
// the Tauri, Wails and C++ shells.

#include "check.h"
#include "clarity/code.h"
#include "shell/shell.h"
#include "tests.h"

static bool open_card(ClarityDb* db, const char* window, const cJSON* with, ClarityWindowState** out) {
    (void)db;
    if (strcmp(window, "Card") != 0) return clarity_fail(CLARITY_INVALID, "there is no window %s", window);
    *out = clarity_opened(window, clarity_json_object(1, "name", cJSON_CreateString("Ada")), with);
    return true;
}

static const char* handler_of(const char* window, const char* event) {
    return strcmp(window, "Card") == 0 && strcmp(event, "open") == 0 ? "opened" : NULL;
}

/// The service a long handler stops, as the person's Stop does while it runs.
static ClarityService* stopping;

static bool dispatch(ClarityDb* db, const ClarityWindowState* state, const char* handler, ClarityReply** out) {
    (void)db;
    if (strcmp(handler, "plan") == 0) {
        // As a long engine call does: it says how far it is until it is stopped.
        const ClarityEngineWatch* watch = clarity_current_watch();
        for (int64_t day = 1;; ++day) {
            watch->progress(day, 365, watch->context);
            if (day == 3) clarity_service_invoke(stopping, "stop_call", cJSON_CreateObject(), NULL);
            if (*watch->stop) return clarity_fail(CLARITY_ENGINE, "engine Planner failed in plan: it was stopped");
        }
    }
    ClarityUi* ui = clarity_ui(state);
    if (strcmp(handler, "opened") == 0) {
        const char* name = cJSON_GetObjectItem(state->data, "name")->valuestring;
        clarity_ui_set_caption(ui, "title", clarity_plain_text(clarity_join("Card of ", name)));
    }
    if (strcmp(handler, "save") == 0) return clarity_refuse(clarity_text_with(clarity_text("{name} is taken"), "name", clarity_slot_text("Ada")));
    if (strcmp(handler, "fail") == 0) return clarity_fail(CLARITY_STORAGE, "locked");
    *out = clarity_ui_reply(ui, state->window, state->data);
    return true;
}

static bool options(ClarityDb* db, const char* entity, cJSON** out) {
    (void)db;
    (void)entity;
    *out = cJSON_CreateArray();
    clarity_add_option(*out, 1, "Ada");
    return true;
}

static bool picture(ClarityDb* db, const char* entity, int64_t id, const char* field, ClarityBytes* out) {
    (void)db;
    static const uint8_t png[] = {0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
    if (strcmp(entity, "Card") != 0 || strcmp(field, "photo") != 0) return clarity_fail(CLARITY_INVALID, "%s has no picture %s", entity, field);
    *out = id == 1 ? (ClarityBytes){png, sizeof png} : (ClarityBytes){NULL, 0};
    return true;
}

/// Draws the card's canvas anew: a dot per letter of its name.
static bool redraw(ClarityDb* db, ClarityReply* reply) {
    (void)db;
    if (strcmp(reply->state.window, "Card") != 0) return true;
    ClarityUi* ui = clarity_ui(&reply->state);
    clarity_ui_begin_drawing(ui, "badge");
    const cJSON* name = cJSON_GetObjectItem(reply->state.data, "name");
    for (size_t i = 0; cJSON_IsString(name) && name->valuestring[i]; ++i) clarity_ui_draw(ui, "badge", cJSON_Parse("{\"op\":\"circle\"}"));
    reply->state = clarity_ui_reply(ui, reply->state.window, reply->state.data)->state;
    return true;
}

/// Renames the card to the answer, after the values written for it.
static bool answer(ClarityDb* db, const ClarityWindowState* state, const char* handler, const cJSON* values, ClarityReply** out) {
    (void)db;
    if (strcmp(handler, "rename") != 0) return clarity_fail(CLARITY_INVALID, "Card has no handler %s to answer", handler);
    const cJSON* name = cJSON_GetArrayItem(values, cJSON_GetArraySize(values) - 1);
    cJSON* data = cJSON_Duplicate(state->data, true);
    cJSON_ReplaceItemInObject(data, "name", cJSON_CreateString(name->valuestring));
    *out = clarity_ui_reply(clarity_ui(state), state->window, data);
    return true;
}

static const ClarityWindows cards = {open_card, handler_of, dispatch, options, picture, redraw, answer};

typedef struct {
    ClarityDesktop desktop;
    const char* title;
    bool closed;
    const char* opened;
    int width, height;
} Fake;

static void fake_title(ClarityDesktop* desktop, const char* title) { ((Fake*)desktop)->title = clarity_copy(title); }
static void fake_close(ClarityDesktop* desktop) { ((Fake*)desktop)->closed = true; }
// The system's folder picker: the person chooses a folder, or cancels.
static const char* fake_folder(ClarityDesktop* desktop, const char* title) {
    (void)desktop;
    return strcmp(title, "Choose a folder") == 0 ? "/home/ada/Pictures" : NULL;
}

static void fake_open(void* context, ClarityDesktop* opener, const char* name, int width, int height) {
    (void)opener;
    Fake* fake = context;
    fake->opened = clarity_copy(name);
    fake->width = width;
    fake->height = height;
}

/// The last link the shell opened, in place of the browser.
static const char* last_link;

static const char* last_line;

static void fake_log(const char* line) { last_line = clarity_copy(line); }

static bool fake_link(const char* address) {
    last_link = clarity_copy(address);
    return true;
}

static cJSON* invoked(ClarityService* s, const char* command, const char* args, ClarityDesktop* caller) {
    return clarity_service_invoke(s, command, cJSON_Parse(args), caller);
}

void the_shell_answers_the_renderers_commands(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    CHECK_JSON(invoked(s, "start", "{}", NULL), "{\"value\":{\"model\":{\"project\":{}},\"main\":\"Card\"}}");
    // The screen names the language text code makes is translated to.
    invoked(s, "set_language", "{\"language\":\"nl\"}", NULL);
    invoked(s, "set_language", "{\"language\":null}", NULL);
    cJSON* opened = invoked(s, "open_window", "{\"window\":\"Card\",\"with\":{\"name\":\"Grace\"}}", NULL);
    const cJSON* title = cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(opened, "value"), "state"), "parts"), "title"), "caption");
    CHECK_JSON(title, "\"Card of Grace\"");
    // Its canvases are drawn anew, when it opens and after every handler.
    const cJSON* badge = cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(opened, "value"), "state"), "parts"), "badge");
    CHECK(cJSON_GetArraySize(cJSON_GetObjectItem(badge, "drawing")) == 5);
    cJSON* again = invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{\"name\":\"Ada\"}},\"handler\":\"noop\"}", NULL);
    badge = cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(again, "value"), "state"), "parts"), "badge");
    CHECK(cJSON_GetArraySize(cJSON_GetObjectItem(badge, "drawing")) == 3);
    // A refusal keeps its slots; other errors are text.
    CHECK_JSON(invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"save\"}", NULL),
               "{\"error\":{\"text\":\"{name} is taken\",\"values\":{\"name\":{\"kind\":\"text\",\"value\":\"Ada\"}}}}");
    CHECK_JSON(invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\"},\"handler\":\"fail\"}", NULL), "{\"error\":\"storage: locked\"}");
    // The answer to a question runs the handler it goes to, and the canvas is drawn again.
    cJSON* answered = invoked(s, "answer_event", "{\"state\":{\"window\":\"Card\",\"data\":{\"name\":\"Ada\"}},\"handler\":\"rename\",\"values\":[7,\"Grace\"]}", NULL);
    CHECK_TEXT(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(answered, "value"), "state"), "data"), "name")->valuestring, "Grace");
    CHECK_JSON(invoked(s, "ref_options", "{\"entity\":\"Customer\"}", NULL), "{\"value\":[{\"id\":1,\"text\":\"Ada\"}]}");
    CHECK_JSON(invoked(s, "picture", "{\"entity\":\"Card\",\"id\":1,\"field\":\"photo\"}", NULL), "{\"value\":\"data:image/png;base64,iVBORw0KGgo=\"}");
    CHECK_JSON(invoked(s, "picture", "{\"entity\":\"Card\",\"id\":2,\"field\":\"photo\"}", NULL), "{\"value\":null}");
    CHECK_JSON(invoked(s, "nonsense", "{}", NULL), "{\"error\":\"there is no command nonsense\"}");
    CHECK_JSON(invoked(s, "start_app", "{}", NULL),
               "{\"value\":{\"state\":{\"window\":\"\",\"instance\":0,\"data\":{},\"parts\":{},\"selected\":{}},\"messages\":[],\"open\":[],\"close\":false}}");

    // A link of a markdown part opens in the browser: only a web or a mail address.
    clarity_service_on_link(s, fake_link);
    CHECK_JSON(invoked(s, "open_link", "{\"address\":\"https://example.com/rules\"}", NULL), "{\"value\":null}");
    CHECK_TEXT(last_link, "https://example.com/rules");
    CHECK_JSON(invoked(s, "open_link", "{\"address\":\"file:///etc/passwd\"}", NULL),
               "{\"error\":\"a link opens only an http, https or mailto address, not file:///etc/passwd\"}");
    CHECK_TEXT(last_link, "https://example.com/rules");

    // An engine beside the screen logs through the shell: into the application's log.
    clarity_service_on_log(s, fake_log);
    CHECK_JSON(invoked(s, "log_line", "{\"line\":\"engine Screen: level 2\"}", NULL), "{\"value\":null}");
    CHECK_TEXT(last_line, "engine Screen: level 2");

    // The window a command came from takes its title, and a window of its own starts where it was opened.
    Fake main = {{"main", fake_title, fake_close}, NULL, false, NULL, 0, 0};
    invoked(s, "set_title", "{\"title\":\"Customer card\"}", &main.desktop);
    CHECK_TEXT(main.title, "Customer card");
    // A folder part's button opens the folder picker of the window it is in: the folder chosen, or none.
    CHECK_JSON(invoked(s, "choose_folder", "{\"title\":\"Choose a folder\"}", &main.desktop), "{\"error\":\"this window has no folder picker\"}");
    main.desktop.choose_folder = fake_folder;
    CHECK_JSON(invoked(s, "choose_folder", "{\"title\":\"Choose a folder\"}", &main.desktop), "{\"value\":\"/home/ada/Pictures\"}");
    CHECK_JSON(invoked(s, "choose_folder", "{\"title\":\"Cancelled\"}", &main.desktop), "{\"value\":null}");
    clarity_service_on_desktop(s, NULL, fake_open, &main);
    invoked(s, "open_apart", "{\"window\":\"Card\",\"with\":{\"name\":\"Linus\"}}", &main.desktop);
    CHECK_TEXT(main.opened, "apart-1");
    // Without a size from the renderer, the standard one.
    CHECK_INT(main.width, 1100);
    CHECK_INT(main.height, 760);
    Fake apart = {{"apart-1", fake_title, fake_close}, NULL, false, NULL, 0, 0};
    CHECK_JSON(invoked(s, "apart_start", "{}", &apart.desktop), "{\"value\":{\"window\":\"Card\",\"with\":{\"name\":\"Linus\"}}}");
    CHECK_TEXT(clarity_service_closed(s, "apart-1"), "main");
    CHECK(!clarity_service_closed(s, "apart-1"));
    invoked(s, "close_apart", "{}", &main.desktop);
    CHECK(main.closed);
    // A window of its own opens at the size of the window it shows.
    invoked(s, "open_apart", "{\"window\":\"Card\",\"with\":{},\"width\":320,\"height\":460}", &main.desktop);
    CHECK_INT(main.width, 320);
    CHECK_INT(main.height, 460);
    clarity_service_free(s);
    clarity_db_close(db);
}

static uint64_t closed_window;

static void closed(uint64_t instance, void* context) {
    (void)context;
    closed_window = instance;
}

/// A window that closes is reported once its own `on close` has run, so an
/// engine with state per window drops what it had of it.
void a_closed_window_is_reported(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    closed_window = 0;
    clarity_on_window_closed(closed, NULL);
    const cJSON* state = cJSON_GetObjectItem(cJSON_GetObjectItem(invoked(s, "open_window", "{\"window\":\"Card\",\"with\":{}}", NULL), "value"), "state");
    uint64_t instance = (uint64_t)cJSON_GetObjectItem(state, "instance")->valuedouble;
    cJSON* args = cJSON_CreateObject();
    cJSON_AddItemToObject(args, "state", cJSON_Duplicate(state, true));
    invoked(s, "close_window", cJSON_PrintUnformatted(args), NULL);
    clarity_off_window_closed(closed, NULL);
    CHECK(instance > 0 && closed_window == instance);
    clarity_service_free(s);
    clarity_db_close(db);
}

static int connect_tries = 0;

/// Opens the databases on the second try, as when the person kept their connection between.
static bool connect_later(ClarityDb** out) {
    if (++connect_tries == 1) return clarity_fail(CLARITY_STORAGE, "still not kept");
    return clarity_db_memory(out);
}

/// The databases did not open (DESIGN 5.27): start says what to show and
/// why, and connect opens them once the person has kept their connection.
void an_unconnected_start_says_why_and_connects_later(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &cards};
    clarity_fail(CLARITY_NOT_KEPT, "the secret setting shopDb is not kept yet");
    ClarityError not_kept = *clarity_error();
    options.unconnected = &not_kept;
    options.unconnected_window = "Preferences";
    options.connect = connect_later;
    connect_tries = 0;
    ClarityService* s = clarity_service_new(&options);
    CHECK_JSON(invoked(s, "start", "{}", NULL),
               "{\"value\":{\"model\":{\"project\":{}},\"main\":\"Card\",\"unconnected\":{\"handler\":null,\"window\":\"Preferences\",\"problem\":{\"key\":\"unconnectedNotKept\"}}}}");
    // While unconnected, the title bar's close goes to the screen, which connects again.
    CHECK(clarity_service_closes_to_screen(s));
    CHECK_JSON(invoked(s, "connect", "{}", NULL), "{\"error\":{\"key\":\"unconnectedFailed\",\"values\":{\"problem\":{\"kind\":\"text\",\"value\":\"still not kept\"}}}}");
    CHECK_JSON(invoked(s, "connect", "{}", NULL), "{\"value\":null}");
    CHECK(!clarity_service_closes_to_screen(s));
    CHECK_JSON(invoked(s, "start", "{}", NULL), "{\"value\":{\"model\":{\"project\":{}},\"main\":\"Card\"}}");
    clarity_service_free(s);
}

/// The scripts the shell ran on a window's screen.
static const char* shown[16];
static size_t shown_count;

static void fake_show(ClarityDesktop* desktop, const char* js) {
    (void)desktop;
    if (shown_count < 16) shown[shown_count] = clarity_copy(js);
    shown_count++;
}

// A long call on the screen (DESIGN 5.12): the handler's engine calls report
// to the screen, at most ten times a second, until the handler ends; Stop
// comes in while it runs.
void a_long_handler_shows_how_far_it_is_and_stops(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    stopping = s;
    Fake main = {{"main", fake_title, fake_close, .show = fake_show}, NULL, false, NULL, 0, 0};
    cJSON* answer = invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{\"name\":\"Ada\"}},\"handler\":\"plan\"}", &main.desktop);
    CHECK_TEXT(cJSON_GetObjectItem(answer, "error")->valuestring, "engine Planner failed in plan: it was stopped");
    REQUIRE(shown_count >= 2);
    CHECK_TEXT(shown[0], "window.dispatchEvent(new CustomEvent(\"clarity-progress\", {detail: {\"done\":1,\"total\":365}}))");
    CHECK_TEXT(shown[shown_count - 1 < 16 ? shown_count - 1 : 15], "window.dispatchEvent(new CustomEvent(\"clarity-progress\", {detail: null}))");
    // A handler that reports nothing tells the screen nothing.
    shown_count = 0;
    invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{\"name\":\"Ada\"}},\"handler\":\"opened\"}", &main.desktop);
    CHECK(shown_count == 0);
    clarity_service_free(s);
    clarity_db_close(db);
}
