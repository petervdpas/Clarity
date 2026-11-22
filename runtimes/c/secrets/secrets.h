// Secret settings (manual/data.settings.md) in the system's protected store, per person
// and per application, never in settings.json: the Credential Manager on
// Windows, the Keychain on macOS, the Secret Service over D-Bus elsewhere
// (GNOME Keyring, KWallet). An entry is the program's name with the
// setting's, as the other stacks keep it.

#pragma once

#include <stdbool.h>

/// The secret `name` of `program` in the scratch arena; NULL when it was
/// never kept or the store cannot be reached.
const char* clarity_secret_get(const char* program, const char* name);
/// Keeps `value` as the secret; "" forgets it. False with an error when the store refuses.
bool clarity_secret_set(const char* program, const char* name, const char* value);
/// Makes the store the one settings and web API calls use (clarity_secret_store).
void clarity_secrets_use(void);
/// The Credential Manager's target of the secret `name` of `program` on
/// Windows, as every stack keeps it (conformance/secret-store.json): the
/// program's name, a colon and the setting's. In the scratch arena.
const char* clarity_secret_windows_target(const char* program, const char* name);
