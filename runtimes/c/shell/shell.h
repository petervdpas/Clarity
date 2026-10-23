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
};

/// Answers the renderer's commands.
typedef struct ClarityService ClarityService;

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
/// What the service asks of the desktop, set by clarity_shell_run: ending
/// the program, and opening a desktop window of its own, named `name`, for
/// the opener, at `width` x `height` (the window's size, or the standard one).
void clarity_service_on_desktop(ClarityService* service, void (*quit)(void* context),
                                void (*open_desktop)(void* context, ClarityDesktop* opener, const char* name, int width, int height), void* context);
/// A desktop window of its own closed: gives its opener's name, so it can
/// reload its data, or NULL. The name stays until the service is freed.
const char* clarity_service_closed(ClarityService* service, const char* name);

/// Shows the application's window and runs until its last window closes.
int clarity_shell_run(const ClarityShellOptions* options);
