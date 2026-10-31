// The edit bus: see edits.h.

#include "clarity/edits.h"

#include <stdlib.h>
#include <string.h>

#include "clarity/db_internal.h"

/// One name an edit changed, with its value before and after as JSON text;
/// NULL when the name was not there.
typedef struct {
    char* name;
    char* before;
    char* after;
} Changed;

/// One edit of the person's.
typedef struct {
    char* part;
    char* scope;
    Changed* changes;
    size_t change_count;
    int64_t* steps;
    size_t step_count;
} Edit;

/// The edits of a stack, the last on top.
typedef struct {
    Edit** items;
    size_t count;
} Edits;

struct ClarityEdits {
    /// The window's data as JSON text, as the last reply or edit left it.
    char* baseline;
    /// The edit that goes on (a drag), or NULL.
    Edit* going;
    Edits done;
    Edits undone;
};

/// How many edits a window keeps; the oldest go first.
enum { MAX_EDITS = 500 };

static void* grown(void* items, size_t count) {
    void* more = realloc(items, count);
    if (!more) abort();
    return more;
}

/// JSON outside the scratch arena, as text; NULL stays NULL.
static char* kept(const cJSON* value) { return value ? clarity_lasting(cJSON_PrintUnformatted(value)) : NULL; }

static Edit* edit_new(const char* part, const char* scope) {
    Edit* edit = calloc(1, sizeof *edit);
    if (!edit) abort();
    edit->part = clarity_lasting(part);
    edit->scope = clarity_lasting(scope);
    return edit;
}

static void add_steps(Edit* edit, const int64_t* steps, size_t count) {
    if (!count) return;
    edit->steps = grown(edit->steps, (edit->step_count + count) * sizeof *edit->steps);
    memcpy(edit->steps + edit->step_count, steps, count * sizeof *steps);
    edit->step_count += count;
}

static void edit_free(Edit* edit) {
    if (!edit) return;
    for (size_t i = 0; i < edit->change_count; ++i) {
        free(edit->changes[i].name);
        free(edit->changes[i].before);
        free(edit->changes[i].after);
    }
    free(edit->changes);
    free(edit->steps);
    free(edit->part);
    free(edit->scope);
    free(edit);
}

static void push(Edits* edits, Edit* edit) {
    edits->items = grown(edits->items, (edits->count + 1) * sizeof *edits->items);
    edits->items[edits->count++] = edit;
}

static void take_out(Edits* edits, size_t at) {
    memmove(edits->items + at, edits->items + at + 1, (edits->count - at - 1) * sizeof *edits->items);
    --edits->count;
}

static void edits_free(Edits* edits) {
    for (size_t i = 0; i < edits->count; ++i) edit_free(edits->items[i]);
    free(edits->items);
    *edits = (Edits){0};
}

ClarityEdits* clarity_edits_new(void) {
    ClarityEdits* bus = calloc(1, sizeof *bus);
    if (!bus) abort();
    return bus;
}

void clarity_edits_free(ClarityEdits* bus) {
    if (!bus) return;
    free(bus->baseline);
    edit_free(bus->going);
    edits_free(&bus->done);
    edits_free(&bus->undone);
    free(bus);
}

static void set_baseline(ClarityEdits* bus, const cJSON* data) {
    free(bus->baseline);
    bus->baseline = kept(data);
}

void clarity_edits_reset(ClarityEdits* bus, const cJSON* data) {
    if (!bus->going) set_baseline(bus, data);
}

void clarity_edits_drag(ClarityEdits* bus, const char* part, const char* scope, const int64_t* steps, size_t step_count) {
    if (!bus->going) bus->going = edit_new(part, scope);
    add_steps(bus->going, steps, step_count);
}

/// Adds to `edit` what changed of `name` from before to after, if anything did.
static void compare(Edit* edit, const char* name, const cJSON* before, const cJSON* after) {
    if (before && after && cJSON_Compare(before, after, true)) return;
    if (!before && !after) return;
    edit->changes = grown(edit->changes, (edit->change_count + 1) * sizeof *edit->changes);
    edit->changes[edit->change_count++] = (Changed){clarity_lasting(name), kept(before), kept(after)};
}

void clarity_edits_edit(ClarityEdits* bus, const char* part, const char* scope, const cJSON* data, const int64_t* steps, size_t step_count) {
    Edit* edit = edit_new(part, scope);
    if (bus->going) {
        add_steps(edit, bus->going->steps, bus->going->step_count);
        edit_free(bus->going);
        bus->going = NULL;
    }
    add_steps(edit, steps, step_count);
    // What changed, by name: every name of either, the one before first.
    cJSON* before = bus->baseline ? cJSON_Parse(bus->baseline) : NULL;
    const cJSON* item;
    cJSON_ArrayForEach(item, before) compare(edit, item->string, item, cJSON_GetObjectItemCaseSensitive(data, item->string));
    cJSON_ArrayForEach(item, data) {
        if (!cJSON_GetObjectItemCaseSensitive(before, item->string)) compare(edit, item->string, NULL, item);
    }
    set_baseline(bus, data);
    if (!edit->change_count && !edit->step_count) {
        edit_free(edit);
        return;
    }
    push(&bus->done, edit);
    if (bus->done.count > MAX_EDITS) {
        edit_free(bus->done.items[0]);
        take_out(&bus->done, 0);
    }
    for (size_t i = bus->undone.count; i-- > 0;) {
        if (strcmp(bus->undone.items[i]->scope, scope) != 0) continue;
        edit_free(bus->undone.items[i]);
        take_out(&bus->undone, i);
    }
}

