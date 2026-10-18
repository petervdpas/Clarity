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
