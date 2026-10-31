// The shell's commands, run without a window: the same commands and JSON as
// the Tauri, Wails and C++ shells.

#include <stddef.h>

#include "check.h"
#include "clarity/code.h"
#include "clarity/sql.h"
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

/// A record a handler keeps, for the undo step its run makes.
typedef struct {
    int64_t id;
    const char* text;
} Memo;

static const char* const memo_columns[] = {"text"};
static void memo_values(const void* record, ClarityValue* out) { out[0] = clarity_text_value(((const Memo*)record)->text); }
static bool memo_from_values(int64_t id, const ClarityValue* v, void* record) {
    Memo* r = record;
    r->id = id;
    return clarity_get_text("text", v[0], &r->text);
}
static const ClarityEntity memo = {
    .name = "Memo", .table = "memo", .key = "id", .source = "", .columns = memo_columns, .column_count = 1,
    .size = sizeof(Memo), .id_at = offsetof(Memo, id), .values = memo_values, .from_values = memo_from_values,
};

/// The service a long handler stops, as the person's Stop does while it runs.
static ClarityService* stopping;

static bool dispatch(ClarityDb* db, const ClarityWindowState* state, const char* handler, ClarityReply** out) {
    if (strcmp(handler, "remember") == 0) {
        Memo kept = {0, "kept"};
        if (!clarity_save(db, &memo, &kept)) return false;
    }
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
    // A handler that adds a task to the window's list.
    if (strcmp(handler, "addTask") == 0) {
        cJSON* data = cJSON_Duplicate(state->data, true);
        cJSON_AddItemToArray(cJSON_GetObjectItem(data, "tasks"), cJSON_Parse("{\"id\":4,\"title\":\"Eggs\",\"up\":null}"));
        *out = clarity_ui_reply(clarity_ui(state), state->window, data);
        return true;
    }
    // A handler that chooses rows: bread in the tree, Milk in the table.
    if (strcmp(handler, "choose") == 0) {
        *out = clarity_ui_reply(clarity_ui(state), state->window, state->data);
        (*out)->state.selected = CLARITY_NEW(ClaritySelected, 2);
        (*out)->state.selected[0] = (ClaritySelected){"outline", 1};
        (*out)->state.selected[1] = (ClaritySelected){"grid", 0};
        (*out)->state.selected_count = 2;
        return true;
    }
    // A wizard's on next that sends it to its last step.
    if (strcmp(handler, "jump") == 0) clarity_ui_set_step(ui, "wizard", 3);
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

static size_t memos(ClarityDb* db) {
    void* items;
    size_t count = 0;
    clarity_find(db, &memo, clarity_query(), &items, &count);
    return count;
}

// Undo (DESIGN 5.29): the runtime keeps the window's edits, each with the
// database steps its handler made, and Ctrl+Z takes them back together.
void the_runtime_keeps_the_edits_and_ctrl_z_takes_them_back(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_db_batch(db, "", "CREATE TABLE memo (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)"));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    // A handler that changed the database answers with its step; one that failed or changed nothing, with none.
    CHECK(cJSON_GetObjectItem(invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"fail\"}", NULL), "error"));
    // A reply: the edits start from the window as it is.
    cJSON* nothing = invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ada\"}},\"handler\":\"noop\"}", NULL);
    CHECK(!cJSON_GetObjectItem(cJSON_GetObjectItem(nothing, "value"), "step"));
    // The person typed a name, then pressed a part whose handler saves: two edits of the form.
    invoked(s, "edit_event", "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ann\"}},\"part\":\"nameBox\",\"scope\":\"form\",\"ends\":true}", NULL);
    cJSON* saved = invoked(s, "edit_event", "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ann\"}},\"handler\":\"remember\",\"part\":\"saveBox\",\"scope\":\"form\",\"ends\":true}", NULL);
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(saved, "value"), "step"), "1");
    CHECK_INT(memos(db), 1);
    const char* back = "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ann\"}},\"scope\":\"form\",\"back\":true}";
    CHECK(cJSON_GetObjectItem(invoked(s, "undo_edit", back, NULL), "value"));
    CHECK_INT(memos(db), 0);
    cJSON* typed = invoked(s, "undo_edit", back, NULL);
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(typed, "value"), "state"), "data"), "{\"name\":\"Ada\"}");
    const char* again = "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ada\"}},\"scope\":\"form\",\"back\":false}";
    cJSON* retyped = invoked(s, "undo_edit", again, NULL);
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(retyped, "value"), "state"), "data"), "{\"name\":\"Ann\"}");
    invoked(s, "undo_edit", again, NULL);
    CHECK_INT(memos(db), 1);
    // A closed window's edits go.
    invoked(s, "close_window", "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ann\"}}}", NULL);
    cJSON* gone = invoked(s, "undo_edit", back, NULL);
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(gone, "value"), "state"), "data"), "{\"name\":\"Ann\"}");
    // The old command is gone.
    CHECK(cJSON_GetObjectItem(invoked(s, "undo_step", "{\"step\":1,\"back\":true}", NULL), "error"));
    clarity_service_free(s);
    clarity_db_close(db);
}

