// What generated code calls beside records and the database: values shown
// as text, text joined and compared, and the files of a store filtered and
// sorted. Text comes from the scratch arena.

#pragma once

#include "clarity/db.h"
#include "clarity/json.h"
#include "clarity/splash.h"
#include "clarity/sql.h"
#include "clarity/ui.h"

/// A value as text, the way `"text" + value` shows it.
const char* clarity_show_integer(int64_t value);
const char* clarity_show_decimal(ClarityDecimal value);
const char* clarity_show_bool(bool value);
const char* clarity_show_date(ClarityDate value);
const char* clarity_show_date_time(ClarityDateTime value);

/// `a + b` of text.
const char* clarity_join(const char* a, const char* b);
/// Whether two texts are the same; NULL is "".
bool clarity_same_text(const char* a, const char* b);

/// Room for `count` files, none in it yet: a list a filter fills.
ClarityFileList clarity_file_list(size_t count);

/// One key of an order of files: a detail, ascending or not.
typedef struct {
    const char* detail;
    bool descending;
} ClarityFileOrder;

ClarityFileOrder clarity_asc(const char* detail);
ClarityFileOrder clarity_desc(const char* detail);

/// Sorts the files by the keys, the first deciding first; text by its bytes, as on every stack.
void clarity_sort_files(ClarityFileList* files, size_t count, const ClarityFileOrder* keys);

/// Lists of plain values in code (`list<integer>`), in the scratch arena.
/// `items` is shared by copies of the struct, so code that reads a list as a
/// value takes a copy of its own with clarity_integers(list.count, list.items).
typedef struct {
    int64_t* items;
    size_t count;
} ClarityIntegers;

typedef struct {
    ClarityDecimal* items;
    size_t count;
} ClarityDecimals;

typedef struct {
    bool* items;
    size_t count;
} ClarityBools;

typedef struct {
    const char** items;
    size_t count;
} ClarityTexts;

typedef struct {
    ClarityDate* items;
    size_t count;
} ClarityDates;

/// A list of `count` items copied from `items` (NULL for none): `[1, 2, 3]`
/// in code, or a copy of a list.
ClarityIntegers clarity_integers(size_t count, const int64_t* items);
ClarityDecimals clarity_decimals(size_t count, const ClarityDecimal* items);
ClarityBools clarity_bools(size_t count, const bool* items);
ClarityTexts clarity_texts(size_t count, const char* const* items);
ClarityDates clarity_dates(size_t count, const ClarityDate* items);

/// `from to to` in code: the whole numbers from `from` to `to`, both
/// counted; none when `to` is below `from` (conformance/lists.json, `ranges`).
ClarityIntegers clarity_range(int64_t from, int64_t to);
/// The bend points of the link a person bent on a canvas now, x, y, x, y, ... (`canvas.via`).
ClarityIntegers clarity_ui_via(const ClarityUi* ui, const char* part);
/// `snapped(value, step)`: the multiple of step nearest to value, a half rounding up; a step of 0 gives the value.
int64_t clarity_snapped(int64_t value, int64_t step);
/// `join(texts, separator)` in code: the texts in order with the separator
/// between each two, in the scratch arena (conformance/lists.json, `joins`).
const char* clarity_joined(ClarityTexts texts, const char* separator);
/* `length(text)`: how many characters a text has, each counted once however many bytes it takes in UTF-8. */
int64_t clarity_length(const char* text);

/* The text functions of code (conformance/texts.json): characters counted once
 * however many bytes they take, positions from 1; results in the scratch arena. */
ClarityTexts clarity_text_words(const char* text);
ClarityTexts clarity_text_split(const char* text, const char* separator);
const char* clarity_text_trim(const char* text);
const char* clarity_text_lower(const char* text);
const char* clarity_text_upper(const char* text);
const char* clarity_text_replace(const char* text, const char* find, const char* with);
bool clarity_text_starts_with(const char* text, const char* prefix);
bool clarity_text_ends_with(const char* text, const char* suffix);
const char* clarity_text_slice(const char* text, int64_t from, int64_t count);
int64_t clarity_text_index_of(const char* text, const char* find);

/// `zip(paths, texts)` (conformance/zip.json): a zip archive of one file per
/// path holding the text of the same place, stored as it is, in order; the
/// lists pair up. The same files give the same bytes.
ClarityBytes clarity_zip(ClarityTexts paths, ClarityTexts texts);

/// `xs + ys` in code (DESIGN 5.26): a new list of the items of both, in order.
ClarityIntegers clarity_integers_join(ClarityIntegers a, ClarityIntegers b);
ClarityDecimals clarity_decimals_join(ClarityDecimals a, ClarityDecimals b);
ClarityBools clarity_bools_join(ClarityBools a, ClarityBools b);
ClarityTexts clarity_texts_join(ClarityTexts a, ClarityTexts b);
ClarityDates clarity_dates_join(ClarityDates a, ClarityDates b);

/// `xs[i]` in code: item i, counted from 1. False, with an error of
/// CLARITY_VALUE, when the position is outside the list.
bool clarity_integers_at(ClarityIntegers list, int64_t position, int64_t* out);
bool clarity_decimals_at(ClarityDecimals list, int64_t position, ClarityDecimal* out);
bool clarity_bools_at(ClarityBools list, int64_t position, bool* out);
bool clarity_texts_at(ClarityTexts list, int64_t position, const char** out);
bool clarity_dates_at(ClarityDates list, int64_t position, ClarityDate* out);

/// `xs[i] = value` in code.
bool clarity_integers_set(ClarityIntegers list, int64_t position, int64_t value);
bool clarity_decimals_set(ClarityDecimals list, int64_t position, ClarityDecimal value);
bool clarity_bools_set(ClarityBools list, int64_t position, bool value);
bool clarity_texts_set(ClarityTexts list, int64_t position, const char* value);
bool clarity_dates_set(ClarityDates list, int64_t position, ClarityDate value);
/// The error of a position outside a list of `count` items: false.
bool clarity_outside(int64_t position, size_t count);
/// `a div b` and `a mod b` in code: the quotient of two whole numbers rounded
/// toward zero, and what is left with the sign of `a`; dividing by zero fails.
bool clarity_whole_div(int64_t a, int64_t b, int64_t* out);
bool clarity_whole_mod(int64_t a, int64_t b, int64_t* out);

/// `list contains item` in code: text as it is, decimals by their value.
bool clarity_integers_holds(ClarityIntegers list, int64_t item);
bool clarity_decimals_holds(ClarityDecimals list, ClarityDecimal item);
bool clarity_bools_holds(ClarityBools list, bool item);
bool clarity_texts_holds(ClarityTexts list, const char* item);
bool clarity_dates_holds(ClarityDates list, ClarityDate item);
