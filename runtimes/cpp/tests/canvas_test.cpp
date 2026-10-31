// Canvas modes (DESIGN 5.31), against the cases every runtime shares
// (conformance/canvas-modes.json).

#include "check.hpp"
#include "clarity/canvas.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

TEST(a_drawing_is_written_and_read_as_plain_svg) {
    json cases = conformance("canvas-modes.json");
    for (const auto& c : cases["svg"]) {
        DrawingUnits units = c["units"].get<DrawingUnits>();
        std::string got = drawing_text(units, c["shapes"]);
        if (json(got) != c["text"]) check::fail(__FILE__, __LINE__, c["name"].get<std::string>() + ": wrote " + got);
        auto [read_units, shapes] = drawing_shapes(c["text"], {1000, 1000});
        if (read_units != units || shapes != c["shapes"]) check::fail(__FILE__, __LINE__, c["name"].get<std::string>() + ": read " + shapes.dump());
    }
    for (const auto& c : cases["read"]) {
        auto [units, shapes] = drawing_shapes(c["text"], {1000, 1000});
        if (c.contains("units") && json(units) != c["units"]) check::fail(__FILE__, __LINE__, c["name"].get<std::string>() + ": units " + json(units).dump());
        if (shapes != c["shapes"]) check::fail(__FILE__, __LINE__, c["name"].get<std::string>() + ": read " + shapes.dump());
    }
}

TEST(a_canvas_action_changes_the_data_or_is_refused) {
    // Kept whole: a loop over a part of a temporary would read freed memory.
    json cases = conformance("canvas-modes.json");
    REQUIRE(!cases["actions"].empty());
    for (const auto& c : cases["actions"]) {
        std::string name = c["name"];
        WindowState state;
        state.data = c["data"];
        CanvasArgs canvas = canvas_args_of(c);
        bool changed = canvas_event(state, canvas, c["action"]);
        bool refused = c.value("refused", false);
        if (changed == refused) {
            check::fail(__FILE__, __LINE__, name + ": changed " + std::to_string(changed));
            continue;
        }
        if (refused) {
            if (state.data != c["data"]) check::fail(__FILE__, __LINE__, name + ": a refused action changed the data");
            continue;
        }
        if (state.data != c["after"]) check::fail(__FILE__, __LINE__, name + ": data " + state.data.dump());
        auto got = state.selected.find(canvas.part);
        json selected = got == state.selected.end() ? json(nullptr) : json(got->second);
        if (selected != c["selected"]) check::fail(__FILE__, __LINE__, name + ": selected " + selected.dump());
    }
}

// An action that does not fit its canvas is no refusal but a mistake.
TEST(a_canvas_action_that_does_not_fit_is_invalid) {
    WindowState state;
    CanvasArgs canvas;
    canvas.mode = "sculpt";
    CHECK_THROWS(InvalidError, canvas_event(state, canvas, json::object()));
    canvas.mode = "vector";
    canvas.path = "drawing";
    CHECK_THROWS(InvalidError, canvas_event(state, canvas, {{"add", {{"op", "star"}}}}));
}
