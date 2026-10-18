// A stored value, as records are written and read: what a record gives to
// save and what a row of the database holds. SQLite keeps decimals and
// dates as text, so no precision is lost.

#pragma once

#include "clarity/decimal.h"
#include "clarity/values.h"

typedef enum {
    /// No value: NULL.
    CLARITY_NULL = 0,
    CLARITY_INTEGER,
    CLARITY_REAL,
    CLARITY_TEXT,
    CLARITY_BYTES,
    CLARITY_DECIMAL,
    CLARITY_BOOL,
    CLARITY_DATE,
    CLARITY_DATE_TIME,
    /// A picture the record has stored already: its column is left as it
    /// is when the record is saved. It never reaches the database.
    CLARITY_KEPT,
} ClarityValueKind;

typedef struct {
    ClarityValueKind kind;
    union {
        int64_t integer;
        double real;
        const char* text;
        ClarityBytes bytes;
        ClarityDecimal decimal;
        bool flag;
        ClarityDate date;
        ClarityDateTime date_time;
    };
} ClarityValue;

/// Values in a growing list of the scratch arena; zeroed it is empty.
typedef struct {
    ClarityValue* items;
    size_t count;
    size_t capacity;
} ClarityValues;

void clarity_values_add(ClarityValues* values, ClarityValue value);

ClarityValue clarity_null(void);
ClarityValue clarity_integer_value(int64_t value);
ClarityValue clarity_text_value(const char* value);
ClarityValue clarity_bytes_value(ClarityBytes value);
ClarityValue clarity_decimal_value(ClarityDecimal value);
ClarityValue clarity_bool_value(bool value);
/// No date gives NULL.
ClarityValue clarity_date_value(ClarityDate value);
ClarityValue clarity_date_time_value(ClarityDateTime value);
/// A reference field's id; 0, no record, gives NULL.
ClarityValue clarity_reference_value(int64_t id);
