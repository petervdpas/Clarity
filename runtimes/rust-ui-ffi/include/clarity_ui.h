/*
 * clarity_ui.h: the C interface of Clarity.UI (DESIGN 7.9, "One Clarity.UI
 * for all five languages"). The library (libclarity_ui.so, clarity_ui.dll,
 * libclarity_ui.dylib) draws, lays out, takes the person's input and runs
 * the desktop host. The program keeps its window model in its own language
 * and answers the library through callbacks.
 *
 * Conventions
 * - Every function returns a ClarityStatus (0 is CLARITY_OK) unless it says
 *   otherwise. A failing call leaves a message for this thread, read with
 *   clarity_last_error. No panic crosses the interface.
 * - Text given to the library is UTF-8 as a pointer and a length in bytes; it
 *   need not end with NUL, and the library copies what it keeps. A NULL
 *   pointer with length 0 is the empty text where text may be empty, or
 *   "none" where the function says so.
 * - Text the library gives back is a NUL-terminated UTF-8 string the caller
 *   frees with clarity_string_free.
 * - A callback answers through a ClarityAnswer: it calls clarity_answer_set
 *   with its bytes (the library copies them). A callback that sets nothing
 *   answers "none". The answer is only valid during the callback.
 * - JSON is used where a value is a tree (a window's value, a part's state,
 *   a reply). The shapes are the ones of the web server's wire (DESIGN 7.3)
 *   and of clarity_runtime::ui (WindowState, Reply, PartState, Text).
 * - Handles (ClarityProgram, ClarityFiles, ClarityScreen, ClarityDriven) are
 *   opaque. A ClarityScreen and a ClarityDriven are used on the thread that
 *   made them. A ClarityProgram's callback may be called from any thread.
 */

#ifndef CLARITY_UI_H
#define CLARITY_UI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define CLARITY_API __declspec(dllimport)
#else
#define CLARITY_API
#endif

/* The version of this interface; it grows when a function or a field is added. */
#define CLARITY_UI_INTERFACE 1

/* ------------------------------------------------------------------------
 * Status and errors
 * ------------------------------------------------------------------------ */

typedef int32_t ClarityStatus;

#define CLARITY_OK 0
/* The call failed; clarity_last_error says why. */
#define CLARITY_FAILED 1
/* The library stopped on a fault of its own; clarity_last_error says where. */
#define CLARITY_PANIC 2
/* An argument is wrong: a NULL handle, text that is no UTF-8, JSON that does not read. */
#define CLARITY_INVALID 3
/* What was asked for is not there: a part of that name, a window. */
#define CLARITY_NOT_FOUND 4

/* The message of the last call on this thread that did not return CLARITY_OK;
 * NULL when there is none. The caller frees it with clarity_string_free. */
CLARITY_API char *clarity_last_error(void);

/* Frees a string the library gave. NULL is let go. */
CLARITY_API void clarity_string_free(char *text);

/* ------------------------------------------------------------------------
 * The library
 * ------------------------------------------------------------------------ */

/* The library's version (the clarity-ui crate's), a static string: never freed. */
CLARITY_API const char *clarity_ui_version(void);

/* The interface version the library has: CLARITY_UI_INTERFACE when header and library match. */
CLARITY_API uint32_t clarity_ui_interface(void);

/* ------------------------------------------------------------------------
 * Answers of callbacks
 * ------------------------------------------------------------------------ */

typedef struct ClarityAnswer ClarityAnswer;

/* Sets what a callback answers: text (UTF-8, often JSON) or bytes. The
 * library copies them. Setting it again replaces it. */
CLARITY_API ClarityStatus clarity_answer_set(ClarityAnswer *answer, const uint8_t *data, size_t len);

/* ------------------------------------------------------------------------
 * The program's files
 * ------------------------------------------------------------------------ */

typedef struct ClarityFiles ClarityFiles;

/* Empty files: no windows, Clarity's own themes, no model. */
CLARITY_API ClarityStatus clarity_files_new(ClarityFiles **out);
CLARITY_API void clarity_files_free(ClarityFiles *files);

