#include "clarity/steps.hpp"

#include <algorithm>

namespace clarity {

int64_t shown_step(std::optional<int64_t> step, int64_t count) { return std::min(std::max(step.value_or(1), int64_t(1)), std::max(count, int64_t(1))); }

std::vector<std::string> step_buttons(int64_t shown, int64_t count) {
    std::vector<std::string> buttons;
    if (shown > 1) buttons.push_back("back");
    buttons.push_back(shown < count ? "next" : "finish");
    return buttons;
}

int64_t step_after_next(int64_t shown, std::optional<int64_t> left, int64_t count) {
    if (shown_step(left, count) == shown && shown < count) return shown + 1;
    return left.value_or(shown);
}

int64_t step_after_back(std::optional<int64_t> step, int64_t count) {
    int64_t shown = shown_step(step, count);
    return shown > 1 ? shown - 1 : shown;
}

}  // namespace clarity
