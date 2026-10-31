// A wizard, the steps part (DESIGN 5.15, conformance/steps.json), walked by
// the runtime: the step it shows, the buttons it has, and what Next and Back
// do. Next runs the handler of `on next` on the step shown, then moves one
// step on, unless the handler refused (the step stays) or set the step
// itself (that step wins). Back goes one step back and runs nothing.

#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace clarity {

/// The step shown when `step` is asked of `count` steps: the nearest there is.
int64_t shown_step(std::optional<int64_t> step, int64_t count);

/// The buttons of step `shown` of `count`: back from the second on, next before the last, finish on the last.
std::vector<std::string> step_buttons(int64_t shown, int64_t count);

/// The step after Next from step `shown`, whose handler left the part's step
/// at `left` (`shown` when it set none).
int64_t step_after_next(int64_t shown, std::optional<int64_t> left, int64_t count);

/// The step after Back: one step back.
int64_t step_after_back(std::optional<int64_t> step, int64_t count);

}  // namespace clarity
