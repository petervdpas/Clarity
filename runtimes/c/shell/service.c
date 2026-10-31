// The service of the shell (DESIGN 7.3 and 5.x): it answers the renderer's
// commands, for the desktop window (shell.c) and for a browser (serve.c).
// It needs nothing of the desktop toolkit: what belongs to a desktop window
// it asks of the hooks clarity_shell_run sets, and a web server sets none.

#include "shell/shell.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "clarity/canvas.h"
#include "clarity/db_internal.h"
#include "clarity/dropdown_options.h"
#include "clarity/edits.h"
#include "clarity/list_fields.h"
#include "clarity/ranges.h"
#include "clarity/record_file.h"
#include "clarity/reorder.h"
#include "clarity/settings.h"
#include "clarity/steps.h"
#include "clarity/store.h"
#include "clarity/views.h"

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
    /// The selected rows the screen sent with the command that runs, before
    /// a handler changed them: a row chosen since shows (DESIGN 1.7).
    ClaritySelected* sent;
    size_t sent_count;
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

/// Draws the window as its data is, running no handler.
static bool drawn(ClarityService* s, const ClarityWindowState* state, ClarityReply** out) {
    *out = clarity_ui_reply(clarity_ui(state), state->window, state->data);
    return s->options.windows->redraw(s->options.db, *out);
}

static bool on_event(ClarityService* s, const ClarityWindowState* state, const char* event, ClarityReply** out) {
    const char* handler = s->options.windows->handler(state->window, event);
    if (handler) return dispatched(s, state, handler, out);
    return drawn(s, state, out);
}

/// A handler run as one undo step.
typedef struct {
    ClarityService* service;
    const ClarityWindowState* state;
    const char* handler;
    ClarityReply* reply;
} Stepping;

static bool dispatched_step(void* context) {
    Stepping* step = context;
    return dispatched(step->service, step->state, step->handler, &step->reply);
}

/// A window's reply, which is where its edits start (DESIGN 5.29): what a
/// handler changed is never the person's edit.
static cJSON* answered(ClarityService* s, ClarityReply* reply) {
    // What the window's tables, trees and charts show, from its data now (DESIGN 1.7).
    clarity_fill_views(&reply->state, s->sent, s->sent_count);
    clarity_edits_reset(clarity_edits_of(reply->state.instance), clarity_edit_data(&reply->state));
    return value(clarity_reply_json(reply));
}

static cJSON* replied(ClarityService* s, const ClarityWindowState* state, const char* event) {
    ClarityReply* reply;
    return on_event(s, state, event, &reply) ? answered(s, reply) : failed();
}

/// Runs a handler, if any, as one database step: the reply, with the step it
/// made in `step` (0 when it changed nothing). No handler: the window as it is.
static bool stepped(ClarityService* s, const ClarityWindowState* state, const char* handler, ClarityReply** out) {
    if (!*handler) return drawn(s, state, out);
    ClarityDb* db = s->options.db;
    int64_t before = clarity_last_step(db);
    Stepping step = {s, state, handler, NULL};
    if (!clarity_step(db, dispatched_step, &step)) return false;
    if (clarity_last_step(db) != before) step.reply->step = clarity_last_step(db);
    *out = step.reply;
    return true;
}

/// Takes an edit's database steps back or does them again, in order; the
/// first the database refuses stops it.
static bool take_steps(const int64_t* steps, size_t count, bool back, void* context) {
    ClarityDb* db = context;
    for (size_t i = 0; i < count; ++i) {
        if (!(back ? clarity_undo(db, steps[i], NULL) : clarity_redo(db, steps[i], NULL))) return false;
    }
    return true;
}

/// Puts `value` as the field `name` of `object`.
static void put(cJSON* object, const char* name, cJSON* value) {
    if (cJSON_GetObjectItemCaseSensitive(object, name)) cJSON_ReplaceItemInObjectCaseSensitive(object, name, value);
    else cJSON_AddItemToObject(object, name, value);
}

