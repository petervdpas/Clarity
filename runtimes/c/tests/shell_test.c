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

static bool dispatch(ClarityDb* db, const ClarityWindowState* state, const char* handler, ClarityReply** out) {
    (void)db;
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

static const ClarityWindows cards = {open_card, handler_of, dispatch, options, picture};

typedef struct {
    ClarityDesktop desktop;
    const char* title;
    bool closed;
    const char* opened;
} Fake;

static void fake_title(ClarityDesktop* desktop, const char* title) { ((Fake*)desktop)->title = clarity_copy(title); }
static void fake_close(ClarityDesktop* desktop) { ((Fake*)desktop)->closed = true; }

static void fake_open(void* context, ClarityDesktop* opener, const char* name) {
    (void)opener;
    ((Fake*)context)->opened = clarity_copy(name);
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
    cJSON* opened = invoked(s, "open_window", "{\"window\":\"Card\",\"with\":{\"name\":\"Grace\"}}", NULL);
    const cJSON* title = cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(cJSON_GetObjectItem(opened, "value"), "state"), "parts"), "title"), "caption");
    CHECK_JSON(title, "\"Card of Grace\"");
    // A refusal keeps its slots; other errors are text.
    CHECK_JSON(invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"save\"}", NULL),
               "{\"error\":{\"text\":\"{name} is taken\",\"values\":{\"name\":{\"kind\":\"text\",\"value\":\"Ada\"}}}}");
    CHECK_JSON(invoked(s, "dispatch_event", "{\"state\":{\"window\":\"Card\"},\"handler\":\"fail\"}", NULL), "{\"error\":\"storage: locked\"}");
    CHECK_JSON(invoked(s, "ref_options", "{\"entity\":\"Customer\"}", NULL), "{\"value\":[{\"id\":1,\"text\":\"Ada\"}]}");
    CHECK_JSON(invoked(s, "picture", "{\"entity\":\"Card\",\"id\":1,\"field\":\"photo\"}", NULL), "{\"value\":\"data:image/png;base64,iVBORw0KGgo=\"}");
    CHECK_JSON(invoked(s, "picture", "{\"entity\":\"Card\",\"id\":2,\"field\":\"photo\"}", NULL), "{\"value\":null}");
    CHECK_JSON(invoked(s, "nonsense", "{}", NULL), "{\"error\":\"there is no command nonsense\"}");
    CHECK_JSON(invoked(s, "start_app", "{}", NULL),
               "{\"value\":{\"state\":{\"window\":\"\",\"data\":{},\"parts\":{},\"selected\":{}},\"messages\":[],\"open\":[],\"close\":false}}");

    // The window a command came from takes its title, and a window of its own starts where it was opened.
    Fake main = {{"main", fake_title, fake_close}, NULL, false, NULL};
    invoked(s, "set_title", "{\"title\":\"Customer card\"}", &main.desktop);
    CHECK_TEXT(main.title, "Customer card");
    clarity_service_on_desktop(s, NULL, fake_open, &main);
    invoked(s, "open_apart", "{\"window\":\"Card\",\"with\":{\"name\":\"Linus\"}}", &main.desktop);
    CHECK_TEXT(main.opened, "apart-1");
    Fake apart = {{"apart-1", fake_title, fake_close}, NULL, false, NULL};
    CHECK_JSON(invoked(s, "apart_start", "{}", &apart.desktop), "{\"value\":{\"window\":\"Card\",\"with\":{\"name\":\"Linus\"}}}");
    CHECK_TEXT(clarity_service_closed(s, "apart-1"), "main");
    CHECK(!clarity_service_closed(s, "apart-1"));
    invoked(s, "close_apart", "{}", &main.desktop);
    CHECK(main.closed);
    clarity_service_free(s);
    clarity_db_close(db);
}
