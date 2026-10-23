// No native windows: the tests, and the systems the part host does not run
// on yet. A child is a number of its own; in the tests that is all a part
// needs.

#include <stdio.h>

#include "platform.h"

#if defined(CLARITY_PARTS_TEST)
static const char* const problem_here = NULL;
#elif defined(_WIN32)
static const char* const problem_here = "custom parts drawn by a native library run on Linux for now, not on Windows";
#elif defined(__APPLE__)
static const char* const problem_here = "custom parts drawn by a native library run on Linux for now, not on macOS";
#else
static const char* const problem_here = "custom parts drawn by a native library need X11";
#endif

static uintptr_t next_child = 1;

bool clarity_platform_open(char* problem, size_t room) {
    if (problem_here) {
        snprintf(problem, room, "%s", problem_here);
        return false;
    }
    return true;
}

uintptr_t clarity_platform_child(uintptr_t parent, int x, int y, int width, int height) {
    (void)parent, (void)x, (void)y, (void)width, (void)height;
    return next_child++;
}

void clarity_platform_move(uintptr_t child, int x, int y, int width, int height) { (void)child, (void)x, (void)y, (void)width, (void)height; }
void clarity_platform_show(uintptr_t child, bool shown) { (void)child, (void)shown; }
void clarity_platform_destroy(uintptr_t child) { (void)child; }
void clarity_platform_flush(void) {}
void clarity_platform_close(void) {}