/* A window's markup (.cxaml) by the window's name ("Main"). */
CLARITY_API ClarityStatus clarity_files_window(ClarityFiles *files, const char *name, size_t name_len, const char *markup, size_t markup_len);
/* An app's frame markup by the app's name. */
CLARITY_API ClarityStatus clarity_files_app(ClarityFiles *files, const char *name, size_t name_len, const char *markup, size_t markup_len);
/* A resolved theme by its name and mode ("shop.light") as JSON. */
CLARITY_API ClarityStatus clarity_files_theme(ClarityFiles *files, const char *name, size_t name_len, const char *json, size_t json_len);
/* The model the program is made with, as JSON: its translations, its language, its themes. */
CLARITY_API ClarityStatus clarity_files_model(ClarityFiles *files, const char *json, size_t json_len);
/* A file of the project's resources folder by its name ("logo.png", "fonts/x.ttf"). */
CLARITY_API ClarityStatus clarity_files_resource(ClarityFiles *files, const char *name, size_t name_len, const uint8_t *bytes, size_t bytes_len);
/* The title of the desktop window. */
CLARITY_API ClarityStatus clarity_files_title(ClarityFiles *files, const char *title, size_t title_len);
/* The size the main window opens at, in the screen's units; 0 for the host's own. */
CLARITY_API ClarityStatus clarity_files_size(ClarityFiles *files, double width, double height);

/* ------------------------------------------------------------------------
 * Language and theme (the person's choices)
 * ------------------------------------------------------------------------ */

/* Starts the choices from the files' model: the languages and themes it
 * offers, and what the person chose in an earlier run, kept in `folder`
 * (NULL keeps choices for this run only). The desktop and a driven run do
 * this themselves. */
CLARITY_API ClarityStatus clarity_choices_start(const ClarityFiles *files, const char *folder, size_t folder_len);

/* The language shown, by its tag ("nl"): translations of the model are used from now on. */
CLARITY_API ClarityStatus clarity_language_choose(const char *tag, size_t tag_len);

/* The theme shown, by its name as the model declares it ("NightOwl"). */
CLARITY_API ClarityStatus clarity_theme_choose(const char *name, size_t name_len);

/* What is chosen and offered, as JSON:
 * {"language":"nl","languages":[{"name","label","current"}],"theme":"NightOwl"|null,"themes":[...]} */
CLARITY_API ClarityStatus clarity_choices_json(char **out);

/* A translated text of the model by its key; `english` when the model has none. */
CLARITY_API ClarityStatus clarity_translate(const char *key, size_t key_len, const char *english, size_t english_len, char **out);

