#include "engine/screen.h"

#include <stdlib.h>
#include <string.h>

#include "clarity/core.h"

#if defined(_WIN32)
#include <windows.h>
typedef SRWLOCK Gate;
static void gate_init(Gate* gate) { InitializeSRWLock(gate); }
static void gate_take(Gate* gate) { AcquireSRWLockExclusive(gate); }
static void gate_give(Gate* gate) { ReleaseSRWLockExclusive(gate); }
static void gate_end(Gate* gate) { (void)gate; }
#else
#include <pthread.h>
typedef pthread_mutex_t Gate;
static void gate_init(Gate* gate) { pthread_mutex_init(gate, NULL); }
static void gate_take(Gate* gate) { pthread_mutex_lock(gate); }
static void gate_give(Gate* gate) { pthread_mutex_unlock(gate); }
static void gate_end(Gate* gate) { pthread_mutex_destroy(gate); }
#endif

struct ClarityScreenEngine {
    ClarityEngine* engine;
    bool pointer;
    bool keys;
    /// Why its frame failed, once one did; NULL while it runs.
    char* ended;
};

bool clarity_screen_engine_load(const char* name, const uint8_t* bytes, size_t size, size_t use_count, const char* const* uses, bool opens, ClarityEngineLimits limits, ClarityScreenEngine** out) {
    // One instance for the canvas, kept from frame to frame; no window's closing takes it.
    ClarityEngine* engine = NULL;
    ClarityEngineDeclaration declaration = {0, NULL, use_count, uses, limits, CLARITY_ENGINE_PER_APP, opens};
    if (!clarity_engine_load(name, bytes, size, declaration, &engine)) return false;
    const char* needed[] = {"clarity_tick", "clarity_draw"};
    for (size_t i = 0; i < 2; ++i) {
        if (clarity_engine_exports(engine, needed[i])) continue;
        clarity_engine_free(engine);
        return clarity_fail(CLARITY_ENGINE, "engine %s lacks %s, which an engine beside the screen needs", name, needed[i]);
    }
    ClarityScreenEngine* screen = calloc(1, sizeof *screen);
    if (!screen) {
        clarity_engine_free(engine);
        return clarity_fail(CLARITY_ENGINE, "engine %s: no room for it", name);
    }
    screen->engine = engine;
    screen->pointer = clarity_engine_exports(engine, "clarity_pointer");
    screen->keys = clarity_engine_exports(engine, "clarity_key");
    *out = screen;
    return true;
}

void clarity_screen_engine_serve(ClarityScreenEngine* engine, ClarityEngineServices services) { clarity_engine_serve(engine->engine, services); }

bool clarity_screen_engine_takes_pointer(const ClarityScreenEngine* engine) { return engine->pointer; }
bool clarity_screen_engine_takes_keys(const ClarityScreenEngine* engine) { return engine->keys; }

static const char* text_of(const cJSON* object, const char* name) {
    const cJSON* item = cJSON_GetObjectItemCaseSensitive(object, name);
    return cJSON_IsString(item) ? item->valuestring : "";
}

/// A whole number of `object`, rounded toward zero; 0 when it has none.
static cJSON* whole_of(const cJSON* object, const char* name) {
    const cJSON* item = cJSON_GetObjectItemCaseSensitive(object, name);
    return cJSON_CreateNumber(cJSON_IsNumber(item) ? (double)(int64_t)item->valuedouble : 0);
}

