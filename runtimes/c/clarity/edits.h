// The edit bus (DESIGN 5.29): per open window, the record of the person's
// edits, kept here in the runtime, not in the screen. Each edit is a message
// with the part, its scope (its form or panel, or the part itself), what it
// changed (each name of the window's data, or #parts.<name> for a part's own
// text or tick, with its value before and after) and the database steps its
// handler made. The screen sends an edit when the person ends one, and Ctrl+Z
// or Ctrl+Y as an undo; the runtime takes the steps back or does them again
// and answers with the window's state as it was. Every runtime keeps the
// record alike (conformance/edit-bus.json).
//
// A bus outlives the command, so it keeps its values outside the scratch
// arena, as JSON text. What it gives back is in the scratch arena. The shell
// runs one command at a time, so the buses need no lock of their own.

#pragma once

#include <stddef.h>
#include <stdint.h>

#include "cjson/cJSON.h"
#include "clarity/ui.h"

/// The record of one window's edits.
typedef struct ClarityEdits ClarityEdits;

/// A record of its own, as a test keeps one; free it with clarity_edits_free.
ClarityEdits* clarity_edits_new(void);
void clarity_edits_free(ClarityEdits* bus);

/// The record of the window numbered `instance`, made when it has none.
ClarityEdits* clarity_edits_of(uint64_t instance);
/// Forgets the edits of a window that closed.
void clarity_edits_closed(uint64_t instance);

/// The window's data as a reply that is no edit left it: the edits start
/// from here. While an edit goes on (a drag), it waits for its end.
void clarity_edits_reset(ClarityEdits* bus, const cJSON* data);
/// An edit that goes on: `steps` are the database steps its handler made;
/// it ends with clarity_edits_edit.
void clarity_edits_drag(ClarityEdits* bus, const char* part, const char* scope, const int64_t* steps, size_t step_count);
/// The end of an edit of `part`: `data` is the window's data now, with what
/// its handler changed, `steps` the database steps that handler made. One
/// that changed nothing and made no step is no message; a new edit ends what
/// could be done again in its scope.
void clarity_edits_edit(ClarityEdits* bus, const char* part, const char* scope, const cJSON* data, const int64_t* steps, size_t step_count);

/// Takes database steps back (`back`) or does them again, in the order given;
/// false, with the error set, when the database refuses.
typedef bool (*ClarityTakeSteps)(const int64_t* steps, size_t count, bool back, void* context);

/// Takes back (`back`) or does again the last edit of `scope`, on `data`, the
/// window's data now. `found` says whether there was one; `out` is the
/// window's data after it. False when `take` refused: the edit stays where it was.
bool clarity_edits_undo(ClarityEdits* bus, const char* scope, const cJSON* data, bool back, ClarityTakeSteps take, void* context, bool* found,
                        cJSON** out);

/// What an edit of a window compares: its data, and each part's own text
/// and tick as #parts.<name>.
cJSON* clarity_edit_data(const ClarityWindowState* state);
/// `state` with `data` as an edit put it back: the window's data, and each
/// part's own text and tick.
ClarityWindowState clarity_with_edit_data(const ClarityWindowState* state, const cJSON* data);
