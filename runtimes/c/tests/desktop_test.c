// The C layer on Clarity.UI (DESIGN 7.9), through the library itself: a
// program driven without a window (DESIGN 1.6) opens its window from the
// files it carries, shows its data, runs its handlers and takes the
// person's edits; the library's own commands are answered (started, a
// record's file, settings, facts); a long call says how far it is and stops
// from another thread; and many threads may ask at once.

#include <stdlib.h>
#include <string.h>

#include "check.h"
#include "clarity/code.h"
#include "clarity/disk.h"
#include "clarity/settings.h"
#include "clarity/splash.h"
#include "desktop/desktop.h"
#include "engine/screen.h"
#include "tests.h"

#if defined(_WIN32)
#include <windows.h>
#else
#include <pthread.h>
#endif

static const char card_markup[] =
    "<Window name=\"Card\">"
    "<Textbox name=\"nameBox\" at=\"1,1\" span=\"4\" bind=\"name\" />"
    "<Button name=\"renameButton\" at=\"1,2\" span=\"3\" caption=\"Rename\" onclick=\"rename\" />"
    "</Window>";

static const ClarityCarried card_windows[] = {{"Card", (const unsigned char*)card_markup, sizeof card_markup - 1}};
static const unsigned char logo[] = {1, 2, 3};
static const ClarityCarried card_resources[] = {{"logo.bin", logo, sizeof logo}};
static const ClarityCarriedFiles card_files = {card_windows, 1, NULL, 0, NULL, 0, card_resources, 1};

/// The person's Stop of a long call, sent from another thread while the handler runs.
static ClarityDesktopProgram* stopping;

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

