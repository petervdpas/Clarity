// The service's commands, run without a window: the same commands and JSON
// as every stack's web wire.

#include <stddef.h>

#include "check.h"
#include "clarity/code.h"
#include "clarity/sql.h"
#include "service/service.h"
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
    ClarityUi* ui = clarity_ui(state);
    if (strcmp(handler, "opened") == 0) {
        const char* name = cJSON_GetObjectItem(state->data, "name")->valuestring;
        clarity_ui_set_caption(ui, "title", clarity_plain_text(clarity_join("Card of ", name)));
    }
    // A wizard's on next that sends it to its last step.
    if (strcmp(handler, "jump") == 0) clarity_ui_set_step(ui, "wizard", 3);
    if (strcmp(handler, "save") == 0) return clarity_refuse(clarity_text_with(clarity_text("{name} is taken"), "name", clarity_slot_text("Ada")));
    if (strcmp(handler, "fail") == 0) return clarity_fail(CLARITY_STORAGE, "locked");
    if (strcmp(handler, "divide") == 0) {
        int64_t n = 0;
        return clarity_whole_div(1, 0, &n);
    }
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


/// The last link the shell opened, in place of the browser.
static const char* last_link;

static const char* last_line;

static void fake_log(const char* line) { last_line = clarity_copy(line); }

static bool fake_link(const char* address) {
    last_link = clarity_copy(address);
    return true;
}

static cJSON* invoked(ClarityService* s, const char* command, const char* args) {
    return clarity_service_invoke(s, command, cJSON_Parse(args));
}

void the_service_answers_the_wires_commands(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityServiceOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    CHECK_JSON(invoked(s, "start", "{}"), "{\"value\":{\"model\":{\"project\":{}},\"main\":\"Card\"}}");
    // The screen names the language text code makes is translated to.
    invoked(s, "set_language", "{\"language\":\"nl\"}");
    invoked(s, "set_language", "{\"language\":null}");
    cJSON* opened = invoked(s, "open_window", "{\"window\":\"Card\",\"with\":{\"name\":\"Grace\"}}");
    const cJSON* title = cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(opened, "value"), "state"), "parts"), "title"), "caption");
    CHECK_JSON(title, "\"Card of Grace\"");
    // Its canvases are drawn anew, when it opens and after every handler.
    const cJSON* badge = cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(opened, "value"), "state"), "parts"), "badge");
    CHECK(cJSON_GetArraySize(cJSON_GetObjectItem(badge, "drawing")) == 5);
    cJSON* again = invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{\"name\":\"Ada\"}},\"handler\":\"noop\"}");
    badge = cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(again, "value"), "state"), "parts"), "badge");
    CHECK(cJSON_GetArraySize(cJSON_GetObjectItem(badge, "drawing")) == 3);
    // A refusal keeps its slots; other errors are text.
    CHECK_JSON(invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"save\"}"),
               "{\"error\":{\"text\":\"{name} is taken\",\"values\":{\"name\":{\"kind\":\"text\",\"value\":\"Ada\"}}}}");
    CHECK_JSON(invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\"},\"handler\":\"fail\"}"), "{\"error\":\"storage: locked\"}");
    // A value error is said in Clarity's own words by its key, which the screen translates.
    CHECK_JSON(invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\"},\"handler\":\"divide\"}"), "{\"error\":{\"key\":\"divisionByZero\",\"text\":\"Division by zero\"}}");
    // The answer to a question runs the handler it goes to, and the canvas is drawn again.
    cJSON* answered = invoked(s, "answer_event", "{\"state\":{\"window\":\"Card\",\"data\":{\"name\":\"Ada\"}},\"handler\":\"rename\",\"values\":[7,\"Grace\"]}");
    CHECK_TEXT(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(answered, "value"), "state"), "data"), "name")->valuestring, "Grace");
    CHECK_JSON(invoked(s, "ref_options", "{\"entity\":\"Customer\"}"), "{\"value\":[{\"id\":1,\"text\":\"Ada\"}]}");
    CHECK_JSON(invoked(s, "picture", "{\"entity\":\"Card\",\"id\":1,\"field\":\"photo\"}"), "{\"value\":\"data:image/png;base64,iVBORw0KGgo=\"}");
    CHECK_JSON(invoked(s, "picture", "{\"entity\":\"Card\",\"id\":2,\"field\":\"photo\"}"), "{\"value\":null}");
    CHECK_JSON(invoked(s, "nonsense", "{}"), "{\"error\":\"there is no command nonsense\"}");
    CHECK_JSON(invoked(s, "start_app", "{}"),
               "{\"value\":{\"state\":{\"window\":\"\",\"instance\":0,\"data\":{},\"parts\":{},\"selected\":{}},\"messages\":[],\"open\":[],\"close\":false}}");

    // A link of a markdown part opens in the browser: only a web or a mail address.
    clarity_service_on_link(s, fake_link);
    CHECK_JSON(invoked(s, "open_link", "{\"address\":\"https://example.com/rules\"}"), "{\"value\":null}");
    CHECK_TEXT(last_link, "https://example.com/rules");
    CHECK_JSON(invoked(s, "open_link", "{\"address\":\"file:///etc/passwd\"}"),
               "{\"error\":\"a link opens only an http, https or mailto address, not file:///etc/passwd\"}");
    CHECK_TEXT(last_link, "https://example.com/rules");

    // An engine beside the screen logs through the shell: into the application's log.
    clarity_service_on_log(s, fake_log);
    CHECK_JSON(invoked(s, "log_line", "{\"line\":\"engine Screen: level 2\"}"), "{\"value\":null}");
    CHECK_TEXT(last_line, "engine Screen: level 2");

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
    ClarityServiceOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    closed_window = 0;
    clarity_on_window_closed(closed, NULL);
    const cJSON* state = cJSON_GetObjectItem(cJSON_GetObjectItem(invoked(s, "open_window", "{\"window\":\"Card\",\"with\":{}}"), "value"), "state");
    uint64_t instance = (uint64_t)cJSON_GetObjectItem(state, "instance")->valuedouble;
    cJSON* args = cJSON_CreateObject();
    cJSON_AddItemToObject(args, "state", cJSON_Duplicate(state, true));
    invoked(s, "close_window", cJSON_PrintUnformatted(args));
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