/// The data of the window a reply shows.
static const cJSON* data_of(const cJSON* answer) {
    return cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(answer, "value"), "state"), "data");
}

static const char* const tasks = "{\"window\":\"Card\",\"instance\":9,\"data\":{\"tasks\":[{\"id\":1,\"position\":1},{\"id\":2,\"position\":2},{\"id\":3,\"position\":3}]}}";

// Reordering (DESIGN 1.7, 5.30): the screen sends the move, and the runtime
// gives the list its order and numbers, chooses the moved row, and keeps the
// move as an edit that Ctrl+Z takes back.
void the_runtime_reorders_a_list_and_ctrl_z_puts_it_back(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_db_batch(db, "", "CREATE TABLE memo (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)"));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    CHECK(clarity_service_watches("reorder_event"));
    invoked(s, "dispatch_event", clarity_format("{\"state\":%s,\"handler\":\"noop\"}", tasks), NULL);
    const char* drag = "{\"row\":2,\"where\":\"before\",\"target\":0}";
    cJSON* moved = invoked(s, "reorder_event", clarity_format("{\"state\":%s,\"part\":\"taskList\",\"list\":\"tasks\",\"by\":\"position\",\"parent\":null,\"move\":%s,\"handler\":null,\"scope\":\"form\"}", tasks, drag), NULL);
    CHECK_JSON(data_of(moved), "{\"tasks\":[{\"id\":3,\"position\":1},{\"id\":1,\"position\":2},{\"id\":2,\"position\":3}]}");
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(moved, "value"), "state"), "selected"), "taskList"), "0");
    // A refused move answers the window as it is.
    cJSON* refused = invoked(s, "reorder_event", clarity_format("{\"state\":%s,\"part\":\"taskList\",\"list\":\"tasks\",\"by\":\"position\",\"move\":{\"row\":0,\"key\":\"up\"},\"scope\":\"form\"}", tasks), NULL);
    CHECK_JSON(data_of(refused), "{\"tasks\":[{\"id\":1,\"position\":1},{\"id\":2,\"position\":2},{\"id\":3,\"position\":3}]}");
    // Ctrl+Z takes the move back.
    cJSON* back = invoked(s, "undo_edit", clarity_format("{\"state\":{\"window\":\"Card\",\"instance\":9,\"data\":%s},\"scope\":\"form\",\"back\":true}", cJSON_PrintUnformatted(data_of(moved))), NULL);
    CHECK_JSON(data_of(back), "{\"tasks\":[{\"id\":1,\"position\":1},{\"id\":2,\"position\":2},{\"id\":3,\"position\":3}]}");
    // In a tree a row takes its new parent; without a scope the handler runs as dispatch_event does.
    const char* tree = "{\"window\":\"Card\",\"instance\":9,\"data\":{\"rows\":[{\"id\":1,\"up\":null,\"n\":1},{\"id\":2,\"up\":null,\"n\":2}]}}";
    cJSON* under = invoked(s, "reorder_event", clarity_format("{\"state\":%s,\"part\":\"outline\",\"list\":\"rows\",\"by\":\"n\",\"parent\":\"up\",\"move\":{\"row\":1,\"key\":\"right\"},\"handler\":\"remember\"}", tree), NULL);
    CHECK_JSON(data_of(under), "{\"rows\":[{\"id\":1,\"up\":null,\"n\":1},{\"id\":2,\"up\":1,\"n\":1}]}");
    CHECK(cJSON_GetObjectItem(cJSON_GetObjectItem(under, "value"), "step"));
    CHECK_INT(memos(db), 1);
    clarity_service_free(s);
    clarity_db_close(db);
}

