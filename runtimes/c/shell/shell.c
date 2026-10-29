// The desktop shell of C applications: a webview per window, on the main
// loop of the first, whose renderer sends its commands to the service
// (service.c). Custom parts draw in native child windows (parts.c).

#include "shell/shell.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "shell/parts.h"
#include "webview/webview.h"

#if defined(__linux__)
#include <gtk/gtk.h>
#endif

#if defined(__APPLE__)
#include <objc/message.h>
#include <objc/runtime.h>
#endif

#if defined(_WIN32)
#define COBJMACROS
#include <windows.h>
#include <shobjidl.h>
#endif

// The desktop: a webview per window, all on the main loop of the first.

typedef struct Window {
    ClarityDesktop desktop;
    webview_t view;
    ClarityService* service;
    struct Window* next;
} Window;

typedef struct {
    ClarityService* service;
    const ClarityShellOptions* options;
    Window* windows;
} Desktop;

static void set_title(ClarityDesktop* desktop, const char* title) { webview_set_title(((Window*)desktop)->view, title); }

static void close_window(ClarityDesktop* desktop) {
#if defined(__linux__)
    gtk_window_close(GTK_WINDOW(webview_get_window(((Window*)desktop)->view)));
#else
    (void)desktop;
#endif
}

// The system's folder picker over a window, for a folder part: the folder
// chosen, or NULL when the person cancelled.
#if defined(__linux__)
static const char* choose_folder(ClarityDesktop* desktop, const char* title) {
    GtkWindow* parent = GTK_WINDOW(webview_get_window(((Window*)desktop)->view));
    GtkWidget* dialog = gtk_file_chooser_dialog_new(title, parent, GTK_FILE_CHOOSER_ACTION_SELECT_FOLDER, "_Cancel", GTK_RESPONSE_CANCEL, "_Select", GTK_RESPONSE_ACCEPT, NULL);
    const char* chosen = NULL;
    if (gtk_dialog_run(GTK_DIALOG(dialog)) == GTK_RESPONSE_ACCEPT) {
        char* folder = gtk_file_chooser_get_filename(GTK_FILE_CHOOSER(dialog));
        if (folder) chosen = clarity_format("%s", folder);
        g_free(folder);
    }
    gtk_widget_destroy(dialog);
    return chosen;
}
#elif defined(_WIN32)
static const char* choose_folder(ClarityDesktop* desktop, const char* title) {
    IFileOpenDialog* dialog = NULL;
    if (FAILED(CoCreateInstance(&CLSID_FileOpenDialog, NULL, CLSCTX_INPROC_SERVER, &IID_IFileOpenDialog, (void**)&dialog))) return NULL;
    FILEOPENDIALOGOPTIONS options = 0;
    IFileOpenDialog_GetOptions(dialog, &options);
    IFileOpenDialog_SetOptions(dialog, options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM);
    wchar_t wide[512];
    if (MultiByteToWideChar(CP_UTF8, 0, title, -1, wide, 512) > 0) IFileOpenDialog_SetTitle(dialog, wide);
    const char* chosen = NULL;
    if (SUCCEEDED(IFileOpenDialog_Show(dialog, (HWND)webview_get_window(((Window*)desktop)->view)))) {
        IShellItem* item = NULL;
        PWSTR path = NULL;
        if (SUCCEEDED(IFileOpenDialog_GetResult(dialog, &item)) && SUCCEEDED(IShellItem_GetDisplayName(item, SIGDN_FILESYSPATH, &path))) {
            char utf8[4096];
            if (WideCharToMultiByte(CP_UTF8, 0, path, -1, utf8, sizeof utf8, NULL, NULL) > 0) chosen = clarity_format("%s", utf8);
            CoTaskMemFree(path);
        }
        if (item) IShellItem_Release(item);
    }
    IFileOpenDialog_Release(dialog);
    return chosen;
}
#elif defined(__APPLE__)
static const char* choose_folder(ClarityDesktop* desktop, const char* title) {
    (void)desktop;
    id panel = ((id (*)(Class, SEL))objc_msgSend)(objc_getClass("NSOpenPanel"), sel_registerName("openPanel"));
    ((void (*)(id, SEL, BOOL))objc_msgSend)(panel, sel_registerName("setCanChooseDirectories:"), YES);
    ((void (*)(id, SEL, BOOL))objc_msgSend)(panel, sel_registerName("setCanChooseFiles:"), NO);
    ((void (*)(id, SEL, BOOL))objc_msgSend)(panel, sel_registerName("setAllowsMultipleSelection:"), NO);
    ((void (*)(id, SEL, BOOL))objc_msgSend)(panel, sel_registerName("setCanCreateDirectories:"), YES);
    id message = ((id (*)(Class, SEL, const char*))objc_msgSend)(objc_getClass("NSString"), sel_registerName("stringWithUTF8String:"), title);
    ((void (*)(id, SEL, id))objc_msgSend)(panel, sel_registerName("setMessage:"), message);
    // NSModalResponseOK.
    if (((long (*)(id, SEL))objc_msgSend)(panel, sel_registerName("runModal")) != 1) return NULL;
    id url = ((id (*)(id, SEL))objc_msgSend)(panel, sel_registerName("URL"));
    const char* path = ((const char* (*)(id, SEL))objc_msgSend)(url, sel_registerName("fileSystemRepresentation"));
    return path ? clarity_format("%s", path) : NULL;
}
#endif

