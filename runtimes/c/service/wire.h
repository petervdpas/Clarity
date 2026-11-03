// Clarity.UI's commands (clarity_ui.h), answered by the service: the same
// on the desktop, where the library asks in the program's process
// (desktop/desktop.h), and on the web, where the library runs in the
// browser and asks over HTTP (serve.h). Nothing here needs the library.

#pragma once

#include "service/service.h"

/// What the program adds to the service's answers.
typedef struct {
    /// The facts of the fields and types the parts read beside the values,
    /// as JSON (clarity_runtime::fields::Facts); NULL for none.
    const char* facts;
    /// The names of the secret settings, as a JSON array; NULL for none.
    const char* secrets;
} ClarityWire;

/// Answers one of Clarity.UI's commands with its arguments: `{"value": ...}`
/// or `{"error": ...}` in the scratch arena, or NULL for "none" (the
/// library's default). `args` is an object the answer may change.
cJSON* clarity_wire_answer(ClarityService* service, const ClarityWire* wire, const char* command, cJSON* args);