/* ------------------------------------------------------------------------
 * The program (clarity_runtime::program::Program, as callbacks)
 * ------------------------------------------------------------------------
 *
 * The library asks the program by command, with its arguments as a JSON
 * object, and the program answers {"value": ...} or {"error": Text}. The
 * commands and their arguments are those of the web server's wire (DESIGN
 * 7.3, clarity-gen's served()), plus the desktop's own:
 *
 *   started        {}                      -> {"main": "Main", "unconnected": null | {"handler","window","problem": Text}}
 *   start_app      {}                      -> Reply
 *   connect        {}                      -> null
 *   open_window    {window, with}          -> Reply
 *   close_window   {state}                 -> Reply
 *   return_to      {state}                 -> Reply
 *   dispatch_event {state, handler}        -> Reply
 *   edit_event     {state, handler, part, scope, ends, readonly, path, editable}  -> Reply
 *   undo_edit      {state, scope, back}    -> Reply
 *   reorder_event  {state, part, scope, list, by, parent, move, handler, readonly} -> Reply
 *   item_event     {state, part, scope, path, item, values, typed, handler, readonly} -> Reply
 *   range_event    {state, part, scope, path, min, max, step, whole, key | at | typed, handler, readonly} -> Reply
 *   canvas_event   {state, part, mode, list, at, path, units, grid, snap, action, scope, handler, readonly} -> Reply
 *   step_event     {state, part, button, steps, next, finish} -> Reply
 *   view_event     {state, part, action}   -> Reply
 *   answer_event   {state, handler, values} -> Reply
 *   ref_options    {entity}                -> [RefOption]
 *   field_values   {entity, field}         -> [string]
 *   find_options   {kind, entity, field, show, typed} -> value
 *   field_bytes    {entity, id, field}     -> base64 string | null
 *   store_file     {store, path}           -> base64 string
 *   read_settings  {}                      -> object
 *   keep_settings  {settings}              -> null
 *   set_language   {language}              -> null
 *   watches        {}                      -> bool
 *   start_watch    {id}  (on the handler's thread, just before it runs)
 *   watch_progress {id}                    -> [done, total] | null
 *   stop_watch     {id}
 *   end_watch      {id}
 *   facts          {}                      -> the program's field facts (fields::Facts JSON) | null
 *   engine_open    {name}                  -> number | null (no engine of that name); error: why it did not start
 *   engine_hears   {id}                    -> {"pointer": bool, "keys": bool}
 *   engine_frame   {id, ms, input: [{"pointer":{kind,x,y}} | {"key":{kind,name}} | {"hear":{name,values}}]}
 *                                          -> {"shapes": [...], "tells": [{"tell","values"}]}
 *   engine_close   {id}
 *
 * A Reply is clarity_runtime::ui::Reply as JSON: {"state", "messages",
 * "open": [{"window", "with", "new_window"?}], "close", "asks"?, "result"?,
 * "hears"?, "copy"?, "shown"?, "step"?, "refused"?, "quit"?, "keys"?}. Besides
 * the window's new state it says what the handler asked of the screen:
 *   open[].new_window  true: the window opens in a desktop window of its own
 *                      (`open ... in new window`); else in place.
 *   quit               true: the program ends; every window closes, windows
 *                      of their own too (the built-in `quit` run by code).
 *   keys               true: Clarity's keys window opens (the built-in `keys`
 *                      run by code).
 * Fields left out are false or empty.
 *
 * Long calls: a program that answers `watches` true has dispatch_event and
 * answer_event run on a worker thread, each between start_watch and
 * end_watch on that same thread; watch_progress and stop_watch come from the
 * screen's thread while it runs, and a stop_watch may come before its
 * start_watch (it then holds once the call starts), and end_watch comes
 * once more from the screen's thread as the call's reply is taken.
 *
 * A command the program does not know may be answered with nothing: the
 * library then takes the trait's default (no watch, no engine, no facts).
 * The callback is called from the screen's thread and from the threads that
 * run handlers, so it must be safe to call from several threads at once.
 */

typedef struct ClarityProgramCallbacks {
    void *user;
    void (*call)(void *user, const char *command, size_t command_len, const char *args, size_t args_len, ClarityAnswer *answer);
    /* The library holds the program no more: `user` may go. May be NULL. */
    void (*release)(void *user);
} ClarityProgramCallbacks;

typedef struct ClarityProgram ClarityProgram;

CLARITY_API ClarityStatus clarity_program_new(const ClarityProgramCallbacks *callbacks, ClarityProgram **out);
/* Lets go of the caller's hold; release runs when the library's last hold goes too. */
CLARITY_API void clarity_program_free(ClarityProgram *program);

/* ------------------------------------------------------------------------
 * Songs
 * ------------------------------------------------------------------------ */

/* Clarity.Audio's table (clarity_audio.h). */
struct ClarityAudioApi;

/* Songs (mp3, flac, ogg) of the sound parts play through Clarity.Audio from
 * now on, on this thread: call it before clarity_desktop_run with
 * clarity_audio_api(). NULL sends them to the speakers as every sound again.
 * The table must live as long as the program. */
CLARITY_API ClarityStatus clarity_ui_use_audio(const struct ClarityAudioApi *api);

/* ------------------------------------------------------------------------
 * The desktop
 * ------------------------------------------------------------------------ */

/* Runs the program's windows in desktop windows until they closed (the
 * desktop host of clarity-ui), on the calling thread (the main thread on
 * macOS). With the environment variable CLARITY_HEADLESS set it is driven
 * from standard input instead (DESIGN 1.6). `exit_code` gets 0 or 1. */
CLARITY_API ClarityStatus clarity_desktop_run(const ClarityProgram *program, const ClarityFiles *files, int32_t *exit_code);

/* Wakes the running desktop from any thread: every window reads its values
 * again and draws. Does nothing while no desktop runs. Never fails. */
CLARITY_API void clarity_desktop_wake(void);

/* ------------------------------------------------------------------------
 * A driven program (headless, DESIGN 1.6)
 * ------------------------------------------------------------------------ */

