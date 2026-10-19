// The binding between a window on the screen and its handlers in the
// backend. The screen sends a window's state with each event; the handler
// runs on it and the reply carries the new state, the messages to show and
// the windows to open or close. The JSON is the other runtimes', so the same
// renderer serves every stack. Everything here lives in the scratch arena.

#pragma once

#include "cjson/cJSON.h"
#include "clarity/text.h"

/// What a handler changed of a part; what is not set keeps the design.
typedef struct {
    const char* name;
    bool has_visible, visible;
    bool has_enabled, enabled;
    const ClarityText* caption;
    const ClarityText* error;
    bool has_readonly, readonly;
    /// What an unbound text input holds (NULL: not set); the screen sends it with each event.
    const char* text;
    /// Whether an unbound checkbox or switch is on.
    bool has_checked, checked;
} ClarityPart;

/// The selected row of a table or list.
typedef struct {
    const char* name;
    int index;
} ClaritySelected;

/// One open window: its data by name, its parts' properties and the
/// selected row of each table or list.
typedef struct {
    const char* window;
    cJSON* data;
    ClarityPart* parts;
    size_t part_count;
    ClaritySelected* selected;
    size_t selected_count;
} ClarityWindowState;

/// How a message shows: info goes away by itself.
typedef enum { CLARITY_INFO, CLARITY_WARNING, CLARITY_ERROR } ClarityLevel;

typedef struct {
    ClarityLevel level;
    const ClarityText* text;
} ClarityMessage;

/// `open Window with name = value, ...`
typedef struct {
    const char* window;
    cJSON* with;
} ClarityOpen;

/// What the screen does after a handler ran.
typedef struct {
    ClarityWindowState state;
    ClarityMessage* messages;
    size_t message_count;
    ClarityOpen* open;
    size_t open_count;
    bool close;
} ClarityReply;

/// What a handler can do to the screen.
typedef struct ClarityUi ClarityUi;

ClarityUi* clarity_ui(const ClarityWindowState* state);

bool clarity_ui_visible(const ClarityUi* ui, const char* part);
void clarity_ui_set_visible(ClarityUi* ui, const char* part, bool value);
bool clarity_ui_enabled(const ClarityUi* ui, const char* part);
void clarity_ui_set_enabled(ClarityUi* ui, const char* part, bool value);
/// The caption a handler set, or `design`.
const char* clarity_ui_caption(const ClarityUi* ui, const char* part, const char* design);
void clarity_ui_set_caption(ClarityUi* ui, const char* part, const ClarityText* value);
/// The message a handler set on an input, or "".
const char* clarity_ui_error(const ClarityUi* ui, const char* part);
void clarity_ui_set_error(ClarityUi* ui, const char* part, const ClarityText* value);
/// Whether an input is read-only: as a handler set it, or `design`.
bool clarity_ui_readonly(const ClarityUi* ui, const char* part, bool design);
void clarity_ui_set_readonly(ClarityUi* ui, const char* part, bool value);
/// The text of an unbound text input.
const char* clarity_ui_text(const ClarityUi* ui, const char* part);
void clarity_ui_set_text(ClarityUi* ui, const char* part, const char* value);
/// Whether an unbound checkbox or switch is on.
bool clarity_ui_checked(const ClarityUi* ui, const char* part);
void clarity_ui_set_checked(ClarityUi* ui, const char* part, bool value);

/// The index of the selected row of a table or list showing `count` rows;
/// an invalid-value error when none is.
bool clarity_ui_selected(const ClarityUi* ui, const char* part, size_t count, size_t* index);

/// Forgets the selected row of a table or list, whose rows were read again
/// or replaced: they are other records.
void clarity_ui_deselect(ClarityUi* ui, const char* part);

void clarity_ui_message(ClarityUi* ui, ClarityLevel level, const ClarityText* text);
/// Asks the screen to open `window` with its data filled by name, as the values are now.
void clarity_ui_open(ClarityUi* ui, const char* window, cJSON* with);
/// Asks the screen to close the window.
void clarity_ui_close(ClarityUi* ui);

/// The reply for `window`, with its data after the handler.
ClarityReply* clarity_ui_reply(const ClarityUi* ui, const char* window, cJSON* data);

/// The state of `window` when it opens: its loaded data with `with` filled in by name.
ClarityWindowState* clarity_opened(const char* window, cJSON* data, const cJSON* with);

cJSON* clarity_window_state_json(const ClarityWindowState* state);
bool clarity_window_state_from_json(const cJSON* in, ClarityWindowState* out);
cJSON* clarity_reply_json(const ClarityReply* reply);
/// `{"id": id, "text": text}` added to `options`: a record a reference field can point at.
void clarity_add_option(cJSON* options, int64_t id, const char* text);
/// An object of `count` pairs of name and value, such as the `with` of `open`.
cJSON* clarity_json_object(int count, ...);