// A list field's typed item (DESIGN 1.7, U1c): the runtime adds it, or
// refuses it and says so, so the screen keeps the typed text.
void the_runtime_adds_a_typed_item_or_refuses_it(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    CHECK(clarity_service_watches("item_event"));
    const char* album = "{\"window\":\"Card\",\"instance\":11,\"data\":{\"album\":{\"tags\":[\"jazz\"],\"years\":[1999]}}}";
    invoked(s, "dispatch_event", clarity_format("{\"state\":%s,\"handler\":\"noop\"}", album), NULL);
    cJSON* added = invoked(s, "item_event", clarity_format("{\"state\":%s,\"part\":\"tags\",\"path\":\"album.tags\",\"item\":\"text\",\"typed\":\"  live \",\"scope\":\"form\"}", album), NULL);
    CHECK_JSON(data_of(added), "{\"album\":{\"tags\":[\"jazz\",\"live\"],\"years\":[1999]}}");
    CHECK(!cJSON_GetObjectItem(cJSON_GetObjectItem(added, "value"), "refused"));
    cJSON* refused = invoked(s, "item_event", clarity_format("{\"state\":%s,\"part\":\"years\",\"path\":\"album.years\",\"item\":\"integer\",\"typed\":\"soon\",\"scope\":\"form\"}", album), NULL);
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(refused, "value"), "refused"), "true");
    CHECK_JSON(data_of(refused), "{\"album\":{\"tags\":[\"jazz\"],\"years\":[1999]}}");
    // An enum's list takes only its values.
    cJSON* polka = invoked(s, "item_event", clarity_format("{\"state\":%s,\"part\":\"tags\",\"path\":\"album.tags\",\"item\":\"enum\",\"values\":[\"jazz\",\"rock\"],\"typed\":\"polka\"}", album), NULL);
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(polka, "value"), "refused"), "true");
    // The added item is an edit that Ctrl+Z takes back.
    cJSON* back = invoked(s, "undo_edit", clarity_format("{\"state\":{\"window\":\"Card\",\"instance\":11,\"data\":%s},\"scope\":\"form\",\"back\":true}", cJSON_PrintUnformatted(data_of(added))), NULL);
    CHECK_JSON(data_of(back), "{\"album\":{\"tags\":[\"jazz\"],\"years\":[1999]}}");
    clarity_service_free(s);
    clarity_db_close(db);
}