bool clarity_edits_undo(ClarityEdits* bus, const char* scope, const cJSON* data, bool back, ClarityTakeSteps take, void* context, bool* found,
                        cJSON** out) {
    Edits* from = back ? &bus->done : &bus->undone;
    Edits* to = back ? &bus->undone : &bus->done;
    *found = false;
    *out = NULL;
    size_t at = from->count;
    while (at-- > 0) {
        if (strcmp(from->items[at]->scope, scope) == 0) break;
    }
    if (at == (size_t)-1) return true;
    *found = true;
    Edit* edit = from->items[at];
    // Back takes the steps back last first; again does them first first.
    int64_t* steps = CLARITY_NEW(int64_t, edit->step_count);
    for (size_t i = 0; i < edit->step_count; ++i) steps[i] = edit->steps[back ? edit->step_count - 1 - i : i];
    if (edit->step_count && !take(steps, edit->step_count, back, context)) return false;
    take_out(from, at);
    push(to, edit);
    cJSON* result = data ? cJSON_Duplicate(data, true) : cJSON_CreateObject();
    for (size_t i = 0; i < edit->change_count; ++i) {
        const Changed* c = &edit->changes[back ? edit->change_count - 1 - i : i];
        const char* value = back ? c->before : c->after;
        cJSON_DeleteItemFromObjectCaseSensitive(result, c->name);
        if (value) cJSON_AddItemToObject(result, c->name, cJSON_Parse(value));
    }
    set_baseline(bus, result);
    *out = result;
    return true;
}

/// The edit bus of each open window, by its number.
typedef struct {
    uint64_t instance;
    ClarityEdits* bus;
} Window;

static Window* windows;
static size_t window_count;

ClarityEdits* clarity_edits_of(uint64_t instance) {
    for (size_t i = 0; i < window_count; ++i) {
        if (windows[i].instance == instance) return windows[i].bus;
    }
    windows = grown(windows, (window_count + 1) * sizeof *windows);
    windows[window_count] = (Window){instance, clarity_edits_new()};
    return windows[window_count++].bus;
}

void clarity_edits_closed(uint64_t instance) {
    for (size_t i = 0; i < window_count; ++i) {
        if (windows[i].instance != instance) continue;
        clarity_edits_free(windows[i].bus);
        windows[i] = windows[--window_count];
        return;
    }
}

/// Names a part's own text or tick among the data an edit compares.
static const char part_state[] = "#parts.";

cJSON* clarity_edit_data(const ClarityWindowState* state) {
    cJSON* data = state->data ? cJSON_Duplicate(state->data, true) : cJSON_CreateObject();
    for (size_t i = 0; i < state->part_count; ++i) {
        const ClarityPart* part = &state->parts[i];
        if (!part->text && !part->has_checked) continue;
        cJSON* own = cJSON_CreateObject();
        if (part->text) cJSON_AddStringToObject(own, "text", part->text);
        if (part->has_checked) cJSON_AddBoolToObject(own, "checked", part->checked);
        cJSON_AddItemToObject(data, clarity_format("%s%s", part_state, part->name), own);
    }
    return data;
}

ClarityWindowState clarity_with_edit_data(const ClarityWindowState* state, const cJSON* data) {
    ClarityWindowState out = *state;
    out.data = cJSON_CreateObject();
    // Room for a part the state has not named yet, for each name of the data.
    out.parts = CLARITY_NEW(ClarityPart, state->part_count + (size_t)cJSON_GetArraySize(data));
    for (size_t i = 0; i < state->part_count; ++i) {
        out.parts[i] = state->parts[i];
        out.parts[i].text = NULL;
        out.parts[i].has_checked = out.parts[i].checked = false;
    }
    const cJSON* item;
    cJSON_ArrayForEach(item, data) {
        if (strncmp(item->string, part_state, sizeof part_state - 1) != 0) {
            cJSON_AddItemToObject(out.data, item->string, cJSON_Duplicate(item, true));
            continue;
        }
        const char* name = item->string + sizeof part_state - 1;
        size_t at = 0;
        while (at < out.part_count && strcmp(out.parts[at].name, name) != 0) ++at;
        if (at == out.part_count) out.parts[out.part_count++] = (ClarityPart){.name = clarity_copy(name)};
        const cJSON* text = cJSON_GetObjectItemCaseSensitive(item, "text");
        const cJSON* checked = cJSON_GetObjectItemCaseSensitive(item, "checked");
        if (cJSON_IsString(text)) out.parts[at].text = clarity_copy(text->valuestring);
        if (cJSON_IsBool(checked)) out.parts[at].has_checked = true, out.parts[at].checked = cJSON_IsTrue(checked);
    }
    return out;
}
