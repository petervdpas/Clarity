// The service of the shell (DESIGN 7.3 and 5.x): it answers the renderer's
// commands, for the desktop window (shell.c) and for a browser (serve.c).
// It needs nothing of the desktop toolkit: what belongs to a desktop window
// it asks of the hooks clarity_shell_run sets, and a web server sets none.

#include "shell/shell.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "clarity/db_internal.h"
#include "clarity/record_file.h"
#include "clarity/settings.h"
#include "clarity/store.h"

#if defined(_WIN32)
#include <windows.h>
#include <shellapi.h>
#else
#include <spawn.h>
extern char** environ;
#endif

typedef struct {
    char* name;
    char* window;
    /// The `with` of the window, as JSON text, since it outlives the command.
    char* with;
    char* opener;
    bool closed;
} Apart;

/// Which desktop window an open window's custom parts are in.
typedef struct {
    uint64_t instance;
    char window[32];
} PartsWindow;

struct ClarityService {
    ClarityShellOptions options;
    /// Set by stop_call, from any thread, to stop the handler that runs.
    volatile int32_t stop;
    PartsWindow* parts_windows;
    size_t parts_window_count;
    Apart* apart;
    size_t apart_count;
    int next;
    void (*quit)(void* context);
    bool (*open_link)(const char* address);
    void (*log_line)(const char* line);
    void (*open_desktop)(void* context, ClarityDesktop* opener, const char* name, int width, int height);
    void* context;
    /// The part host's side of the parts commands; none without a desktop.
    ClarityPartsHost parts;
    /// The title bar's close of the main window goes to the screen.
    bool to_screen;
};

bool clarity_service_closes_to_screen(const ClarityService* service) { return service->to_screen; }

ClarityService* clarity_service_new(const ClarityShellOptions* options) {
    clarity_start();
    ClarityService* service = calloc(1, sizeof *service);
    if (service) service->options = *options;
    // Text code makes is translated from the model's translations.
    clarity_load_translations(options->model);
    return service;
}

void clarity_service_free(ClarityService* service) {
    if (!service) return;
    for (size_t i = 0; i < service->apart_count; ++i) {
        free(service->apart[i].name);
        free(service->apart[i].window);
        free(service->apart[i].with);
        free(service->apart[i].opener);
    }
    free(service->apart);
    free(service->parts_windows);
    free(service);
}

void clarity_window_size(int* width, int* height) {
    if (*width <= 0 || *height <= 0) {
        *width = 1100;
        *height = 760;
    }
}

bool clarity_open_in_browser(const char* address) {
#if defined(_WIN32)
    return (INT_PTR)ShellExecuteA(NULL, "open", address, NULL, NULL, SW_SHOWNORMAL) > 32;
#else
#if defined(__APPLE__)
    char* argv[] = {"open", (char*)address, NULL};
#else
    char* argv[] = {"xdg-open", (char*)address, NULL};
#endif
    pid_t pid;
    return posix_spawnp(&pid, argv[0], NULL, NULL, argv, environ) == 0;
#endif
}

void clarity_service_on_link(ClarityService* service, bool (*open_link)(const char* address)) { service->open_link = open_link; }

/// Whether a link of a markdown part may open: a web or a mail address, as on every stack.
static bool link_opens(const char* address) {
    return strncmp(address, "http://", 7) == 0 || strncmp(address, "https://", 8) == 0 || strncmp(address, "mailto:", 7) == 0;
}

void clarity_service_on_log(ClarityService* service, void (*log_line)(const char* line)) { service->log_line = log_line; }

/// A line into the application's log: the standard error, where the backend's engines write.
static void log_to_stderr(const char* line) { fprintf(stderr, "%s\n", line); }

void clarity_service_on_parts(ClarityService* service, ClarityPartsHost parts) { service->parts = parts; }

void clarity_service_on_desktop(ClarityService* service, void (*quit)(void*), void (*open_desktop)(void*, ClarityDesktop*, const char*, int, int), void* context) {
    service->quit = quit;
    service->open_desktop = open_desktop;
    service->context = context;
}

static cJSON* value(cJSON* v) {
    cJSON* out = cJSON_CreateObject();
    cJSON_AddItemToObject(out, "value", v ? v : cJSON_CreateNull());
    return out;
}

static cJSON* error_text(const char* message) {
    cJSON* out = cJSON_CreateObject();
    cJSON_AddStringToObject(out, "error", message);
    return out;
}

