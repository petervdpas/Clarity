// Lists of values as fields (U1c): `list<text>`, `list<integer>`,
// `list<decimal>`, `list<date>` and `list<Genre>`, kept in one column as
// compact JSON, text as it is, decimals as text and dates as `2026-10-23`,
// and the same array on the way to the screen (conformance/list-fields.json).
// A list of an enum's values goes by name, through the functions Clarity
// generates for each enum.

#pragma once

#include "clarity/code.h"

/// A list as its column keeps it.
ClarityValue clarity_texts_value(ClarityTexts list);
ClarityValue clarity_integers_value(ClarityIntegers list);
ClarityValue clarity_decimals_value(ClarityDecimals list);
ClarityValue clarity_dates_value(ClarityDates list);

/// The items of a list column; none when it is null or empty.
bool clarity_get_texts(const char* column, ClarityValue value, ClarityTexts* out);
bool clarity_get_integers(const char* column, ClarityValue value, ClarityIntegers* out);
bool clarity_get_decimals(const char* column, ClarityValue value, ClarityDecimals* out);
bool clarity_get_dates(const char* column, ClarityValue value, ClarityDates* out);

/// A list as it travels to the screen.
cJSON* clarity_json_texts(ClarityTexts list);
cJSON* clarity_json_integers(ClarityIntegers list);
cJSON* clarity_json_decimals(ClarityDecimals list);
cJSON* clarity_json_dates(ClarityDates list);

/// The list `name` of `in`; `out` stays as it is when it is missing or null.
bool clarity_json_get_texts(const cJSON* in, const char* name, ClarityTexts* out);
bool clarity_json_get_integers(const cJSON* in, const char* name, ClarityIntegers* out);
bool clarity_json_get_decimals(const cJSON* in, const char* name, ClarityDecimals* out);
bool clarity_json_get_dates(const cJSON* in, const char* name, ClarityDates* out);
