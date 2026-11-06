// The thin C layer on Clarity.UI (DESIGN 7.9): the library asks by command
// (clarity_ui.h) and the service answers, one command at a time. The
// library calls from the screen's thread and from the thread that runs a
// long handler, so the service takes turns; how far a long call is and its
// Stop are answered beside it, without waiting for the turn.

#include "desktop/desktop.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "clarity/core.h"
#include "clarity/splash.h"
#include "clarity/watch.h"

struct ClarityDesktopProgram {
    const ClarityDesktopOptions* options;
    ClarityService* service;
    ClarityProgram* handle;
    /// One command at a time on the service and its databases.
    ClarityGate turn;
    /// The long calls, apart from the turn: the screen asks while a handler holds it.
    ClarityWatches* watches;
};

#if defined(_MSC_VER) && !defined(__clang__)
#define THREAD_LOCAL __declspec(thread)
#else
#define THREAD_LOCAL _Thread_local
#endif

/// How many custom part callbacks run on this thread now: their values are in its scratch arena.
static THREAD_LOCAL int in_part;

void clarity_desktop_part_begin(void) { ++in_part; }
void clarity_desktop_part_end(void) { --in_part; }

/// Ends a command: its values go, unless a part's callback that is still running holds the arena.
static void command_done(void) {
    if (in_part == 0) clarity_scratch_reset();
}

static char* copied(const char* text) {
    size_t size = strlen(text) + 1;
    char* out = malloc(size);
    if (out) memcpy(out, text, size);
    return out;
}

/// Plain memory for JSON text made in the scratch arena.
static char* printed(const cJSON* json) {
    char* text = cJSON_PrintUnformatted(json);
    return text ? copied(text) : NULL;
}

static uint64_t id_of(const cJSON* args) {
    const cJSON* id = cJSON_GetObjectItemCaseSensitive(args, "id");
    return cJSON_IsNumber(id) ? (uint64_t)id->valuedouble : 0;
}

// -- Commands ----------------------------------------------------------------

static char* value_of(bool yes) { return copied(yes ? "{\"value\":true}" : "{\"value\":false}"); }

/// The service's answer to `command`, as the library reads it (service/wire.h).
/// A window that opens starts from the settings the person kept, as a page's
/// starts from the page's.
static char* served(ClarityDesktopProgram* p, const char* command, cJSON* args) {
    ClarityWire wire = {p->options->facts, p->options->secrets};
    bool opening = strcmp(command, "open_window") == 0;
    if (opening) {
        cJSON* kept = cJSON_GetObjectItemCaseSensitive(clarity_wire_answer(p->service, &wire, "read_settings", cJSON_CreateObject()), "value");
        clarity_service_page(p->service, kept);
    }
    cJSON* answer = clarity_wire_answer(p->service, &wire, command, args);
    if (opening) clarity_service_page(p->service, NULL);
    return answer ? printed(answer) : NULL;
}

char* clarity_desktop_answer(ClarityDesktopProgram* p, const char* command, const char* args, size_t args_len) {
    if (clarity_watches_command(command)) {
        cJSON* given = cJSON_ParseWithLength(args ? args : "{}", args ? args_len : 2);
        uint64_t id = id_of(given);
        command_done();
        return clarity_watches_answer(p->watches, command, id);
    }
    if (strcmp(command, "watches") == 0) return value_of(p->options->watches);
    // Asked from the screen's thread while the app's on start holds the turn.
    if (strcmp(command, "splash_status") == 0) {
        char* said = clarity_splash_status();
        size_t size = (said ? strlen(said) : 4) + 16;
        char* text = malloc(size);
        if (text) snprintf(text, size, "{\"value\":%s}", said ? said : "null");
        free(said);
        command_done();
        return text;
    }
    // Engines beside the screen run in the program, each canvas's by its number,
    // without waiting for the service's turn: each instance takes its own.
    if (strncmp(command, "engine_", 7) == 0) {
        if (!p->options->screen_engines) return NULL;
        cJSON* given = args && args_len ? cJSON_ParseWithLength(args, args_len) : cJSON_CreateObject();
        if (!cJSON_IsObject(given)) given = cJSON_CreateObject();
        cJSON* answered = p->options->screen_engines(command, given);
        char* answer = answered ? printed(answered) : NULL;
        command_done();
        return answer;
    }
    clarity_gate_take(&p->turn);
    cJSON* given = args && args_len ? cJSON_ParseWithLength(args, args_len) : cJSON_CreateObject();
    char* answer;
    if (!cJSON_IsObject(given)) {
        cJSON* out = cJSON_CreateObject();
        cJSON_AddStringToObject(out, "error", clarity_format("the library sent %s with JSON that does not read", command));
        answer = printed(out);
    } else {
        answer = served(p, command, given);
    }
    // The command is answered: its values are done with.
    command_done();
    clarity_gate_give(&p->turn);
    return answer;
}

