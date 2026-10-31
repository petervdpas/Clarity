// A wizard, the steps part (DESIGN 5.15, conformance/steps.json), decided
// here and not by the screen: the step it shows, the buttons it has, and
// where Next and Back go. Next runs the handler of `on next` on the step
// shown, then moves one step on, unless the handler refused (the step stays)
// or set the step itself (that step wins). Back goes one step back.

#pragma once

#include <stddef.h>
#include <stdint.h>

/// The step shown when the part is on `step` of `count` steps: the nearest there is.
int64_t clarity_shown_step(int64_t step, int64_t count);
/// The buttons of step `shown` of `count`, in `out`: back from the second
/// on, next before the last, finish on the last. How many there are.
size_t clarity_step_buttons(int64_t shown, int64_t count, const char* out[2]);
/// The step after Next from step `shown`, when `now` is the part's step
/// after its handler ran (still `shown` when it set none, or there is none).
int64_t clarity_step_after_next(int64_t shown, int64_t now, int64_t count);
/// The step after Back from the part's `step`.
int64_t clarity_step_back(int64_t step, int64_t count);
