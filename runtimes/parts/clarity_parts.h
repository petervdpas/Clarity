// Clarity's part host (DESIGN 6.5): runs the custom parts outside
// libraries draw. It makes a native child window for each part where the
// stack says the part is, calls the library's frame about 60 times a
// second, and passes values, hears and tells between the stack and the
// libraries. Everything runs on a thread of its own, so a stack calls it
// from any thread, and a tell reaches the stack on that thread.
//
// The host of the C and C++ stacks, whose parts are C or C++: built with a
// project's generated glue and its parts into the shared library the
// program loads. The Rust and Go runtimes host their parts in their own
// language (clarity_runtime::parts, the Go runtime's parts package).

#pragma once

#include <stdbool.h>
#include <stdint.h>

#include "cJSON.h"

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define CLARITY_PARTS_API __declspec(dllexport)
#else
#define CLARITY_PARTS_API __attribute__((visibility("default")))
#endif

// -- What a stack calls ---------------------------------------------------

/// What a part told: its window (the open window's number), the part's name
/// in that window, the tell (`change` for its value) and the tell's values as
/// a JSON array. Called on the part host's thread.
typedef void (*ClarityPartsTold)(uint64_t window, const char* part, const char* tell, const char* values, void* context);

/// Starts the part host. Gives false, with `clarity_parts_problem` saying
/// why, when parts cannot run here.
CLARITY_PARTS_API bool clarity_parts_start(ClarityPartsTold told, void* context);

/// Why the part host could not start, or "".
CLARITY_PARTS_API const char* clarity_parts_problem(void);

/// Ends every part and the part host's thread.
CLARITY_PARTS_API void clarity_parts_stop(void);

/// Where a part of kind `kind` (its declared name) is: inside the native
/// window `parent` (on Linux an X11 window), at `x`, `y`, `width` by
/// `height` in that window's pixels, shown or hidden. The first call makes
/// the part; later calls move, resize, show or hide it.
CLARITY_PARTS_API void clarity_parts_place(uint64_t window, const char* part, const char* kind, uintptr_t parent, int x, int y, int width, int height, bool shown);

/// Sets a property of a part (`value` for what it binds, `enabled`), the value
/// as JSON. A value set before the part is placed is given to it once made.
CLARITY_PARTS_API void clarity_parts_set(uint64_t window, const char* part, const char* name, const char* value);

/// Gives a part a hear with its values as a JSON array.
CLARITY_PARTS_API void clarity_parts_hear(uint64_t window, const char* part, const char* hear, const char* values);

/// Ends every part of a window that closed.
CLARITY_PARTS_API void clarity_parts_close(uint64_t window);

/// Waits until everything asked before has been done. For tests.
CLARITY_PARTS_API void clarity_parts_wait(void);

// -- What the generated glue gives ----------------------------------------

/// The host side of one part, which its library tells through.
typedef struct ClarityPartHost ClarityPartHost;

/// A kind of part: its declared name and the library's functions, through
/// the glue that turns them into plain calls.
typedef struct {
    const char* name;
    void* (*create)(void* parent, int width, int height, ClarityPartHost* host);
    void (*resize)(void* part, int width, int height);
    void (*frame)(void* part);
    void (*destroy)(void* part);
    /// A property by name with its value.
    void (*set)(void* part, const char* name, const cJSON* value);
    /// A hear by name with its values, an array.
    void (*hear)(void* part, const char* hear, const cJSON* values);
} ClarityPartKind;

/// Every kind of part of the project, ended by one without a name. The glue defines it.
extern const ClarityPartKind clarity_part_kinds[];

/// A part tells: `tell` with its values, an array the host takes.
void clarity_parts_tell(ClarityPartHost* host, const char* tell, cJSON* values);

#ifdef __cplusplus
}
#endif