// A slider's and a spin box's value (DESIGN 1.7, U1c): after a key, a drag
// or typed text, the runtime keeps it within its range and steps.
void the_runtime_moves_a_value_within_its_range(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    CHECK(clarity_service_watches("range_event"));
    const char* mixer = "{\"window\":\"Card\",\"instance\":12,\"data\":{\"volume\":5,\"mix\":{\"balance\":\"0.2\"}}}";
    const char* volume = "\"part\":\"volume\",\"path\":\"volume\",\"min\":\"0\",\"max\":\"10\",\"step\":\"1\",\"whole\":true";
    CHECK_JSON(data_of(invoked(s, "range_event", clarity_format("{\"state\":%s,%s,\"key\":\"up\"}", mixer, volume), NULL)), "{\"volume\":6,\"mix\":{\"balance\":\"0.2\"}}");
    CHECK_JSON(data_of(invoked(s, "range_event", clarity_format("{\"state\":%s,%s,\"typed\":\"120\"}", mixer, volume), NULL)), "{\"volume\":10,\"mix\":{\"balance\":\"0.2\"}}");
    CHECK_JSON(data_of(invoked(s, "range_event", clarity_format("{\"state\":%s,%s,\"at\":3.4}", mixer, volume), NULL)), "{\"volume\":3,\"mix\":{\"balance\":\"0.2\"}}");
    // A decimal stays text, exact.
    const char* balance = "\"part\":\"balance\",\"path\":\"mix.balance\",\"min\":\"0\",\"max\":\"1\",\"step\":\"0.1\",\"whole\":false";
    CHECK_JSON(data_of(invoked(s, "range_event", clarity_format("{\"state\":%s,%s,\"key\":\"up\",\"scope\":\"form\"}", mixer, balance), NULL)), "{\"volume\":5,\"mix\":{\"balance\":\"0.3\"}}");
    CHECK(cJSON_GetObjectItem(invoked(s, "range_event", clarity_format("{\"state\":%s,%s,\"typed\":\"loud\"}", mixer, volume), NULL), "error"));
    clarity_service_free(s);
    clarity_db_close(db);
}

/// The step a reply's wizard is on.
static const cJSON* step_of(const cJSON* answer) {
    return cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(answer, "value"), "state"), "parts"), "wizard"), "step");
}

// A wizard's buttons (DESIGN 1.7, 5.15): the runtime runs on next and on
// finish and decides the step.
void the_runtime_walks_a_wizard(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_db_batch(db, "", "CREATE TABLE memo (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)"));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    CHECK(clarity_service_watches("step_event"));
    const char* first = "{\"window\":\"Card\",\"data\":{}}";
    const char* second = "{\"window\":\"Card\",\"data\":{},\"parts\":{\"wizard\":{\"step\":2}}}";
    // Without on next, next moves on.
    CHECK_JSON(step_of(invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"next\",\"steps\":3,\"next\":null}", first), NULL)), "2");
    CHECK_JSON(step_of(invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"next\",\"steps\":3,\"next\":\"noop\"}", first), NULL)), "2");
    // A refusal keeps the step: the screen shows it.
    CHECK(cJSON_GetObjectItem(invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"next\",\"steps\":3,\"next\":\"save\"}", first), NULL), "error"));
    // A handler that sets the step decides where the wizard goes.
    CHECK_JSON(step_of(invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"next\",\"steps\":3,\"next\":\"jump\"}", first), NULL)), "3");
    // Back runs nothing.
    CHECK_JSON(step_of(invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"back\",\"steps\":3,\"next\":\"remember\"}", second), NULL)), "1");
    CHECK_INT(memos(db), 0);
    // Finish runs on finish, as one database step.
    cJSON* finished = invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"finish\",\"steps\":2,\"finish\":\"remember\"}", second), NULL);
    CHECK(cJSON_GetObjectItem(cJSON_GetObjectItem(finished, "value"), "step"));
    CHECK_INT(memos(db), 1);
    clarity_service_free(s);
    clarity_db_close(db);
}

static bool search_options(ClarityDb* db, const char* entity, cJSON** out) {
    (void)db;
    (void)entity;
    *out = cJSON_Parse("[{\"id\":1,\"text\":\"Jansen\",\"fields\":{\"city\":\"Utrecht\"}},{\"id\":2,\"text\":\"de Vries\",\"fields\":{\"city\":\"Gent\"}}]");
    return true;
}

static bool city_values(ClarityDb* db, const char* entity, const char* field, cJSON** out) {
    (void)db;
    if (strcmp(entity, "Customer") != 0 || strcmp(field, "city") != 0) return clarity_fail(CLARITY_INVALID, "%s has no field %s", entity, field);
    *out = cJSON_Parse("[\"Utrecht\",\"amsterdam\",\"\",\"Amstelveen\",\"Utrecht\"]");
    return true;
}

