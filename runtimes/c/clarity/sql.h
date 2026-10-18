// Reading stored values back into the fields of a record, used by the
// records Clarity generates: each entity's `from_values` reads a row with
// these, its `values` writes one with the constructors of value.h. Each
// driver stores a value its own way: SQLite keeps decimals and dates as
// text, so no precision is lost. A value that does not fit is a storage
// error naming its column.

#pragma once

#include "clarity/picture.h"
#include "clarity/value.h"

bool clarity_get_text(const char* column, ClarityValue value, const char** out);
bool clarity_get_integer(const char* column, ClarityValue value, int64_t* out);
bool clarity_get_decimal(const char* column, ClarityValue value, ClarityDecimal* out);
bool clarity_get_bool(const char* column, ClarityValue value, bool* out);
/// NULL is no date.
bool clarity_get_date(const char* column, ClarityValue value, ClarityDate* out);
bool clarity_get_date_time(const char* column, ClarityValue value, ClarityDateTime* out);
/// NULL is no record: 0.
bool clarity_get_reference(const char* column, ClarityValue value, int64_t* out);
/// Whether a record has a picture: what the select list reads of one kept
/// in the database (1 or 0), or the path of its file in a file store.
bool clarity_get_picture(const char* column, ClarityValue value, ClarityPicture** out);
