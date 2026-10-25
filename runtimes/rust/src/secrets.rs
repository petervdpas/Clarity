//! Secret settings (DESIGN 5.27): kept in the system's protected store, per
//! person and per application, never in settings.json. Windows keeps them in
//! the Credential Manager, macOS in the Keychain, other systems in the Secret
//! Service (GNOME Keyring, KWallet). An entry is the application's program
//! name with the setting's name. On Windows it is kept as every stack keeps
//! it (conformance/secret-store.json): the target `program:name` and the
//! value's UTF-8 bytes, not keyring's own `name.program` and UTF-16.

/// The secret `name` of `program`; none when it was never kept, or the store
/// cannot be reached.
pub fn get(program: &str, name: &str) -> Option<String> {
    let entry = entry(program, name).ok()?;
    if cfg!(windows) { from_windows_blob(&entry.get_secret().ok()?) } else { entry.get_password().ok() }
}

/// Keeps `value` as the secret `name` of `program`; empty text forgets it.
pub fn set(program: &str, name: &str, value: &str) -> crate::Result<()> {
    let entry = entry(program, name).map_err(|err| crate::Error::Invalid(format!("the system's protected store: {err}")))?;
    let done = if value.is_empty() {
        match entry.delete_credential() {
            Err(keyring::Error::NoEntry) => Ok(()),
            other => other,
        }
    } else if cfg!(windows) {
        entry.set_secret(&windows_blob(value))
    } else {
        entry.set_password(value)
    };
    done.map_err(|err| crate::Error::Invalid(format!("cannot keep the secret '{name}' in the system's protected store: {err}")))
}

fn entry(program: &str, name: &str) -> keyring::Result<keyring::Entry> {
    if cfg!(windows) { keyring::Entry::new_with_target(&windows_target(program, name), program, name) } else { keyring::Entry::new(program, name) }
}

/// The Credential Manager's target of the secret `name` of `program`.
pub fn windows_target(program: &str, name: &str) -> String {
    format!("{program}:{name}")
}

/// What the Credential Manager keeps of a secret: its UTF-8 bytes.
pub fn windows_blob(value: &str) -> Vec<u8> {
    value.as_bytes().to_vec()
}

/// The secret a Credential Manager blob holds; none when it is not UTF-8.
pub fn from_windows_blob(blob: &[u8]) -> Option<String> {
    String::from_utf8(blob.to_vec()).ok()
}