static bool frame_of(ClarityScreenEngine* screen, double ms, const cJSON* input, cJSON** out) {
    ClarityEngine* engine = screen->engine;
    cJSON* told = cJSON_CreateArray();
    cJSON* ignored = NULL;
    const cJSON* given;
    if (!cJSON_IsArray(input)) input = NULL;
    cJSON_ArrayForEach(given, input) {
        if (!cJSON_IsObject(given)) continue;
        const cJSON* pointer = cJSON_GetObjectItemCaseSensitive(given, "pointer");
        const cJSON* key = cJSON_GetObjectItemCaseSensitive(given, "key");
        const cJSON* hear = cJSON_GetObjectItemCaseSensitive(given, "hear");
        if (pointer && screen->pointer) {
            cJSON* args = cJSON_CreateArray();
            cJSON_AddItemToArray(args, cJSON_CreateString(text_of(pointer, "kind")));
            cJSON_AddItemToArray(args, whole_of(pointer, "x"));
            cJSON_AddItemToArray(args, whole_of(pointer, "y"));
            if (!clarity_engine_call_telling(engine, "clarity_pointer", args, told, &ignored)) return false;
        } else if (key && screen->keys) {
            cJSON* args = cJSON_CreateArray();
            cJSON_AddItemToArray(args, cJSON_CreateString(text_of(key, "kind")));
            cJSON_AddItemToArray(args, cJSON_CreateString(text_of(key, "name")));
            if (!clarity_engine_call_telling(engine, "clarity_key", args, told, &ignored)) return false;
        } else if (hear) {
            const cJSON* values = cJSON_GetObjectItemCaseSensitive(hear, "values");
            cJSON* args = cJSON_IsArray(values) ? cJSON_Duplicate(values, true) : cJSON_CreateArray();
            if (!clarity_engine_call_telling(engine, text_of(hear, "name"), args, told, &ignored)) return false;
        }
    }
    cJSON* tick = cJSON_CreateArray();
    cJSON_AddItemToArray(tick, cJSON_CreateNumber(ms));
    if (!clarity_engine_call_telling(engine, "clarity_tick", tick, told, &ignored)) return false;
    cJSON* shapes = NULL;
    if (!clarity_engine_call_telling(engine, "clarity_draw", cJSON_CreateArray(), told, &shapes)) return false;
    if (!cJSON_IsArray(shapes)) return clarity_fail(CLARITY_ENGINE, "engine %s failed in clarity_draw: it gave no list of shapes", clarity_engine_name(engine));
    cJSON* drawn = cJSON_CreateObject();
    cJSON_AddItemToObject(drawn, "shapes", shapes);
    cJSON_AddItemToObject(drawn, "tells", told);
    *out = drawn;
    return true;
}

bool clarity_screen_engine_frame(ClarityScreenEngine* screen, double ms, const cJSON* input, cJSON** out) {
    if (screen->ended) return clarity_fail(CLARITY_ENGINE, "%s", screen->ended);
    if (frame_of(screen, ms, input, out)) return true;
    // Ended: every frame after says why.
    const char* why = clarity_error()->message;
    size_t size = strlen(why) + 1;
    screen->ended = malloc(size);
    if (screen->ended) memcpy(screen->ended, why, size);
    return false;
}

void clarity_screen_engine_free(ClarityScreenEngine* screen) {
    if (!screen) return;
    clarity_engine_free(screen->engine);
    free(screen->ended);
    free(screen);
}

// -- Instances by number -----------------------------------------------------

/// An open instance; the list holds it once, and every frame that runs once more.
typedef struct Held {
    uint64_t id;
    Gate turn;
    ClarityScreenEngine* engine;
    int holders;
    struct Held* next;
} Held;

struct ClarityScreenInstances {
    ClarityScreenMaker make;
    Gate gate;
    Held* open;
    uint64_t next;
};

ClarityScreenInstances* clarity_screen_instances_new(ClarityScreenMaker make) {
    ClarityScreenInstances* instances = calloc(1, sizeof *instances);
    if (!instances) return NULL;
    instances->make = make;
    gate_init(&instances->gate);
    return instances;
}

static void let_go(ClarityScreenInstances* instances, Held* held) {
    gate_take(&instances->gate);
    bool last = --held->holders == 0;
    gate_give(&instances->gate);
    if (!last) return;
    clarity_screen_engine_free(held->engine);
    gate_end(&held->turn);
    free(held);
}

void clarity_screen_instances_free(ClarityScreenInstances* instances) {
    if (!instances) return;
    while (instances->open) clarity_screen_close(instances, instances->open->id);
    gate_end(&instances->gate);
    free(instances);
}

uint64_t clarity_screen_open(ClarityScreenInstances* instances, ClarityScreenEngine* engine) {
    Held* held = calloc(1, sizeof *held);
    if (!held) {
        clarity_screen_engine_free(engine);
        return 0;
    }
    gate_init(&held->turn);
    held->engine = engine;
    held->holders = 1;
    gate_take(&instances->gate);
    held->id = ++instances->next;
    held->next = instances->open;
    instances->open = held;
    gate_give(&instances->gate);
    return held->id;
}

