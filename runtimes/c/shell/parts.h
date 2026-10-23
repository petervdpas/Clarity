// Custom parts in the shell (DESIGN 6.5): the native parts of a program are
// in libclarity_parts beside it, which the shell loads when it is there and
// asks to place, set and tell its parts. A program without native parts has
// no such library, and nothing here does anything.

#pragma once

#include <stdbool.h>
#include <stdint.h>

/// What a part told: its window's number, the part, the tell and its values as JSON.
typedef void (*ClarityPartsTold)(uint64_t window, const char* part, const char* tell, const char* values, void* context);

/// Whether the program has native parts: libclarity_parts beside it.
bool clarity_shell_parts_present(void);

/// Loads and starts them; NULL, or why they cannot run. `told` is called on the parts' own thread.
const char* clarity_shell_parts_start(ClarityPartsTold told, void* context);

void clarity_shell_parts_place(uint64_t window, const char* part, const char* kind, uintptr_t parent, int x, int y, int width, int height, bool shown);
void clarity_shell_parts_set(uint64_t window, const char* part, const char* name, const char* value);
void clarity_shell_parts_hear(uint64_t window, const char* part, const char* hear, const char* values);
void clarity_shell_parts_close(uint64_t window);
void clarity_shell_parts_stop(void);

/// The native window of a webview's widget that parts draw in: on Linux the
/// X11 window of a GTK widget; 0 where there is none.
uintptr_t clarity_shell_parts_parent(void* widget);
