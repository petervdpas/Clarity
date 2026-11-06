// What the splash screen says is loading (splash.status = @loadingData):
// code sets it while the app's on start runs, and the screen asks for it a
// few times a second until the main window shows. It is a text as a message
// is one, so the screen shows a key in the person's language.

#pragma once

#include "clarity/text.h"

/// Code says what loads now.
void clarity_splash_set_status(const ClarityText* text);
/// What code last said loads, as the wire writes a text, for the caller to free; NULL while it said nothing.
char* clarity_splash_status(void);
