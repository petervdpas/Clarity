// The data formats of code (docs/FORMATS-PLAN.md) through Clarity.Formats,
// the one implementation every stack calls: `read Order from json text`
// reads the format and fits it to the record, which the record's own
// `_from_json` then decodes; `json of value` writes the value as its
// `_json` writes it and the library writes it in the format, in Clarity's
// one form. Carried only when the code reads or writes a format
// (clarity_use_formats in clarity.cmake links the library).

#pragma once

#include <stdbool.h>

#include "cjson/cJSON.h"

/// Reads `input` in `format` with its `options` (a JSON object, "" for none)
/// as `kind` describes it: what was fitted, parsed, in `*out`; or fails
/// with Clarity.Formats' message, word for word as every stack says it.
bool clarity_formats_read(const char* format, const char* input, const char* options, const char* kind, cJSON** out);

/// `value` (as a record's `_json` wrote it) written in `format` with its
/// `options`, as `kind` describes it, in the scratch arena.
const char* clarity_formats_write(const char* format, const char* value, const char* options, const char* kind);