// -- The program as the library holds it -------------------------------------

static void call(void* user, const char* command, size_t command_len, const char* args, size_t args_len, ClarityAnswer* answer) {
    char name[64];
    if (command_len >= sizeof name) return;
    memcpy(name, command, command_len);
    name[command_len] = '\0';
    char* text = clarity_desktop_answer(user, name, args, args_len);
    if (text) clarity_answer_set(answer, (const uint8_t*)text, strlen(text));
    free(text);
}

static void release(void* user) {
    ClarityDesktopProgram* p = user;
    clarity_service_free(p->service);
    clarity_watches_free(p->watches);
    free(p);
}

ClarityDesktopProgram* clarity_desktop_program_new(const ClarityDesktopOptions* options) {
    ClarityDesktopProgram* p = calloc(1, sizeof *p);
    if (!p) return NULL;
    p->options = options;
    clarity_gate_init(&p->turn);
    p->watches = clarity_watches_new();
    p->service = clarity_service_new(&options->service);
    ClarityProgramCallbacks callbacks = {p, call, release};
    if (!p->service || !p->watches || clarity_program_new(&callbacks, &p->handle) != CLARITY_OK) {
        clarity_service_free(p->service);
        clarity_watches_free(p->watches);
        free(p);
        return NULL;
    }
    return p;
}

const ClarityProgram* clarity_desktop_program(const ClarityDesktopProgram* program) { return program->handle; }

void clarity_desktop_program_free(ClarityDesktopProgram* program) {
    if (program) clarity_program_free(program->handle);
}

// -- Files -------------------------------------------------------------------

typedef ClarityStatus (*AddText)(ClarityFiles*, const char*, size_t, const char*, size_t);

static bool added(ClarityFiles* files, AddText add, const ClarityCarried* each, size_t count) {
    for (size_t i = 0; i < count; ++i)
        if (add(files, each[i].name, strlen(each[i].name), (const char*)each[i].data, each[i].size) != CLARITY_OK) return false;
    return true;
}

ClarityFiles* clarity_desktop_files(const ClarityDesktopOptions* options) {
    ClarityFiles* files = NULL;
    if (clarity_files_new(&files) != CLARITY_OK) return NULL;
    const ClarityCarriedFiles* carried = options->files;
    const ClarityServiceOptions* service = &options->service;
    bool ok = !carried || (added(files, clarity_files_window, carried->windows, carried->window_count) && added(files, clarity_files_app, carried->apps, carried->app_count) &&
                           added(files, clarity_files_theme, carried->themes, carried->theme_count));
    for (size_t i = 0; ok && carried && i < carried->resource_count; ++i) {
        const ClarityCarried* r = &carried->resources[i];
        ok = clarity_files_resource(files, r->name, strlen(r->name), r->data, r->size) == CLARITY_OK;
    }
    ok = ok && (!service->model || clarity_files_model(files, service->model, strlen(service->model)) == CLARITY_OK);
    ok = ok && (!service->title || clarity_files_title(files, service->title, strlen(service->title)) == CLARITY_OK);
    ok = ok && clarity_files_size(files, service->width > 0 ? service->width : 0, service->height > 0 ? service->height : 0) == CLARITY_OK;
    if (ok) return files;
    clarity_files_free(files);
    return NULL;
}

/// Says why the library refused, on the standard error.
static int refused(const char* what) {
    char* why = clarity_last_error();
    fprintf(stderr, "%s: %s\n", what, why ? why : "the library gave no reason");
    clarity_string_free(why);
    return 1;
}

int clarity_desktop_start(const ClarityDesktopOptions* options) {
    if (clarity_ui_interface() != CLARITY_UI_INTERFACE) {
        fprintf(stderr, "Clarity.UI's library has interface %u, this program was built for %d\n", clarity_ui_interface(), CLARITY_UI_INTERFACE);
        return 1;
    }
    if (options->register_parts) options->register_parts();
    ClarityFiles* files = clarity_desktop_files(options);
    if (!files) return refused("the program's files");
    ClarityDesktopProgram* program = clarity_desktop_program_new(options);
    if (!program) {
        clarity_files_free(files);
        return refused("the program");
    }
    int32_t code = 1;
    ClarityStatus status = clarity_desktop_run(program->handle, files, &code);
    clarity_desktop_program_free(program);
    clarity_files_free(files);
    if (status != CLARITY_OK) return refused("the desktop");
    return code;
}
