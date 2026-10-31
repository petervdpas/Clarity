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
    /// Whether a foldable panel or form is folded, as the person or a handler
    /// left it; without `has_folded` it is as designed (conformance/folding.json).
    bool has_folded, folded;
    /// Whether a collapsible sidebar is folded to its button, and whether a
    /// maximizable one fills the window (conformance/sidebars.json).
    bool has_collapsed, collapsed;
    /// A canvas's grid in its units (0 for none) and whether code snaps to it, as a handler set them.
    bool has_grid, has_snap, snap;
    int64_t grid;
    /// What a canvas's clear() fills it with, as a handler set it (empty is the theme's panel); NULL keeps the design.
    const char* background;
    /// A canvas's grid dots, as a handler set them: pixels across, and color (empty is the theme's border).
    bool has_dot_size;
    int64_t dot_size;
    const char* dot_color;
    bool has_maximized, maximized;
    /// The open tab of a ribbon, counted from 1 (conformance/ribbons.json).
    bool has_tab;
    int64_t tab;
    /// The step a wizard (a steps part) is on, counted from 1 (conformance/steps.json).
    bool has_step;
    int64_t step;
    /// The open section of an accordion, counted from 1, 0 for none (conformance/accordions.json).
    bool has_open;
    int64_t open;
    /// What an unbound text input holds (NULL: not set); the screen sends it with each event.
    const char* text;
    /// Whether an unbound checkbox or switch is on.
    bool has_checked, checked;
    /// A canvas's drawing: an array of its shapes, in order (NULL: none;
    /// conformance/drawing.json).
    cJSON* drawing;
    /// Where a canvas was last clicked, in its units: an object with x and y,
    /// and whatever else the screen tells about the pointer, kept as it came
    /// (NULL: no click yet).
    cJSON* pointer;
    /// A custom part's properties by name, as code set them: an object (NULL:
    /// none set; conformance/custom-parts.json).
    cJSON* values;
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
    /// The number of this open window, its own while it is open: what belongs
    /// to it, such as an engine's state per window, goes by it.
    uint64_t instance;
    cJSON* data;
    ClarityPart* parts;
    size_t part_count;
    ClaritySelected* selected;
    size_t selected_count;
    /// What the window's tables, lists, trees and charts show, by part name
    /// (DESIGN 1.7, clarity/views.h): an object, NULL when it has none.
    cJSON* views;
} ClarityWindowState;

/// How a message shows: info goes away by itself.
typedef enum { CLARITY_INFO, CLARITY_WARNING, CLARITY_ERROR } ClarityLevel;

typedef struct {
    ClarityLevel level;
    const ClarityText* text;
} ClarityMessage;

/// A text a handler put in a text value, which the screen shows translated.
typedef struct {
    const char* data;
    const ClarityText* text;
} ClarityShown;

/// `open Window with name = value, ...`
typedef struct {
    const char* window;
    cJSON* with;
} ClarityOpen;

/// `ask ... then handler(values)` (DESIGN 5.14): a question the screen asks
/// in Clarity's own dialog when the handler has finished. Its answer runs
/// `handler` with `values` and, for a value, the answer last.
typedef struct {
    const ClarityText* question;
    /// What it asks for, as the screen reads it: clarity_ask_confirm or clarity_ask_value.
    cJSON* form;
    const char* handler;
    cJSON* values;
} ClarityAsk;

/// A question with yes and no (NULL: OK and Cancel); the handler runs on yes.
cJSON* clarity_ask_confirm(const ClarityText* yes, const ClarityText* no);

/// A question for a value of `type` (text, integer, decimal, money, date,
/// datetime, duration, enum or record), of the enum or entity `of` (or
/// NULL). `only` narrows the choices to the records' ids or the enum's
/// values (an array, or NULL); `start` is where a typed value starts (or
/// NULL). Takes `only` and `start`.
cJSON* clarity_ask_value(const char* type, const char* of, cJSON* only, cJSON* start);

/// A dialog window to open with `with` (its data by name, which it takes);
/// the answer is what it closes with.
cJSON* clarity_ask_window(const char* window, cJSON* with);