/// The databases did not open (manual/data.settings.md): start says what to show and
/// why, and connect opens them once the person has kept their connection.
void an_unconnected_start_says_why_and_connects_later(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityServiceOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", db, &cards};
    clarity_fail(CLARITY_NOT_KEPT, "the secret setting shopDb is not kept yet");
    ClarityError not_kept = *clarity_error();
    options.unconnected = &not_kept;
    options.unconnected_window = "Preferences";
    options.connect = connect_later;
    connect_tries = 0;
    ClarityService* s = clarity_service_new(&options);
    CHECK_JSON(invoked(s, "start", "{}"),
               "{\"value\":{\"model\":{\"project\":{}},\"main\":\"Card\",\"unconnected\":{\"handler\":null,\"window\":\"Preferences\",\"problem\":{\"key\":\"unconnectedNotKept\"}}}}");
    CHECK_JSON(invoked(s, "connect", "{}"), "{\"error\":{\"key\":\"unconnectedFailed\",\"values\":{\"problem\":{\"kind\":\"text\",\"value\":\"still not kept\"}}}}");
    CHECK_JSON(invoked(s, "connect", "{}"), "{\"value\":null}");
    CHECK_JSON(invoked(s, "start", "{}"), "{\"value\":{\"model\":{\"project\":{}},\"main\":\"Card\"}}");
    clarity_service_free(s);
    // The empty database stays the caller's, who closes it once: the service closes only what it opened.
    clarity_db_close(db);
}

static size_t memos(ClarityDb* db) {
    void* items;
    size_t count = 0;
    clarity_find(db, &memo, clarity_query(), &items, &count);
    return count;
}

