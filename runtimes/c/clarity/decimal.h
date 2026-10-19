// An exact decimal number for `decimal` and `money` values: a whole number
// of units and how many of its digits are decimals (its scale). C has no
// decimal type of its own; this one keeps what it is given, so 1250.50
// stays 1250.50 when it is stored and shown. Zeroed it is 0.

#pragma once

#include "clarity/core.h"

/// The most decimals a value keeps; more are rounded off.
#define CLARITY_MAX_SCALE 28
/// The decimals a division gives, before trailing zeros go.
#define CLARITY_DIVISION_SCALE 16

typedef struct {
    __int128 units;
    int scale;
} ClarityDecimal;

/// `units` with `scale` decimals: clarity_decimal(125050, 2) is 1250.50.
ClarityDecimal clarity_decimal(int64_t units, int scale);
/// A whole number as a decimal.
ClarityDecimal clarity_decimal_of(int64_t whole);
/// Text such as `-1250.50`; an invalid-value error for anything else.
bool clarity_decimal_parse(const char* text, ClarityDecimal* out);
/// A number Clarity wrote that is too long for clarity_decimal: from its
/// text, which the checker found a number.
ClarityDecimal clarity_decimal_from_text(const char* text);
/// The value as text, with its decimals: `1250.50`.
const char* clarity_decimal_text(ClarityDecimal value);
/// The value with `scale` decimals, rounded half away from zero.
ClarityDecimal clarity_decimal_rescaled(ClarityDecimal value, int scale);
/// `round(value, places)` in code: at most `places` decimals, rounded half
/// away from zero; a value with fewer stays as it is (conformance/rounding.json).
ClarityDecimal clarity_decimal_round(ClarityDecimal value, int places);
/// `sin(degrees)` and `cos(degrees)` in code, rounded to 10 decimals
/// (conformance/drawing.json).
ClarityDecimal clarity_decimal_sin(ClarityDecimal degrees);
ClarityDecimal clarity_decimal_cos(ClarityDecimal degrees);
/// A value of code as a number a canvas draws with.
double clarity_units(ClarityDecimal value);

ClarityDecimal clarity_decimal_neg(ClarityDecimal a);
ClarityDecimal clarity_decimal_add(ClarityDecimal a, ClarityDecimal b);
ClarityDecimal clarity_decimal_sub(ClarityDecimal a, ClarityDecimal b);
ClarityDecimal clarity_decimal_mul(ClarityDecimal a, ClarityDecimal b);
/// An invalid-value error when dividing by zero.
bool clarity_decimal_div(ClarityDecimal a, ClarityDecimal b, ClarityDecimal* out);
/// By value, so 1.50 equals 1.5: below zero, zero or above zero.
int clarity_decimal_compare(ClarityDecimal a, ClarityDecimal b);