/// What the screen does after a handler ran.
typedef struct {
    ClarityWindowState state;
    ClarityMessage* messages;
    size_t message_count;
    ClarityShown* shown;
    size_t shown_count;
    ClarityOpen* open;
    size_t open_count;
    bool close;
    /// The questions to ask when the handler has finished, in order.
    ClarityAsk* asks;
    size_t ask_count;
    /// What a dialog closes with (`close with value`), or NULL.
    cJSON* result;
    /// The hears given to custom parts, in order: an array of {part, hear,
    /// values} (NULL: none).
    cJSON* hears;
    /// The text the handler copied (`copy text`), for the screen to put on the clipboard; NULL: none.
    const ClarityText* copy;
    /// The id of the database step the handler made (DESIGN 5.29), which
    /// the screen's edit keeps to undo it by; 0 when it changed nothing.
    int64_t step;
    /// The runtime refused what the person gave (a list field's typed item):
    /// the screen keeps it and marks it.
    bool refused;
} ClarityReply;

/// What a handler can do to the screen.
typedef struct ClarityUi ClarityUi;

ClarityUi* clarity_ui(const ClarityWindowState* state);

/// The number of the open window the handler runs in.
uint64_t clarity_ui_instance(const ClarityUi* ui);

/// Has `listen` hear of every window that closes, by its number: an engine
/// with state per window drops what it had of it.
void clarity_on_window_closed(void (*listen)(uint64_t instance, void* context), void* context);

/// Stops `listen` hearing of windows that close.
void clarity_off_window_closed(void (*listen)(uint64_t instance, void* context), void* context);

/// Tells what listens that window `instance` has closed, once its own `on close` has run.
void clarity_window_closed(uint64_t instance);

