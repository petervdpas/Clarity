// Reordering (DESIGN 5.30), decided here and not by the screen: a row of a
// list, table, tree or tree table moved by a drag or by Alt and an arrow
// key. The result is the list's new order, each row's parent (in a tree) and
// each row's number at its level, which goes into the field of `reorder by`.
// A move that changes no parent or number is refused (conformance/reorder.json).

#pragma once

#include <stddef.h>
#include <stdint.h>

#include "cjson/cJSON.h"
#include "clarity/core.h"

/// A list after a move, in the scratch arena: the old places in the new
/// order, and each row's parent (NULL for none) and number in that order.
typedef struct {
    size_t count;
    size_t* order;
    const cJSON** parents;
    int64_t* numbers;
} ClarityReordered;

/// The rows (each with its `id` and, in a tree, its `parent`) after `move`,
/// a drag `{row, where, target}` or a key `{row, key}`. False when the move
/// changes nothing or is no move: the list stays as it is.
bool clarity_reorder(const cJSON* rows, const cJSON* move, bool tree, ClarityReordered* out);
