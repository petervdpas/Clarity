// Canvas modes (DESIGN 5.31), decided here in the runtime: the screen sends
// what the person did on a canvas (an object dropped, a stroke painted, a
// shape added, changed, moved or deleted) and the runtime changes the
// window's data, snapped to the canvas's grid, and chooses what it touched.
// A vector drawing is kept as SVG text in one plain form, which every runtime
// writes and reads alike (conformance/canvas-modes.json).

#pragma once

#include <stdbool.h>
#include <stdint.h>

#include "cjson/cJSON.h"
#include "clarity/core.h"

/// A canvas's mode (objects, paint or vector) and the mode's parameters, as
/// a canvas_event carries them. `at` are an object's x and y fields; `units`
/// NULL is 1000 x 1000.
typedef struct {
    const char* mode;
    const char* list;
    const char* at[2];
    const char* path;
    const int* units;
    int64_t grid;
    bool snap;
    bool readonly;
} ClarityCanvas;

typedef enum { CLARITY_CANVAS_FAILED, CLARITY_CANVAS_REFUSED, CLARITY_CANVAS_CHANGED } ClarityCanvasDone;

/// What an action touched, as a table's chosen row is kept: the object's
/// place in its list or the shape's place in the drawing, from 0, when `selects`.
typedef struct {
    bool selects;
    int selected;
} ClarityCanvasTouched;

/// Applies the person's `action` on a canvas to `data`, changed in place. An
/// action that changes nothing, or names no object or shape, is refused, as
/// every action on a readonly canvas; an action that is none fails.
ClarityCanvasDone clarity_canvas_event(cJSON* data, const ClarityCanvas* canvas, const cJSON* action, ClarityCanvasTouched* touched);

/// A drawing's SVG text: its units and its shapes, one element on a line.
const char* clarity_drawing_text(const int units[2], const cJSON* shapes);

/// The shapes of a drawing's SVG text; `units` become its own when it names
/// them. A line it does not read as a shape is kept as {op: "svg", text}.
cJSON* clarity_drawing_shapes(const char* text, int units[2]);
