// The rows a tree shows (DESIGN 1.7, conformance/trees.json), decided here
// and not by the screen: a list's rows arranged by their parent, a row
// whose parent is in the list under it and any other at the top, in the
// order of the list. A loop of parents breaks at its row that comes first.

#pragma once

#include <stddef.h>

#include "cjson/cJSON.h"
#include "clarity/core.h"

/// The parent of each row as its place in the list, -1 at the top, in the
/// scratch arena: `ids` and `parents` (NULL for none) by place.
int* clarity_tree_parents(const cJSON** ids, const cJSON** parents, size_t n);
/// The rows shown, top to bottom, each as [place, depth, whether rows are
/// under it]: `rows` are {id, parent}, `open` the ids of the rows shown open.
cJSON* clarity_shown_tree(const cJSON* rows, const cJSON* open);
/// The ids of the rows above the row at place `at`, top first: what a
/// selected row opens.
cJSON* clarity_tree_above(const cJSON* rows, int at);
