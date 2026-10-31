// The dropdown's options of U1c, found by the runtime
// (conformance/dropdown-options.json). A `typed` dropdown offers the values
// its field already has: each once, without empty text, those containing
// the typed text (ignoring case), sorted ignoring case and then as written.
// A `search` dropdown offers the records whose text or shown fields hold
// every typed word, ignoring case, in the order they came.

#pragma once

#include <string>
#include <vector>

#include "clarity/ui.hpp"

namespace clarity {

/// The suggestions of a typed dropdown for what is typed.
std::vector<std::string> suggestions(const std::vector<std::string>& values, const std::string& typed);

/// The records a search dropdown offers for what is typed, matched on their
/// text and the fields in `show` (every field when it names none).
std::vector<RefOption> searched(const std::vector<RefOption>& options, const std::string& typed, const std::vector<std::string>& show);

}  // namespace clarity