// Undo (manual/code.handlers.md): the runtime keeps the window's edits, each with the
// database steps its handler made, and Ctrl+Z takes them back together.
void the_runtime_keeps_the_edits_and_ctrl_z_takes_them_back(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_db_batch(db, "", "CREATE TABLE memo (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)"));
    ClarityServiceOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    // A handler that changed the database answers with its step; one that failed or changed nothing, with none.
    CHECK(cJSON_GetObjectItem(invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"fail\"}"), "error"));
    // A reply: the edits start from the window as it is.
    cJSON* nothing = invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ada\"}},\"handler\":\"noop\"}");
    CHECK(!cJSON_GetObjectItem(cJSON_GetObjectItem(nothing, "value"), "step"));
    // The person typed a name, then pressed a part whose handler saves: two edits of the form.
    invoked(s, "edit_event", "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ann\"}},\"part\":\"nameBox\",\"scope\":\"form\",\"ends\":true}");
    cJSON* saved = invoked(s, "edit_event", "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ann\"}},\"handler\":\"remember\",\"part\":\"saveBox\",\"scope\":\"form\",\"ends\":true}");
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(saved, "value"), "step"), "1");
    CHECK_INT(memos(db), 1);
    const char* back = "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ann\"}},\"scope\":\"form\",\"back\":true}";
    CHECK(cJSON_GetObjectItem(invoked(s, "undo_edit", back), "value"));
    CHECK_INT(memos(db), 0);
    cJSON* typed = invoked(s, "undo_edit", back);
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(typed, "value"), "state"), "data"), "{\"name\":\"Ada\"}");
    const char* again = "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ada\"}},\"scope\":\"form\",\"back\":false}";
    cJSON* retyped = invoked(s, "undo_edit", again);
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(retyped, "value"), "state"), "data"), "{\"name\":\"Ann\"}");
    invoked(s, "undo_edit", again);
    CHECK_INT(memos(db), 1);
    // A closed window's edits go.
    invoked(s, "close_window", "{\"state\":{\"window\":\"Card\",\"instance\":7,\"data\":{\"name\":\"Ann\"}}}");
    cJSON* gone = invoked(s, "undo_edit", back);
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(gone, "value"), "state"), "data"), "{\"name\":\"Ann\"}");
    // The old command is gone.
    CHECK(cJSON_GetObjectItem(invoked(s, "undo_step", "{\"step\":1,\"back\":true}"), "error"));
    clarity_service_free(s);
    clarity_db_close(db);
}

/// The data of the window a reply shows.
static const cJSON* data_of(const cJSON* answer) {
    return cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(answer, "value"), "state"), "data");
}

static const char* const tasks = "{\"window\":\"Card\",\"instance\":9,\"data\":{\"tasks\":[{\"id\":1,\"position\":1},{\"id\":2,\"position\":2},{\"id\":3,\"position\":3}]}}";

// A canvas (manual/start.how-it-works.md, manual/parts.graphics.md): the screen sends what the person did, and the
// runtime snaps it, chooses what it touched, runs on change as one edit that
// Ctrl+Z takes back, and refuses an action that changes nothing.
void the_runtime_moves_an_object_and_edits_a_drawing(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_db_batch(db, "", "CREATE TABLE memo (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)"));
    ClarityServiceOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    const char* types = "[{\"id\":1,\"x\":0,\"y\":0},{\"id\":2,\"x\":40,\"y\":40}]";
    const char* card = clarity_format("{\"window\":\"Card\",\"instance\":31,\"data\":{\"types\":%s,\"note\":{\"drawing\":\"\"}}}", types);
    invoked(s, "dispatch_event", clarity_format("{\"state\":%s,\"handler\":\"noop\"}", card));
    const char* diagram = "\"part\":\"diagram\",\"scope\":\"diagram\",\"mode\":\"objects\",\"list\":\"types\",\"at\":[\"x\",\"y\"],\"grid\":20,\"snap\":true,\"handler\":\"remember\"";
    cJSON* moved = invoked(s, "canvas_event", clarity_format("{\"state\":%s,%s,\"action\":{\"move\":{\"id\":2,\"x\":67,\"y\":31}}}", card, diagram));
    CHECK_JSON(cJSON_GetObjectItem(data_of(moved), "types"), "[{\"id\":1,\"x\":0,\"y\":0},{\"id\":2,\"x\":60,\"y\":40}]");
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(moved, "value"), "state"), "selected"), "{\"diagram\":1}");
    CHECK(cJSON_GetObjectItem(cJSON_GetObjectItem(moved, "value"), "step"));
    CHECK_INT(memos(db), 1);
    // Ctrl+Z takes the move back.
    cJSON* back = invoked(s, "undo_edit", clarity_format("{\"state\":{\"window\":\"Card\",\"instance\":31,\"data\":%s},\"scope\":\"diagram\",\"back\":true}", cJSON_PrintUnformatted(data_of(moved))));
    CHECK_JSON(cJSON_GetObjectItem(data_of(back), "types"), types);
    // Dropped where it is: refused, running nothing.
    cJSON* refused = invoked(s, "canvas_event", clarity_format("{\"state\":%s,%s,\"action\":{\"move\":{\"id\":1,\"x\":5,\"y\":-4}}}", card, diagram));
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(refused, "value"), "refused"), "true");
    CHECK_INT(memos(db), 0);
    cJSON* drawn = invoked(s, "canvas_event", clarity_format("{\"state\":%s,\"part\":\"drawing\",\"scope\":\"drawing\",\"mode\":\"vector\",\"path\":\"note.drawing\",\"units\":[400,300],\"action\":{\"add\":{\"op\":\"rect\",\"x\":1,\"y\":2,\"width\":3,\"height\":4,\"color\":\"#ff0000\"}}}", card));
    CHECK_TEXT(cJSON_GetObjectItem(cJSON_GetObjectItem(data_of(drawn), "note"), "drawing")->valuestring,
               "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 400 300\">\n<rect x=\"1\" y=\"2\" width=\"3\" height=\"4\" fill=\"#ff0000\"/>\n</svg>\n");
    clarity_service_free(s);
    clarity_db_close(db);
}

