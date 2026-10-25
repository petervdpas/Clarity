// Secret settings (DESIGN 5.27) in the system's protected store, per person
// and per application, never in settings.json: the Credential Manager on
// Windows, the Keychain on macOS, the Secret Service over D-Bus elsewhere
// (GNOME Keyring, KWallet). An entry is the program's name with the
// setting's, as the other stacks keep it.

#pragma once

#include <optional>
#include <string>

namespace clarity {

/// The secret `name` of `program`; none when it was never kept or the store cannot be reached.
std::optional<std::string> secret_get(const std::string& program, const std::string& name);
/// Keeps `value` as the secret; "" forgets it. Throws a StorageError when the store refuses.
void secret_set(const std::string& program, const std::string& name, const std::string& value);
/// Makes the store the one settings and web API calls use (secret_store).
void secrets_use();

}  // namespace clarity
