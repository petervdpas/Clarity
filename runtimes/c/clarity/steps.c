#include "clarity/steps.h"

int64_t clarity_shown_step(int64_t step, int64_t count) {
    int64_t last = count > 1 ? count : 1;
    return step < 1 ? 1 : step > last ? last : step;
}

size_t clarity_step_buttons(int64_t shown, int64_t count, const char* out[2]) {
    size_t n = 0;
    if (shown > 1) out[n++] = "back";
    out[n++] = shown < count ? "next" : "finish";
    return n;
}

int64_t clarity_step_after_next(int64_t shown, int64_t now, int64_t count) {
    // A handler that set the step itself decides; else one step on.
    if (clarity_shown_step(now, count) == shown && shown < count) return shown + 1;
    return now;
}

int64_t clarity_step_back(int64_t step, int64_t count) {
    int64_t shown = clarity_shown_step(step, count);
    return shown > 1 ? shown - 1 : step;
}
