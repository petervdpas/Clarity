// The desktop of C applications (DESIGN 7.9): Clarity.UI, the one library
// every language draws with, shows the windows, and the program answers it
// through the service (service/service.h). This is the thin C layer between
// them: the program's files go in, the library's commands are answered, and
// the program's custom parts are kinds of part of the library.

#pragma once

#include <stdbool.h>
#include <stddef.h>

#include "clarity_ui.h"
// The library's statuses CLARITY_INVALID and CLARITY_NOT_FOUND are kinds of
// error of the runtime too: here the runtime's names count, and a status of
// the library is read as CLARITY_OK or not.
#undef CLARITY_INVALID
#undef CLARITY_NOT_FOUND
#include "service/carried.h"
#include "service/service.h"
#include "service/watches.h"
#include "service/wire.h"

typedef struct {
    /// What the service answers from: the program's windows, its databases,
    /// its model, its title and size.
    ClarityServiceOptions service;
    const ClarityCarriedFiles* files;
    /// The facts of the fields and types the parts read beside the values,
    /// as JSON (clarity_runtime::fields::Facts); NULL for none.
    const char* facts;
    /// The names of the secret settings, as a JSON array; NULL for none.
    const char* secrets;
    /// Whether an engine says how far a long call is: the library then runs
    /// handlers on its worker thread, and Stop stops them (DESIGN 5.12).
    bool watches;
    /// Registers the program's custom parts with the library (clarity_custom_register); NULL for none.
    void (*register_parts)(void);
    /// Answers the library's commands of engines beside the screen (engine_open,
    /// engine_hears, engine_frame, engine_close; DESIGN 5.12) as {"value"} or
    /// {"error"}, NULL for none (clarity_screen_answer); NULL when the program has none.
    cJSON* (*screen_engines)(const char* command, const cJSON* args);
} ClarityDesktopOptions;

/// The program as the library asks it.
typedef struct ClarityDesktopProgram ClarityDesktopProgram;

/// The program over a new service; NULL when the library refused it
/// (clarity_last_error says why). `options` must outlast it.
ClarityDesktopProgram* clarity_desktop_program_new(const ClarityDesktopOptions* options);
/// The handle the library takes.
const ClarityProgram* clarity_desktop_program(const ClarityDesktopProgram* program);
/// Lets go of it; the service goes once the library holds it no more.
void clarity_desktop_program_free(ClarityDesktopProgram* program);

/// Answers one of the library's commands (clarity_ui.h) with its arguments
/// as JSON: `{"value": ...}` or `{"error": ...}` as text the caller frees,
/// or NULL for "none" (the library's default). From any thread.
char* clarity_desktop_answer(ClarityDesktopProgram* program, const char* command, const char* args, size_t args_len);

/// A custom part's callback runs on the screen's thread, and what it tells
/// may reach the program before it returns: between these two the layer
/// keeps the scratch arena of the thread, which the callback's values live
/// in, and empties it at a later command. The glue of src/parts.c calls them.
void clarity_desktop_part_begin(void);
void clarity_desktop_part_end(void);

/// The program's files as the library takes them; NULL when one was
/// refused (clarity_last_error says why). Freed with clarity_files_free.
ClarityFiles* clarity_desktop_files(const ClarityDesktopOptions* options);

/// Shows the program's main window until its last window closed (or, with
/// CLARITY_HEADLESS set, drives it from standard input): the exit status.
int clarity_desktop_start(const ClarityDesktopOptions* options);
