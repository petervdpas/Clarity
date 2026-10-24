// JSON in code (DESIGN 5.23): `json of value` and `read Order from json
// text`. A service's JSON is not Clarity's own, so what is read is fitted to
// the entity first: names match ignoring case, underscores and hyphens,
// numbers and strings of numbers stand for each other, what is left out
// keeps its default. Every runtime fits alike (conformance/json.json); the
// record's own reader (its `_from_json`) then reads what was fitted.

#pragma once

#include <stdbool.h>
#include <stddef.h>

#include "cjson/cJSON.h"

/// What a field of an entity takes from JSON.
typedef enum {
    CLARITY_JSON_TEXT,
    CLARITY_JSON_INTEGER,
    /// Decimals and money, written as exact text.
    CLARITY_JSON_DECIMAL,
    CLARITY_JSON_BOOL,
    CLARITY_JSON_DATE,
    CLARITY_JSON_DATE_TIME,
    /// One of `values`, matched in any case.
    CLARITY_JSON_ENUM,
    /// A list whose items fit `item`.
    CLARITY_JSON_VALUES,
    /// Never read: a reference, a picture, a file, a list of records.
    CLARITY_JSON_OTHER,
} ClarityJsonIs;

typedef struct ClarityJsonKind {
    ClarityJsonIs is;
    const char* const* values;
    size_t count;
    const struct ClarityJsonKind* item;
} ClarityJsonKind;

/// A field of an entity, as JSON is fitted to it.
typedef struct {
    const char* name;
    ClarityJsonKind kind;
} ClarityJsonField;

/// The JSON of a value as text, in the scratch arena.
const char* clarity_json_of(const cJSON* value);

/// Reads `text` as one record of `entity` (or, with `many`, a list of them),
/// starting at `at` (object names with dots between; NULL or "" for the
/// top), fitted to the `count` fields over `base`, a new record as JSON.
/// Gives what the record's reader reads, or fails with why it does not fit
/// (a value error).
bool clarity_json_fit(const char* text, const char* at, bool many, const char* entity, const ClarityJsonField* fields, size_t count, const cJSON* base, cJSON** out);