/// The last error as the screen shows it: a refusal keeps its slots, so
/// the screen fills them in the person's format; other errors are text.
static cJSON* failed(void) {
    const ClarityError* e = clarity_error();
    if (e->kind == CLARITY_REFUSED && e->text) {
        cJSON* out = cJSON_CreateObject();
        cJSON_AddItemToObject(out, "error", clarity_text_json(e->text));
        return out;
    }
    return error_text(e->message);
}

static const char* text_of(const cJSON* args, const char* key) {
    const cJSON* v = cJSON_GetObjectItemCaseSensitive(args, key);
    return cJSON_IsString(v) ? v->valuestring : "";
}

/// A window's own event: its handler runs when it has one.
/// Runs a handler, then draws the window's canvases anew.
static bool dispatched(ClarityService* s, const ClarityWindowState* state, const char* handler, ClarityReply** out) {
    return s->options.windows->dispatch(s->options.db, state, handler, out) && s->options.windows->redraw(s->options.db, *out);
}

static bool on_event(ClarityService* s, const ClarityWindowState* state, const char* event, ClarityReply** out) {
    const char* handler = s->options.windows->handler(state->window, event);
    if (handler) return dispatched(s, state, handler, out);
    *out = clarity_ui_reply(clarity_ui(state), state->window, state->data);
    return s->options.windows->redraw(s->options.db, *out);
}

static cJSON* replied(ClarityService* s, const ClarityWindowState* state, const char* event) {
    ClarityReply* reply;
    return on_event(s, state, event, &reply) ? value(clarity_reply_json(reply)) : failed();
}

static cJSON* answer_command(ClarityService* s, const char* command, const cJSON* args, ClarityDesktop* caller);

void clarity_service_stop(ClarityService* service) { service->stop = 1; }

bool clarity_service_watches(const char* command) {
    static const char* const watched[] = {"start_app", "open_window", "close_window", "return_to", "dispatch_event", "answer_event"};
    for (size_t i = 0; i < sizeof watched / sizeof *watched; ++i)
        if (strcmp(command, watched[i]) == 0) return true;
    return false;
}

