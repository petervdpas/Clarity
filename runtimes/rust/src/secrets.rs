//! Secret settings (DESIGN 5.27): kept in the system's protected store, per
//! person and per application, never in settings.json. Windows keeps them in
//! the Credential Manager, macOS in the Keychain, other systems in the Secret
//! Service (GNOME Keyring, KWallet). An entry is the application's program
//! name with the setting's name.

/// The secret `name` of `program`; none when it was never kept, or the store
/// cannot be reached.
pub fn get(program: &str, name: &str) -> Option<String> {
    keyring::Entry::new(program, name).ok()?.get_password().ok()
}

/// Keeps `value` as the secret `name` of `program`; empty text forgets it.
pub fn set(program: &str, name: &str, value: &str) -> crate::Result<()> {
    let entry = keyring::Entry::new(program, name).map_err(|err| crate::Error::Invalid(format!("the system's protected store: {err}")))?;
    let done = if value.is_empty() {
        match entry.delete_credential() {
            Err(keyring::Error::NoEntry) => Ok(()),
            other => other,
        }
    } else {
        entry.set_password(value)
    };
    done.map_err(|err| crate::Error::Invalid(format!("cannot keep the secret '{name}' in the system's protected store: {err}")))
}