/// The answer to a call without a command's name.
static cJSON* unnamed(void) {
    cJSON* answer = cJSON_CreateObject();
    cJSON_AddStringToObject(answer, "error", "a command needs its name");
    return answer;
}

// The renderer calls clarityInvoke(command, args); it answers like the other shells.
static void on_invoke(const char* id, const char* request, void* arg) {
    Window* w = arg;
    cJSON* call = cJSON_Parse(request);
    const cJSON* command = cJSON_GetArrayItem(call, 0);
    const cJSON* args = cJSON_GetArrayItem(call, 1);
    cJSON* empty = cJSON_CreateObject();
    cJSON* answer = cJSON_IsString(command) ? clarity_service_invoke(w->service, command->valuestring, cJSON_IsObject(args) ? args : empty, &w->desktop)
                                            : unnamed();
    webview_return(w->view, id, 0, cJSON_PrintUnformatted(answer));
    // The command is answered: its values are done with.
    clarity_scratch_reset();
}

#if defined(__linux__) || defined(_WIN32)
// The native window of the webview, which custom parts draw in: its X11
// window on Linux, the window itself on Windows, where the part's window is
// a child beside the webview's.
static uintptr_t part_parent(ClarityDesktop* desktop) {
#if defined(_WIN32)
    return clarity_shell_parts_parent(webview_get_native_handle(((Window*)desktop)->view, WEBVIEW_NATIVE_HANDLE_KIND_UI_WINDOW));
#else
    return clarity_shell_parts_parent(webview_get_native_handle(((Window*)desktop)->view, WEBVIEW_NATIVE_HANDLE_KIND_UI_WIDGET));
#endif
}
#endif

static Window* open_window(Desktop* desktop, const char* name, int width, int height) {
    Window* w = calloc(1, sizeof *w);
    if (!w) return NULL;
    snprintf(w->desktop.name, sizeof w->desktop.name, "%s", name);
    w->desktop.set_title = set_title;
    w->desktop.close = close_window;
#if defined(__linux__) || defined(_WIN32) || defined(__APPLE__)
    w->desktop.choose_folder = choose_folder;
#endif
#if defined(__linux__) || defined(_WIN32)
    w->desktop.part_parent = part_parent;
#endif
    w->service = desktop->service;
    w->view = webview_create(0, NULL);
    webview_set_title(w->view, desktop->options->title);
    webview_set_size(w->view, width, height, WEBVIEW_HINT_NONE);
    webview_bind(w->view, "clarityInvoke", on_invoke, w);
    webview_set_html(w->view, desktop->options->html);
    w->next = desktop->windows;
    desktop->windows = w;
    return w;
}