#if defined(_WIN32)
static DWORD WINAPI stop_from_elsewhere(void* arg) {
#else
static void* stop_from_elsewhere(void* arg) {
#endif
    free(clarity_desktop_answer(stopping, "stop_watch", "{\"id\":5}", 8));
    (void)arg;
    return 0;
}

static bool dispatch(ClarityDb* db, const ClarityWindowState* state, const char* handler, ClarityReply** out) {
    (void)db;
    if (strcmp(handler, "plan") == 0) {
        // As a long engine call does: it says how far it is until it is stopped, from another thread.
        const ClarityEngineWatch* watch = clarity_current_watch();
        if (!watch) return clarity_fail(CLARITY_ENGINE, "no watch");
        for (int64_t day = 1; day < 100000; ++day) {
            watch->progress(day, 365, watch->context);
            if (day == 3) {
                char* progress = clarity_desktop_answer(stopping, "watch_progress", "{\"id\":5}", 8);
                CHECK_TEXT(progress, "{\"value\":[3,365]}");
                free(progress);
#if defined(_WIN32)
                HANDLE thread = CreateThread(NULL, 0, stop_from_elsewhere, NULL, 0, NULL);
                WaitForSingleObject(thread, INFINITE);
                CloseHandle(thread);
#else
                pthread_t thread;
                pthread_create(&thread, NULL, stop_from_elsewhere, NULL);
                pthread_join(thread, NULL);
#endif
            }
            if (*watch->stop) return clarity_fail(CLARITY_ENGINE, "engine Planner failed in plan: it was stopped");
        }
        return clarity_fail(CLARITY_ENGINE, "it was never stopped");
    }
    cJSON* data = cJSON_Duplicate(state->data, true);
    if (strcmp(handler, "rename") == 0) cJSON_ReplaceItemInObject(data, "name", cJSON_CreateString("Grace"));
    ClarityUi* ui = clarity_ui(state);
    // The built-in quit and keys, run by code.
    if (strcmp(handler, "quitIt") == 0) clarity_ui_quit(ui);
    if (strcmp(handler, "keysNow") == 0) clarity_ui_show_keys(ui);
    *out = clarity_ui_reply(ui, state->window, data);
    return true;
}

static bool options(ClarityDb* db, const char* entity, cJSON** out) {
    (void)db;
    (void)entity;
    *out = cJSON_CreateArray();
    return true;
}

static bool picture(ClarityDb* db, const char* entity, int64_t id, const char* field, ClarityBytes* out) {
    (void)db;
    (void)entity;
    (void)field;
    static const uint8_t png[] = {0x89, 'P', 'N', 'G'};
    *out = id == 1 ? (ClarityBytes){png, sizeof png} : (ClarityBytes){NULL, 0};
    return true;
}

static bool redraw(ClarityDb* db, ClarityReply* reply) {
    (void)db;
    (void)reply;
    return true;
}

static const ClarityWindows cards = {open_card, handler_of, dispatch, options, picture, redraw, NULL, NULL};

static ClarityDesktopOptions card_options(ClarityDb* db) {
    ClarityDesktopOptions options = {{"shop", "Shop", "Card", "{\"project\":{}}", db, &cards, 600, 400}, &card_files, "{\"windows\":{\"Card\":{\"name\":\"text\"}}}", NULL, true, NULL};
    return options;
}

static cJSON* asked(ClarityDesktopProgram* program, const char* command, const char* args) {
    char* text = clarity_desktop_answer(program, command, args, strlen(args));
    cJSON* answer = text ? cJSON_Parse(text) : NULL;
    free(text);
    return answer;
}

static cJSON* step(ClarityDriven* driven, const char* json) {
    char* answer = NULL;
    clarity_driven_step(driven, json, strlen(json), &answer);
    cJSON* out = answer ? cJSON_Parse(answer) : NULL;
    clarity_string_free(answer);
    CHECK(cJSON_IsTrue(cJSON_GetObjectItem(out, "ok")));
    return out;
}

// The library's own commands, answered from the service.
void the_layer_answers_the_librarys_commands(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityDesktopOptions options = card_options(db);
    ClarityDesktopProgram* program = clarity_desktop_program_new(&options);
    REQUIRE(program != NULL);
    CHECK_JSON(asked(program, "started", "{}"), "{\"value\":{\"main\":\"Card\",\"unconnected\":null}}");
    CHECK_JSON(asked(program, "watches", "{}"), "{\"value\":true}");
    // What the splash screen says is loading, as code set it.
    clarity_splash_set_status(clarity_plain_text("Loading the catalogue"));
    CHECK_JSON(asked(program, "splash_status", "{}"), "{\"value\":\"Loading the catalogue\"}");
    CHECK_JSON(asked(program, "facts", "{}"), "{\"value\":{\"windows\":{\"Card\":{\"name\":\"text\"}}}}");
    // A record's stored file is base64, or null when it has none.
    CHECK_JSON(asked(program, "field_bytes", "{\"entity\":\"Card\",\"id\":1,\"field\":\"photo\"}"), "{\"value\":\"iVBORw==\"}");
    CHECK_JSON(asked(program, "field_bytes", "{\"entity\":\"Card\",\"id\":2,\"field\":\"photo\"}"), "{\"value\":null}");
    // Engines beside the screen take the library's default.
    CHECK(asked(program, "engine_open", "{\"name\":\"Board\"}") == NULL);
    CHECK_JSON(asked(program, "nonsense", "{}"), "{\"error\":\"there is no command nonsense\"}");
    CHECK_JSON(asked(program, "dispatch_event", "not json"), "{\"error\":\"the library sent dispatch_event with JSON that does not read\"}");
    clarity_desktop_program_free(program);
    clarity_db_close(db);
}

static ClarityScreenInstances* screens;

static bool make_screen(const char* name, ClarityScreenEngine** out) {
    *out = NULL;
    if (strcmp(name, "Screen") != 0) return true;
    ClarityBytes bytes = {0};
    if (!clarity_read_file(clarity_format("%s/engines/screen.wasm", CLARITY_CONFORMANCE), &bytes)) return false;
    static const char* const uses[] = {"log"};
    return clarity_screen_engine_load("Screen", bytes.data, bytes.size, 1, uses, true, (ClarityEngineLimits){5, 256}, out);
}

static cJSON* screen_answer(const char* command, const cJSON* args) { return clarity_screen_answer(screens, command, args); }

// Engines beside the screen (DESIGN 5.12): the program answers the library's
// engine_* commands from its instances, one per canvas, by number.
void the_layer_runs_engines_beside_the_screen(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    screens = clarity_screen_instances_new(make_screen);
    ClarityDesktopOptions options = card_options(db);
    options.screen_engines = screen_answer;
    ClarityDesktopProgram* program = clarity_desktop_program_new(&options);
    REQUIRE(program != NULL);
    CHECK(asked(program, "engine_open", "{\"name\":\"Board\"}") == NULL);
    CHECK_JSON(asked(program, "engine_open", "{\"name\":\"Screen\"}"), "{\"value\":1}");
    CHECK_JSON(asked(program, "engine_hears", "{\"id\":1}"), "{\"value\":{\"pointer\":true,\"keys\":true}}");
    cJSON* drawn = asked(program, "engine_frame", "{\"id\":1,\"ms\":16,\"input\":[{\"pointer\":{\"kind\":\"down\",\"x\":40,\"y\":20}}]}");
    CHECK_JSON(cJSON_GetObjectItem(drawn, "value"),
               "{\"shapes\":[{\"op\":\"clear\",\"color\":\"background\"},{\"op\":\"circle\",\"x\":40,\"y\":20,\"r\":5,\"color\":\"accent\"},{\"op\":\"text\",\"x\":80,\"y\":90,\"text\":\"16 ms\",\"color\":\"text\",\"size\":8}],\"tells\":[]}");
    CHECK_JSON(asked(program, "engine_close", "{\"id\":1}"), "{\"value\":null}");
    CHECK_JSON(asked(program, "engine_frame", "{\"id\":1,\"ms\":16,\"input\":[]}"), "{\"error\":\"The engine ended.\"}");
    clarity_desktop_program_free(program);
    clarity_screen_instances_free(screens);
    clarity_db_close(db);
}

// A long call (DESIGN 5.12): on the handler's thread between start_watch and
// end_watch; how far it is and Stop come from other threads while it runs.
void a_long_call_says_how_far_it_is_and_stops(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityDesktopOptions options = card_options(db);
    ClarityDesktopProgram* program = clarity_desktop_program_new(&options);
    REQUIRE(program != NULL);
    stopping = program;
    CHECK_JSON(asked(program, "watch_progress", "{\"id\":5}"), "{\"value\":null}");
    CHECK(asked(program, "start_watch", "{\"id\":5}") == NULL);
    cJSON* answer = asked(program, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{\"name\":\"Ada\"}},\"handler\":\"plan\"}");
    CHECK_TEXT(cJSON_GetObjectItem(answer, "error")->valuestring, "engine Planner failed in plan: it was stopped");
    asked(program, "end_watch", "{\"id\":5}");
    CHECK(clarity_current_watch() == NULL);
    // Once more from the screen's thread: nothing left of it.
    asked(program, "end_watch", "{\"id\":5}");
    CHECK_JSON(asked(program, "watch_progress", "{\"id\":5}"), "{\"value\":null}");
    // A stop before its start holds once the call starts.
    asked(program, "stop_watch", "{\"id\":6}");
    asked(program, "start_watch", "{\"id\":6}");
    CHECK(clarity_current_watch() && *clarity_current_watch()->stop);
    asked(program, "end_watch", "{\"id\":6}");
    CHECK(clarity_current_watch() == NULL);
    clarity_desktop_program_free(program);
    clarity_db_close(db);
}

typedef struct {
    ClarityDesktopProgram* program;
    int failures;
} Asking;

#if defined(_WIN32)
static DWORD WINAPI ask_many(void* arg) {
#else
static void* ask_many(void* arg) {
#endif
    Asking* a = arg;
    const char* args = "{\"state\":{\"window\":\"Card\",\"data\":{\"name\":\"Ada\"}},\"handler\":\"rename\"}";
    for (int i = 0; i < 200; ++i) {
        char* text = clarity_desktop_answer(a->program, "dispatch_event", args, strlen(args));
        if (!text || !strstr(text, "\"Grace\"")) a->failures++;
        free(text);
        clarity_scratch_reset();
    }
    return 0;
}

// The library asks from the screen's thread and the handlers' at once: each answer is whole.
void the_layer_answers_several_threads_at_once(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityDesktopOptions options = card_options(db);
    ClarityDesktopProgram* program = clarity_desktop_program_new(&options);
    REQUIRE(program != NULL);
    Asking asking[4];
    for (int i = 0; i < 4; ++i) asking[i] = (Asking){program, 0};
#if defined(_WIN32)
    HANDLE threads[4];
    for (int i = 0; i < 4; ++i) threads[i] = CreateThread(NULL, 0, ask_many, &asking[i], 0, NULL);
    WaitForMultipleObjects(4, threads, TRUE, INFINITE);
    for (int i = 0; i < 4; ++i) CloseHandle(threads[i]);
#else
    pthread_t threads[4];
    for (int i = 0; i < 4; ++i) pthread_create(&threads[i], NULL, ask_many, &asking[i]);
    for (int i = 0; i < 4; ++i) pthread_join(threads[i], NULL);
#endif
    for (int i = 0; i < 4; ++i) CHECK_INT(asking[i].failures, 0);
    clarity_desktop_program_free(program);
    clarity_db_close(db);
}

// The whole program on the library, driven as a person uses it: its window
// from the markup it carries, its data shown, a handler run, an edit taken.
void a_c_program_runs_on_clarity_ui(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityDesktopOptions options = card_options(db);
    ClarityFiles* files = clarity_desktop_files(&options);
    REQUIRE(files != NULL);
    ClarityDesktopProgram* program = clarity_desktop_program_new(&options);
    REQUIRE(program != NULL);
    ClarityDriven* driven = NULL;
    REQUIRE(clarity_driven_start(clarity_desktop_program(program), files, &driven) == CLARITY_OK);
    cJSON* read = step(driven, "{\"do\":\"read\"}");
    CHECK_TEXT(cJSON_GetStringValue(cJSON_GetObjectItem(read, "window")), "Card");
    CHECK_JSON(cJSON_GetObjectItem(read, "data"), "{\"name\":\"Ada\"}");
    step(driven, "{\"do\":\"click\",\"part\":\"renameButton\"}");
    read = step(driven, "{\"do\":\"read\"}");
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(read, "data"), "name"), "\"Grace\"");
    // The person types a name: the window's data takes it.
    step(driven, "{\"do\":\"focus\",\"part\":\"nameBox\"}");
    step(driven, "{\"do\":\"key\",\"key\":\"End\"}");
    step(driven, "{\"do\":\"type\",\"text\":\" Hopper\"}");
    step(driven, "{\"do\":\"key\",\"key\":\"Tab\"}");
    read = step(driven, "{\"do\":\"read\"}");
    CHECK_JSON(cJSON_GetObjectItem(cJSON_GetObjectItem(read, "data"), "name"), "\"Grace Hopper\"");
    clarity_driven_free(driven);
    clarity_desktop_program_free(program);
    clarity_files_free(files);
    clarity_db_close(db);
}

// A custom part's tell may reach the program before the part's callback
// returns, on the same thread: the callback's values in the scratch arena
// stay until it ends.
void a_command_inside_a_parts_callback_keeps_its_values(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityDesktopOptions options = card_options(db);
    ClarityDesktopProgram* program = clarity_desktop_program_new(&options);
    REQUIRE(program != NULL);
    const char* renaming = "{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"rename\"}";
    clarity_desktop_part_begin();
    const char* held = clarity_copy("the part's value");
    size_t used = clarity_scratch_used();
    free(clarity_desktop_answer(program, "dispatch_event", renaming, strlen(renaming)));
    CHECK_TEXT(held, "the part's value");
    CHECK(clarity_scratch_used() >= used);
    clarity_desktop_part_end();
    // Outside a callback a command empties it again.
    free(clarity_desktop_answer(program, "dispatch_event", renaming, strlen(renaming)));
    CHECK(clarity_scratch_used() == 0);
    clarity_desktop_program_free(program);
    clarity_db_close(db);
}

// The built-in quit and keys run by a handler reach the library in its reply,
// once: the next reply says neither.
void code_quits_and_opens_the_keys_window_through_the_layer(void) {
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    ClarityDesktopOptions options = card_options(db);
    ClarityDesktopProgram* program = clarity_desktop_program_new(&options);
    REQUIRE(program != NULL);
    cJSON* quit = asked(program, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"quitIt\"}");
    CHECK(cJSON_IsTrue(cJSON_GetObjectItem(cJSON_GetObjectItem(quit, "value"), "quit")));
    CHECK(!cJSON_GetObjectItem(cJSON_GetObjectItem(quit, "value"), "keys"));
    cJSON* keys = asked(program, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"keysNow\"}");
    CHECK(cJSON_IsTrue(cJSON_GetObjectItem(cJSON_GetObjectItem(keys, "value"), "keys")));
    CHECK(!cJSON_GetObjectItem(cJSON_GetObjectItem(keys, "value"), "quit"));
    cJSON* plain = asked(program, "dispatch_event", "{\"state\":{\"window\":\"Card\",\"data\":{}},\"handler\":\"rename\"}");
    CHECK(!cJSON_GetObjectItem(cJSON_GetObjectItem(plain, "value"), "quit"));
    CHECK(!cJSON_GetObjectItem(cJSON_GetObjectItem(plain, "value"), "keys"));
    clarity_desktop_program_free(program);
    clarity_db_close(db);
}

static const char flip_markup[] = "<Window name=\"Card\"><Button name=\"flipButton\" at=\"1,1\" span=\"3\" caption=\"Flip\" onclick=\"flip\" /></Window>";
static const ClarityCarried flip_windows[] = {{"Card", (const unsigned char*)flip_markup, sizeof flip_markup - 1}};
static const ClarityCarriedFiles flip_files = {flip_windows, 1, NULL, 0, NULL, 0, NULL, 0};

/// What each run of flip saw of showDone: 't' or 'f'.
static char flips_saw[8];

static bool open_with_settings(ClarityDb* db, const char* window, const cJSON* with, ClarityWindowState** out) {
    (void)db;
    *out = clarity_opened(window, clarity_json_object(1, "settings", cJSON_Parse("{\"id\":0,\"showDone\":true,\"theme\":\"\"}")), with);
    return true;
}

static bool flip(ClarityDb* db, const ClarityWindowState* state, const char* handler, ClarityReply** out) {
    (void)db;
    (void)handler;
    cJSON* data = cJSON_Duplicate(state->data, true);
    cJSON* settings = cJSON_GetObjectItemCaseSensitive(data, "settings");
    bool shown = cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(settings, "showDone"));
    size_t at = strlen(flips_saw);
    if (at + 1 < sizeof flips_saw) flips_saw[at] = shown ? 't' : 'f';
    cJSON_ReplaceItemInObjectCaseSensitive(settings, "showDone", cJSON_CreateBool(!shown));
    *out = clarity_ui_reply(clarity_ui(state), state->window, data);
    return true;
}