typedef struct ClarityDriven ClarityDriven;

/* Starts the program as the desktop does, on a screen without a window. */
CLARITY_API ClarityStatus clarity_driven_start(const ClarityProgram *program, const ClarityFiles *files, ClarityDriven **out);
/* Does one step (JSON, clarity_ui::headless: {"do":"click","part":"save"}) and answers it as JSON.
 * A step that could not be done returns CLARITY_FAILED and still gives its answer (with "error"); free it either way. */
CLARITY_API ClarityStatus clarity_driven_step(ClarityDriven *driven, const char *step, size_t step_len, char **answer);
CLARITY_API void clarity_driven_free(ClarityDriven *driven);

/* ------------------------------------------------------------------------
 * The part host (clarity_ui::parts::PartHost, as callbacks)
 * ------------------------------------------------------------------------
 *
 * What a window's parts read and change, answered from the program's own
 * window model. Every pointer but `user`, `read`, `write`, `state`,
 * `set_state`, `run`, `text` and `changes` may be NULL: the library then
 * takes PartHost's default. Text arguments are UTF-8 (pointer, length); a
 * NULL pointer is "none". JSON shapes:
 *   value      any JSON value
 *   state      PartState: {"visible","enabled","caption","error","readonly",...}
 *   call       {"event":"click","handler":"save","args":["7"]}
 *   part       {"kind":"dropdown","props":{...}} (the part's spec as its markup gives it)
 *   choices    [{"value":..., "label":"...", "cells":["..."] | null}]
 *   field      {"label","format","required","min","max","refers","parent","color","help"}
 *   formats    {"language","currency"}
 *   message    {"level":"info"|"warning"|"error","text": Text}
 *   hears      [{"part","hear","values"}]
 *   drawn      {"shapes":[...],"tells":[{"tell","values"}]} or {"error":"why"}
 * Booleans are int32_t: 1 true, 0 false; `pointed` and `acted` answer -1
 * for "the window has no command for it". A place is -1 for none.
 * These callbacks run on the screen's thread only.
 */

typedef struct ClarityPartHostCallbacks {
    void *user;
    void (*read)(void *user, const char *path, size_t path_len, ClarityAnswer *value);
    void (*rows)(void *user, const char *path, size_t path_len, ClarityAnswer *rows_json);
    void (*write)(void *user, const char *part, size_t part_len, const char *path, size_t path_len, const char *value, size_t value_len);
    void (*edited)(void *user, const char *part, size_t part_len);
    void (*state)(void *user, const char *name, size_t name_len, ClarityAnswer *state);
    void (*set_state)(void *user, const char *name, size_t name_len, const char *state, size_t state_len);
    int32_t (*run)(void *user, const char *part, size_t part_len, const char *call, size_t call_len);
    int32_t (*can_run)(void *user, const char *part, size_t part_len, const char *call, size_t call_len);
    void (*text)(void *user, const char *written, size_t written_len, ClarityAnswer *text);
    void (*type_of)(void *user, const char *path, size_t path_len, ClarityAnswer *type_name);
    void (*choices)(void *user, const char *path, size_t path_len, ClarityAnswer *choices);
    void (*offered)(void *user, const char *part, size_t part_len, const char *path, size_t path_len, ClarityAnswer *choices);
    void (*field)(void *user, const char *path, size_t path_len, ClarityAnswer *field);
    void (*formats)(void *user, ClarityAnswer *formats);
    void (*open_address)(void *user, const char *address, size_t address_len);
    void (*resource)(void *user, const char *name, size_t name_len, ClarityAnswer *bytes);
    void (*icon_path)(void *user, const char *name, size_t name_len, ClarityAnswer *svg_path);
    uint64_t (*changes)(void *user);
    void (*fetch)(void *user, const char *bind, size_t bind_len, const char *store_path, size_t store_path_len, ClarityAnswer *bytes);
    void (*say)(void *user, const char *message, size_t message_len);
    int32_t (*pointed)(void *user, const char *part, size_t part_len, const char *event, size_t event_len, const char *at, size_t at_len);
    int32_t (*acted)(void *user, const char *part, size_t part_len, const char *action, size_t action_len);
    void (*told)(void *user, const char *part, size_t part_len, const char *tell, size_t tell_len, const char *values, size_t values_len);
    void (*bound_of)(void *user, const char *part, size_t part_len, ClarityAnswer *path);
    void (*keys_of)(void *user, const char *command, size_t command_len, const char *designed, size_t designed_len, ClarityAnswer *keys_json);
    void (*hears)(void *user, const char *part, size_t part_len, ClarityAnswer *hears);
    int64_t (*selected)(void *user, const char *part, size_t part_len);
    void (*select)(void *user, const char *part, size_t part_len, int64_t place);
    void (*window_name)(void *user, ClarityAnswer *name);
    /* Engines beside the screen, by number (DESIGN 5.12). engine_start answers {"id": n} or {"error": "why"}; nothing: no engines. */
    void (*engine_start)(void *user, const char *name, size_t name_len, ClarityAnswer *started);
    /* Bit 1: it takes the pointer; bit 2: it takes keys. */
    int32_t (*engine_hears)(void *user, uint64_t id);
    void (*engine_frame)(void *user, uint64_t id, double ms, const char *input, size_t input_len);
    void (*engine_drawn)(void *user, uint64_t id, ClarityAnswer *drawn);
    void (*engine_stop)(void *user, uint64_t id);
    /* The library holds the host no more. May be NULL. */
    void (*release)(void *user);
} ClarityPartHostCallbacks;

