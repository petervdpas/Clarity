// What the splash screen says is loading (splash.status = @loadingData):
// code sets it while the app's on start runs, and the screen asks for it a
// few times a second until the main window shows. It is a text as a message
// is one, so the screen shows a key in the person's language.

#pragma once

#include <optional>

#include "clarity/text.hpp"

namespace clarity::splash {

/// Code says what loads now.
void set_status(const Text& text);
/// What code last said loads; none while it said nothing.
std::optional<Text> status();

}  // namespace clarity::splash