static const ClarityWindows customers = {open_card, handler_of, dispatch, search_options, picture, redraw, answer, city_values};

// A dropdown's options (DESIGN 1.7, U1c): the runtime picks what it offers for typed text.
void the_runtime_finds_a_dropdowns_options(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &customers};
    ClarityService* s = clarity_service_new(&options);
    CHECK_JSON(invoked(s, "find_options", "{\"kind\":\"typed\",\"entity\":\"Customer\",\"field\":\"city\",\"typed\":\"amst\"}", NULL), "{\"value\":[\"Amstelveen\",\"amsterdam\"]}");
    CHECK_JSON(invoked(s, "find_options", "{\"kind\":\"search\",\"entity\":\"Customer\",\"show\":[\"city\"],\"typed\":\"gent\"}", NULL),
               "{\"value\":[{\"id\":2,\"text\":\"de Vries\",\"fields\":{\"city\":\"Gent\"}}]}");
    CHECK_JSON(invoked(s, "find_options", "{\"kind\":\"search\",\"entity\":\"Customer\",\"typed\":\"\"}", NULL),
               "{\"value\":[{\"id\":1,\"text\":\"Jansen\",\"fields\":{\"city\":\"Utrecht\"}},{\"id\":2,\"text\":\"de Vries\",\"fields\":{\"city\":\"Gent\"}}]}");
    CHECK(cJSON_GetObjectItem(invoked(s, "find_options", "{\"kind\":\"typed\",\"entity\":\"Customer\",\"field\":\"name\",\"typed\":\"\"}", NULL), "error"));
    clarity_service_free(s);
    clarity_db_close(db);
}

/// The view of `part` in a reply.
static const cJSON* view_of(const cJSON* answer, const char* part) {
    return cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(answer, "value"), "state"), "views"), part);
}

static const char* const planner =
    "{\"window\":\"Card\",\"data\":{\"tasks\":[{\"id\":1,\"title\":\"Milk\",\"up\":null},{\"id\":2,\"title\":\"bread\",\"up\":1},{\"id\":3,\"title\":\"apples\",\"up\":null}],"
    "\"sales\":[{\"city\":\"Gent\",\"amount\":\"2\"},{\"city\":\"Gent\",\"amount\":3},{\"city\":\"Ams\",\"amount\":1}]},"
    "\"views\":{\"grid\":{\"kind\":\"table\",\"list\":\"tasks\",\"columns\":[{\"field\":\"title\",\"kind\":\"text\"}],\"order\":{\"field\":\"title\",\"desc\":false},\"page\":2,\"at\":1},"
    "\"outline\":{\"kind\":\"tree\",\"list\":\"tasks\",\"parent\":\"up\",\"key\":\"id\",\"open\":[]},"
    "\"chart\":{\"kind\":\"chart\",\"list\":\"sales\",\"by\":\"city\",\"byKind\":\"text\",\"measure\":\"sum\",\"of\":\"amount\"}}}";

