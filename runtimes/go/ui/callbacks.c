// The tables of callbacks package ui gives the library: each callback hands
// its call to the Go function exported for it (exports.go).

#include "callbacks.h"
#include "_cgo_export.h"

#if defined(_WIN32)
#include <windows.h>
uint64_t clarity_go_thread(void) { return (uint64_t)GetCurrentThreadId(); }
#else
#include <pthread.h>
uint64_t clarity_go_thread(void) { return (uint64_t)(uintptr_t)pthread_self(); }
#endif

// -- The program ----------------------------------------------------------------

static void program_call(void *user, const char *command, size_t command_len, const char *args, size_t args_len, ClarityAnswer *answer) {
    goProgramCall((uintptr_t)user, (char *)command, command_len, (char *)args, args_len, answer);
}

static void program_release(void *user) { goProgramRelease((uintptr_t)user); }

ClarityStatus clarity_go_program_new(uintptr_t handle, ClarityProgram **out) {
    ClarityProgramCallbacks callbacks = {(void *)handle, program_call, program_release};
    return clarity_program_new(&callbacks, out);
}

// -- Custom parts -----------------------------------------------------------------

static void *custom_make(void *user) { return (void *)goCustomMake((uintptr_t)user); }

static void custom_set(void *part, const char *name, size_t name_len, const char *value, size_t value_len, ClarityCustomCx *cx) {
    goCustomSet((uintptr_t)part, (char *)name, name_len, (char *)value, value_len, cx);
}

static void custom_hear(void *part, const char *hear, size_t hear_len, const char *values, size_t values_len, ClarityCustomCx *cx) {
    goCustomHear((uintptr_t)part, (char *)hear, hear_len, (char *)values, values_len, cx);
}

static void custom_draw(void *part, ClarityCanvas *canvas, double x, double y, double width, double height) {
    goCustomDraw((uintptr_t)part, canvas, x, y, width, height);
}

static void custom_start(void *part, ClarityCustomCx *cx) { goCustomStart((uintptr_t)part, cx); }

static int32_t custom_focusable(void *part) { return goCustomFocusable((uintptr_t)part); }

static void custom_pressed(void *part, const ClarityPointer *at, ClarityCustomCx *cx) {
    goCustomPointer((uintptr_t)part, 0, at->x, at->y, at->modifiers, at->clicks, cx);
}

static void custom_moved(void *part, const ClarityPointer *at, ClarityCustomCx *cx) {
    goCustomPointer((uintptr_t)part, 1, at->x, at->y, at->modifiers, at->clicks, cx);
}

static void custom_released(void *part, const ClarityPointer *at, ClarityCustomCx *cx) {
    goCustomPointer((uintptr_t)part, 2, at->x, at->y, at->modifiers, at->clicks, cx);
}

static void custom_click(void *part, ClarityCustomCx *cx) { goCustomClick((uintptr_t)part, cx); }

static int32_t custom_wheel(void *part, double dx, double dy, uint32_t modifiers, ClarityCustomCx *cx) {
    return goCustomWheel((uintptr_t)part, dx, dy, modifiers, cx);
}

static int32_t custom_key(void *part, const char *name, size_t name_len, uint32_t modifiers, ClarityCustomCx *cx) {
    return goCustomKey((uintptr_t)part, (char *)name, name_len, modifiers, cx);
}

static void custom_text(void *part, const char *text, size_t text_len, ClarityCustomCx *cx) {
    goCustomText((uintptr_t)part, (char *)text, text_len, cx);
}

static void custom_focus_changed(void *part, int32_t focused, ClarityCustomCx *cx) {
    goCustomFocusChanged((uintptr_t)part, focused, cx);
}

static void custom_wake(void *part, ClarityCustomCx *cx) { goCustomWake((uintptr_t)part, cx); }

static void custom_stop(void *part) { goCustomStop((uintptr_t)part); }

static void custom_tooltip(void *part, double x, double y, ClarityAnswer *tip) { goCustomTooltip((uintptr_t)part, x, y, tip); }

static void custom_destroy(void *part) { goCustomDestroy((uintptr_t)part); }

ClarityStatus clarity_go_custom_register(const char *kind, size_t kind_len, uintptr_t handle) {
    ClarityCustomCallbacks callbacks = {0};
    callbacks.user = (void *)handle;
    callbacks.make = custom_make;
    callbacks.set = custom_set;
    callbacks.hear = custom_hear;
    callbacks.draw = custom_draw;
    callbacks.start = custom_start;
    callbacks.focusable = custom_focusable;
    callbacks.pointer_pressed = custom_pressed;
    callbacks.pointer_moved = custom_moved;
    callbacks.pointer_released = custom_released;
    callbacks.click = custom_click;
    callbacks.wheel = custom_wheel;
    callbacks.key = custom_key;
    callbacks.text = custom_text;
    callbacks.focus_changed = custom_focus_changed;
    callbacks.wake = custom_wake;
    callbacks.stop = custom_stop;
    callbacks.tooltip_at = custom_tooltip;
    callbacks.destroy = custom_destroy;
    return clarity_custom_register(kind, kind_len, &callbacks);
}
