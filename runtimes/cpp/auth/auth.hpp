// Signing people in through Clarity.Auth (docs/APP-CAPABILITIES-PLAN.md,
// step 4), the one implementation every stack calls through its C table
// (clarity_auth.h). Carried only when the project has a `signin` block
// (clarity_use_auth in clarity.cmake links the library). Db::add_signin,
// declared in clarity/db.hpp, is defined here; what the service, the web
// server and the command line need of it is in clarity/people.hpp.

#pragma once

#include <cstdint>
#include <optional>
#include <string>

#include <nlohmann/json.hpp>

#include "clarity/db.hpp"

namespace clarity {

/// Text assigned to a `hashed` field: what is stored, by the field's method
/// (`argon2id`, `sha256` for a token field, `sha256-lines` for recovery
/// codes). With a `minimum` (the strategy's `minimum N`), text that does not
/// fit is refused first with secretTooShort, its slot `minimum`.
std::string hashed(const std::string& method, const std::string& text, std::optional<int64_t> minimum = std::nullopt);

/// `newRecoveryCodes(count)` in code: the codes as lines of text, shown once.
std::string new_recovery_codes(int64_t count);

/// `newToken()` in code.
std::string new_token();

/// One operation of Clarity.Auth by its name, its arguments and result as
/// JSON. Throws a ValueError with the library's problem when it fails.
nlohmann::json auth_call(const std::string& op, const nlohmann::json& args);

}  // namespace clarity