static Window* main_window(Desktop* desktop) {
    Window* w = desktop->windows;
    while (w && w->next) w = w->next;
    return w;
}

static void quit(void* context) { webview_terminate(main_window(context)->view); }

#if defined(__linux__)
typedef struct {
    Desktop* desktop;
    char name[32];
} Closing;

// When a window of its own closes, its opener is told, so it can reload its data.
static void on_destroy(GtkWidget* widget, gpointer data) {
    (void)widget;
    Closing* c = data;
    const char* opener = clarity_service_closed(c->desktop->service, c->name);
    for (Window* w = c->desktop->windows; opener && w; w = w->next)
        if (strcmp(w->desktop.name, opener) == 0) webview_eval(w->view, "window.dispatchEvent(new Event('clarity-return'))");
    free(c);
}
#endif

static void open_desktop(void* context, ClarityDesktop* opener, const char* name, int width, int height) {
    (void)opener;
    Desktop* desktop = context;
    Window* w = open_window(desktop, name, width, height);
#if defined(__linux__)
    Closing* c = calloc(1, sizeof *c);
    if (w && c) {
        c->desktop = desktop;
        snprintf(c->name, sizeof c->name, "%s", name);
        g_signal_connect(G_OBJECT(webview_get_window(w->view)), "destroy", G_CALLBACK(on_destroy), c);
    }
#else
    (void)w;
#endif
}

/// A tell, carried from the part host's thread to the window's: the
/// renderer of the part's window hears it.
typedef struct {
    Desktop* desktop;
    uint64_t window;
    char* js;
} Telling;

static void tell_on_window(webview_t view, void* arg) {
    (void)view;
    Telling* t = arg;
    const char* name = clarity_service_parts_window(t->desktop->service, t->window);
    for (Window* w = t->desktop->windows; name && w; w = w->next) {
        if (strcmp(w->desktop.name, name) == 0) webview_eval(w->view, t->js);
    }
    free(t->js);
    free(t);
}

/// `text` as a JSON string, in plain memory: the part host's thread does not
/// use the scratch arena of the commands.
static char* json_string(const char* text) {
    size_t room = strlen(text) * 6 + 3;
    char* out = malloc(room);
    if (!out) return NULL;
    char* at = out;
    *at++ = '"';
    for (const unsigned char* c = (const unsigned char*)text; *c; ++c) {
        if (*c == '"' || *c == '\\') {
            *at++ = '\\';
            *at++ = (char)*c;
        } else if (*c < 0x20) {
            at += sprintf(at, "\\u%04x", *c);
        } else {
            *at++ = (char)*c;
        }
    }
    *at++ = '"';
    *at = '\0';
    return out;
}

static void on_told(uint64_t window, const char* part, const char* tell, const char* values, void* context) {
    Desktop* desktop = context;
    Telling* t = calloc(1, sizeof *t);
    char* quoted_part = json_string(part);
    char* quoted_tell = json_string(tell);
    size_t room = (quoted_part ? strlen(quoted_part) : 0) + (quoted_tell ? strlen(quoted_tell) : 0) + strlen(values) + 96;
    if (t && quoted_part && quoted_tell && (t->js = malloc(room))) {
        snprintf(t->js, room, "window.clarityTold && window.clarityTold(%llu, %s, %s, %s)", (unsigned long long)window, quoted_part, quoted_tell, values);
        t->desktop = desktop;
        t->window = window;
        webview_dispatch(main_window(desktop)->view, tell_on_window, t);
    } else {
        free(t);
    }
    free(quoted_part);
    free(quoted_tell);
}