/* ------------------------------------------------------------------------
 * A screen without a window (for tests and hosts of their own)
 * ------------------------------------------------------------------------ */

typedef struct ClarityScreen ClarityScreen;

/* Pointer buttons. */
#define CLARITY_BUTTON_LEFT 0
#define CLARITY_BUTTON_MIDDLE 1
#define CLARITY_BUTTON_RIGHT 2

/* Keys held, as bits. */
#define CLARITY_SHIFT 1
#define CLARITY_CONTROL 2
#define CLARITY_ALT 4
#define CLARITY_META 8

/* A window made from its markup (a whole <Window> element) with `host`
 * behind its parts, at 1280 by 800 in Clarity's light look. `app` is the
 * app's frame markup (NULL for none). The host's callbacks are copied;
 * release runs when the screen is freed. */
CLARITY_API ClarityStatus clarity_screen_new(const char *markup, size_t markup_len, const char *app, size_t app_len, const ClarityPartHostCallbacks *host, ClarityScreen **out);
CLARITY_API void clarity_screen_free(ClarityScreen *screen);

/* The look: a resolved theme as JSON over Clarity's own (NULL: Clarity's), light or dark. */
CLARITY_API ClarityStatus clarity_screen_look(ClarityScreen *screen, const char *theme, size_t theme_len, int32_t dark);
CLARITY_API ClarityStatus clarity_screen_resize(ClarityScreen *screen, double width, double height, float scale);
CLARITY_API ClarityStatus clarity_screen_pointer_move(ClarityScreen *screen, double x, double y);
CLARITY_API ClarityStatus clarity_screen_pointer_leave(ClarityScreen *screen);
CLARITY_API ClarityStatus clarity_screen_pointer_press(ClarityScreen *screen, double x, double y, int32_t button, uint32_t clicks, uint32_t modifiers);
CLARITY_API ClarityStatus clarity_screen_pointer_release(ClarityScreen *screen, double x, double y, int32_t button);
/* A press and release of the left button at one point. */
CLARITY_API ClarityStatus clarity_screen_click(ClarityScreen *screen, double x, double y);
CLARITY_API ClarityStatus clarity_screen_wheel(ClarityScreen *screen, double x, double y, double dx, double dy, uint32_t modifiers);
/* A key by its name ("Enter", "Tab", "Down", "A", "7", "F2"); `taken` gets 1 when something took it (may be NULL). */
CLARITY_API ClarityStatus clarity_screen_key(ClarityScreen *screen, const char *name, size_t name_len, uint32_t modifiers, int32_t *taken);
CLARITY_API ClarityStatus clarity_screen_key_release(ClarityScreen *screen, const char *name, size_t name_len, uint32_t modifiers);
CLARITY_API ClarityStatus clarity_screen_type(ClarityScreen *screen, const char *text, size_t text_len);
/* Lets `ms` milliseconds pass: timers wake, work off the screen's thread comes back. */
CLARITY_API ClarityStatus clarity_screen_settle(ClarityScreen *screen, double ms);
/* The host's values changed: the parts read them again and the screen draws anew. */
CLARITY_API ClarityStatus clarity_screen_changed(ClarityScreen *screen);
/* Draws the screen into `rgba` (8 bits per channel, premultiplied, rows of
 * `stride` bytes), `width` by `height` pixels, at the screen's scale. */
