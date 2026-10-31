// Canvas modes (DESIGN 5.31), decided here in the runtime: the screen sends
// what the person did on a canvas (an object dropped, a stroke painted, a
// shape added, changed, moved or deleted) and the runtime changes the
// window's data, snapped to the canvas's grid, and chooses what it touched.
// A vector drawing is kept as SVG text in one plain form, which every runtime
// writes and reads alike (conformance/canvas-modes.json).

#pragma once

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <utility>

#include <nlohmann/json.hpp>

#include "clarity/ui.hpp"

namespace clarity {

/// A drawing's width and height in its own units.
using DrawingUnits = std::array<int64_t, 2>;

/// A canvas's mode and the mode's parameters, as a canvas_event carries them.
struct CanvasArgs {
    std::string part;
    std::string mode;
    std::string list;
    std::array<std::string, 2> at;
    std::string path;
    std::optional<DrawingUnits> units;
    int64_t grid = 0;
    bool snap = false;
    bool readonly = false;
};

/// The canvas a message names: part, mode, list, at, path, units, grid, snap and readonly.
CanvasArgs canvas_args_of(const nlohmann::json& args);

/// Applies the person's action on a canvas to state, and says whether it
/// changed anything; an action that changes nothing, or names no object or
/// shape, is refused (false), as every action on a readonly canvas. Throws
/// InvalidError for an action that does not fit the canvas.
bool canvas_event(WindowState& state, const CanvasArgs& canvas, const nlohmann::json& action);

/// A drawing's SVG text: its units and its shapes, one element on a line.
std::string drawing_text(const DrawingUnits& units, const nlohmann::json& shapes);

/// A drawing's SVG text read back: its units (`units` when it names none)
/// and its shapes. A line it does not read as a shape is kept as it is.
std::pair<DrawingUnits, nlohmann::json> drawing_shapes(const std::string& text, DrawingUnits units);

}  // namespace clarity
