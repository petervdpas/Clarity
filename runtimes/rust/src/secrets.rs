//! Secret settings (DESIGN 5.27): kept in the system's protected store, per
//! person and per application, never in settings.json. Windows keeps them in
//! the Credential Manager, macOS in the Keychain, other systems in the Secret
//! Service (GNOME Keyring, KWallet). An entry is the application's program
//! name with the setting's name. On Windows it is kept as every stack keeps
//! it (conformance/secret-store.json): a generic credential with the target
//! `program:name`, the value's UTF-8 bytes, and local-machine persistence,
//! written through Win32 itself; keyring would write enterprise persistence,
//! which roams with the person's account, so a secret forgotten here could
//! come back from it.

/// The secret `name` of `program`; none when it was never kept, or the store
/// cannot be reached.
pub fn get(program: &str, name: &str) -> Option<String> {
    #[cfg(windows)]
    {
        from_windows_blob(&credentials::read(&windows_target(program, name))?)
    }
    #[cfg(not(windows))]
    {
        keyring::Entry::new(program, name).ok()?.get_password().ok()
    }
}

/// Keeps `value` as the secret `name` of `program`; empty text forgets it.
pub fn set(program: &str, name: &str, value: &str) -> crate::Result<()> {
    #[cfg(windows)]
    let done = {
        let target = windows_target(program, name);
        if value.is_empty() { credentials::delete(&target) } else { credentials::write(&target, name, &windows_blob(value)) }
    };
    #[cfg(not(windows))]
    let done = {
        let entry = keyring::Entry::new(program, name).map_err(|err| crate::Error::Invalid(format!("the system's protected store: {err}")))?;
        let done = if value.is_empty() {
            match entry.delete_credential() {
                Err(keyring::Error::NoEntry) => Ok(()),
                other => other,
            }
        } else {
            entry.set_password(value)
        };
        done.map_err(|err| err.to_string())
    };
    done.map_err(|err| crate::Error::Invalid(format!("cannot keep the secret '{name}' in the system's protected store: {err}")))
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

/// How the Credential Manager keeps a secret on every stack: for this
/// machine, never roaming with the account (`CRED_PERSIST_LOCAL_MACHINE`).
pub const WINDOWS_PERSIST: &str = "local machine";

/// The Credential Manager through Win32: generic credentials, kept for this machine.
#[cfg(windows)]
mod credentials {
    use windows_sys::Win32::Foundation::{ERROR_NOT_FOUND, GetLastError};
    use windows_sys::Win32::Security::Credentials::{CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree, CredReadW, CredWriteW};

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// The blob of the generic credential `target`; none when there is none.
    pub fn read(target: &str) -> Option<Vec<u8>> {
        let target = wide(target);
        let mut found: *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: `target` is a NUL-terminated UTF-16 string that outlives the
        // call; on success `found` points at a credential Windows allocated,
        // read before CredFree gives it back.
        unsafe {
            if CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut found) == 0 {
                return None;
            }
            let credential = &*found;
            let blob = if credential.CredentialBlob.is_null() { Vec::new() } else { std::slice::from_raw_parts(credential.CredentialBlob, credential.CredentialBlobSize as usize).to_vec() };
            CredFree(found as *const _);
            Some(blob)
        }
    }

    /// Writes the generic credential `target`, its user `user` and blob `blob`.
    pub fn write(target: &str, user: &str, blob: &[u8]) -> Result<(), String> {
        let target = wide(target);
        let user = wide(user);
        let mut blob = blob.to_vec();
        // SAFETY: an all-zero CREDENTIALW is valid (no comment, attributes or
        // alias); every pointer set below outlives the call.
        let mut credential: CREDENTIALW = unsafe { std::mem::zeroed() };
        credential.Type = CRED_TYPE_GENERIC;
        credential.TargetName = target.as_ptr() as *mut u16;
        credential.UserName = user.as_ptr() as *mut u16;
        credential.CredentialBlobSize = blob.len() as u32;
        credential.CredentialBlob = blob.as_mut_ptr();
        credential.Persist = CRED_PERSIST_LOCAL_MACHINE;
        // SAFETY: `credential` is filled in as above.
        if unsafe { CredWriteW(&credential, 0) } == 0 {
            // SAFETY: reads the calling thread's last error.
            return Err(format!("the Credential Manager refused it (error {})", unsafe { GetLastError() }));
        }
        Ok(())
    }

    /// Deletes the generic credential `target`; none to delete is no error.
    pub fn delete(target: &str) -> Result<(), String> {
        let target = wide(target);
        // SAFETY: `target` is a NUL-terminated UTF-16 string that outlives the call.
        if unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0 {
            // SAFETY: reads the calling thread's last error.
            let error = unsafe { GetLastError() };
            if error != ERROR_NOT_FOUND {
                return Err(format!("the Credential Manager refused to forget it (error {error})"));
            }
        }
        Ok(())
    }
}