// The shown rows (DESIGN 1.7): every reply fills what the window's tables,
// trees and charts show from its data, so a handler's change shows at once.
void a_reply_fills_the_shown_rows_of_its_views(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    cJSON* reply = invoked(s, "dispatch_event", clarity_format("{\"state\":%s,\"handler\":\"noop\"}", planner), NULL);
    CHECK_JSON(cJSON_GetObjectItem(view_of(reply, "grid"), "shown"), "[2,1]");
    CHECK_JSON(cJSON_GetObjectItem(view_of(reply, "grid"), "pages"), "{\"page\":1,\"pages\":2,\"from\":1,\"to\":2,\"count\":3}");
    CHECK_JSON(cJSON_GetObjectItem(view_of(reply, "outline"), "shown"), "[[0,0,true],[2,0,false]]");
    CHECK_JSON(cJSON_GetObjectItem(view_of(reply, "chart"), "groups"), "[{\"key\":\"Ams\",\"value\":1},{\"key\":\"Gent\",\"value\":5}]");
    // A row a handler chose shows: the tree opens the rows above it, the table turns to its page.
    cJSON* chosen = invoked(s, "dispatch_event", clarity_format("{\"state\":%s,\"handler\":\"choose\"}", planner), NULL);
    CHECK_JSON(cJSON_GetObjectItem(view_of(chosen, "outline"), "open"), "[1]");
    CHECK_JSON(cJSON_GetObjectItem(view_of(chosen, "outline"), "shown"), "[[0,0,true],[1,1,false],[2,0,false]]");
    CHECK_JSON(cJSON_GetObjectItem(view_of(chosen, "grid"), "shown"), "[0]");
    CHECK_JSON(cJSON_GetObjectItem(view_of(chosen, "grid"), "at"), "2");
    // A handler that adds a row: the table shows it.
    cJSON* added = invoked(s, "dispatch_event", clarity_format("{\"state\":%s,\"handler\":\"addTask\"}", planner), NULL);
    CHECK_JSON(cJSON_GetObjectItem(view_of(added, "grid"), "pages"), "{\"page\":1,\"pages\":2,\"from\":1,\"to\":2,\"count\":4}");
    CHECK_JSON(cJSON_GetObjectItem(view_of(added, "outline"), "shown"), "[[0,0,true],[2,0,false],[3,0,false]]");
    // A window without views has none in its reply.
    CHECK(!cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"noop\"}", NULL), "value"), "state"), "views"));
    clarity_service_free(s);
    clarity_db_close(db);
}

/// view_event on the state a reply gave.
static cJSON* viewed(ClarityService* s, const cJSON* reply, const char* part, const char* action) {
    const char* state = cJSON_PrintUnformatted(cJSON_GetObjectItem(cJSON_GetObjectItem(reply, "value"), "state"));
    return invoked(s, "view_event", clarity_format("{\"state\":%s,\"part\":\"%s\",\"action\":%s}", state, part, action), NULL);
}

// The person sorts a table, turns its page, or opens or closes a tree's row:
// the runtime changes the view and answers the window. No handler, no edit.
void a_view_event_sorts_pages_opens_and_closes(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityShellOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", "", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    CHECK(!clarity_service_watches("view_event"));
    cJSON* first = invoked(s, "dispatch_event", clarity_format("{\"state\":%s,\"handler\":\"choose\"}", planner), NULL);
    // The chosen row stays chosen, and the person turns away from its page.
    cJSON* page = viewed(s, first, "grid", "{\"page\":1}");
    CHECK_JSON(cJSON_GetObjectItem(view_of(page, "grid"), "shown"), "[2,1]");
    // Sorting by the same field again sorts the other way, from the first page.
    cJSON* sorted = viewed(s, page, "grid", "{\"sort\":\"title\"}");
    CHECK_JSON(cJSON_GetObjectItem(view_of(sorted, "grid"), "order"), "{\"field\":\"title\",\"desc\":true}");
    CHECK_JSON(cJSON_GetObjectItem(view_of(sorted, "grid"), "shown"), "[0,1]");
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(view_of(sorted, "grid"), "pages"), "page"), "1");
    // A row closed above the chosen row stays closed, and opens again.
    cJSON* closed = viewed(s, first, "outline", "{\"close\":1}");
    CHECK_JSON(cJSON_GetObjectItem(view_of(closed, "outline"), "shown"), "[[0,0,true],[2,0,false]]");
    cJSON* opened = viewed(s, closed, "outline", "{\"open\":1}");
    CHECK_JSON(cJSON_GetObjectItem(view_of(opened, "outline"), "open"), "[1]");
    CHECK(cJSON_GetObjectItem(viewed(s, first, "nothing", "{\"page\":1}"), "error"));
    CHECK(cJSON_GetObjectItem(viewed(s, first, "grid", "{\"jump\":1}"), "error"));
    clarity_service_free(s);
    clarity_db_close(db);
}
