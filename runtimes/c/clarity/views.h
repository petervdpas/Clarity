// The shown rows of a window (DESIGN 1.7): its state carries `views`, for
// each table, list, tree or chart by its name what the screen shows of its
// list and how. The runtime fills in what shows after every reply, so a
// handler that changed a list is shown at once; the screen only draws it.

#pragma once

#include "cjson/cJSON.h"
#include "clarity/ui.h"

/// The value at `path` of a window's data: `album.tags` is the field `tags`
/// of the record `album`, `readings` a plain value. NULL when it is not there.
cJSON* clarity_data_at(const cJSON* data, const char* path);

/// Fills each view of `state` from its data: a table's `shown` and `pages`,
/// a tree's `shown`, a chart's `groups`. A part whose chosen row differs
/// from `sent` (the selected rows the screen sent, before a handler ran)
/// shows that row: a table turns to its page, a tree opens the rows above
/// it. The views are a copy of their own, in the scratch arena.
void clarity_fill_views(ClarityWindowState* state, const ClaritySelected* sent, size_t sent_count);

/// The view of `part` after the person's `action`: `{"sort": field}`,
/// `{"page": n}`, `{"open": id}` or `{"close": id}`. False, with the error
/// set, when the window has no view of `part` or the action is none.
bool clarity_view_event(ClarityWindowState* state, const char* part, const cJSON* action);
