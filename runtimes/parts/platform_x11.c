// Native windows on Linux: X11 child windows, on a connection of the part
// host's own, so nothing is shared with the stack's toolkit across threads.
// A program with native parts runs under X11 (XWayland on a Wayland
// desktop), which its shell sees to before its toolkit starts.

#include <X11/Xlib.h>
#include <stdio.h>

#include "platform.h"

static Display* display;

bool clarity_platform_open(char* problem, size_t room) {
    display = XOpenDisplay(NULL);
    if (!display) {
        snprintf(problem, room, "custom parts need X11, and no X display could be opened");
        return false;
    }
    return true;
}

uintptr_t clarity_platform_child(uintptr_t parent, int x, int y, int width, int height) {
    Window child = XCreateSimpleWindow(display, (Window)parent, x, y, width > 0 ? width : 1, height > 0 ? height : 1, 0, 0, BlackPixel(display, DefaultScreen(display)));
    XFlush(display);
    return (uintptr_t)child;
}

void clarity_platform_move(uintptr_t child, int x, int y, int width, int height) {
    XMoveResizeWindow(display, (Window)child, x, y, width > 0 ? width : 1, height > 0 ? height : 1);
}

void clarity_platform_show(uintptr_t child, bool shown) {
    if (shown) {
        XMapRaised(display, (Window)child);
    } else {
        XUnmapWindow(display, (Window)child);
    }
}

void clarity_platform_destroy(uintptr_t child) {
    XDestroyWindow(display, (Window)child);
}

void clarity_platform_flush(void) {
    if (display) XFlush(display);
}

void clarity_platform_close(void) {
    if (display) XCloseDisplay(display);
    display = NULL;
}