/// Instance `id`, held for the caller; NULL when it is gone.
static Held* held_of(ClarityScreenInstances* instances, uint64_t id) {
    gate_take(&instances->gate);
    Held* held = instances->open;
    while (held && held->id != id) held = held->next;
    if (held) ++held->holders;
    gate_give(&instances->gate);
    return held;
}

bool clarity_screen_frame(ClarityScreenInstances* instances, uint64_t id, double ms, const cJSON* input, cJSON** out) {
    Held* held = held_of(instances, id);
    if (!held) return clarity_fail(CLARITY_ENGINE, CLARITY_SCREEN_ENGINE_GONE);
    gate_take(&held->turn);
    bool ok = held->engine ? clarity_screen_engine_frame(held->engine, ms, input, out) : clarity_fail(CLARITY_ENGINE, CLARITY_SCREEN_ENGINE_GONE);
    gate_give(&held->turn);
    let_go(instances, held);
    return ok;
}

cJSON* clarity_screen_hears(ClarityScreenInstances* instances, uint64_t id) {
    Held* held = held_of(instances, id);
    if (!held) return cJSON_CreateNull();
    cJSON* out = cJSON_CreateNull();
    gate_take(&held->turn);
    if (held->engine) {
        out = cJSON_CreateObject();
        cJSON_AddBoolToObject(out, "pointer", held->engine->pointer);
        cJSON_AddBoolToObject(out, "keys", held->engine->keys);
    }
    gate_give(&held->turn);
    let_go(instances, held);
    return out;
}

void clarity_screen_close(ClarityScreenInstances* instances, uint64_t id) {
    gate_take(&instances->gate);
    Held** at = &instances->open;
    while (*at && (*at)->id != id) at = &(*at)->next;
    Held* held = *at;
    if (held) *at = held->next;
    gate_give(&instances->gate);
    if (!held) return;
    // A frame that runs keeps it until it ends.
    gate_take(&held->turn);
    clarity_screen_engine_free(held->engine);
    held->engine = NULL;
    gate_give(&held->turn);
    let_go(instances, held);
}

size_t clarity_screen_count(ClarityScreenInstances* instances) {
    gate_take(&instances->gate);
    size_t count = 0;
    for (Held* held = instances->open; held; held = held->next) ++count;
    gate_give(&instances->gate);
    return count;
}

static cJSON* value_of(cJSON* value) {
    cJSON* out = cJSON_CreateObject();
    cJSON_AddItemToObject(out, "value", value);
    return out;
}

static cJSON* error_of(void) {
    cJSON* out = cJSON_CreateObject();
    cJSON_AddStringToObject(out, "error", clarity_error()->message);
    return out;
}

cJSON* clarity_screen_answer(ClarityScreenInstances* instances, const char* command, const cJSON* args) {
    const cJSON* id_item = cJSON_GetObjectItemCaseSensitive(args, "id");
    uint64_t id = cJSON_IsNumber(id_item) && id_item->valuedouble > 0 ? (uint64_t)id_item->valuedouble : 0;
    if (strcmp(command, "engine_open") == 0) {
        const cJSON* name = cJSON_GetObjectItemCaseSensitive(args, "name");
        ClarityScreenEngine* made = NULL;
        if (!instances->make) return NULL;
        if (!instances->make(cJSON_IsString(name) ? name->valuestring : "", &made)) return error_of();
        if (!made) return NULL;
        return value_of(cJSON_CreateNumber((double)clarity_screen_open(instances, made)));
    }
    if (strcmp(command, "engine_hears") == 0) return value_of(clarity_screen_hears(instances, id));
    if (strcmp(command, "engine_frame") == 0) {
        const cJSON* ms = cJSON_GetObjectItemCaseSensitive(args, "ms");
        cJSON* drawn = NULL;
        if (!clarity_screen_frame(instances, id, cJSON_IsNumber(ms) ? ms->valuedouble : 0, cJSON_GetObjectItemCaseSensitive(args, "input"), &drawn)) return error_of();
        return value_of(drawn);
    }
    if (strcmp(command, "engine_close") == 0) {
        clarity_screen_close(instances, id);
        return value_of(cJSON_CreateNull());
    }
    return NULL;
}
