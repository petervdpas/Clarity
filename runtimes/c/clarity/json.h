// The fields of records and windows as JSON, as the renderer sends and
// reads them on every stack: decimals as text so no precision is lost,
// dates as `2026-10-17`, no date and an empty reference as null.
//
// Each reader leaves `out` as it is when the field is missing or null, so a
// record read from JSON keeps its defaults; a value that does not fit is an
// invalid-value error.

#pragma once

#include "cjson/cJSON.h"
#include "clarity/picture.h"

cJSON* clarity_json_text(const char* value);
/// Bytes as they travel: base64 text.
cJSON* clarity_json_bytes(ClarityBytes value);
cJSON* clarity_json_integer(int64_t value);
cJSON* clarity_json_decimal(ClarityDecimal value);
cJSON* clarity_json_bool(bool value);
cJSON* clarity_json_date(ClarityDate value);
cJSON* clarity_json_date_time(ClarityDateTime value);
/// A reference field's id; 0 is null.
cJSON* clarity_json_reference(int64_t id);

/// The field `name` of `in`, or NULL when it is missing or null.
const cJSON* clarity_json_given(const cJSON* in, const char* name);

bool clarity_json_get_text(const cJSON* in, const char* name, const char** out);
/// Bytes from the base64 text `name` of `in`.
bool clarity_json_get_bytes(const cJSON* in, const char* name, ClarityBytes* out);
bool clarity_json_get_integer(const cJSON* in, const char* name, int64_t* out);
bool clarity_json_get_decimal(const cJSON* in, const char* name, ClarityDecimal* out);
bool clarity_json_get_bool(const cJSON* in, const char* name, bool* out);
bool clarity_json_get_date(const cJSON* in, const char* name, ClarityDate* out);
bool clarity_json_get_date_time(const cJSON* in, const char* name, ClarityDateTime* out);
bool clarity_json_get_reference(const cJSON* in, const char* name, int64_t* out);
bool clarity_json_get_picture(const cJSON* in, const char* name, ClarityPicture** out);
