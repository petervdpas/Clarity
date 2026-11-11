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
#include "clarity/core.h"

/// Reads `input` in `format` with its `options` (a JSON object, "" for none)
/// as `kind` describes it: what was fitted, parsed, in `*out`; or fails
/// with Clarity.Formats' message, word for word as every stack says it.
bool clarity_formats_read(const char* format, const char* input, const char* options, const char* kind, cJSON** out);

/// `value` (as a record's `_json` wrote it) written in `format` with its
/// `options`, as `kind` describes it, in the scratch arena.
const char* clarity_formats_write(const char* format, const char* value, const char* options, const char* kind);

/// The file `input` in `format` as a table (interface 2), JSON
/// {"columns":[...],"rows":[[...],...]} in the scratch arena: the file's
/// columns in its order, the cells of the columns in `fields` (JSON
/// [{"name":"site","kind":"text"},...]) fitted as SQLite keeps them. Fails
/// with Clarity.Formats' message.
bool clarity_formats_read_table(const char* format, ClarityBytes input, const char* options, const char* fields, const char** out);

/// `table` (as clarity_formats_read_table gives it) written back in
/// `format` (interface 2): the whole file, or for JSON the list inside
/// `original`, the rest kept; the bytes in the scratch arena.
bool clarity_formats_write_table(const char* format, ClarityBytes original, const char* table, const char* options, const char* fields, ClarityBytes* out);