bool clarity_ui_visible(const ClarityUi* ui, const char* part);
void clarity_ui_set_visible(ClarityUi* ui, const char* part, bool value);
bool clarity_ui_enabled(const ClarityUi* ui, const char* part);
void clarity_ui_set_enabled(ClarityUi* ui, const char* part, bool value);
/// The caption a handler set, or `design`.
const char* clarity_ui_caption(const ClarityUi* ui, const char* part, const char* design);
void clarity_ui_set_caption(ClarityUi* ui, const char* part, const ClarityText* value);
/// Starts an empty drawing on a canvas, before its draw handler runs.
void clarity_ui_begin_drawing(ClarityUi* ui, const char* part);
/// Adds a shape to a canvas's drawing: {"op": "line", ...}; it takes `shape`.
void clarity_ui_draw(ClarityUi* ui, const char* part, cJSON* shape);
/// Where a canvas was last clicked, across (`canvas.x`) and down (`canvas.y`); 0 before any click.
int64_t clarity_ui_x(const ClarityUi* ui, const char* part);
int64_t clarity_ui_y(const ClarityUi* ui, const char* part);
/// Where a drag on a canvas started, across (`canvas.fromX`) and down (`canvas.fromY`); 0 without one.
int64_t clarity_ui_from_x(const ClarityUi* ui, const char* part);
int64_t clarity_ui_from_y(const ClarityUi* ui, const char* part);
/// The name of the link a person bent on a canvas (`canvas.bent`, "" without one); its bend points are clarity_ui_via in code.h.
const char* clarity_ui_bent(const ClarityUi* ui, const char* part);
/// The message a handler set on an input, or "".
const char* clarity_ui_error(const ClarityUi* ui, const char* part);
void clarity_ui_set_error(ClarityUi* ui, const char* part, const ClarityText* value);
/// Whether an input is read-only: as a handler set it, or `design`.
bool clarity_ui_readonly(const ClarityUi* ui, const char* part, bool design);
void clarity_ui_set_readonly(ClarityUi* ui, const char* part, bool value);
/// Whether a foldable part is folded: as the person or a handler left it, or `design`.
bool clarity_ui_folded(const ClarityUi* ui, const char* part, bool design);
void clarity_ui_set_folded(ClarityUi* ui, const char* part, bool value);
/// Whether a collapsible sidebar is folded to its button: as left, or `design`.
bool clarity_ui_collapsed(const ClarityUi* ui, const char* part, bool design);
void clarity_ui_set_collapsed(ClarityUi* ui, const char* part, bool value);
/// A canvas's grid (`canvas.grid`) and whether code snaps to it (`canvas.snap`), as its line says until a handler sets them.
int64_t clarity_ui_grid(const ClarityUi* ui, const char* part, int64_t design);
void clarity_ui_set_grid(ClarityUi* ui, const char* part, int64_t value);
bool clarity_ui_snap(const ClarityUi* ui, const char* part, bool design);
void clarity_ui_set_snap(ClarityUi* ui, const char* part, bool value);
/// What a canvas's clear() fills it with (`canvas.background`), as its line says until a handler sets it.
const char* clarity_ui_background(const ClarityUi* ui, const char* part, const char* design);
void clarity_ui_set_background(ClarityUi* ui, const char* part, const char* value);
/// A canvas's grid dots (`canvas.dotSize`, `canvas.dotColor`), as its line says until a handler sets them.
int64_t clarity_ui_dot_size(const ClarityUi* ui, const char* part, int64_t design);
void clarity_ui_set_dot_size(ClarityUi* ui, const char* part, int64_t value);
const char* clarity_ui_dot_color(const ClarityUi* ui, const char* part, const char* design);
void clarity_ui_set_dot_color(ClarityUi* ui, const char* part, const char* value);
/// Whether a maximizable sidebar fills the window.
bool clarity_ui_maximized(const ClarityUi* ui, const char* part);
void clarity_ui_set_maximized(ClarityUi* ui, const char* part, bool value);
/// A property of a custom part as code set it, or NULL when it was not set.
const cJSON* clarity_ui_part_value(const ClarityUi* ui, const char* part, const char* name);
/// Sets a property of a custom part; it takes `value`.
void clarity_ui_set_part_value(ClarityUi* ui, const char* part, const char* name, cJSON* value);
/// Gives a custom part a hear with its values (an array, which it takes), sent with the reply.
void clarity_ui_hear(ClarityUi* ui, const char* part, const char* hear, cJSON* values);
/// The open tab of a ribbon, counted from 1.
int64_t clarity_ui_tab(const ClarityUi* ui, const char* part);
void clarity_ui_set_tab(ClarityUi* ui, const char* part, int64_t value);
/// The step a wizard is on, counted from 1.
int64_t clarity_ui_step(const ClarityUi* ui, const char* part);
void clarity_ui_set_step(ClarityUi* ui, const char* part, int64_t value);
/// The open section of an accordion, counted from 1; 0 when none is open.
int64_t clarity_ui_open_section(const ClarityUi* ui, const char* part);
void clarity_ui_set_open_section(ClarityUi* ui, const char* part, int64_t value);
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
/// `copy text`: the screen puts it, as it shows, on the clipboard when the handler is done; the last copy counts.
void clarity_ui_copy(ClarityUi* ui, const ClarityText* text);
/// The text value `data` shows `text`, translated and formatted by the screen.
void clarity_ui_set_shown(ClarityUi* ui, const char* data, const ClarityText* text);
/// Asks the screen to open `window` with its data filled by name, as the values are now.
void clarity_ui_open(ClarityUi* ui, const char* window, cJSON* with);
/// Asks the screen to close the window.
void clarity_ui_close(ClarityUi* ui);
/// Closes the dialog with `value` (which it takes), for the handler that asked for it.
void clarity_ui_close_with(ClarityUi* ui, cJSON* value);
/// Asks `question` when the handler has finished; the answer runs `handler`
/// with `values` (an array, which it takes) and, for a value, the answer.
void clarity_ui_ask(ClarityUi* ui, const ClarityText* question, cJSON* form, const char* handler, cJSON* values);

/// The reply for `window`, with its data after the handler.
ClarityReply* clarity_ui_reply(const ClarityUi* ui, const char* window, cJSON* data);

/// The state of `window` when it opens: its loaded data with `with` filled in by name.
ClarityWindowState* clarity_opened(const char* window, cJSON* data, const cJSON* with);

cJSON* clarity_window_state_json(const ClarityWindowState* state);
bool clarity_window_state_from_json(const cJSON* in, ClarityWindowState* out);
cJSON* clarity_reply_json(const ClarityReply* reply);
/// `{"id": id, "text": text}` added to `options`: a record a reference field can point at.
/// Adds a record a reference can point at; a search dropdown's fields go in
/// its "fields" (U1c).
cJSON* clarity_add_option(cJSON* options, int64_t id, const char* text);
/// An object of `count` pairs of name and value, such as the `with` of `open`.
cJSON* clarity_json_object(int count, ...);
