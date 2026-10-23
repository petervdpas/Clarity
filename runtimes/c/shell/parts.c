#include "shell/parts.h"

#include <limits.h>
#include <stdio.h>
#include <string.h>

#if defined(__linux__) || defined(__APPLE__)
#include <dlfcn.h>
#include <unistd.h>
#endif

#if defined(__linux__)
#include <gdk/gdkx.h>
#include <gtk/gtk.h>
#endif

typedef bool (*StartFn)(ClarityPartsTold, void*);
typedef const char* (*ProblemFn)(void);
typedef void (*StopFn)(void);
typedef void (*PlaceFn)(uint64_t, const char*, const char*, uintptr_t, int, int, int, int, bool);
typedef void (*SetFn)(uint64_t, const char*, const char*, const char*);
typedef void (*CloseFn)(uint64_t);

static struct {
    StartFn start;
    ProblemFn problem;
    StopFn stop;
    PlaceFn place;
    SetFn set;
    SetFn hear;
    CloseFn close;
} library;

static bool started;

/// libclarity_parts beside the program, in `path`; false where it cannot be found.
static bool beside_program(char* path, size_t room) {
#if defined(__linux__)
    ssize_t n = readlink("/proc/self/exe", path, room - 1);
    if (n <= 0) return false;
    path[n] = '\0';
    char* slash = strrchr(path, '/');
    if (!slash || (size_t)(slash - path) + 20 >= room) return false;
    strcpy(slash + 1, "libclarity_parts.so");
    return true;
#else
    (void)path, (void)room;
    return false;
#endif
}

bool clarity_shell_parts_present(void) {
#if defined(__linux__) || defined(__APPLE__)
    char path[PATH_MAX];
    return beside_program(path, sizeof path) && access(path, R_OK) == 0;
#else
    return false;
#endif
}

const char* clarity_shell_parts_start(ClarityPartsTold told, void* context) {
#if defined(__linux__) || defined(__APPLE__)
    static char problem[512];
    if (started) return NULL;
    char path[PATH_MAX];
    if (!beside_program(path, sizeof path)) return "the program's custom parts cannot be found";
    void* handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!handle) {
        snprintf(problem, sizeof problem, "the program's custom parts cannot load: %s", dlerror());
        return problem;
    }
    *(void**)&library.start = dlsym(handle, "clarity_parts_start");
    *(void**)&library.problem = dlsym(handle, "clarity_parts_problem");
    *(void**)&library.stop = dlsym(handle, "clarity_parts_stop");
    *(void**)&library.place = dlsym(handle, "clarity_parts_place");
    *(void**)&library.set = dlsym(handle, "clarity_parts_set");
    *(void**)&library.hear = dlsym(handle, "clarity_parts_hear");
    *(void**)&library.close = dlsym(handle, "clarity_parts_close");
    if (!library.start || !library.problem || !library.stop || !library.place || !library.set || !library.hear || !library.close) {
        return "the program's custom parts library is not Clarity's";
    }
    if (!library.start(told, context)) {
        snprintf(problem, sizeof problem, "%s", library.problem());
        return problem;
    }
    started = true;
    return NULL;
#else
    (void)told, (void)context;
    return "custom parts drawn by a native library run on Linux for now";
#endif
}

void clarity_shell_parts_place(uint64_t window, const char* part, const char* kind, uintptr_t parent, int x, int y, int width, int height, bool shown) {
    if (started) library.place(window, part, kind, parent, x, y, width, height, shown);
}

void clarity_shell_parts_set(uint64_t window, const char* part, const char* name, const char* value) {
    if (started) library.set(window, part, name, value);
}

void clarity_shell_parts_hear(uint64_t window, const char* part, const char* hear, const char* values) {
    if (started) library.hear(window, part, hear, values);
}

void clarity_shell_parts_close(uint64_t window) {
    if (started) library.close(window);
}

uintptr_t clarity_shell_parts_parent(void* widget) {
#if defined(__linux__)
    GdkWindow* window = widget ? gtk_widget_get_window(GTK_WIDGET(widget)) : NULL;
    if (!window || !GDK_IS_X11_WINDOW(window)) return 0;
    gdk_window_ensure_native(window);
    return (uintptr_t)GDK_WINDOW_XID(window);
#else
    (void)widget;
    return 0;
#endif
}

void clarity_shell_parts_stop(void) {
    if (started) library.stop();
    started = false;
}
