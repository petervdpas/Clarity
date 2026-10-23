// See parts_linux.c.

#pragma once

#include <stdbool.h>
#include <stdint.h>

// Implemented in Go (parts_linux.go): a part told.
extern void clarityGoTold(uint64_t window, char* part, char* tell, char* values);

const char* clarity_go_parts_start(const char* path);
void clarity_go_parts_place(uint64_t window, const char* part, const char* kind, uintptr_t parent, int x, int y, int width, int height, bool shown);
void clarity_go_parts_set(uint64_t window, const char* part, const char* name, const char* value);
void clarity_go_parts_hear(uint64_t window, const char* part, const char* hear, const char* values);
void clarity_go_parts_close(uint64_t window);
void clarity_go_parts_stop(void);
uintptr_t clarity_go_parts_parent(void* window);
