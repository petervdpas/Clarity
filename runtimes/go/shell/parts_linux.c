// Custom parts in the Go shell (DESIGN 6.5), the part of it in C: loading
// libclarity_parts beside the program and calling it, and finding the X11
// window of the webview that parts draw in. Linux only, through cgo.

#include "parts_linux.h"

#include <dlfcn.h>
#include <gdk/gdkx.h>
#include <gtk/gtk.h>
#include <stdio.h>
#include <string.h>

typedef bool (*StartFn)(void (*)(uint64_t, const char*, const char*, const char*, void*), void*);
typedef const char* (*ProblemFn)(void);
typedef void (*StopFn)(void);
typedef void (*PlaceFn)(uint64_t, const char*, const char*, uintptr_t, int, int, int, int, bool);
typedef void (*SetFn)(uint64_t, const char*, const char*, const char*);
typedef void (*CloseFn)(uint64_t);

static StartFn start_fn;
static ProblemFn problem_fn;
static StopFn stop_fn;
static PlaceFn place_fn;
static SetFn set_fn;
static SetFn hear_fn;
static CloseFn close_fn;
static bool started;

static void on_told(uint64_t window, const char* part, const char* tell, const char* values, void* context) {
    (void)context;
    clarityGoTold(window, (char*)part, (char*)tell, (char*)values);
}

const char* clarity_go_parts_start(const char* path) {
    static char problem[512];
    if (started) return NULL;
    void* handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!handle) {
        snprintf(problem, sizeof problem, "the program's custom parts cannot load: %s", dlerror());
        return problem;
    }
    *(void**)&start_fn = dlsym(handle, "clarity_parts_start");
    *(void**)&problem_fn = dlsym(handle, "clarity_parts_problem");
    *(void**)&stop_fn = dlsym(handle, "clarity_parts_stop");
    *(void**)&place_fn = dlsym(handle, "clarity_parts_place");
    *(void**)&set_fn = dlsym(handle, "clarity_parts_set");
    *(void**)&hear_fn = dlsym(handle, "clarity_parts_hear");
    *(void**)&close_fn = dlsym(handle, "clarity_parts_close");
    if (!start_fn || !problem_fn || !stop_fn || !place_fn || !set_fn || !hear_fn || !close_fn) return "the program's custom parts library is not Clarity's";
    if (!start_fn(on_told, NULL)) {
        snprintf(problem, sizeof problem, "%s", problem_fn());
        return problem;
    }
    started = true;
    return NULL;
}

void clarity_go_parts_place(uint64_t window, const char* part, const char* kind, uintptr_t parent, int x, int y, int width, int height, bool shown) {
    if (started) place_fn(window, part, kind, parent, x, y, width, height, shown);
}

void clarity_go_parts_set(uint64_t window, const char* part, const char* name, const char* value) {
    if (started) set_fn(window, part, name, value);
}

void clarity_go_parts_hear(uint64_t window, const char* part, const char* hear, const char* values) {
    if (started) hear_fn(window, part, hear, values);
}

void clarity_go_parts_close(uint64_t window) {
    if (started) close_fn(window);
}

void clarity_go_parts_stop(void) {
    if (started) stop_fn();
    started = false;
}

/// The WebKitWebView among a widget and its children.
static GtkWidget* web_view_in(GtkWidget* widget) {
    if (strcmp(G_OBJECT_TYPE_NAME(widget), "WebKitWebView") == 0) return widget;
    if (!GTK_IS_CONTAINER(widget)) return NULL;
    GList* children = gtk_container_get_children(GTK_CONTAINER(widget));
    GtkWidget* found = NULL;
    for (GList* c = children; c && !found; c = c->next) found = web_view_in(GTK_WIDGET(c->data));
    g_list_free(children);
    return found;
}

uintptr_t clarity_go_parts_parent(void* window) {
    if (!window) return 0;
    GtkWidget* view = web_view_in(GTK_WIDGET(window));
    GdkWindow* native = view ? gtk_widget_get_window(view) : gtk_widget_get_window(GTK_WIDGET(window));
    if (!native || !GDK_IS_X11_WINDOW(native)) return 0;
    gdk_window_ensure_native(native);
    return (uintptr_t)GDK_WINDOW_XID(native);
}
