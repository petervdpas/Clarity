// The shell of C applications: a desktop window with a webview showing the
// web renderer every web stack ships, and the commands the renderer sends,
// answered by the program's windows. The commands and their JSON are the
// Tauri, Wails and C++ shells', so the same renderer serves every stack.

#pragma once

#include "clarity/db.h"
#include "clarity/ui.h"

/// What the shell asks of the program's windows.
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
    /// handler (conformance/drawing.json). The shell redraws after a window
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
    /// The windows, frame and data dictionary the renderer draws from, as JSON.
    const char* model;
    /// The renderer's page, with its script, styles and pictures in it.
    const char* html;
    ClarityDb* db;
    const ClarityWindows* windows;
    /// The size the main window's desktop window opens at; zero for the standard size.
    int width, height;
    /// Why the databases did not open (DESIGN 5.27), with `db` an empty one in
    /// memory until `connect` opens them; NULL when they opened. What the
    /// screen shows first then: the app's `on unconnected` handler, or else
    /// the window that keeps the connection; NULL for none.
    const ClarityError* unconnected;
    const char* unconnected_handler;
    const char* unconnected_window;
    bool (*connect)(ClarityDb** out);
} ClarityShellOptions;

/// The size a desktop window opens at: its window's, or the standard size
/// when `width` or `height` is not above zero.
void clarity_window_size(int* width, int* height);

/// A desktop window, as the commands of the renderer in it see it.
typedef struct ClarityDesktop ClarityDesktop;
struct ClarityDesktop {
    char name[32];
    void (*set_title)(ClarityDesktop* desktop, const char* title);
    void (*close)(ClarityDesktop* desktop);
    /// The native window custom parts draw in (DESIGN 6.5): on Linux the X11
    /// window of the webview; NULL or 0 where there is none.
    uintptr_t (*part_parent)(ClarityDesktop* desktop);
    /// Opens the system's folder picker over the window, for a folder part:
    /// the folder chosen, in the scratch arena, or NULL when none was.
    const char* (*choose_folder)(ClarityDesktop* desktop, const char* title);
};

/// Answers the renderer's commands.
typedef struct ClarityService ClarityService;

/// The desktop window an open window's custom parts are in, by the open
/// window's number; NULL when none of its parts was placed.
const char* clarity_service_parts_window(ClarityService* service, uint64_t instance);

ClarityService* clarity_service_new(const ClarityShellOptions* options);
void clarity_service_free(ClarityService* service);
/// Runs a command: the same commands, with the same arguments, as the
/// Tauri shell has. Answers `{"value": ...}` or `{"error": ...}`, in the scratch arena.
cJSON* clarity_service_invoke(ClarityService* service, const char* command, const cJSON* args, ClarityDesktop* caller);
/// Opens a link in the person's browser; whether it could.
bool clarity_open_in_browser(const char* address);
/// How the service opens a link of a markdown part: clarity_open_in_browser
/// unless set otherwise, as tests do.
void clarity_service_on_link(ClarityService* service, bool (*open_link)(const char* address));
/// Where the service writes a line of an engine beside the screen: the
/// standard error, the application's log, unless set otherwise, as tests do.
void clarity_service_on_log(ClarityService* service, void (*log_line)(const char* line));
/// The part host's side of the commands of custom parts (DESIGN 6.5), which
/// clarity_shell_run sets; a web server has none, and the commands do nothing.
typedef struct {
    void (*place)(uint64_t window, const char* part, const char* kind, uintptr_t parent, int x, int y, int width, int height, bool shown);
    void (*set)(uint64_t window, const char* part, const char* name, const char* value);
    void (*hear)(uint64_t window, const char* part, const char* hear, const char* values);
    void (*close)(uint64_t window);
} ClarityPartsHost;
void clarity_service_on_parts(ClarityService* service, ClarityPartsHost parts);
/// What the service asks of the desktop, set by clarity_shell_run: ending
/// the program, and opening a desktop window of its own, named `name`, for
/// the opener, at `width` x `height` (the window's size, or the standard one).
void clarity_service_on_desktop(ClarityService* service, void (*quit)(void* context),
                                void (*open_desktop)(void* context, ClarityDesktop* opener, const char* name, int width, int height), void* context);
/// A desktop window of its own closed: gives its opener's name, so it can
/// reload its data, or NULL. The name stays until the service is freed.
const char* clarity_service_closed(ClarityService* service, const char* name);
/// Whether the title bar's close of the main window goes to the screen:
/// while the databases are not open, closing it connects again (DESIGN 5.27).
bool clarity_service_closes_to_screen(const ClarityService* service);

/// Shows the application's window and runs until its last window closes.
int clarity_shell_run(const ClarityShellOptions* options);