/// Puts `value` at `path` of the window's data; an error when the record it is in is not there.
static bool put_at_path(cJSON* data, const char* path, cJSON* value) {
    const char* dot = strrchr(path, '.');
    cJSON* record = dot ? clarity_data_at(data, clarity_copy_n(path, (size_t)(dot - path))) : data;
    if (!*path || !cJSON_IsObject(record)) return clarity_fail(CLARITY_INVALID, "the window has no %s", path);
    put(record, dot ? dot + 1 : path, value);
    return true;
}

/// A number as text, as the range functions take it: NULL for none.
static const char* number_text(const cJSON* v) {
    if (cJSON_IsString(v)) return v->valuestring;
    if (cJSON_IsNumber(v)) return clarity_format("%.15g", v->valuedouble);
    return NULL;
}

/// `state` with a copy of its data, for the runtime to change.
static ClarityWindowState with_own_data(const ClarityWindowState* state) {
    ClarityWindowState out = *state;
    out.data = cJSON_Duplicate(state->data, true);
    return out;
}

/// `state` with row `index` of `part` selected.
static ClarityWindowState with_selected(const ClarityWindowState* state, const char* part, int index) {
    ClarityWindowState out = *state;
    out.selected = CLARITY_NEW(ClaritySelected, state->selected_count + 1);
    out.selected_count = 0;
    for (size_t i = 0; i < state->selected_count; ++i)
        if (strcmp(state->selected[i].name, part) != 0) out.selected[out.selected_count++] = state->selected[i];
    out.selected[out.selected_count++] = (ClaritySelected){clarity_copy(part), index};
    return out;
}

/// The window as it is, after an action the runtime refused or that changed nothing.
static cJSON* unchanged(ClarityService* s, const ClarityWindowState* state, bool refused) {
    ClarityReply* reply;
    if (!drawn(s, state, &reply)) return failed();
    reply->refused = refused;
    return answered(s, reply);
}

/// What the screen changed of a readonly part, or of a table's column it
/// does not edit, taken back (DESIGN 1.7): the window's data as its edit
/// baseline had it, answered refused. No handler runs and no edit is kept.
static cJSON* taken_back(ClarityService* s, const ClarityWindowState* state) {
    cJSON* baseline = clarity_edits_baseline(clarity_edits_of(state->instance));
    ClarityWindowState back = baseline ? clarity_with_edit_data(state, baseline) : *state;
    return unchanged(s, &back, true);
}

static bool is_readonly(const cJSON* args) { return cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(args, "readonly")); }

static bool named(const cJSON* names, const char* name) {
    const cJSON* item;
    cJSON_ArrayForEach(item, names) {
        if (cJSON_IsString(item) && strcmp(item->valuestring, name) == 0) return true;
    }
    return false;
}

/// Whether the rows at `path` changed from `before` to `after` only in the
/// fields of `editable`: the same rows, in the same order.
static bool edits_only(const cJSON* before, const cJSON* after, const char* path, const cJSON* editable) {
    const cJSON* was = clarity_data_at(before, path);
    const cJSON* now = clarity_data_at(after, path);
    if (!cJSON_IsArray(was) || !cJSON_IsArray(now)) return (!was && !now) || (was && now && cJSON_Compare(was, now, true));
    if (cJSON_GetArraySize(was) != cJSON_GetArraySize(now)) return false;
    for (const cJSON *a = was->child, *b = now->child; a && b; a = a->next, b = b->next) {
        const cJSON* field;
        cJSON_ArrayForEach(field, a) {
            const cJSON* other = cJSON_GetObjectItemCaseSensitive(b, field->string);
            if (!named(editable, field->string) && !(other && cJSON_Compare(field, other, true))) return false;
        }
        cJSON_ArrayForEach(field, b) {
            if (!named(editable, field->string) && !cJSON_GetObjectItemCaseSensitive(a, field->string)) return false;
        }
    }
    return true;
}

/// The end of an edit the runtime made of the window's data (DESIGN 1.7),
/// as edit_event ends one: its handler runs as one database step and, with
/// a scope, the edit goes on the window's record. No handler: the window as it is.
static cJSON* edit_made(ClarityService* s, const ClarityWindowState* changed, const cJSON* args) {
    ClarityReply* reply;
    if (!stepped(s, changed, text_of(args, "handler"), &reply)) return failed();
    const char* scope = text_of(args, "scope");
    if (*scope) clarity_edits_edit(clarity_edits_of(changed->instance), text_of(args, "part"), scope, clarity_edit_data(&reply->state), &reply->step, reply->step ? 1 : 0);
    return answered(s, reply);
}

