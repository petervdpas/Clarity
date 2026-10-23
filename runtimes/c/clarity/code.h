// What generated code calls beside records and the database: values shown
// as text, text joined and compared, and the files of a store filtered and
// sorted. Text comes from the scratch arena.

#pragma once

#include "clarity/db.h"
#include "clarity/json.h"
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

/// `list contains item` in code: text as it is, decimals by their value.
bool clarity_integers_holds(ClarityIntegers list, int64_t item);
bool clarity_decimals_holds(ClarityDecimals list, ClarityDecimal item);
bool clarity_bools_holds(ClarityBools list, bool item);
bool clarity_texts_holds(ClarityTexts list, const char* item);
bool clarity_dates_holds(ClarityDates list, ClarityDate item);

/// What an engine written in Clarity keeps from one command to the next,
/// outside the scratch arena: `kept` takes a copy of `list` (of `text`), and
/// what it held before goes.
void clarity_integers_keep(ClarityIntegers* kept, ClarityIntegers list);
void clarity_decimals_keep(ClarityDecimals* kept, ClarityDecimals list);
void clarity_bools_keep(ClarityBools* kept, ClarityBools list);
void clarity_texts_keep(ClarityTexts* kept, ClarityTexts list);
void clarity_dates_keep(ClarityDates* kept, ClarityDates list);
void clarity_text_keep(const char** kept, const char* text);
/// What was kept goes: the instance it belonged to is gone.
void clarity_integers_drop(ClarityIntegers* kept);
void clarity_decimals_drop(ClarityDecimals* kept);
void clarity_bools_drop(ClarityBools* kept);
void clarity_texts_drop(ClarityTexts* kept);
void clarity_dates_drop(ClarityDates* kept);
void clarity_text_drop(const char** kept);

/// The numbers `random(max)` gives: splitmix64, the sequence every runtime
/// gives alike from the same seed.
typedef struct {
    uint64_t state;
} ClaritySplitmix;

/// A whole number from 1 to max; false, with an error of CLARITY_VALUE, for a max below 1.
bool clarity_splitmix_next(ClaritySplitmix* random, int64_t max, int64_t* out);
/// `random(max)` in an engine written in Clarity, from a sequence seeded
/// once per run of the application.
bool clarity_random(int64_t max, int64_t* out);
