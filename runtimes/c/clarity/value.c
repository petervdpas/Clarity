#include "clarity/value.h"

void clarity_values_add(ClarityValues* values, ClarityValue value) {
    clarity_grow((void**)&values->items, &values->capacity, values->count, sizeof *values->items);
    values->items[values->count++] = value;
}

ClarityValue clarity_null(void) { return (ClarityValue){.kind = CLARITY_NULL}; }
ClarityValue clarity_integer_value(int64_t value) { return (ClarityValue){.kind = CLARITY_INTEGER, .integer = value}; }
ClarityValue clarity_text_value(const char* value) { return (ClarityValue){.kind = CLARITY_TEXT, .text = value ? value : ""}; }
ClarityValue clarity_bytes_value(ClarityBytes value) { return (ClarityValue){.kind = CLARITY_BYTES, .bytes = value}; }
ClarityValue clarity_decimal_value(ClarityDecimal value) { return (ClarityValue){.kind = CLARITY_DECIMAL, .decimal = value}; }
ClarityValue clarity_bool_value(bool value) { return (ClarityValue){.kind = CLARITY_BOOL, .flag = value}; }

ClarityValue clarity_date_value(ClarityDate value) {
    return clarity_date_is_none(value) ? clarity_null() : (ClarityValue){.kind = CLARITY_DATE, .date = value};
}

ClarityValue clarity_date_time_value(ClarityDateTime value) {
    return clarity_date_time_is_none(value) ? clarity_null() : (ClarityValue){.kind = CLARITY_DATE_TIME, .date_time = value};
}

ClarityValue clarity_reference_value(int64_t id) { return id ? clarity_integer_value(id) : clarity_null(); }
