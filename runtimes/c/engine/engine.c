#include "engine/engine.h"

#include <stdlib.h>
#include <string.h>

#include "clarity/text.h"
#include "wasm3/wasm3.h"

struct ClarityEngine {
    char* name;
    uint8_t* bytes;
    uint32_t size;
    IM3Environment environment;
};

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

bool clarity_engine_load(const char* name, const uint8_t* bytes, size_t size, size_t function_count, const char* const* functions,
                         ClarityEngine** out) {
    if (size < 8 || memcmp(bytes, "\0asm", 4) != 0) return clarity_fail(CLARITY_ENGINE, "engine %s is no WebAssembly module", name);
    const char* import;
    bool exported;
    if (!read_module(bytes, size, &import, "memory", 2, &exported)) return clarity_fail(CLARITY_ENGINE, "engine %s is no WebAssembly module", name);
    if (import) return clarity_fail(CLARITY_ENGINE, "engine %s imports %s; an engine imports nothing", name, import);
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
    *out = engine;
    return true;
}

bool clarity_engine_call(ClarityEngine* engine, const char* function, cJSON* args, cJSON** out) {
    const char* input = cJSON_PrintUnformatted(args);
    uint32_t len = (uint32_t)strlen(input);
    IM3Runtime runtime = m3_NewRuntime(engine->environment, ENGINE_STACK, NULL);
    IM3Module module;
    M3Result failed = runtime ? m3_ParseModule(engine->environment, &module, engine->bytes, engine->size) : "no room for it";
    if (!failed && (failed = m3_LoadModule(runtime, module))) m3_FreeModule(module);
    IM3Function alloc = NULL, free_room = NULL, run = NULL;
    if (!failed) failed = m3_FindFunction(&alloc, runtime, "clarity_alloc");
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
    cJSON* answer = NULL;
    if (!failed) {
        // The memory may have grown during the call.
        memory = m3_GetMemory(runtime, &memory_size, 0);
        uint32_t place = (uint32_t)((uint64_t)packed >> 32), size = (uint32_t)packed;
        if (!memory || (uint64_t)place + size > memory_size) {
            failed = "its answer is outside its memory";
        } else {
            answer = cJSON_ParseWithLength((const char*)memory + place, size);
            if (!answer) failed = "its answer is no JSON";
            m3_CallV(free_room, (int32_t)place, (int32_t)size);
        }
    }
    if (runtime) m3_FreeRuntime(runtime);
    if (failed) return clarity_fail(CLARITY_ENGINE, "engine %s failed in %s: %s", engine->name, function, failed);
    const cJSON* error = cJSON_GetObjectItemCaseSensitive(answer, "error");
    if (cJSON_IsString(error)) return clarity_refuse(clarity_plain_text(error->valuestring));
    cJSON* value = cJSON_DetachItemFromObjectCaseSensitive(answer, "value");
    if (!value) return clarity_fail(CLARITY_ENGINE, "engine %s failed in %s: its answer has no value", engine->name, function);
    *out = value;
    return true;
}

void clarity_engine_free(ClarityEngine* engine) {
    if (!engine) return;
    m3_FreeEnvironment(engine->environment);
    free(engine->bytes);
    free(engine->name);
    free(engine);
}
