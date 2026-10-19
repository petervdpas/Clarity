#include "clarity/ui.h"

#include <stdarg.h>
#include <string.h>

struct ClarityUi {
    ClarityWindowState state;
    size_t part_room;
    ClarityMessage* messages;
    size_t message_count, message_room;
    ClarityOpen* open;
    size_t open_count, open_room;
    bool close;
};

ClarityUi* clarity_ui(const ClarityWindowState* state) {
    ClarityUi* ui = CLARITY_NEW(ClarityUi, 1);
    ui->state = *state;
    // The parts are changed in a copy of their own.
    ui->state.parts = CLARITY_NEW(ClarityPart, state->part_count);
    if (state->part_count) memcpy(ui->state.parts, state->parts, state->part_count * sizeof *state->parts);
    ui->part_room = state->part_count;
    // So are the selected rows, which deselect removes.
    ui->state.selected = CLARITY_NEW(ClaritySelected, state->selected_count);
    if (state->selected_count) memcpy(ui->state.selected, state->selected, state->selected_count * sizeof *state->selected);
    return ui;
}

static const ClarityPart* found(const ClarityUi* ui, const char* part) {
    for (size_t i = 0; i < ui->state.part_count; ++i)
        if (strcmp(ui->state.parts[i].name, part) == 0) return &ui->state.parts[i];
    return NULL;
}

static ClarityPart* part_of(ClarityUi* ui, const char* part) {
    ClarityPart* p = (ClarityPart*)found(ui, part);
    if (p) return p;
    clarity_grow((void**)&ui->state.parts, &ui->part_room, ui->state.part_count, sizeof *ui->state.parts);
    p = &ui->state.parts[ui->state.part_count++];
    *p = (ClarityPart){.name = clarity_copy(part)};
    return p;
}

bool clarity_ui_visible(const ClarityUi* ui, const char* part) {
    const ClarityPart* p = found(ui, part);
    return p && p->has_visible ? p->visible : true;
}

void clarity_ui_set_visible(ClarityUi* ui, const char* part, bool value) {
    ClarityPart* p = part_of(ui, part);
    p->has_visible = true;
    p->visible = value;
}

bool clarity_ui_enabled(const ClarityUi* ui, const char* part) {
    const ClarityPart* p = found(ui, part);
    return p && p->has_enabled ? p->enabled : true;
}

void clarity_ui_set_enabled(ClarityUi* ui, const char* part, bool value) {
    ClarityPart* p = part_of(ui, part);
    p->has_enabled = true;
    p->enabled = value;
}

const char* clarity_ui_caption(const ClarityUi* ui, const char* part, const char* design) {
    const ClarityPart* p = found(ui, part);
    return p && p->caption ? clarity_text_show(p->caption) : design;
}

void clarity_ui_set_caption(ClarityUi* ui, const char* part, const ClarityText* value) { part_of(ui, part)->caption = value; }

const char* clarity_ui_error(const ClarityUi* ui, const char* part) {
    const ClarityPart* p = found(ui, part);
    return p && p->error ? clarity_text_show(p->error) : "";
}

void clarity_ui_set_error(ClarityUi* ui, const char* part, const ClarityText* value) { part_of(ui, part)->error = value; }

bool clarity_ui_readonly(const ClarityUi* ui, const char* part, bool design) {
    const ClarityPart* p = found(ui, part);
    return p && p->has_readonly ? p->readonly : design;
}

void clarity_ui_set_readonly(ClarityUi* ui, const char* part, bool value) {
    ClarityPart* p = part_of(ui, part);
    p->has_readonly = true;
    p->readonly = value;
}

const char* clarity_ui_text(const ClarityUi* ui, const char* part) {
    const ClarityPart* p = found(ui, part);
    return p && p->text ? p->text : "";
}

void clarity_ui_set_text(ClarityUi* ui, const char* part, const char* value) { part_of(ui, part)->text = clarity_copy(value); }

bool clarity_ui_checked(const ClarityUi* ui, const char* part) {
    const ClarityPart* p = found(ui, part);
    return p && p->has_checked ? p->checked : false;
}

void clarity_ui_set_checked(ClarityUi* ui, const char* part, bool value) {
    ClarityPart* p = part_of(ui, part);
    p->has_checked = true;
    p->checked = value;
}