/// The step a reply's wizard is on.
static const cJSON* step_of(const cJSON* answer) {
    return cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(answer, "value"), "state"), "parts"), "wizard"), "step");
}

// A wizard's buttons (manual/start.how-it-works.md, manual/parts.containers.md): the runtime runs on next and on
// finish and decides the step.
void the_runtime_walks_a_wizard(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_db_batch(db, "", "CREATE TABLE memo (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)"));
    ClarityServiceOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    const char* first = "{\"window\":\"Card\",\"data\":{}}";
    const char* second = "{\"window\":\"Card\",\"data\":{},\"parts\":{\"wizard\":{\"step\":2}}}";
    // Without on next, next moves on.
    CHECK_JSON(step_of(invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"next\",\"steps\":3,\"next\":null}", first))), "2");
    CHECK_JSON(step_of(invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"next\",\"steps\":3,\"next\":\"noop\"}", first))), "2");
    // A refusal keeps the step: the screen shows it.
    CHECK(cJSON_GetObjectItem(invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"next\",\"steps\":3,\"next\":\"save\"}", first)), "error"));
    // A handler that sets the step decides where the wizard goes.
    CHECK_JSON(step_of(invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"next\",\"steps\":3,\"next\":\"jump\"}", first))), "3");
    // Back runs nothing.
    CHECK_JSON(step_of(invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"back\",\"steps\":3,\"next\":\"remember\"}", second))), "1");
    CHECK_INT(memos(db), 0);
    // Finish runs on finish, as one database step.
    cJSON* finished = invoked(s, "step_event", clarity_format("{\"state\":%s,\"part\":\"wizard\",\"button\":\"finish\",\"steps\":2,\"finish\":\"remember\"}", second));
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

// A dropdown's options (manual/start.how-it-works.md): the runtime picks what it offers for typed text.
void the_runtime_finds_a_dropdowns_options(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityServiceOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", db, &customers};
    ClarityService* s = clarity_service_new(&options);
    // The records of the entity; the screen searches them as the person types.
    CHECK_JSON(invoked(s, "find_options", "{\"entity\":\"Customer\",\"show\":[\"city\"]}"),
               "{\"value\":[{\"id\":1,\"text\":\"Jansen\",\"fields\":{\"city\":\"Utrecht\"}},{\"id\":2,\"text\":\"de Vries\",\"fields\":{\"city\":\"Gent\"}}]}");
    clarity_service_free(s);
    clarity_db_close(db);
}

/// The window Card's data a command is answered with.
static cJSON* answer_data(ClarityService* s, const char* command, const char* args) { return (cJSON*)data_of(invoked(s, command, args)); }