static const ClarityWindows flipping = {open_with_settings, handler_of, flip, options, picture, redraw, NULL, NULL};

/// One run of the program: its window opens, the person clicks Flip once.
static void run_flip_once(ClarityDb* db) {
    ClarityDesktopOptions options = {{"clarity-desktop-settings-test", "Flip", "Card", "{\"project\":{}}", db, &flipping, 600, 400}, &flip_files, NULL, NULL, false, NULL};
    ClarityFiles* files = clarity_desktop_files(&options);
    REQUIRE(files != NULL);
    ClarityDesktopProgram* program = clarity_desktop_program_new(&options);
    REQUIRE(program != NULL);
    ClarityDriven* driven = NULL;
    REQUIRE(clarity_driven_start(clarity_desktop_program(program), files, &driven) == CLARITY_OK);
    step(driven, "{\"do\":\"click\",\"part\":\"flipButton\"}");
    clarity_driven_free(driven);
    clarity_desktop_program_free(program);
    clarity_files_free(files);
}

// A window starts from the settings the person kept, and settings a handler
// changes are kept: the next run's handler sees them.
void a_window_starts_from_the_kept_settings_and_a_handlers_change_is_kept(void) {
    const char* folder = clarity_format("%s/clarity-desktop-settings-c-%d", temp_folder(), rand());
#ifdef _WIN32
    const char* was = getenv("APPDATA") ? clarity_format("%s", getenv("APPDATA")) : "";
    _putenv_s("APPDATA", folder);
#else
    setenv("XDG_CONFIG_HOME", folder, 1);
#endif
    ClarityDb* db;
    REQUIRE_OK(clarity_db_memory(&db));
    memset(flips_saw, 0, sizeof flips_saw);
    run_flip_once(db);
    CHECK_JSON(cJSON_GetObjectItem(clarity_read_settings("clarity-desktop-settings-test"), "showDone"), "false");
    run_flip_once(db);
    CHECK_TEXT(flips_saw, "tf");
    clarity_db_close(db);
    remove_tree(folder);
#ifdef _WIN32
    _putenv_s("APPDATA", was);
#endif
}
