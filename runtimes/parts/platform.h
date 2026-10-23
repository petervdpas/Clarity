// The native windows of the part host, one file per kind of system:
// platform_x11.c on Linux, platform_none.c where parts cannot run yet and in
// the tests. Called only on the part host's thread.

#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/// Readies the system's windows; false with why in `problem` when it cannot.
bool clarity_platform_open(char* problem, size_t room);
/// A child window of `parent` at x, y of width by height, hidden.
uintptr_t clarity_platform_child(uintptr_t parent, int x, int y, int width, int height);
void clarity_platform_move(uintptr_t child, int x, int y, int width, int height);
void clarity_platform_show(uintptr_t child, bool shown);
void clarity_platform_destroy(uintptr_t child);
/// Sends what was asked of the windows on its way.
void clarity_platform_flush(void);
void clarity_platform_close(void);