// A readonly part (manual/start.how-it-works.md): the runtime takes back what the screen
// changed, runs no handler and records no edit, and says it refused.
void a_readonly_part_takes_back_what_the_screen_changed(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    REQUIRE_OK(clarity_db_batch(db, "", "CREATE TABLE memo (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)"));
    ClarityServiceOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"instance\":21,\"data\":{\"name\":\"Ada\",\"volume\":5,\"tags\":[\"jazz\"],\"rows\":[{\"id\":1,\"n\":1},{\"id\":2,\"n\":2}]}},\"handler\":\"noop\"}");
    const char* typed = "{\"window\":\"Card\",\"instance\":21,\"data\":{\"name\":\"Ann\",\"volume\":5,\"tags\":[\"jazz\"],\"rows\":[{\"id\":1,\"n\":1},{\"id\":2,\"n\":2}]}}";
    cJSON* edited = invoked(s, "edit_event", clarity_format("{\"state\":%s,\"part\":\"nameBox\",\"scope\":\"form\",\"handler\":\"remember\",\"ends\":true,\"readonly\":true}", typed));
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(edited, "value"), "refused"), "true");
    CHECK_JSON(cJSON_GetObjectItem(data_of(edited), "name"), "\"Ada\"");
    CHECK_INT(memos(db), 0);
    // Nothing was recorded, so Ctrl+Z finds nothing to take back.
    CHECK_JSON(answer_data(s, "undo_edit", "{\"state\":{\"window\":\"Card\",\"instance\":21,\"data\":{\"name\":\"Zed\"}},\"scope\":\"form\",\"back\":true}"), "{\"name\":\"Zed\"}");
    CHECK_INT(memos(db), 0);
    clarity_service_free(s);
    clarity_db_close(db);
}

// A table edits only the columns of its `edit` (manual/start.how-it-works.md): a change to
// any other field of its rows, or rows added, removed or moved, is taken back.
void a_table_takes_back_a_change_to_a_column_it_does_not_edit(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityServiceOptions options = {"shop", "Shop", "Card", "{\"project\":{}}", db, &cards};
    ClarityService* s = clarity_service_new(&options);
    const char* rows = "[{\"id\":1,\"title\":\"Milk\",\"hours\":2},{\"id\":2,\"title\":\"Eggs\",\"hours\":1}]";
    invoked(s, "dispatch_event", clarity_format("{\"state\":{\"window\":\"Card\",\"instance\":22,\"data\":{\"tasks\":%s}},\"handler\":\"noop\"}", rows));
    const char* edit = "\"part\":\"grid\",\"scope\":\"form\",\"ends\":true,\"path\":\"tasks\",\"editable\":[\"hours\"]";
    // A cell of an edited column: kept.
    cJSON* hours = invoked(s, "edit_event", clarity_format("{\"state\":{\"window\":\"Card\",\"instance\":22,\"data\":{\"tasks\":[{\"id\":1,\"title\":\"Milk\",\"hours\":3},{\"id\":2,\"title\":\"Eggs\",\"hours\":1}]}},%s}", edit));
    CHECK(!cJSON_GetObjectItem(cJSON_GetObjectItem(hours, "value"), "refused"));
    CHECK_JSON(data_of(hours), "{\"tasks\":[{\"id\":1,\"title\":\"Milk\",\"hours\":3},{\"id\":2,\"title\":\"Eggs\",\"hours\":1}]}");
    const char* now = "[{\"id\":1,\"title\":\"Milk\",\"hours\":3},{\"id\":2,\"title\":\"Eggs\",\"hours\":1}]";
    // Another column: taken back.
    cJSON* title = invoked(s, "edit_event", clarity_format("{\"state\":{\"window\":\"Card\",\"instance\":22,\"data\":{\"tasks\":[{\"id\":1,\"title\":\"Bread\",\"hours\":3},{\"id\":2,\"title\":\"Eggs\",\"hours\":1}]}},%s}", edit));
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(title, "value"), "refused"), "true");
    CHECK_JSON(cJSON_GetObjectItem(data_of(title), "tasks"), now);
    // A row more, or the rows in another order: taken back.
    cJSON* more = invoked(s, "edit_event", clarity_format("{\"state\":{\"window\":\"Card\",\"instance\":22,\"data\":{\"tasks\":[{\"id\":1,\"title\":\"Milk\",\"hours\":3},{\"id\":2,\"title\":\"Eggs\",\"hours\":1},{\"id\":3,\"hours\":1}]}},%s}", edit));
    CHECK_JSON(cJSON_GetObjectItem(data_of(more), "tasks"), now);
    cJSON* swapped = invoked(s, "edit_event", clarity_format("{\"state\":{\"window\":\"Card\",\"instance\":22,\"data\":{\"tasks\":[{\"id\":2,\"title\":\"Eggs\",\"hours\":1},{\"id\":1,\"title\":\"Milk\",\"hours\":3}]}},%s}", edit));
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(swapped, "value"), "refused"), "true");
    CHECK_JSON(cJSON_GetObjectItem(data_of(swapped), "tasks"), now);
    clarity_service_free(s);
    clarity_db_close(db);
}
