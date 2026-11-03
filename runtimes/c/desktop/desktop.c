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
#include "clarity/watch.h"

#if defined(_WIN32)
#include <windows.h>
#else
#include <pthread.h>
#endif

#if defined(_MSC_VER) && !defined(__clang__)
#define THREAD_LOCAL __declspec(thread)
#else
#define THREAD_LOCAL _Thread_local
#endif

#if defined(_WIN32)
typedef SRWLOCK Gate;
static void gate_init(Gate* gate) { InitializeSRWLock(gate); }
static void gate_take(Gate* gate) { AcquireSRWLockExclusive(gate); }
static void gate_give(Gate* gate) { ReleaseSRWLockExclusive(gate); }
#else
typedef pthread_mutex_t Gate;
static void gate_init(Gate* gate) { pthread_mutex_init(gate, NULL); }
static void gate_take(Gate* gate) { pthread_mutex_lock(gate); }
static void gate_give(Gate* gate) { pthread_mutex_unlock(gate); }
#endif

/// A long call the library watches, by the number it gave.
typedef struct Watch {
    struct ClarityDesktopProgram* program;
    uint64_t id;
    ClarityEngineWatch watch;
    volatile int32_t stop;
    int64_t done, total;
    bool told, started;
    const ClarityEngineWatch* was;
    struct Watch* next;
} Watch;

struct ClarityDesktopProgram {
    const ClarityDesktopOptions* options;
    ClarityService* service;
    ClarityProgram* handle;
    /// One command at a time on the service and its databases.
    Gate turn;
    /// The watches, apart from the turn: the screen asks while a handler holds it.
    Gate watching;
    Watch* watches;
};

/// The watch whose handler runs on this thread.
static THREAD_LOCAL Watch* mine;

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

// -- Long calls --------------------------------------------------------------

static Watch* found(ClarityDesktopProgram* p, uint64_t id) {
    for (Watch* w = p->watches; w; w = w->next)
        if (w->id == id) return w;
    return NULL;
}

static Watch* found_or_new(ClarityDesktopProgram* p, uint64_t id) {
    Watch* w = found(p, id);
    if (w) return w;
    w = calloc(1, sizeof *w);
    if (!w) return NULL;
    w->program = p;
    w->id = id;
    w->next = p->watches;
    p->watches = w;
    return w;
}

static void dropped(ClarityDesktopProgram* p, Watch* gone) {
    for (Watch** at = &p->watches; *at; at = &(*at)->next) {
        if (*at == gone) {
            *at = gone->next;
            free(gone);
            return;
        }
    }
}

/// An engine call says how far it is, on the handler's thread.
static void progressed(int64_t done, int64_t total, void* context) {
    Watch* w = context;
    gate_take(&w->program->watching);
    w->done = done;
    w->total = total;
    w->told = true;
    gate_give(&w->program->watching);
}

static uint64_t id_of(const cJSON* args) {
    const cJSON* id = cJSON_GetObjectItemCaseSensitive(args, "id");
    return cJSON_IsNumber(id) ? (uint64_t)id->valuedouble : 0;
}

/// start_watch, watch_progress, stop_watch and end_watch (clarity_ui.h, "Long calls").
static char* watched(ClarityDesktopProgram* p, const char* command, uint64_t id) {
    char* answer = NULL;
    gate_take(&p->watching);
    if (strcmp(command, "start_watch") == 0) {
        // On the handler's thread, just before it runs: its engine calls report here.
        Watch* w = found_or_new(p, id);
        if (w) {
            w->started = true;
            w->watch.progress = progressed;
            w->watch.context = w;
            w->watch.stop = &w->stop;
            w->was = clarity_watching(&w->watch);
            mine = w;
        }
    } else if (strcmp(command, "watch_progress") == 0) {
        Watch* w = found(p, id);
        char text[96];
        if (w && w->told) snprintf(text, sizeof text, "{\"value\":[%lld,%lld]}", (long long)w->done, (long long)w->total);
        else snprintf(text, sizeof text, "{\"value\":null}");
        answer = copied(text);
    } else if (strcmp(command, "stop_watch") == 0) {
        // A stop may come before its start: it holds once the call starts.
        Watch* w = found_or_new(p, id);
        if (w) w->stop = 1;
    } else if (strcmp(command, "end_watch") == 0) {
        Watch* w = found(p, id);
        if (w && mine == w) {
            clarity_watching(w->was);
            mine = NULL;
            dropped(p, w);
        } else if (w && !w->started) {
            // Stopped and never started.
            dropped(p, w);
        }
    }
    gate_give(&p->watching);
    return answer;
}