CLARITY_API ClarityStatus clarity_screen_draw(ClarityScreen *screen, uint8_t *rgba, int32_t width, int32_t height, size_t stride);
/* The place of the part named `name` in the screen's units: x, y, width, height. CLARITY_NOT_FOUND when none shows. */
CLARITY_API ClarityStatus clarity_screen_part_place(ClarityScreen *screen, const char *name, size_t name_len, double *x, double *y, double *width, double *height);
/* The focus to the part named `name` (its first element that takes the focus). */
CLARITY_API ClarityStatus clarity_screen_focus(ClarityScreen *screen, const char *name, size_t name_len);
/* The name of the part with the focus (inside it counts); NULL in `out` when none has it. */
CLARITY_API ClarityStatus clarity_screen_focused(ClarityScreen *screen, char **out);
/* How many overlays (menus, dropdowns, dialogs) are open. */
CLARITY_API ClarityStatus clarity_screen_overlays(ClarityScreen *screen, int32_t *count);
/* The parts that show with their places, as JSON: [{"name","x","y","width","height"}]. */
CLARITY_API ClarityStatus clarity_screen_parts_json(ClarityScreen *screen, char **out);
/* Does one step of the driver, as clarity_driven_step does (click a part by
 * name, keys, typed text, "read" what shows), and answers it as JSON. A step
 * that could not be done fails (CLARITY_NOT_FOUND for a part that does not
 * show) and gives no answer. */
CLARITY_API ClarityStatus clarity_screen_step(ClarityScreen *screen, const char *step, size_t step_len, char **answer);

/* ------------------------------------------------------------------------
 * Custom parts (DESIGN 5.19, 7.9)
 * ------------------------------------------------------------------------
 *
 * A program's own kind of part, as a table of callbacks. `make` gives a new
 * part's own pointer; every other callback gets that pointer as `part`.
 * Every pointer but `make`, `set`, `hear` and `draw` may be NULL (it does
 * nothing). A ClarityCustomCx is valid only during the callback it is given
 * to; through it the part says what changed and tells its window.
 * Values are JSON. Runs on the screen's thread.
 */

typedef struct ClarityCustomCx ClarityCustomCx;
typedef struct ClarityCanvas ClarityCanvas;

typedef struct ClarityPointer {
    double x;           /* in the screen's units */
    double y;
    uint32_t modifiers; /* CLARITY_SHIFT and the others */
    uint32_t clicks;
} ClarityPointer;

typedef struct ClarityCustomCallbacks {
    void *user;
    void *(*make)(void *user);
    void (*set)(void *part, const char *name, size_t name_len, const char *value, size_t value_len, ClarityCustomCx *cx);
    void (*hear)(void *part, const char *hear, size_t hear_len, const char *values, size_t values_len, ClarityCustomCx *cx);
    void (*draw)(void *part, ClarityCanvas *canvas, double x, double y, double width, double height);
    void (*start)(void *part, ClarityCustomCx *cx);
    int32_t (*focusable)(void *part);
    void (*pointer_pressed)(void *part, const ClarityPointer *at, ClarityCustomCx *cx);
    void (*pointer_moved)(void *part, const ClarityPointer *at, ClarityCustomCx *cx);
    void (*pointer_released)(void *part, const ClarityPointer *at, ClarityCustomCx *cx);
    void (*click)(void *part, ClarityCustomCx *cx);
    int32_t (*wheel)(void *part, double dx, double dy, uint32_t modifiers, ClarityCustomCx *cx);
    /* A key by its name, as clarity_screen_key names them. */
    int32_t (*key)(void *part, const char *name, size_t name_len, uint32_t modifiers, ClarityCustomCx *cx);
    void (*text)(void *part, const char *text, size_t text_len, ClarityCustomCx *cx);
    void (*focus_changed)(void *part, int32_t focused, ClarityCustomCx *cx);
    void (*wake)(void *part, ClarityCustomCx *cx);
    void (*stop)(void *part);
    void (*tooltip_at)(void *part, double x, double y, ClarityAnswer *tip);
    /* The part is gone: its pointer may go. */
    void (*destroy)(void *part);
} ClarityCustomCallbacks;