// While the databases are not open, the title bar's close of the main window
// goes to the screen, which connects again (DESIGN 5.27): each system's own
// close is held back and the screen is asked instead.
static const char* const CLOSE_REQUESTED = "window.clarityCloseRequested && window.clarityCloseRequested()";
static Desktop* closing_desktop;

/// Whether the close of the main window goes to the screen; when it does, it is asked.
static bool close_to_screen(void) {
    if (!closing_desktop || !clarity_service_closes_to_screen(closing_desktop->service)) return false;
    webview_eval(main_window(closing_desktop)->view, CLOSE_REQUESTED);
    return true;
}

#if defined(__linux__)
static gboolean on_delete(GtkWidget* widget, GdkEvent* event, gpointer data) {
    (void)widget;
    (void)event;
    (void)data;
    return close_to_screen() ? TRUE : FALSE;
}
#elif defined(_WIN32)
static WNDPROC window_proc;
static LRESULT CALLBACK on_message(HWND hwnd, UINT message, WPARAM wp, LPARAM lp) {
    if (message == WM_CLOSE && close_to_screen()) return 0;
    return CallWindowProcW(window_proc, hwnd, message, wp, lp);
}
#elif defined(__APPLE__)
static id main_ns_window;
static BOOL should_close(id self, SEL selector, id sender) {
    (void)self;
    (void)selector;
    return sender == main_ns_window && close_to_screen() ? NO : YES;
}
#endif

static void hold_back_close(Desktop* desktop, Window* first) {
    closing_desktop = desktop;
#if defined(__linux__)
    g_signal_connect(G_OBJECT(webview_get_window(first->view)), "delete-event", G_CALLBACK(on_delete), NULL);
#elif defined(_WIN32)
    HWND hwnd = (HWND)webview_get_window(first->view);
    window_proc = (WNDPROC)SetWindowLongPtrW(hwnd, GWLP_WNDPROC, (LONG_PTR)on_message);
#elif defined(__APPLE__)
    main_ns_window = (id)webview_get_window(first->view);
    id delegate = ((id (*)(id, SEL))objc_msgSend)(main_ns_window, sel_registerName("delegate"));
    if (delegate) class_addMethod(object_getClass(delegate), sel_registerName("windowShouldClose:"), (IMP)should_close, "c@:@");
#else
    (void)first;
#endif
}

int clarity_shell_run(const ClarityShellOptions* options) {
#if defined(__linux__)
    // WebKitGTK 2.52 paints parts of a new window black through its DMABUF
    // renderer; it must be off before the web process starts.
    setenv("WEBKIT_DISABLE_DMABUF_RENDERER", "1", 0);
    // Custom parts draw in X11 child windows: under Wayland the toolkit runs on XWayland.
    if (clarity_shell_parts_present()) setenv("GDK_BACKEND", "x11", 0);
#endif
    Desktop desktop = {clarity_service_new(options), options, NULL};
    if (!desktop.service) return 1;
    clarity_service_on_desktop(desktop.service, quit, open_desktop, &desktop);
    clarity_service_on_parts(desktop.service, (ClarityPartsHost){clarity_shell_parts_place, clarity_shell_parts_set, clarity_shell_parts_hear, clarity_shell_parts_close});
    int width = options->width, height = options->height;
    clarity_window_size(&width, &height);
    Window* first = open_window(&desktop, "main", width, height);
    if (!first) return 1;
    hold_back_close(&desktop, first);
    if (clarity_shell_parts_present()) {
        const char* problem = clarity_shell_parts_start(on_told, &desktop);
        if (problem) fprintf(stderr, "%s\n", problem);
    }
    webview_run(first->view);
    closing_desktop = NULL;
    clarity_shell_parts_stop();
    for (Window* w = desktop.windows; w;) {
        Window* next = w->next;
        webview_destroy(w->view);
        free(w);
        w = next;
    }
    clarity_service_free(desktop.service);
    return 0;
}
