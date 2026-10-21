#include "engine/engine.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "clarity/text.h"
#include "clarity/ui.h"
#include "wasm3/m3_env.h"
#include "wasm3/wasm3.h"

/// How many services an engine may use.
#define SERVICE_COUNT 4

struct ClarityEngine {
    char* name;
    uint8_t* bytes;
    uint32_t size;
    IM3Environment environment;
    bool uses[SERVICE_COUNT];
    ClarityEngineLimits limits;
    ClarityEngineState state;
    bool opens;
    ClarityEngineServices services;
    /// The instances an engine with state keeps between calls, by window
    /// (one, at 0, for state per app).
    struct Kept {
        uint64_t window;
        IM3Runtime runtime;
    }* kept;
    size_t kept_count;
};

/// Takes the instance kept for `window` out of the engine; NULL when there is none.
static IM3Runtime take_kept(ClarityEngine* engine, uint64_t window) {
    for (size_t i = 0; i < engine->kept_count; ++i) {
        if (engine->kept[i].window != window) continue;
        IM3Runtime runtime = engine->kept[i].runtime;
        engine->kept[i] = engine->kept[--engine->kept_count];
        return runtime;
    }
    return NULL;
}

static void keep(ClarityEngine* engine, uint64_t window, IM3Runtime runtime) {
    engine->kept = realloc(engine->kept, sizeof *engine->kept * (engine->kept_count + 1));
    engine->kept[engine->kept_count++] = (struct Kept){window, runtime};
}

#if defined(_MSC_VER) && !defined(__clang__)
#define THREAD_LOCAL __declspec(thread)
#else
#define THREAD_LOCAL _Thread_local
#endif

/// Why a call stopped when it ran past its time limit.
static const char LATE[] = "clarity: the call ran past its time limit";

/// When the call running on this thread must end, in milliseconds; 0 when none runs.
static THREAD_LOCAL int64_t deadline;
/// Yields since the clock was last read: reading it at every turn would slow loops down.
static THREAD_LOCAL uint32_t turns;

