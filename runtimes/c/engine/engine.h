// Engines (DESIGN 5.11): processing modules compiled once to WebAssembly,
// which every stack hosts alike (conformance/engines.json). An engine
// computes; it imports only the services of Clarity it declares (`uses
// clock, random, log`), so it reaches no database, file or network.
// Each call runs on a fresh instance of the module, so an engine keeps no
// state from one call to the next, unless it is declared with `state per
// app` (one instance serves every call) or `state per window` (each open
// window has an instance of its own, dropped when the window closes). Every
// call runs within the limits of its engine:
// a call that runs too long is stopped, and an engine never gets more memory
// than its limit. Hosted by wasm3; a generated application
// has this code only when it declares an engine.

#pragma once

#include <stddef.h>
#include <stdint.h>

#include "cjson/cJSON.h"
#include "clarity/core.h"

typedef struct ClarityEngine ClarityEngine;

/// What one call of an engine may take: how long it may run, and how much
/// memory the engine may grow to.
typedef struct {
    uint32_t seconds;
    uint32_t megabytes;
} ClarityEngineLimits;

/// Whether an engine keeps an instance from call to call.
typedef enum {
    /// Every call runs on a fresh instance.
    CLARITY_ENGINE_NO_STATE,
    /// One instance serves every call of the application (`state per app`).
    CLARITY_ENGINE_PER_APP,
    /// Each open window has an instance of its own (`state per window`).
    CLARITY_ENGINE_PER_WINDOW,
} ClarityEngineState;

/// An engine as its declaration gives it: its functions, the services it
/// uses, the limits of a call, whether it keeps state and whether it has
/// `on open` (the engine's `clarity_open`, run on every new instance).
typedef struct {
    size_t function_count;
    const char* const* functions;
    size_t use_count;
    const char* const* uses;
    ClarityEngineLimits limits;
    ClarityEngineState state;
    bool opens;
} ClarityEngineDeclaration;

/// What the services give: the system's clock, a fresh seed for every call
/// and the standard error, unless a test fixes the clock (milliseconds since
/// 1970) or the seed, or keeps the lines of the log.
typedef struct {
    bool fixed_clock;
    int64_t clock;
    bool fixed_seed;
    uint64_t seed;
    /// Takes a line of the log, as "engine <Name>: <text>"; NULL for the standard error.
    void (*log)(const char* line, void* context);
    void* log_context;
    /// The language `language` gives in place of the one the screen picked; NULL for the texts as written.
    bool fixed_language;
    const char* language;
} ClarityEngineServices;

/// Loads `bytes` as engine `name` as it is declared; refuses
/// (CLARITY_ENGINE) a module that is no engine, imports anything but the
/// services it uses, or lacks one of its functions. The engine lives until
/// clarity_engine_free.
bool clarity_engine_load(const char* name, const uint8_t* bytes, size_t size, ClarityEngineDeclaration declaration, ClarityEngine** out);

/// Gives the engine `services` in place of the system's.
void clarity_engine_serve(ClarityEngine* engine, ClarityEngineServices services);

/// Calls `function` with `args`, a JSON array it takes over: its value, the
/// engine's refusal in its own words (CLARITY_REFUSED), or what went wrong
/// running it (CLARITY_ENGINE).
bool clarity_engine_call(ClarityEngine* engine, const char* function, cJSON* args, cJSON** out);

/// Calls `function` in open window `window`: for an engine with state per
/// window, on that window's own instance.
bool clarity_engine_call_in(ClarityEngine* engine, uint64_t window, const char* function, cJSON* args, cJSON** out);

/// Drops the instance window `window` had, when it closes.
void clarity_engine_close_window(ClarityEngine* engine, uint64_t window);

void clarity_engine_free(ClarityEngine* engine);
