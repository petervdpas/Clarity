// What a window says when code fails at a value or a record: Clarity's own
// words by a literal key (translated where the project has them), while the
// error's message stays the English of the shared cases.

#include "check.h"
#include "clarity/code.h"
#include "clarity/decimal.h"
#include "clarity/text.h"
#include "tests.h"

/// The words a window shows for the last error, as the screen gets them.
static const char* said_json(void) {
    const ClarityText* said = clarity_said(clarity_error());
    return said ? cJSON_PrintUnformatted(clarity_text_json(said)) : "none";
}

void runtime_errors_are_said_by_their_keys(void) {
    CHECK(!clarity_outside(10, 1));
    CHECK_TEXT(clarity_error()->message, "there is no item 10 in a list of 1");
    CHECK_TEXT(said_json(), "{\"key\":\"itemOutsideList\",\"text\":\"There is no item {position} in a list of {count}\",\"values\":{\"position\":{\"kind\":\"integer\",\"value\":10},\"count\":{\"kind\":\"integer\",\"value\":1}}}");

    int64_t n = 0;
    CHECK(!clarity_whole_div(1, 0, &n));
    CHECK_TEXT(clarity_error()->message, "division by zero");
    CHECK_TEXT(said_json(), "{\"key\":\"divisionByZero\",\"text\":\"Division by zero\"}");
    ClarityDecimal d = {0};
    CHECK(!clarity_decimal_div(clarity_decimal_from_text("1"), clarity_decimal_from_text("0"), &d));
    CHECK_TEXT(said_json(), "{\"key\":\"divisionByZero\",\"text\":\"Division by zero\"}");

    CHECK(!clarity_whole_add(INT64_MAX, 1, &n));
    CHECK_TEXT(clarity_error()->message, "a number too large");
    CHECK_TEXT(said_json(), "{\"key\":\"numberTooLarge\",\"text\":\"The number is too large\"}");
    CHECK(!clarity_decimal_mul(clarity_decimal_from_text("79228162514264337593543950335"), clarity_decimal_from_text("2"), &d));
    CHECK_TEXT(said_json(), "{\"key\":\"numberTooLarge\",\"text\":\"The number is too large\"}");

    CHECK(!clarity_not_found("Customer", 3));
    CHECK_TEXT(said_json(), "{\"key\":\"recordNotThere\",\"text\":\"There is no {table} with id {id}\",\"values\":{\"table\":{\"kind\":\"text\",\"value\":\"Customer\"},\"id\":{\"kind\":\"integer\",\"value\":3}}}");
    CHECK(!clarity_in_use("Customer", 3));
    CHECK_TEXT(said_json(), "{\"key\":\"recordStillUsed\",\"text\":\"This {table} is still used by other records\",\"values\":{\"table\":{\"kind\":\"text\",\"value\":\"Customer\"}}}");

    // A refusal is said in its own words; an error of its own has none of Clarity's.
    CHECK(!clarity_refuse(clarity_key_text("tooLate")));
    CHECK_TEXT(said_json(), "{\"key\":\"tooLate\"}");
    CHECK(!clarity_fail(CLARITY_STORAGE, "locked"));
    CHECK_TEXT(said_json(), "none");
}