static int64_t now_ms(void) {
    struct timespec now;
    timespec_get(&now, TIME_UTC);
    return (int64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000;
}

/// The engine whose call runs on this thread, and the state of its random sequence.
static THREAD_LOCAL const ClarityEngine* serving;
static THREAD_LOCAL uint64_t sequence;

/// The next number of splitmix64, the random sequence every host gives alike.
static int64_t splitmix(uint64_t* state) {
    *state += 0x9E3779B97F4A7C15u;
    uint64_t z = *state;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9u;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBu;
    return (int64_t)(z ^ (z >> 31));
}

/// A fresh seed for a call: the time, a count and an address, mixed.
static uint64_t fresh_seed(void) {
    static THREAD_LOCAL uint64_t count;
    struct timespec now;
    timespec_get(&now, TIME_UTC);
    uint64_t state = ((uint64_t)now.tv_sec * 1000000000u + (uint64_t)now.tv_nsec) ^ (++count << 32) ^ (uint64_t)(uintptr_t)&count;
    return (uint64_t)splitmix(&state);
}

static m3ApiRawFunction(clock_service) {
    (void)_ctx;
    (void)_mem;
    (void)runtime;
    m3ApiReturnType(int64_t);
    if (serving->services.fixed_clock) m3ApiReturn(serving->services.clock);
    struct timespec now;
    timespec_get(&now, TIME_UTC);
    m3ApiReturn((int64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000);
}

static m3ApiRawFunction(random_service) {
    (void)_ctx;
    (void)_mem;
    (void)runtime;
    m3ApiReturnType(int64_t);
    m3ApiReturn(splitmix(&sequence));
}

static m3ApiRawFunction(log_service) {
    (void)_ctx;
    m3ApiGetArg(int32_t, at);
    m3ApiGetArg(int32_t, len);
    uint32_t size = 0;
    uint8_t* memory = m3_GetMemory(runtime, &size, 0);
    if (!memory || at < 0 || len < 0 || (uint64_t)(uint32_t)at + (uint32_t)len > size) m3ApiTrap(m3Err_trapOutOfBoundsMemoryAccess);
    (void)_mem;
    const char* line = clarity_format("engine %s: %.*s", serving->name, (int)len, (const char*)memory + at);
    if (serving->services.log) {
        serving->services.log(line, serving->services.log_context);
    } else {
        fprintf(stderr, "%s\n", line);
    }
    m3ApiSuccess();
}

// The language as it is at the call, written into the engine's room; gives its whole length.
static m3ApiRawFunction(language_service) {
    (void)_ctx;
    m3ApiReturnType(int32_t);
    m3ApiGetArg(int32_t, at);
    m3ApiGetArg(int32_t, room);
    uint32_t size = 0;
    uint8_t* memory = m3_GetMemory(runtime, &size, 0);
    if (!memory || at < 0 || room < 0 || (uint64_t)(uint32_t)at + (uint32_t)room > size) m3ApiTrap(m3Err_trapOutOfBoundsMemoryAccess);
    (void)_mem;
    const char* language = serving->services.fixed_language ? serving->services.language : clarity_language();
    size_t length = language ? strlen(language) : 0;
    memcpy(memory + at, language ? language : "", length < (size_t)room ? length : (size_t)room);
    m3ApiReturn((int32_t)length);
}

/// The services an engine may use: functions of module `clarity`, with their
/// types as wasm3 writes signatures.
static const struct {
    const char* name;
    const char* type;
    M3RawCall function;
} SERVICES[SERVICE_COUNT] = {
    {"clock", "I()", clock_service},
    {"random", "I()", random_service},
    {"log", "v(ii)", log_service},
    {"language", "i(ii)", language_service},
};

/// Asked by wasm3 at every call and every turn of a loop.
static M3Result watch(void) {
    if (deadline == 0 || ++turns % 4096 != 0) return m3Err_none;
    return now_ms() >= deadline ? LATE : m3Err_none;
}

static void window_closed(uint64_t window, void* engine);

/// The room an engine's calls run with.
#define ENGINE_STACK (1u << 20)

/// What a WebAssembly module imports first, and whether it exports `name`
/// as a function (kind 0) or a memory (kind 2): enough to tell an engine.
typedef struct {
    const uint8_t* bytes;
    size_t size;
    size_t at;
    bool broken;
} Reader;

static uint32_t leb(Reader* r) {
    uint32_t value = 0;
    for (int shift = 0; shift < 35; shift += 7) {
        if (r->at >= r->size) break;
        uint8_t byte = r->bytes[r->at++];
        value |= (uint32_t)(byte & 0x7f) << shift;
        if (!(byte & 0x80)) return value;
    }
    r->broken = true;
    return 0;
}

static const char* name_of(Reader* r) {
    uint32_t len = leb(r);
    if (r->broken || r->at + len > r->size) {
        r->broken = true;
        return "";
    }
    const char* text = clarity_format("%.*s", (int)len, (const char*)r->bytes + r->at);
    r->at += len;
    return text;
}

/// A value type as wasm3 writes it in a signature.
static char value_type(uint8_t type) {
    switch (type) {
        case 0x7f: return 'i';
        case 0x7e: return 'I';
        case 0x7d: return 'f';
        case 0x7c: return 'F';
        default: return '?';
    }
}

/// Checks every import of the module against the services the engine uses:
/// a function of module `clarity`, declared in `uses`, of the service's type.
static bool check_imports(const char* engine, const uint8_t* bytes, size_t size, size_t use_count, const char* const* uses) {
    Reader r = {bytes, size, 8, false};
    const char** types = NULL;
    uint32_t type_count = 0;
    while (r.at < size && !r.broken) {
        uint8_t section = bytes[r.at++];
        uint32_t length = leb(&r);
        size_t end = r.at + length;
        if (r.broken || end > size) break;
        if (section == 1) {
            type_count = leb(&r);
            types = clarity_alloc(sizeof(char*) * (type_count + 1));
            for (uint32_t t = 0; t < type_count && !r.broken; ++t) {
                r.at++;  // 0x60, a function type
                char params[64] = "", results[64] = "";
                uint32_t n = leb(&r);
                for (uint32_t i = 0; i < n && r.at < size; ++i) params[i < 63 ? i : 63] = value_type(bytes[r.at++]);
                uint32_t m = leb(&r);
                for (uint32_t i = 0; i < m && r.at < size; ++i) results[i < 63 ? i : 63] = value_type(bytes[r.at++]);
                types[t] = clarity_format("%s(%s)", m == 0 ? "v" : results, params);
            }
        } else if (section == 2) {
            uint32_t count = leb(&r);
            for (uint32_t i = 0; i < count && !r.broken; ++i) {
                const char* from = name_of(&r);
                const char* what = name_of(&r);
                uint8_t kind = r.at < size ? bytes[r.at++] : 0xff;
                const char* type = "";
                if (kind == 0) {
                    uint32_t index = leb(&r);
                    if (index < type_count) type = types[index];
                } else if (kind == 1) {
                    r.at++;  // the type of the table's elements
                    uint8_t flags = r.at < size ? bytes[r.at++] : 0;
                    leb(&r);
                    if (flags & 1) leb(&r);
                } else if (kind == 2) {
                    uint8_t flags = r.at < size ? bytes[r.at++] : 0;
                    leb(&r);
                    if (flags & 1) leb(&r);
                } else if (kind == 3) {
                    r.at += 2;
                }
                bool used = false;
                for (size_t u = 0; u < use_count; ++u) used = used || strcmp(uses[u], what) == 0;
                if (strcmp(from, "clarity") != 0 || !used)
                    return clarity_fail(CLARITY_ENGINE, "engine %s imports %s.%s; an engine imports only the services it uses", engine, from, what);
                bool fits = false;
                for (size_t s = 0; s < SERVICE_COUNT; ++s) fits = fits || (strcmp(SERVICES[s].name, what) == 0 && strcmp(SERVICES[s].type, type) == 0);
                if (!fits) return clarity_fail(CLARITY_ENGINE, "engine %s imports clarity.%s with another type than Clarity gives it", engine, what);
            }
        }
        r.at = end;
    }
    return true;
}

/// The module's first import (NULL when none), and whether it exports
/// function `function` (or, with kind 2, memory `function`).
static bool read_module(const uint8_t* bytes, size_t size, const char** import, const char* function, uint8_t kind, bool* exported) {
    Reader r = {bytes, size, 8, false};
    *import = NULL;
    *exported = false;
    while (r.at < size && !r.broken) {
        uint8_t section = bytes[r.at++];
        uint32_t length = leb(&r);
        size_t end = r.at + length;
        if (r.broken || end > size) return false;
        if (section == 2 && !*import) {
            if (leb(&r) > 0) {
                const char* from = name_of(&r);
                const char* what = name_of(&r);
                *import = clarity_format("%s.%s", from, what);
            }
        } else if (section == 7) {
            uint32_t count = leb(&r);
            for (uint32_t i = 0; i < count && !r.broken; ++i) {
                const char* export = name_of(&r);
                uint8_t found = r.at < size ? bytes[r.at++] : 0xff;
                leb(&r);
                if (found == kind && strcmp(export, function) == 0) *exported = true;
            }
        }
        r.at = end;
    }
    return !r.broken;
}

bool clarity_engine_load(const char* name, const uint8_t* bytes, size_t size, ClarityEngineDeclaration declaration, ClarityEngine** out) {
    size_t function_count = declaration.function_count, use_count = declaration.use_count;
    const char* const* functions = declaration.functions;
    const char* const* uses = declaration.uses;
    if (size < 8 || memcmp(bytes, "\0asm", 4) != 0) return clarity_fail(CLARITY_ENGINE, "engine %s is no WebAssembly module", name);
    const char* import;
    bool exported;
    if (!read_module(bytes, size, &import, "memory", 2, &exported)) return clarity_fail(CLARITY_ENGINE, "engine %s is no WebAssembly module", name);
    if (!check_imports(name, bytes, size, use_count, uses)) return false;
    const char* interface[] = {"clarity_alloc", "clarity_free"};
    for (size_t i = 0; i < 2; ++i) {
        bool has;
        read_module(bytes, size, &import, interface[i], 0, &has);
        if (!has) return clarity_fail(CLARITY_ENGINE, "engine %s lacks %s of the engine interface", name, interface[i]);
    }
    if (!exported) return clarity_fail(CLARITY_ENGINE, "engine %s lacks the memory of the engine interface", name);
    for (size_t i = 0; i < function_count; ++i) {
        bool has;
        read_module(bytes, size, &import, functions[i], 0, &has);
        if (!has) return clarity_fail(CLARITY_ENGINE, "engine %s has no function %s", name, functions[i]);
    }
    if (declaration.opens) {
        bool has;
        read_module(bytes, size, &import, "clarity_open", 0, &has);
        if (!has) return clarity_fail(CLARITY_ENGINE, "engine %s lacks clarity_open, which its on open needs", name);
    }
    // wasm3 must be able to read it too.
    IM3Environment environment = m3_NewEnvironment();
    IM3Module module;
    M3Result parsed = m3_ParseModule(environment, &module, bytes, (uint32_t)size);
    if (parsed) {
        m3_FreeEnvironment(environment);
        return clarity_fail(CLARITY_ENGINE, "engine %s is no WebAssembly module: %s", name, parsed);
    }
    m3_FreeModule(module);
    // The engine outlives every command, so it lives outside the scratch arena.
    ClarityEngine* engine = malloc(sizeof *engine);
    engine->name = strdup(name);
    engine->bytes = malloc(size);
    memcpy(engine->bytes, bytes, size);
    engine->size = (uint32_t)size;
    engine->environment = environment;
    engine->limits = declaration.limits;
    engine->state = declaration.state;
    engine->opens = declaration.opens;
    engine->kept = NULL;
    engine->kept_count = 0;
    // A window that closes takes its instance along.
    if (engine->state == CLARITY_ENGINE_PER_WINDOW) clarity_on_window_closed(window_closed, engine);
    engine->services = (ClarityEngineServices){0};
    for (size_t s = 0; s < SERVICE_COUNT; ++s) {
        engine->uses[s] = false;
        for (size_t u = 0; u < use_count; ++u) engine->uses[s] = engine->uses[s] || strcmp(uses[u], SERVICES[s].name) == 0;
    }
    m3_YieldHook = watch;
    *out = engine;
    return true;
}

/// Writes `input` into the instance, runs `function` and reads its answer as JSON.
static M3Result invoke(IM3Runtime runtime, const char* function, const char* input, cJSON** answer) {
    uint32_t len = (uint32_t)strlen(input);
    IM3Function alloc = NULL, free_room = NULL, run = NULL;
    M3Result failed = m3_FindFunction(&alloc, runtime, "clarity_alloc");
    if (!failed) failed = m3_FindFunction(&free_room, runtime, "clarity_free");
    if (!failed) failed = m3_FindFunction(&run, runtime, function);
    int32_t at = 0;
    if (!failed) failed = m3_CallV(alloc, (int32_t)len);
    if (!failed) failed = m3_GetResultsV(alloc, &at);
    uint32_t memory_size = 0;
    uint8_t* memory = failed ? NULL : m3_GetMemory(runtime, &memory_size, 0);
    if (!failed && (!memory || (uint64_t)(uint32_t)at + len > memory_size)) failed = "the arguments do not fit its memory";
    if (!failed) memcpy(memory + (uint32_t)at, input, len);
    int64_t packed = 0;
    if (!failed) failed = m3_CallV(run, at, (int32_t)len);
    if (!failed) failed = m3_GetResultsV(run, &packed);
    if (failed) return failed;
    // The memory may have grown during the call.
    memory = m3_GetMemory(runtime, &memory_size, 0);
    uint32_t place = (uint32_t)((uint64_t)packed >> 32), size = (uint32_t)packed;
    if (!memory || (uint64_t)place + size > memory_size) return "its answer is outside its memory";
    *answer = cJSON_ParseWithLength((const char*)memory + place, size);
    if (!*answer) return "its answer is no JSON";
    return m3_CallV(free_room, (int32_t)place, (int32_t)size);
}

/// A new instance of the module with its services, opened when the engine has `on open`.
static M3Result instance(ClarityEngine* engine, IM3Runtime* out) {
    IM3Runtime runtime = m3_NewRuntime(engine->environment, ENGINE_STACK, NULL);
    if (!runtime) return "no room for it";
    IM3Module module;
    M3Result failed = m3_ParseModule(engine->environment, &module, engine->bytes, engine->size);
    if (!failed && (failed = m3_LoadModule(runtime, module))) m3_FreeModule(module);
    // The memory never grows past the limit: memory.grow answers -1 beyond it.
    uint32_t pages = engine->limits.megabytes * 16;
    if (!failed && runtime->memory.maxPages > pages) runtime->memory.maxPages = pages;
    // The services it uses; one it does not import stays unlinked.
    for (size_t s = 0; s < SERVICE_COUNT && !failed; ++s) {
        if (!engine->uses[s]) continue;
        M3Result linked = m3_LinkRawFunction(module, "clarity", SERVICES[s].name, SERVICES[s].type, SERVICES[s].function);
        if (linked != m3Err_functionLookupFailed) failed = linked;
    }
    if (!failed && engine->opens) {
        cJSON* opened = NULL;
        M3Result open_failed = invoke(runtime, "clarity_open", "[]", &opened);
        const cJSON* why = cJSON_GetObjectItemCaseSensitive(opened, "error");
        if (open_failed == LATE) {
            failed = LATE;
        } else if (open_failed || !opened) {
            failed = "its on open gave no answer";
        } else if (cJSON_IsString(why)) {
            failed = clarity_format("its on open refused: %s", why->valuestring);
        } else if (!cJSON_HasObjectItem(opened, "value")) {
            failed = "its on open gave no answer";
        }
    }
    if (failed) {
        m3_FreeRuntime(runtime);
        return failed;
    }
    *out = runtime;
    return m3Err_none;
}

bool clarity_engine_call(ClarityEngine* engine, const char* function, cJSON* args, cJSON** out) {
    return clarity_engine_call_in(engine, 0, function, args, out);
}

/// Hears of a window that closes, for an engine with state per window.
static void window_closed(uint64_t window, void* engine) {
    clarity_engine_close_window(engine, window);
}

void clarity_engine_close_window(ClarityEngine* engine, uint64_t window) {
    if (engine->state != CLARITY_ENGINE_PER_WINDOW) return;
    IM3Runtime runtime = take_kept(engine, window);
    if (runtime) m3_FreeRuntime(runtime);
}

bool clarity_engine_call_in(ClarityEngine* engine, uint64_t window, const char* function, cJSON* args, cJSON** out) {
    const char* input = cJSON_PrintUnformatted(args);
    serving = engine;
    sequence = engine->services.fixed_seed ? engine->services.seed : fresh_seed();
    deadline = now_ms() + (int64_t)engine->limits.seconds * 1000;
    turns = 0;
    // An engine with state keeps its instance, unless a call fails or runs
    // past its limit: then its memory may be half changed, so the next call
    // starts a new one.
    uint64_t key = engine->state == CLARITY_ENGINE_PER_WINDOW ? window : 0;
    IM3Runtime runtime = engine->state == CLARITY_ENGINE_NO_STATE ? NULL : take_kept(engine, key);
    M3Result failed = runtime ? m3Err_none : instance(engine, &runtime);
    cJSON* answer = NULL;
    if (!failed) failed = invoke(runtime, function, input, &answer);
    deadline = 0;
    if (failed) {
        if (runtime) m3_FreeRuntime(runtime);
    } else if (engine->state != CLARITY_ENGINE_NO_STATE) {
        // The engine answered, even with a refusal, so its memory is whole.
        keep(engine, key, runtime);
    } else {
        m3_FreeRuntime(runtime);
    }
    if (failed == LATE) {
        uint32_t seconds = engine->limits.seconds;
        return clarity_fail(CLARITY_ENGINE, "engine %s failed in %s: it ran longer than %u second%s", engine->name, function, seconds, seconds == 1 ? "" : "s");
    }
    if (failed) return clarity_fail(CLARITY_ENGINE, "engine %s failed in %s: %s", engine->name, function, failed);
    const cJSON* error = cJSON_GetObjectItemCaseSensitive(answer, "error");
    // An engine refuses by the key of its text (its `says`), or in its own words.
    if (cJSON_IsString(error)) return clarity_refuse(clarity_key_text(error->valuestring));
    cJSON* value = cJSON_DetachItemFromObjectCaseSensitive(answer, "value");
    if (!value) return clarity_fail(CLARITY_ENGINE, "engine %s failed in %s: its answer has no value", engine->name, function);
    *out = value;
    return true;
}

void clarity_engine_serve(ClarityEngine* engine, ClarityEngineServices services) {
    engine->services = services;
}

void clarity_engine_free(ClarityEngine* engine) {
    if (!engine) return;
    if (engine->state == CLARITY_ENGINE_PER_WINDOW) clarity_off_window_closed(window_closed, engine);
    for (size_t i = 0; i < engine->kept_count; ++i) m3_FreeRuntime(engine->kept[i].runtime);
    free(engine->kept);
    m3_FreeEnvironment(engine->environment);
    free(engine->bytes);
    free(engine->name);
    free(engine);
}
