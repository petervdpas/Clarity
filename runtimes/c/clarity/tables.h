// The rows a table or list shows (DESIGN 1.7), decided here and not by the
// screen: their order (conformance/table-order.json), text ignoring case,
// numbers as numbers, a choice by the label it shows, an empty value last
// either way and equal rows in the order of the data; and the page shown
// of a table shown a page at a time (conformance/table-pages.json).

#pragma once

#include <stddef.h>
#include <stdint.h>

#include "cjson/cJSON.h"
#include "clarity/core.h"

/// The places in the data of `rows`, in the order they show, in the scratch
/// arena. `columns` are {field, kind, choices}, `order` is {field, desc} or
/// null; choices are [value, label] pairs.
size_t* clarity_shown_order(const cJSON* rows, const cJSON* columns, const cJSON* order);
/// The order a click on the header of `field` gives: by it, then the other way.
cJSON* clarity_next_order(const cJSON* order, const char* field);

/// A page of a table: the page shown, how many there are, and the first and
/// last row shown counted from 1 (0 and 0 for none).
typedef struct {
    int64_t page, pages, from, to;
} ClarityPage;

/// Page `page` of `count` rows, `size` to a page, within the pages there are.
ClarityPage clarity_page_of(int64_t count, int64_t size, int64_t page);
/// The text between a page bar's buttons: `51-100 / 230`, or `0 / 0`.
const char* clarity_page_text(ClarityPage page, int64_t count);
/// The page that shows the row at place `at` of the shown order, counted from 0.
int64_t clarity_page_with(int64_t size, int64_t at);