/* Adds a custom part of `kind` (its declaration's name, "Gauge"). The table is copied. */
CLARITY_API ClarityStatus clarity_custom_register(const char *kind, size_t kind_len, const ClarityCustomCallbacks *callbacks);

/* What a custom part does through its cx. */
CLARITY_API ClarityStatus clarity_custom_bounds(const ClarityCustomCx *cx, double *x, double *y, double *width, double *height);
CLARITY_API ClarityStatus clarity_custom_enabled(const ClarityCustomCx *cx, int32_t *enabled);
CLARITY_API ClarityStatus clarity_custom_invalidate(ClarityCustomCx *cx);
/* It wakes (its `wake` runs) after `ms` milliseconds, once. */
CLARITY_API ClarityStatus clarity_custom_wake_in(ClarityCustomCx *cx, double ms);
CLARITY_API ClarityStatus clarity_custom_animate(ClarityCustomCx *cx, int32_t on);
CLARITY_API ClarityStatus clarity_custom_focus(ClarityCustomCx *cx);
/* The person changed its value (JSON): what it binds takes it and `on change` runs. */
CLARITY_API ClarityStatus clarity_custom_change(ClarityCustomCx *cx, const char *value, size_t value_len);
/* It tells `tell` with its values (a JSON array): the handler wired to it runs. */
CLARITY_API ClarityStatus clarity_custom_tell(ClarityCustomCx *cx, const char *tell, size_t tell_len, const char *values, size_t values_len);

/* Drawing on the canvas a custom part's `draw` gets. Colors are 0xAARRGGBB. */
CLARITY_API ClarityStatus clarity_canvas_fill(ClarityCanvas *canvas, double x, double y, double width, double height, uint32_t color, double radius);
CLARITY_API ClarityStatus clarity_canvas_stroke(ClarityCanvas *canvas, double x, double y, double width, double height, uint32_t color, double line_width, double radius);
CLARITY_API ClarityStatus clarity_canvas_line(ClarityCanvas *canvas, double x1, double y1, double x2, double y2, uint32_t color, double line_width);
CLARITY_API ClarityStatus clarity_canvas_oval(ClarityCanvas *canvas, double x, double y, double width, double height, uint32_t color, int32_t filled, double line_width);
/* An SVG path's data ("M0 0 L10 10 Z"), filled or stroked. */
CLARITY_API ClarityStatus clarity_canvas_path(ClarityCanvas *canvas, const char *svg, size_t svg_len, uint32_t color, int32_t filled, double line_width);
/* Text in a box: align 0 left, 1 center, 2 right; size 0 is the look's text size. */
CLARITY_API ClarityStatus clarity_canvas_text(ClarityCanvas *canvas, const char *text, size_t text_len, double x, double y, double width, double height, uint32_t color, double size, int32_t bold, int32_t align);
/* How wide text is at a size (0: the look's text size). */
CLARITY_API ClarityStatus clarity_canvas_measure(ClarityCanvas *canvas, const char *text, size_t text_len, double size, int32_t bold, double *width);
/* RGBA pixels (unpremultiplied, rows of `stride` bytes) drawn into a box. */
CLARITY_API ClarityStatus clarity_canvas_pixels(ClarityCanvas *canvas, const uint8_t *rgba, uint32_t width, uint32_t height, size_t stride, double x, double y, double box_width, double box_height);
/* Clips what follows to a box until clarity_canvas_restore. */
CLARITY_API ClarityStatus clarity_canvas_clip(ClarityCanvas *canvas, double x, double y, double width, double height);
CLARITY_API ClarityStatus clarity_canvas_restore(ClarityCanvas *canvas);
/* A color of the look by its role ("accent", "text", "border"), as 0xAARRGGBB. */
CLARITY_API ClarityStatus clarity_canvas_look_color(ClarityCanvas *canvas, const char *role, size_t role_len, uint32_t *color);

#ifdef __cplusplus
}
#endif

#endif /* CLARITY_UI_H */