bool clarity_ui_selected(const ClarityUi* ui, const char* part, size_t count, size_t* index) {
    for (size_t i = 0; i < ui->state.selected_count; ++i) {
        const ClaritySelected* s = &ui->state.selected[i];
        if (strcmp(s->name, part) == 0 && s->index >= 0 && (size_t)s->index < count) {
            *index = (size_t)s->index;
            return true;
        }
    }
    return clarity_fail(CLARITY_INVALID, "no row of %s is selected", part);
}

void clarity_ui_deselect(ClarityUi* ui, const char* part) {
    size_t kept = 0;
    for (size_t i = 0; i < ui->state.selected_count; ++i) {
        if (strcmp(ui->state.selected[i].name, part) != 0) ui->state.selected[kept++] = ui->state.selected[i];
    }
    ui->state.selected_count = kept;
}

void clarity_ui_message(ClarityUi* ui, ClarityLevel level, const ClarityText* text) {
    clarity_grow((void**)&ui->messages, &ui->message_room, ui->message_count, sizeof *ui->messages);
    ui->messages[ui->message_count++] = (ClarityMessage){level, text};
}

void clarity_ui_open(ClarityUi* ui, const char* window, cJSON* with) {
    clarity_grow((void**)&ui->open, &ui->open_room, ui->open_count, sizeof *ui->open);
    ui->open[ui->open_count++] = (ClarityOpen){clarity_copy(window), with ? with : cJSON_CreateObject()};
}

void clarity_ui_close(ClarityUi* ui) { ui->close = true; }

ClarityReply* clarity_ui_reply(const ClarityUi* ui, const char* window, cJSON* data) {
    ClarityReply* r = CLARITY_NEW(ClarityReply, 1);
    r->state = ui->state;
    r->state.window = clarity_copy(window);
    r->state.data = cJSON_IsObject(data) ? data : cJSON_CreateObject();
    r->messages = ui->messages;
    r->message_count = ui->message_count;
    r->open = ui->open;
    r->open_count = ui->open_count;
    r->close = ui->close;
    return r;
}

ClarityWindowState* clarity_opened(const char* window, cJSON* data, const cJSON* with) {
    ClarityWindowState* s = CLARITY_NEW(ClarityWindowState, 1);
    s->window = clarity_copy(window);
    s->data = cJSON_IsObject(data) ? data : cJSON_CreateObject();
    const cJSON* item;
    cJSON_ArrayForEach(item, with) {
        cJSON* copy = cJSON_Duplicate(item, true);
        if (cJSON_GetObjectItemCaseSensitive(s->data, item->string)) cJSON_ReplaceItemInObjectCaseSensitive(s->data, item->string, copy);
        else cJSON_AddItemToObject(s->data, item->string, copy);
    }
    return s;
}

static cJSON* part_json(const ClarityPart* p) {
    cJSON* out = cJSON_CreateObject();
    if (p->has_visible) cJSON_AddBoolToObject(out, "visible", p->visible);
    if (p->has_enabled) cJSON_AddBoolToObject(out, "enabled", p->enabled);
    if (p->caption) cJSON_AddItemToObject(out, "caption", clarity_text_json(p->caption));
    if (p->error) cJSON_AddItemToObject(out, "error", clarity_text_json(p->error));
    if (p->has_readonly) cJSON_AddBoolToObject(out, "readonly", p->readonly);
    if (p->text) cJSON_AddStringToObject(out, "text", p->text);
    if (p->has_checked) cJSON_AddBoolToObject(out, "checked", p->checked);
    return out;
}

static bool part_from_json(const cJSON* in, ClarityPart* p) {
    *p = (ClarityPart){.name = clarity_copy(in->string)};
    const cJSON* v;
    if (cJSON_IsBool(v = cJSON_GetObjectItemCaseSensitive(in, "visible"))) p->has_visible = true, p->visible = cJSON_IsTrue(v);
    if (cJSON_IsBool(v = cJSON_GetObjectItemCaseSensitive(in, "enabled"))) p->has_enabled = true, p->enabled = cJSON_IsTrue(v);
    if (cJSON_IsBool(v = cJSON_GetObjectItemCaseSensitive(in, "readonly"))) p->has_readonly = true, p->readonly = cJSON_IsTrue(v);
    if (cJSON_IsBool(v = cJSON_GetObjectItemCaseSensitive(in, "checked"))) p->has_checked = true, p->checked = cJSON_IsTrue(v);
    if (cJSON_IsString(v = cJSON_GetObjectItemCaseSensitive(in, "text"))) p->text = clarity_copy(v->valuestring);
    ClarityText* text;
    v = cJSON_GetObjectItemCaseSensitive(in, "caption");
    if (v && !cJSON_IsNull(v)) {
        if (!clarity_text_from_json(v, &text)) return false;
        p->caption = text;
    }
    v = cJSON_GetObjectItemCaseSensitive(in, "error");
    if (v && !cJSON_IsNull(v)) {
        if (!clarity_text_from_json(v, &text)) return false;
        p->error = text;
    }
    return true;
}

