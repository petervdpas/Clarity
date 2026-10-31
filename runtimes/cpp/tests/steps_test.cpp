// A wizard's steps (DESIGN 5.15), walked by the runtime, against the cases
// every runtime shares (conformance/steps.json, `walk`).

#include <algorithm>

#include "check.hpp"
#include "clarity/steps.hpp"
#include "conformance.hpp"

using namespace clarity;
using nlohmann::json;

TEST(a_wizard_walks_its_steps_as_every_runtime_does) {
    for (const auto& c : conformance("steps.json")["walk"]) {
        std::string name = c["name"];
        int64_t count = c["steps"];
        const json& handler = c["handler"];
        std::optional<int64_t> step;
        if (c.contains("start")) step = c["start"].get<int64_t>();
        json ran = json::array();
        for (const auto& button : c["do"]) {
            int64_t shown = shown_step(step, count);
            if (button == "back") {
                step = step_after_back(step, count);
            } else if (button == "finish") {
                ran.push_back("finish " + std::to_string(shown));
            } else {
                // Next: the handler runs on the step shown; a refusal keeps it.
                std::optional<int64_t> left = shown;
                if (!handler.is_null()) {
                    ran.push_back("next " + std::to_string(shown));
                    auto refuses = handler.value("refuses", std::vector<int64_t>{});
                    if (std::find(refuses.begin(), refuses.end(), shown) != refuses.end()) {
                        step = shown;
                        continue;
                    }
                    if (handler.contains("jumps") && handler["jumps"]["from"] == shown) left = handler["jumps"]["to"].get<int64_t>();
                }
                step = step_after_next(shown, left, count);
            }
        }
        const json& expect = c["expect"];
        int64_t shown = shown_step(step, count);
        if (shown != expect["shown"]) check::fail(__FILE__, __LINE__, name + ": shown " + std::to_string(shown));
        if (json(step_buttons(shown, count)) != expect["buttons"]) check::fail(__FILE__, __LINE__, name + ": buttons " + json(step_buttons(shown, count)).dump());
        if (ran != expect["ran"]) check::fail(__FILE__, __LINE__, name + ": ran " + ran.dump());
    }
}