/// A row of a list moved by a drag or a key (DESIGN 5.30): the list in its
/// new order, each record's number in `by` and, in a tree, its parent, and
/// the moved row selected.
static cJSON* reordered(ClarityService* s, const ClarityWindowState* state, const cJSON* args) {
    if (is_readonly(args)) return taken_back(s, state);
    const char* list = text_of(args, "list");
    const char* by = text_of(args, "by");
    const char* parent = text_of(args, "parent");
    const cJSON* move = cJSON_GetObjectItemCaseSensitive(args, "move");
    const cJSON* records = clarity_data_at(state->data, list);
    cJSON* rows = cJSON_CreateArray();
    const cJSON* record;
    const cJSON* each = cJSON_IsArray(records) ? records : rows;
    cJSON_ArrayForEach(record, each) {
        cJSON* row = cJSON_CreateObject();
        cJSON_AddItemToArray(rows, row);
        const cJSON* id = cJSON_GetObjectItemCaseSensitive(record, "id");
        cJSON_AddItemToObject(row, "id", id ? cJSON_Duplicate(id, true) : cJSON_CreateNull());
        const cJSON* up = *parent ? cJSON_GetObjectItemCaseSensitive(record, parent) : NULL;
        if (up) cJSON_AddItemToObject(row, "parent", cJSON_Duplicate(up, true));
    }
    ClarityReordered moved;
    if (!clarity_reorder(rows, move, *parent != '\0', &moved)) return unchanged(s, state, false);
    cJSON* ordered = cJSON_CreateArray();
    int row = (int)cJSON_GetObjectItemCaseSensitive(move, "row")->valuedouble, chosen = 0;
    for (size_t i = 0; i < moved.count; ++i) {
        cJSON* r = cJSON_Duplicate(cJSON_GetArrayItem(records, (int)moved.order[i]), true);
        if (*by) put(r, by, cJSON_CreateNumber((double)moved.numbers[i]));
        if (*parent) put(r, parent, moved.parents[i] ? cJSON_Duplicate(moved.parents[i], true) : cJSON_CreateNull());
        cJSON_AddItemToArray(ordered, r);
        if ((int)moved.order[i] == row) chosen = (int)i;
    }
    ClarityWindowState changed = with_own_data(state);
    if (!put_at_path(changed.data, list, ordered)) return failed();
    changed = with_selected(&changed, text_of(args, "part"), chosen);
    return edit_made(s, &changed, args);
}

/// A typed item for a list field (U1c): added, or refused so the screen keeps the text.
static cJSON* item_added(ClarityService* s, const ClarityWindowState* state, const cJSON* args) {
    if (is_readonly(args)) return taken_back(s, state);
    const char* path = text_of(args, "path");
    const cJSON* items = clarity_data_at(state->data, path);
    cJSON* gives = clarity_item_added(text_of(args, "item"), cJSON_GetObjectItemCaseSensitive(args, "values"), items, text_of(args, "typed"));
    if (!gives) return unchanged(s, state, true);
    // Empty text or an item the list has adds nothing.
    if (cJSON_GetArraySize(gives) == (cJSON_IsArray(items) ? cJSON_GetArraySize(items) : -1)) return unchanged(s, state, false);
    ClarityWindowState changed = with_own_data(state);
    if (!put_at_path(changed.data, path, gives)) return failed();
    return edit_made(s, &changed, args);
}

