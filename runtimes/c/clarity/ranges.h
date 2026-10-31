// The slider and the spin box of U1c, decided here and not by the screen
// (conformance/ranges.json): how a key, a drag or a typed number moves a
// value within its range. Numbers are text, so decimals stay exact.

#pragma once

#include "clarity/core.h"

/// A range: its lowest and highest value (NULL for no end) and its step.
typedef struct {
    const char* min;
    const char* max;
    const char* step;
} ClarityRange;

/// The value after `key` (up, down, pageUp, pageDown, home, end), within
/// the range; no value (NULL) counts as the lowest, or 0. NULL, with the
/// error set, for text that is no number or a key that is none.
const char* clarity_range_keyed(const char* value, ClarityRange range, const char* key);
/// The value a slider takes where it is dragged to: the nearest step from
/// the lowest value, at most the highest step within the range.
const char* clarity_range_dragged(const char* at, ClarityRange range);
/// A typed number, kept within the range.
const char* clarity_range_typed(const char* value, ClarityRange range);
