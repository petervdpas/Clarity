// The values records hold beside C's own: a calendar date and a date with
// a time of day, without a time zone, as Clarity's date and datetime. They
// read and write text as the renderer and the other stacks do: 2026-10-17
// and 2026-10-17T09:30:00. Zeroed, a date is no date: Clarity's empty date.

#pragma once

#include "clarity/core.h"

typedef struct {
    int year;
    int month;
    int day;
} ClarityDate;

/// A date with a time of day, to the second.
typedef struct {
    int year;
    int month;
    int day;
    int hour;
    int minute;
    int second;
} ClarityDateTime;

/// `2026-10-17`; false for text that is no date.
bool clarity_date_parse(const char* text, ClarityDate* out);
/// `2026-10-17T09:30:00`, also with a space or without seconds.
bool clarity_date_time_parse(const char* text, ClarityDateTime* out);
/// `2026-10-17`; "" for no date.
const char* clarity_date_text(ClarityDate value);
/// `2026-10-17T09:30:00`; "" for none.
const char* clarity_date_time_text(ClarityDateTime value);

bool clarity_date_is_none(ClarityDate value);
bool clarity_date_time_is_none(ClarityDateTime value);
/// Below zero, zero or above zero; no date sorts first, as in the other runtimes.
int clarity_date_compare(ClarityDate a, ClarityDate b);
int clarity_date_time_compare(ClarityDateTime a, ClarityDateTime b);

/// The date where the application runs: Clarity's `today()`.
ClarityDate clarity_today(void);
/// The date and time where the application runs: Clarity's `now()`.
ClarityDateTime clarity_now(void);