/// A slider's or a spin box's value after a key, a drag or typed text (U1c).
static cJSON* ranged(ClarityService* s, const ClarityWindowState* state, const cJSON* args) {
    if (is_readonly(args)) return taken_back(s, state);
    const char* path = text_of(args, "path");
    const char* step = number_text(cJSON_GetObjectItemCaseSensitive(args, "step"));
    ClarityRange range = {number_text(cJSON_GetObjectItemCaseSensitive(args, "min")), number_text(cJSON_GetObjectItemCaseSensitive(args, "max")), step ? step : "1"};
    const cJSON* now = clarity_data_at(state->data, path);
    const cJSON* key = cJSON_GetObjectItemCaseSensitive(args, "key");
    const cJSON* at = cJSON_GetObjectItemCaseSensitive(args, "at");
    const char* after = cJSON_IsString(key) ? clarity_range_keyed(number_text(now), range, key->valuestring)
                        : at                ? clarity_range_dragged(number_text(at), range)
                                            : clarity_range_typed(number_text(cJSON_GetObjectItemCaseSensitive(args, "typed")), range);
    if (!after) return failed();
    cJSON* value = cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(args, "whole")) ? cJSON_CreateNumber(strtod(after, NULL)) : cJSON_CreateString(after);
    if (now && cJSON_Compare(now, value, true)) return unchanged(s, state, false);
    ClarityWindowState changed = with_own_data(state);
    if (!put_at_path(changed.data, path, value)) return failed();
    return edit_made(s, &changed, args);
}

/// `state` with nothing of `part` selected.
static ClarityWindowState without_selected(const ClarityWindowState* state, const char* part) {
    ClarityWindowState out = *state;
    out.selected = CLARITY_NEW(ClaritySelected, state->selected_count + 1);
    out.selected_count = 0;
    for (size_t i = 0; i < state->selected_count; ++i)
        if (strcmp(state->selected[i].name, part) != 0) out.selected[out.selected_count++] = state->selected[i];
    return out;
}

/// The person's action on a canvas (DESIGN 5.31): an object dropped, a
/// stroke painted, or a shape added, changed, moved or deleted, snapped to
/// the canvas's grid, with what it touched selected. An action that changes
/// nothing is refused and runs nothing.
static cJSON* canvas_changed(ClarityService* s, const ClarityWindowState* state, const cJSON* args) {
    if (is_readonly(args)) return taken_back(s, state);
    const char* part = text_of(args, "part");
    const cJSON* at = cJSON_GetObjectItemCaseSensitive(args, "at");
    const cJSON* grid = cJSON_GetObjectItemCaseSensitive(args, "grid");
    const cJSON* sized = cJSON_GetObjectItemCaseSensitive(args, "units");
    int units[2];
    bool has_units = cJSON_GetArraySize(sized) == 2 && cJSON_IsNumber(cJSON_GetArrayItem(sized, 0)) && cJSON_IsNumber(cJSON_GetArrayItem(sized, 1));
    if (has_units) units[0] = (int)cJSON_GetArrayItem(sized, 0)->valuedouble, units[1] = (int)cJSON_GetArrayItem(sized, 1)->valuedouble;
    ClarityCanvas canvas = {text_of(args, "mode"), text_of(args, "list"), {"", ""}, text_of(args, "path"), has_units ? units : NULL,
                            cJSON_IsNumber(grid) ? (int64_t)grid->valuedouble : 0, cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(args, "snap")), false};
    if (strcmp(canvas.mode, "objects") == 0) {
        if (cJSON_GetArraySize(at) != 2 || !cJSON_IsString(cJSON_GetArrayItem(at, 0)) || !cJSON_IsString(cJSON_GetArrayItem(at, 1)))
            return error_text("a canvas of objects names their x and y fields, as at: [\"x\", \"y\"]");
        canvas.at[0] = cJSON_GetArrayItem(at, 0)->valuestring;
        canvas.at[1] = cJSON_GetArrayItem(at, 1)->valuestring;
    }
    ClarityWindowState changed = with_own_data(state);
    ClarityCanvasTouched touched;
    switch (clarity_canvas_event(changed.data, &canvas, cJSON_GetObjectItemCaseSensitive(args, "action"), &touched)) {
    case CLARITY_CANVAS_FAILED: return failed();
    case CLARITY_CANVAS_REFUSED: return unchanged(s, state, true);
    case CLARITY_CANVAS_CHANGED: break;
    }
    changed = touched.selects ? with_selected(&changed, part, touched.selected) : without_selected(&changed, part);
    return edit_made(s, &changed, args);
}

