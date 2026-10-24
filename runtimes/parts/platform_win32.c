// Native windows on Windows: Win32 child windows of the shell's own window,
// made on the part host's thread, which owns them and answers their
// messages. A window that does not clip its siblings paints over whatever
// shares its place, and the webview of every shell is such a window, so the
// parent's children are made to clip before a part is put among them.

#include <windows.h>

#include <stdio.h>

#include "platform.h"

/// The window class of every part's window.
static const wchar_t* const CLASS = L"ClarityPart";

static HINSTANCE instance;

static LRESULT CALLBACK window_proc(HWND window, UINT message, WPARAM w, LPARAM l) {
    // The part draws the window itself, so its messages take their usual course.
    return DefWindowProcW(window, message, w, l);
}

/// Makes a window of the parent's own children clip its siblings, so it
/// leaves the part's place alone however the two are ordered.
static BOOL CALLBACK clip_siblings(HWND window, LPARAM parent) {
    if ((LPARAM)GetParent(window) != parent) return TRUE;
    LONG style = GetWindowLongW(window, GWL_STYLE);
    if (style && !(style & WS_CLIPSIBLINGS)) {
        SetWindowLongW(window, GWL_STYLE, style | WS_CLIPSIBLINGS);
        SetWindowPos(window, NULL, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);
    }
    return TRUE;
}

bool clarity_platform_open(char* problem, size_t room) {
    instance = GetModuleHandleW(NULL);
    if (!instance) {
        snprintf(problem, room, "custom parts need the program's own module, which Windows did not give");
        return false;
    }
    WNDCLASSW class_of_parts = {0};
    class_of_parts.lpfnWndProc = window_proc;
    class_of_parts.hInstance = instance;
    // IDC_ARROW is an integer resource, which the wide call takes as it is.
    class_of_parts.hCursor = LoadCursorW(NULL, (LPCWSTR)IDC_ARROW);
    class_of_parts.hbrBackground = (HBRUSH)GetStockObject(BLACK_BRUSH);
    class_of_parts.lpszClassName = CLASS;
    // The class is registered once; a second window finds it there already.
    RegisterClassW(&class_of_parts);
    return true;
}

uintptr_t clarity_platform_child(uintptr_t parent, int x, int y, int width, int height) {
    HWND child = CreateWindowExW(0, CLASS, NULL, WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN, x, y, width > 0 ? width : 1, height > 0 ? height : 1,
                                 (HWND)parent, NULL, instance, NULL);
    if (!child) return 0;
    EnumChildWindows((HWND)parent, clip_siblings, (LPARAM)parent);
    // Above the webview, which fills the same parent.
    SetWindowPos(child, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    return (uintptr_t)child;
}

void clarity_platform_move(uintptr_t child, int x, int y, int width, int height) {
    SetWindowPos((HWND)child, HWND_TOP, x, y, width > 0 ? width : 1, height > 0 ? height : 1, SWP_NOACTIVATE);
}

void clarity_platform_show(uintptr_t child, bool shown) {
    // SW_SHOWNA: shown without taking the keyboard from the window.
    ShowWindow((HWND)child, shown ? SW_SHOWNA : SW_HIDE);
}

void clarity_platform_destroy(uintptr_t child) {
    DestroyWindow((HWND)child);
}

void clarity_platform_flush(void) {
    // The messages of the part windows, which belong to this thread. The
    // host asks for this between frames, so they are answered about sixty
    // times a second and the shell is never kept waiting.
    MSG message;
    while (PeekMessageW(&message, NULL, 0, 0, PM_REMOVE)) {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
}

void clarity_platform_close(void) {}