// -- Commands ----------------------------------------------------------------

static char* value_of(bool yes) { return copied(yes ? "{\"value\":true}" : "{\"value\":false}"); }

/// `args` with the names of the secret settings, which never go to the settings file.
static void with_secrets(ClarityDesktopProgram* p, cJSON* args) {
    if (!p->options->secrets) return;
    cJSON_DeleteItemFromObjectCaseSensitive(args, "secrets");
    cJSON_AddItemToObject(args, "secrets", cJSON_Parse(p->options->secrets));
}

/// The service's answer to `command`, as the library reads it.
static char* served(ClarityDesktopProgram* p, const char* command, cJSON* args) {
    // started: the main window and why the databases did not open, from the service's start.
    if (strcmp(command, "started") == 0) {
        cJSON* start = clarity_service_invoke(p->service, "start", args);
        const cJSON* value = cJSON_GetObjectItemCaseSensitive(start, "value");
        if (!cJSON_IsObject(value)) return printed(start);
        cJSON* out = cJSON_CreateObject();
        cJSON* started = cJSON_AddObjectToObject(out, "value");
        const cJSON* main_window = cJSON_GetObjectItemCaseSensitive(value, "main");
        cJSON_AddItemToObject(started, "main", cJSON_Duplicate(main_window, true));
        const cJSON* unconnected = cJSON_GetObjectItemCaseSensitive(value, "unconnected");
        cJSON_AddItemToObject(started, "unconnected", unconnected ? cJSON_Duplicate(unconnected, true) : cJSON_CreateNull());
        return printed(out);
    }
    // A stored file of a record, as base64.
    if (strcmp(command, "field_bytes") == 0) return printed(clarity_service_invoke(p->service, "file_data", args));
    // A file of a store: base64, where the web screen takes a data URL.
    if (strcmp(command, "store_file") == 0) {
        cJSON* answer = clarity_service_invoke(p->service, "store_file", args);
        cJSON* value = cJSON_GetObjectItemCaseSensitive(answer, "value");
        const char* comma = cJSON_IsString(value) ? strchr(value->valuestring, ',') : NULL;
        if (comma && strncmp(value->valuestring, "data:", 5) == 0) cJSON_SetValuestring(value, comma + 1);
        return printed(answer);
    }
    if (strcmp(command, "read_settings") == 0 || strcmp(command, "keep_settings") == 0) with_secrets(p, args);
    return printed(clarity_service_invoke(p->service, command, args));
}

char* clarity_desktop_answer(ClarityDesktopProgram* p, const char* command, const char* args, size_t args_len) {
    if (strcmp(command, "start_watch") == 0 || strcmp(command, "watch_progress") == 0 || strcmp(command, "stop_watch") == 0 || strcmp(command, "end_watch") == 0) {
        cJSON* given = cJSON_ParseWithLength(args ? args : "{}", args ? args_len : 2);
        uint64_t id = id_of(given);
        command_done();
        return watched(p, command, id);
    }
    if (strcmp(command, "watches") == 0) return value_of(p->options->watches);
    if (strcmp(command, "facts") == 0) {
        if (!p->options->facts) return NULL;
        size_t size = strlen(p->options->facts) + 16;
        char* out = malloc(size);
        if (out) snprintf(out, size, "{\"value\":%s}", p->options->facts);
        return out;
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
    gate_take(&p->turn);
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
    gate_give(&p->turn);
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
    while (p->watches) dropped(p, p->watches);
    free(p);
}

ClarityDesktopProgram* clarity_desktop_program_new(const ClarityDesktopOptions* options) {
    ClarityDesktopProgram* p = calloc(1, sizeof *p);
    if (!p) return NULL;
    p->options = options;
    gate_init(&p->turn);
    gate_init(&p->watching);
    p->service = clarity_service_new(&options->service);
    ClarityProgramCallbacks callbacks = {p, call, release};
    if (!p->service || clarity_program_new(&callbacks, &p->handle) != CLARITY_OK) {
        clarity_service_free(p->service);
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