/// `state` with the wizard `part` on `step`.
static ClarityWindowState on_step(const ClarityWindowState* state, const char* part, int64_t step) {
    ClarityUi* ui = clarity_ui(state);
    clarity_ui_set_step(ui, part, step);
    return clarity_ui_reply(ui, state->window, state->data)->state;
}

/// Next, Back or Finish on a wizard (DESIGN 5.15): Next runs `on next` on
/// the step shown, stays when it refused or failed, keeps the step it set,
/// else moves on; Back goes back and runs nothing; Finish runs `on finish`.
static cJSON* stepped_wizard(ClarityService* s, const ClarityWindowState* state, const cJSON* args) {
    const char* part = text_of(args, "part");
    const char* button = text_of(args, "button");
    const cJSON* steps = cJSON_GetObjectItemCaseSensitive(args, "steps");
    int64_t count = cJSON_IsNumber(steps) ? (int64_t)steps->valuedouble : 1;
    int64_t step = clarity_ui_step(clarity_ui(state), part);
    int64_t shown = clarity_shown_step(step, count);
    ClarityReply* reply;
    if (strcmp(button, "back") == 0) {
        ClarityWindowState back = on_step(state, part, clarity_step_back(step, count));
        return drawn(s, &back, &reply) ? answered(s, reply) : failed();
    }
    ClarityWindowState on = on_step(state, part, shown);
    if (strcmp(button, "finish") == 0) return stepped(s, &on, text_of(args, "finish"), &reply) ? answered(s, reply) : failed();
    if (strcmp(button, "next") != 0) return error_text(clarity_format("a wizard has no button %s", button));
    // A refusal or a failure answers as dispatch_event does: the step stays.
    if (!stepped(s, &on, text_of(args, "next"), &reply)) return failed();
    int64_t now = clarity_ui_step(clarity_ui(&reply->state), part);
    int64_t after = clarity_step_after_next(shown, now, count);
    if (after != now) reply->state = on_step(&reply->state, part, after);
    return answered(s, reply);
}

/// What a dropdown offers for typed text (U1c): the values a field has, or
/// the records a reference can point at. The list, not a window.
static cJSON* found_options(ClarityService* s, const cJSON* args) {
    const ClarityWindows* windows = s->options.windows;
    const char* kind = text_of(args, "kind");
    const char* typed = text_of(args, "typed");
    cJSON* found;
    if (strcmp(kind, "typed") == 0) {
        if (!windows->values) return value(cJSON_CreateArray());
        if (!windows->values(s->options.db, text_of(args, "entity"), text_of(args, "field"), &found)) return failed();
        return value(clarity_suggestions(found, typed));
    }
    if (strcmp(kind, "search") == 0) {
        if (!windows->options(s->options.db, text_of(args, "entity"), &found)) return failed();
        const cJSON* show = cJSON_GetObjectItemCaseSensitive(args, "show");
        return value(clarity_searched(found, cJSON_IsArray(show) ? show : NULL, typed));
    }
    return error_text(clarity_format("a dropdown is typed or search, not %s", kind));
}

static cJSON* answer_command(ClarityService* s, const char* command, const cJSON* args, ClarityDesktop* caller);

void clarity_service_stop(ClarityService* service) { service->stop = 1; }