static int64_t now_ms(void) {
    struct timespec now;
    timespec_get(&now, TIME_UTC);
    return (int64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000;
}

/// What a watched handler's engine calls report, on its way to the screen.
typedef struct {
    ClarityDesktop* caller;
    int64_t last;
    bool told;
} Told;

static void tell_screen(int64_t done, int64_t total, void* context) {
    Told* t = context;
    int64_t now = now_ms();
    if (t->told && now - t->last < 100 && done < total) return;
    t->last = now;
    t->told = true;
    char js[160];
    snprintf(js, sizeof js, "window.dispatchEvent(new CustomEvent(\"clarity-progress\", {detail: {\"done\":%lld,\"total\":%lld}}))", (long long)done, (long long)total);
    t->caller->show(t->caller, js);
}

cJSON* clarity_service_invoke(ClarityService* s, const char* command, const cJSON* args, ClarityDesktop* caller) {
    // The person stops the handler that runs, from the screen's thread.
    if (strcmp(command, "stop_call") == 0) {
        clarity_service_stop(s);
        return value(NULL);
    }
    if (!clarity_service_watches(command) || !caller || !caller->show) return answer_command(s, command, args, caller);
    // The handler's engine calls say how far they are: to the screen at most
    // ten times a second, and that it ended once it did.
    s->stop = 0;
    Told told = {caller, 0, false};
    ClarityEngineWatch watch = {tell_screen, &told, &s->stop};
    const ClarityEngineWatch* was = clarity_watching(&watch);
    cJSON* answer = answer_command(s, command, args, caller);
    clarity_watching(was);
    if (told.told) caller->show(caller, "window.dispatchEvent(new CustomEvent(\"clarity-progress\", {detail: null}))");
    return answer;
}

static cJSON* answer_command(ClarityService* s, const char* command, const cJSON* args, ClarityDesktop* caller) {
    ClarityDb* db = s->options.db;
    const ClarityWindows* windows = s->options.windows;
    ClarityWindowState state = {.window = "", .data = cJSON_CreateObject()};
    const cJSON* given = cJSON_GetObjectItemCaseSensitive(args, "state");
    if (given && !clarity_window_state_from_json(given, &state)) return failed();
    const cJSON* with = cJSON_GetObjectItemCaseSensitive(args, "with");
    cJSON* empty = cJSON_CreateObject();
    if (!cJSON_IsObject(with)) with = empty;

    if (strcmp(command, "start") == 0) {
        cJSON* started = cJSON_CreateObject();
        cJSON_AddItemToObject(started, "model", cJSON_Parse(s->options.model));
        cJSON_AddStringToObject(started, "main", s->options.main);
        if (s->options.unconnected) {
            cJSON* why = cJSON_AddObjectToObject(started, "unconnected");
            cJSON_AddItemToObject(why, "handler", s->options.unconnected_handler ? cJSON_CreateString(s->options.unconnected_handler) : cJSON_CreateNull());
            cJSON_AddItemToObject(why, "window", s->options.unconnected_window ? cJSON_CreateString(s->options.unconnected_window) : cJSON_CreateNull());
            cJSON_AddItemToObject(why, "problem", clarity_text_json(clarity_unconnected_reason(s->options.unconnected)));
            s->to_screen = true;
        }
        return value(started);
    }
    if (strcmp(command, "connect") == 0) {
        // The databases again, as when the person has kept their connection.
        if (!s->options.connect) return value(cJSON_CreateNull());
        ClarityDb* opened = NULL;
        if (!s->options.connect(&opened)) {
            cJSON* out = cJSON_CreateObject();
            cJSON_AddItemToObject(out, "error", clarity_text_json(clarity_unconnected_reason(clarity_error())));
            return out;
        }
        clarity_db_close(s->options.db);
        s->options.db = opened;
        s->options.unconnected = NULL;
        s->to_screen = false;
        return value(cJSON_CreateNull());
    }
    if (strcmp(command, "read_settings") == 0) {
        // The settings the person kept (DESIGN 5.17); the screen takes back those that still fit.
        return value(clarity_read_settings_with(s->options.name, cJSON_GetObjectItemCaseSensitive(args, "secrets")));
    }
    if (strcmp(command, "keep_settings") == 0) {
        // The person's settings, as the screen has them now.
        const cJSON* settings = cJSON_GetObjectItemCaseSensitive(args, "settings");
        if (!cJSON_IsObject(settings)) return error_text("keep_settings takes the settings as an object");
        // Its secret settings go to the system's protected store, never the file (DESIGN 5.27).
        if (!clarity_keep_settings_with(s->options.name, settings, cJSON_GetObjectItemCaseSensitive(args, "secrets"))) return failed();
        return value(cJSON_CreateNull());
    }
    if (strcmp(command, "open_window") == 0) {
        ClarityWindowState* opened;
        if (!windows->open(db, text_of(args, "window"), with, &opened)) return failed();
        return replied(s, opened, "open");
    }
    if (strcmp(command, "close_window") == 0) {
        ClarityReply* reply;
        if (!on_event(s, &state, "close", &reply)) return failed();
        // The window has closed: what it had of an engine goes.
        clarity_window_closed(state.instance);
        return value(clarity_reply_json(reply));
    }
    if (strcmp(command, "return_to") == 0) return replied(s, &state, "return");
    if (strcmp(command, "start_app") == 0) return replied(s, &state, "start");
    if (strcmp(command, "dispatch_event") == 0) {
        ClarityReply* reply;
        if (!dispatched(s, &state, text_of(args, "handler"), &reply)) return failed();
        return value(clarity_reply_json(reply));
    }
    if (strcmp(command, "answer_event") == 0) {
        // The answer to a question a handler asked runs the handler it goes to.
        const cJSON* values = cJSON_GetObjectItemCaseSensitive(args, "values");
        ClarityReply* reply;
        if (!windows->answer) return error_text("this program asks nothing");
        if (!windows->answer(db, &state, text_of(args, "handler"), cJSON_IsArray(values) ? values : cJSON_CreateArray(), &reply)) return failed();
        if (!windows->redraw(db, reply)) return failed();
        return value(clarity_reply_json(reply));
    }
    if (strcmp(command, "ref_options") == 0) {
        cJSON* options;
        if (!windows->options(db, text_of(args, "entity"), &options)) return failed();
        return value(options);
    }
    if (strcmp(command, "field_values") == 0) {
        cJSON* values;
        if (!windows->values) return value(cJSON_CreateArray());
        if (!windows->values(db, text_of(args, "entity"), text_of(args, "field"), &values)) return failed();
        return value(values);
    }
    if (strcmp(command, "picture") == 0) {
        // A picture of a record, fetched on its own, as the page shows it.
        const cJSON* id = cJSON_GetObjectItemCaseSensitive(args, "id");
        ClarityBytes bytes = {0};
        if (!windows->picture(db, text_of(args, "entity"), cJSON_IsNumber(id) ? (int64_t)id->valuedouble : 0, text_of(args, "field"), &bytes)) return failed();
        if (!bytes.data) return value(NULL);
        const char* url;
        return clarity_data_url(bytes, &url) ? value(cJSON_CreateString(url)) : failed();
    }
    if (strcmp(command, "file_data") == 0) {
        // The file of a file field of a record, fetched on its own, as base64.
        const cJSON* id = cJSON_GetObjectItemCaseSensitive(args, "id");
        ClarityBytes bytes = {0};
        if (!windows->picture(db, text_of(args, "entity"), cJSON_IsNumber(id) ? (int64_t)id->valuedouble : 0, text_of(args, "field"), &bytes)) return failed();
        return value(bytes.data ? cJSON_CreateString(clarity_base64(bytes)) : NULL);
    }
    if (strcmp(command, "open_file") == 0) {
        // Opens a file in the program the system uses for its kind, from a temporary copy.
        const char* file = NULL;
        if (!clarity_temporary_copy(text_of(args, "name"), clarity_from_base64(text_of(args, "data")), &file)) return failed();
        bool (*open)(const char*) = s->open_link ? s->open_link : clarity_open_in_browser;
        return open(file) ? value(NULL) : error_text(clarity_format("cannot open %s", file));
    }
    if (strcmp(command, "save_file") == 0) {
        // Saves a copy of a file in the person's Downloads folder; where it went.
        const char* file = NULL;
        if (!clarity_save_copy(text_of(args, "name"), clarity_from_base64(text_of(args, "data")), &file)) return failed();
        return value(cJSON_CreateString(file));
    }
    if (strcmp(command, "store_file") == 0) {
        // A picture linked from a file of a store, before its record is saved.
        ClarityBytes bytes = {0};
        const char* url;
        if (!clarity_store_file(db, text_of(args, "store"), text_of(args, "path"), &bytes) || !clarity_data_url(bytes, &url)) return failed();
        return value(cJSON_CreateString(url));
    }
    if (strcmp(command, "set_language") == 0) {
        // The language the screen picked, which text code makes is translated to.
        const cJSON* language = cJSON_GetObjectItemCaseSensitive(args, "language");
        clarity_set_language(cJSON_IsString(language) ? language->valuestring : NULL);
        return value(NULL);
    }
    if (strcmp(command, "choose_folder") == 0) {
        // A folder part's button: the system's folder picker over the window it is in.
        if (!caller || !caller->choose_folder) return error_text("this window has no folder picker");
        const char* folder = caller->choose_folder(caller, text_of(args, "title"));
        return value(folder ? cJSON_CreateString(folder) : NULL);
    }
    if (strcmp(command, "set_title") == 0) {
        if (caller) caller->set_title(caller, text_of(args, "title"));
        return value(NULL);
    }
    if (strcmp(command, "open_link") == 0) {
        const char* address = text_of(args, "address");
        if (!link_opens(address)) return error_text(clarity_format("a link opens only an http, https or mailto address, not %s", address));
        bool (*open_link)(const char*) = s->open_link ? s->open_link : clarity_open_in_browser;
        return open_link(address) ? value(NULL) : error_text(clarity_format("cannot open %s", address));
    }
    if (strcmp(command, "log_line") == 0) {
        (s->log_line ? s->log_line : log_to_stderr)(text_of(args, "line"));
        return value(NULL);
    }
    if (strcmp(command, "quit_app") == 0) {
        s->to_screen = false;
        if (s->quit) s->quit(s->context);
        return value(NULL);
    }
    if (strcmp(command, "open_apart") == 0) {
        if (!caller || !s->open_desktop) return error_text("a window of its own needs the desktop");
        Apart* more = realloc(s->apart, (s->apart_count + 1) * sizeof *more);
        if (!more) return error_text("out of memory");
        s->apart = more;
        Apart* a = &s->apart[s->apart_count++];
        *a = (Apart){clarity_lasting(clarity_format("apart-%d", ++s->next)), clarity_lasting(text_of(args, "window")),
                     clarity_lasting(cJSON_PrintUnformatted(with)), clarity_lasting(caller->name), false};
        const cJSON* w = cJSON_GetObjectItemCaseSensitive(args, "width");
        const cJSON* h = cJSON_GetObjectItemCaseSensitive(args, "height");
        int width = cJSON_IsNumber(w) ? (int)w->valuedouble : 0, height = cJSON_IsNumber(h) ? (int)h->valuedouble : 0;
        clarity_window_size(&width, &height);
        s->open_desktop(s->context, caller, a->name, width, height);
        return value(NULL);
    }
    if (strcmp(command, "apart_start") == 0) {
        for (size_t i = 0; caller && i < s->apart_count; ++i) {
            const Apart* a = &s->apart[i];
            if (a->closed || strcmp(a->name, caller->name) != 0) continue;
            cJSON* start = cJSON_CreateObject();
            cJSON_AddStringToObject(start, "window", a->window);
            cJSON_AddItemToObject(start, "with", cJSON_Parse(a->with));
            return value(start);
        }
        return value(NULL);
    }
    // Custom parts (DESIGN 6.5): where each is and what it is given, for the part host.
    if (strcmp(command, "place_part") == 0) {
        const cJSON* instance = cJSON_GetObjectItemCaseSensitive(args, "instance");
        uint64_t window = cJSON_IsNumber(instance) ? (uint64_t)instance->valuedouble : 0;
        if (caller) {
            size_t i = 0;
            while (i < s->parts_window_count && s->parts_windows[i].instance != window) ++i;
            if (i == s->parts_window_count) {
                PartsWindow* grown = realloc(s->parts_windows, (s->parts_window_count + 1) * sizeof *grown);
                if (grown) {
                    s->parts_windows = grown;
                    s->parts_windows[s->parts_window_count++].instance = window;
                }
            }
            if (i < s->parts_window_count) snprintf(s->parts_windows[i].window, sizeof s->parts_windows[i].window, "%s", caller->name);
        }
        const cJSON* n;
        int x = cJSON_IsNumber(n = cJSON_GetObjectItemCaseSensitive(args, "x")) ? n->valueint : 0;
        int y = cJSON_IsNumber(n = cJSON_GetObjectItemCaseSensitive(args, "y")) ? n->valueint : 0;
        int width = cJSON_IsNumber(n = cJSON_GetObjectItemCaseSensitive(args, "width")) ? n->valueint : 0;
        int height = cJSON_IsNumber(n = cJSON_GetObjectItemCaseSensitive(args, "height")) ? n->valueint : 0;
        uintptr_t parent = caller && caller->part_parent ? caller->part_parent(caller) : 0;
        if (s->parts.place) s->parts.place(window, text_of(args, "part"), text_of(args, "kind"), parent, x, y, width, height, cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(args, "shown")));
        return value(NULL);
    }
    if (strcmp(command, "set_part") == 0 || strcmp(command, "hear_part") == 0 || strcmp(command, "close_parts") == 0) {
        const cJSON* instance = cJSON_GetObjectItemCaseSensitive(args, "instance");
        uint64_t window = cJSON_IsNumber(instance) ? (uint64_t)instance->valuedouble : 0;
        if (command[0] == 's') {
            const cJSON* given = cJSON_GetObjectItemCaseSensitive(args, "value");
            if (s->parts.set) s->parts.set(window, text_of(args, "part"), text_of(args, "name"), given ? cJSON_PrintUnformatted(given) : "null");
        } else if (command[0] == 'h') {
            const cJSON* given = cJSON_GetObjectItemCaseSensitive(args, "values");
            if (s->parts.hear) s->parts.hear(window, text_of(args, "part"), text_of(args, "hear"), given ? cJSON_PrintUnformatted(given) : "[]");
        } else {
            if (s->parts.close) s->parts.close(window);
        }
        return value(NULL);
    }
    if (strcmp(command, "close_apart") == 0) {
        if (caller) caller->close(caller);
        return value(NULL);
    }
    return error_text(clarity_format("there is no command %s", command));
}

const char* clarity_service_parts_window(ClarityService* s, uint64_t instance) {
    for (size_t i = 0; i < s->parts_window_count; ++i) {
        if (s->parts_windows[i].instance == instance) return s->parts_windows[i].window;
    }
    return NULL;
}

const char* clarity_service_closed(ClarityService* s, const char* name) {
    for (size_t i = 0; i < s->apart_count; ++i) {
        Apart* a = &s->apart[i];
        if (a->closed || strcmp(a->name, name) != 0) continue;
        a->closed = true;
        return a->opener;
    }
    return NULL;
}