cJSON* clarity_window_state_json(const ClarityWindowState* s) {
    cJSON* out = cJSON_CreateObject();
    cJSON_AddStringToObject(out, "window", s->window ? s->window : "");
    cJSON_AddItemToObject(out, "data", s->data ? cJSON_Duplicate(s->data, true) : cJSON_CreateObject());
    cJSON* parts = cJSON_AddObjectToObject(out, "parts");
    for (size_t i = 0; i < s->part_count; ++i) cJSON_AddItemToObject(parts, s->parts[i].name, part_json(&s->parts[i]));
    cJSON* selected = cJSON_AddObjectToObject(out, "selected");
    for (size_t i = 0; i < s->selected_count; ++i) cJSON_AddNumberToObject(selected, s->selected[i].name, s->selected[i].index);
    return out;
}

bool clarity_window_state_from_json(const cJSON* in, ClarityWindowState* out) {
    ClarityWindowState s = {0};
    const cJSON* window = cJSON_GetObjectItemCaseSensitive(in, "window");
    s.window = clarity_copy(cJSON_IsString(window) ? window->valuestring : "");
    const cJSON* data = cJSON_GetObjectItemCaseSensitive(in, "data");
    s.data = cJSON_IsObject(data) ? cJSON_Duplicate(data, true) : cJSON_CreateObject();
    const cJSON* parts = cJSON_GetObjectItemCaseSensitive(in, "parts");
    int n = cJSON_GetArraySize(parts);
    s.parts = CLARITY_NEW(ClarityPart, n);
    const cJSON* item;
    cJSON_ArrayForEach(item, parts) {
        if (!part_from_json(item, &s.parts[s.part_count])) return false;
        ++s.part_count;
    }
    const cJSON* selected = cJSON_GetObjectItemCaseSensitive(in, "selected");
    s.selected = CLARITY_NEW(ClaritySelected, cJSON_GetArraySize(selected));
    cJSON_ArrayForEach(item, selected) {
        if (!cJSON_IsNumber(item)) continue;
        s.selected[s.selected_count++] = (ClaritySelected){clarity_copy(item->string), (int)item->valuedouble};
    }
    *out = s;
    return true;
}

static const char* level_name(ClarityLevel level) {
    switch (level) {
    case CLARITY_WARNING: return "warning";
    case CLARITY_ERROR: return "error";
    default: return "info";
    }
}

cJSON* clarity_reply_json(const ClarityReply* r) {
    cJSON* out = cJSON_CreateObject();
    cJSON_AddItemToObject(out, "state", clarity_window_state_json(&r->state));
    cJSON* messages = cJSON_AddArrayToObject(out, "messages");
    for (size_t i = 0; i < r->message_count; ++i) {
        cJSON* m = cJSON_CreateObject();
        cJSON_AddStringToObject(m, "level", level_name(r->messages[i].level));
        cJSON_AddItemToObject(m, "text", clarity_text_json(r->messages[i].text));
        cJSON_AddItemToArray(messages, m);
    }
    cJSON* open = cJSON_AddArrayToObject(out, "open");
    for (size_t i = 0; i < r->open_count; ++i) {
        cJSON* o = cJSON_CreateObject();
        cJSON_AddStringToObject(o, "window", r->open[i].window);
        cJSON_AddItemToObject(o, "with", cJSON_Duplicate(r->open[i].with, true));
        cJSON_AddItemToArray(open, o);
    }
    cJSON_AddBoolToObject(out, "close", r->close);
    return out;
}

void clarity_add_option(cJSON* options, int64_t id, const char* text) {
    cJSON* o = cJSON_CreateObject();
    cJSON_AddNumberToObject(o, "id", (double)id);
    cJSON_AddStringToObject(o, "text", text ? text : "");
    cJSON_AddItemToArray(options, o);
}

cJSON* clarity_json_object(int count, ...) {
    cJSON* out = cJSON_CreateObject();
    va_list args;
    va_start(args, count);
    for (int i = 0; i < count; ++i) {
        const char* name = va_arg(args, const char*);
        cJSON* value = va_arg(args, cJSON*);
        cJSON_AddItemToObject(out, name, value);
    }
    va_end(args);
    return out;
}