bool clarity_service_watches(const char* command) {
    static const char* const watched[] = {"start_app", "open_window", "close_window", "return_to", "dispatch_event", "answer_event", "edit_event",
                                         "reorder_event", "item_event", "range_event", "step_event", "canvas_event"};
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
    s->sent = CLARITY_NEW(ClaritySelected, state.selected_count + 1);
    if (state.selected_count) memcpy(s->sent, state.selected, state.selected_count * sizeof *state.selected);
    // Every reply shows the chosen rows (their page, the tree rows above them
    // open), as a handler or a move may have changed where they are; only the
    // person's own view action leaves the view as they set it.
    s->sent_count = strcmp(command, "view_event") == 0 ? state.selected_count : 0;
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
        // The window has closed: what it had of an engine goes, and its edits.
        clarity_window_closed(state.instance);
        clarity_edits_closed(state.instance);
        return value(clarity_reply_json(reply));
    }
    if (strcmp(command, "return_to") == 0) return replied(s, &state, "return");
    if (strcmp(command, "start_app") == 0) return replied(s, &state, "start");
    if (strcmp(command, "dispatch_event") == 0) {
        // One undo step (DESIGN 5.29): the reply carries its id when the handler changed the database.
        int64_t before = clarity_last_step(db);
        Stepping step = {s, &state, text_of(args, "handler"), NULL};
        if (!clarity_step(db, dispatched_step, &step)) return failed();
        if (clarity_last_step(db) != before) step.reply->step = clarity_last_step(db);
        return answered(s, step.reply);
    }
    if (strcmp(command, "edit_event") == 0) {
        // The person ended an edit of a part (DESIGN 5.29): its handler runs as
        // one database step, and the edit goes on the window's record with
        // what it changed. A drag goes on until its drop. A readonly part, or a
        // table's column it does not edit, changes nothing (DESIGN 1.7).
        if (is_readonly(args)) return taken_back(s, &state);
        const cJSON* editable = cJSON_GetObjectItemCaseSensitive(args, "editable");
        cJSON* baseline = clarity_edits_baseline(clarity_edits_of(state.instance));
        if (cJSON_IsArray(editable) && baseline && !edits_only(baseline, state.data, text_of(args, "path"), editable)) return taken_back(s, &state);
        ClarityReply* reply;
        if (!stepped(s, &state, text_of(args, "handler"), &reply)) return failed();
        ClarityEdits* bus = clarity_edits_of(state.instance);
        size_t count = reply->step ? 1 : 0;
        if (cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(args, "ends")))
            clarity_edits_edit(bus, text_of(args, "part"), text_of(args, "scope"), clarity_edit_data(&reply->state), &reply->step, count);
        else
            clarity_edits_drag(bus, text_of(args, "part"), text_of(args, "scope"), &reply->step, count);
        return answered(s, reply);
    }
    // What the person did to a part, which the runtime decides (DESIGN 1.7).
    if (strcmp(command, "reorder_event") == 0) return reordered(s, &state, args);
    if (strcmp(command, "item_event") == 0) return item_added(s, &state, args);
    if (strcmp(command, "range_event") == 0) return ranged(s, &state, args);
    if (strcmp(command, "canvas_event") == 0) return canvas_changed(s, &state, args);
    if (strcmp(command, "step_event") == 0) return stepped_wizard(s, &state, args);
    if (strcmp(command, "find_options") == 0) return found_options(s, args);
    if (strcmp(command, "view_event") == 0) {
        // The person sorted, turned a page, or opened or closed a tree's row: no handler, no edit.
        ClarityReply* reply;
        if (!clarity_view_event(&state, text_of(args, "part"), cJSON_GetObjectItemCaseSensitive(args, "action"))) return failed();
        return drawn(s, &state, &reply) ? answered(s, reply) : failed();
    }
    if (strcmp(command, "undo_edit") == 0) {
        // Ctrl+Z (back) or Ctrl+Y on a scope: its last edit taken back or done
        // again, with its database steps, and the window as it was.
        bool back = cJSON_IsTrue(cJSON_GetObjectItemCaseSensitive(args, "back"));
        bool found;
        cJSON* data;
        if (!clarity_edits_undo(clarity_edits_of(state.instance), text_of(args, "scope"), clarity_edit_data(&state), back, take_steps, db, &found, &data))
            return failed();
        ClarityWindowState undone = found ? clarity_with_edit_data(&state, data) : state;
        ClarityReply* reply;
        return drawn(s, &undone, &reply) ? answered(s, reply) : failed();
    }
    if (strcmp(command, "answer_event") == 0) {
        // The answer to a question a handler asked runs the handler it goes to.
        const cJSON* values = cJSON_GetObjectItemCaseSensitive(args, "values");
        ClarityReply* reply;
        if (!windows->answer) return error_text("this program asks nothing");
        if (!windows->answer(db, &state, text_of(args, "handler"), cJSON_IsArray(values) ? values : cJSON_CreateArray(), &reply)) return failed();
        if (!windows->redraw(db, reply)) return failed();
        return answered(s, reply);
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
