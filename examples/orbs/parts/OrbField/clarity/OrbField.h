// Custom part OrbField (DESIGN 5.19): what its library implements and the tells it
// calls. Written by Clarity from the part's declaration, and written again
// when the declaration changes; the files beside this folder are yours.
//
// Every function is called on the part host's thread, one at a time. An
// integer is int64_t and a bool is bool; every other value (text, a decimal,
// money, a date, an enum) is text in its plain form, valid during the call.

#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// The part as the library keeps it.
typedef struct OrbFieldPart OrbFieldPart;
/// Where the part tells what happened; given to OrbField_create.
typedef struct OrbFieldHost OrbFieldHost;

/// Makes the part in `parent`, the native window it draws in (an X11 window
/// on Linux), `width` by `height` pixels.
OrbFieldPart* OrbField_create(void* parent, int width, int height, OrbFieldHost* host);
/// The window it draws in changed size.
void OrbField_resize(OrbFieldPart* part, int width, int height);
/// About 60 times a second while the part is shown: draw, and read input.
void OrbField_frame(OrbFieldPart* part);
/// The window closed; the part is not called again.
void OrbField_destroy(OrbFieldPart* part);
/// Whether the person can use it, as code sets `enabled`.
void OrbField_set_enabled(OrbFieldPart* part, bool enabled);
/// Property speed, as the window or code sets it.
void OrbField_set_speed(OrbFieldPart* part, int64_t speed);
/// What code tells it: OrbField.restart.
void OrbField_restart(OrbFieldPart* part);

// Called by the library; Clarity implements them.
/// Tells caught: its handler runs with these values.
void OrbField_caught(OrbFieldHost* host, int64_t points);
/// Tells finished: its handler runs with these values.
void OrbField_finished(OrbFieldHost* host, int64_t points);

#ifdef __cplusplus
}
#endif
