// The service of C applications: the commands of the web wire (DESIGN 7.3),
// answered by the program's windows. Clarity.UI asks them on the desktop
// (desktop/desktop.h) and the shared web screen in a browser (serve.h); the
// commands and their JSON are every stack's.

#pragma once

#include "clarity/db.h"
#include "clarity/ui.h"
#include "clarity/watch.h"

/// What the service asks of the program's windows.
typedef struct {
    /// The state of `window` when it opens: its data loaded, with `with` filled in by name.
    bool (*open)(ClarityDb* db, const char* window, const cJSON* with, ClarityWindowState** out);
    /// The handler of a window's own event (open, close, return, start), or
    /// NULL; the app's start has no window.
    const char* (*handler)(const char* window, const char* event);
    /// Runs a handler on a window's state.
    bool (*dispatch)(ClarityDb* db, const ClarityWindowState* state, const char* handler, ClarityReply** out);
    /// The records a reference field of `entity` can point at, as an array of {id, text}.
    bool (*options)(ClarityDb* db, const char* entity, cJSON** out);
    /// The picture of image field `field` of record `id` of `entity`; `out->data` is NULL when it has none.
    bool (*picture)(ClarityDb* db, const char* entity, int64_t id, const char* field, ClarityBytes* out);
    /// Draws the canvases of the window `reply` shows anew, each by its draw
    /// handler (conformance/drawing.json). The service redraws after a window
    /// opens and after every handler.
    bool (*redraw)(ClarityDb* db, ClarityReply* reply);
    /// Runs the handler the answer to a question goes to (DESIGN 5.14),
    /// with the values written for it and the answer last.
    bool (*answer)(ClarityDb* db, const ClarityWindowState* state, const char* handler, const cJSON* values, ClarityReply** out);
    /// The values text field `field` of `entity` already has, which a typed
    /// dropdown offers (U1c), as an array of text.
    bool (*values)(ClarityDb* db, const char* entity, const char* field, cJSON** out);
} ClarityWindows;

typedef struct {
    /// The program's name; `title` the title of its window until a Clarity window gives its own.
    const char* name;
    const char* title;
    /// The window the application opens with.
    const char* main;
    /// The model: the windows, frame, data dictionary and translations, as JSON.
    const char* model;
    /// The web screen's page, with its script, styles and pictures in it,
    /// which the web target serves; NULL on the desktop.
    const char* html;
    ClarityDb* db;
    const ClarityWindows* windows;
    /// The size the main window's desktop window opens at; zero for the library's own.
    int width, height;
    /// Why the databases did not open (DESIGN 5.27), with `db` an empty one in
    /// memory until `connect` opens them; NULL when they opened. What the
    /// screen shows first then: the app's `on unconnected` handler, or else
    /// the window that keeps the connection; NULL for none.
    const ClarityError* unconnected;
    const char* unconnected_handler;
    const char* unconnected_window;
    bool (*connect)(ClarityDb** out);
} ClarityServiceOptions;

/// Answers the commands of the web wire.
typedef struct ClarityService ClarityService;

ClarityService* clarity_service_new(const ClarityServiceOptions* options);
void clarity_service_free(ClarityService* service);
/// Runs a command, with its arguments as the web wire has them. Answers
/// `{"value": ...}` or `{"error": ...}`, in the scratch arena.
cJSON* clarity_service_invoke(ClarityService* service, const char* command, const cJSON* args);
/// Opens a link in the person's browser; whether it could.
bool clarity_open_in_browser(const char* address);
/// How the service opens a link of a markdown part: clarity_open_in_browser
/// unless set otherwise, as tests do.
void clarity_service_on_link(ClarityService* service, bool (*open_link)(const char* address));
/// Where the service writes a line of an engine beside the screen: the
/// standard error, the application's log, unless set otherwise, as tests do.
void clarity_service_on_log(ClarityService* service, void (*log_line)(const char* line));
