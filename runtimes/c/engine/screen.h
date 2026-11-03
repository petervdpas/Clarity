// Engines beside the screen (DESIGN 5.12, conformance/screen-engines.json)
// as Clarity.UI runs them in the program's own process: one instance per
// canvas, which keeps its memory from frame to frame. Each frame hands over
// the input since the last (clarity_pointer, clarity_key, a hear by its
// function's name), then calls clarity_tick with the milliseconds since the
// last frame and clarity_draw for the canvas's shapes; what the answers
// tell comes back with the frame. A frame that fails ends the engine.

#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "engine/engine.h"

/// An engine beside the screen for one canvas.
typedef struct ClarityScreenEngine ClarityScreenEngine;

/// Loads `bytes` as engine `name` for one canvas; refuses (CLARITY_ENGINE)
/// a module that is no engine, imports more than `uses` or lacks its frame
/// loop. With `opens`, clarity_open runs before the first frame.
bool clarity_screen_engine_load(const char* name, const uint8_t* bytes, size_t size, size_t use_count, const char* const* uses, bool opens, ClarityEngineLimits limits, ClarityScreenEngine** out);

/// The engine with `services` in place of the system's.
void clarity_screen_engine_serve(ClarityScreenEngine* engine, ClarityEngineServices services);

/// Whether it takes the pointer and keys: it exports clarity_pointer and clarity_key.
bool clarity_screen_engine_takes_pointer(const ClarityScreenEngine* engine);
bool clarity_screen_engine_takes_keys(const ClarityScreenEngine* engine);

/// One frame: `input` first, as Clarity.UI sends it ([{"pointer":{kind,x,y}} |
/// {"key":{kind,name}} | {"hear":{name,values}}]), then `ms`. Gives
/// {"shapes": [...], "tells": [...]}; fails (CLARITY_ENGINE), and once a
/// frame failed every frame fails alike.
bool clarity_screen_engine_frame(ClarityScreenEngine* engine, double ms, const cJSON* input, cJSON** out);

void clarity_screen_engine_free(ClarityScreenEngine* engine);

/// What a frame of an instance that is gone answers.
#define CLARITY_SCREEN_ENGINE_GONE "The engine ended."

/// Makes a new engine by its name: true with *out NULL when the program has
/// no engine of that name; false when it does not start.
typedef bool (*ClarityScreenMaker)(const char* name, ClarityScreenEngine** out);

/// The engines beside the screen a program has open, each by its number: the
/// screen holds only the number. A frame runs with only its own instance
/// held, so the canvases of several windows draw at once.
typedef struct ClarityScreenInstances ClarityScreenInstances;

ClarityScreenInstances* clarity_screen_instances_new(ClarityScreenMaker make);
void clarity_screen_instances_free(ClarityScreenInstances* instances);

/// Keeps `engine` open: its number.
uint64_t clarity_screen_open(ClarityScreenInstances* instances, ClarityScreenEngine* engine);
/// One frame of instance `id`; one that is gone says so.
bool clarity_screen_frame(ClarityScreenInstances* instances, uint64_t id, double ms, const cJSON* input, cJSON** out);
/// {"pointer", "keys"} of instance `id`; JSON null for one that is gone.
cJSON* clarity_screen_hears(ClarityScreenInstances* instances, uint64_t id);
void clarity_screen_close(ClarityScreenInstances* instances, uint64_t id);
size_t clarity_screen_count(ClarityScreenInstances* instances);

/// Answers Clarity.UI's commands of engines (clarity_ui.h: engine_open,
/// engine_hears, engine_frame, engine_close) as {"value"} or {"error"};
/// NULL for another command or a name the program has no engine of.
cJSON* clarity_screen_answer(ClarityScreenInstances* instances, const char* command, const cJSON* args);
