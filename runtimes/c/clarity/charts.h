// The groups a chart shows (DESIGN 1.7, conformance/charts.json), decided
// here and not by the screen: a list's rows grouped by a value, each group
// counted, summed or averaged, in the order of their values. The axis and
// the colors of the slices are drawing, and stay the screen's.

#pragma once

#include "cjson/cJSON.h"

/// The groups, [{key, value}], of `rows` ({by, of}) grouped by a value of
/// kind `by` (text, number, date, enum, bool), with `measure` count, sum or
/// average; `order` gives an enum's values in their order.
cJSON* clarity_chart_groups(const cJSON* rows, const char* by, const char* measure, const cJSON* order);
