// The dropdown's options of U1c, decided here and not by the screen
// (conformance/dropdown-options.json). A `typed` dropdown offers the values
// its field already has: each once, without empty text, those containing
// the typed text (ignoring case), sorted ignoring case and then as written.
// A `search` dropdown offers the records whose text or shown fields hold
// every typed word, ignoring case, in the order they came.

#pragma once

#include "cjson/cJSON.h"

/// The suggestions of a typed dropdown, an array of text, for `typed`.
cJSON* clarity_suggestions(const cJSON* values, const char* typed);
/// The options (each {id, text, fields}) a search dropdown offers for
/// `typed`; `show` names the fields it shows, NULL for all of them.
cJSON* clarity_searched(const cJSON* options, const cJSON* show, const char* typed);
