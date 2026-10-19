// Engines (DESIGN 5.11): processing modules compiled once to WebAssembly,
// which every stack hosts alike (conformance/engines.json). An engine
// computes; it imports nothing, so it reaches no database, file or network.
// Each call runs on a fresh instance of the module, so an engine keeps no
// state from one call to the next. Hosted by wasm3; a generated application
// has this code only when it declares an engine.

#pragma once

#include <stddef.h>
#include <stdint.h>

#include "cjson/cJSON.h"
#include "clarity/core.h"

typedef struct ClarityEngine ClarityEngine;

/// Loads `bytes` as engine `name` with the functions it is declared with;
/// refuses (CLARITY_ENGINE) a module that is no engine, imports anything, or
/// lacks one of them. The engine lives until clarity_engine_free.
bool clarity_engine_load(const char* name, const uint8_t* bytes, size_t size, size_t function_count, const char* const* functions,
                         ClarityEngine** out);

/// Calls `function` with `args`, a JSON array it takes over: its value, the
/// engine's refusal in its own words (CLARITY_REFUSED), or what went wrong
/// running it (CLARITY_ENGINE).
bool clarity_engine_call(ClarityEngine* engine, const char* function, cJSON* args, cJSON** out);

void clarity_engine_free(ClarityEngine* engine);
