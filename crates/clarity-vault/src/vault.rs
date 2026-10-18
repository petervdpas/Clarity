//! The vault folder: a plain name and value store,
//! encrypted. The key comes from the master password through Argon2id and
//! lives only in memory, zeroed when the vault locks. Each value is sealed
//! with AES-256-GCM under a fresh nonce, bound to its vault and its name by
//! the additional data `domain || vaultId || name`, so a record copied
//! from another vault or renamed in this one does not open.

use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

use crate::error::{Result, VaultError};

const HEADER_FILE: &str = "vault.json";
const SECRETS_DIR: &str = "secrets";
const SECRET_EXT: &str = ".secret";
const FORMAT_VERSION: u32 = 1;
const AES_GCM: &str = "aes-256-gcm";
const ARGON2ID: &str = "argon2id";
/// The fixed first part of every record's additional data. It is part of
/// the format: another value would make every existing vault unreadable.
const DOMAIN: &str = "Clarity";
const CANARY_SLOT: &str = "canary";
const CANARY: &[u8] = b"canary-v1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;

/// The Argon2id work factors of a vault. They are in its header, so a vault
/// opens with its own, whatever created it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Params {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

/// The work factors of a new vault: 64 MiB, three passes, four lanes.
pub const DEFAULT_PARAMS: Params = Params { memory_kib: 65536, iterations: 3, parallelism: 4 };

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Header {
    version: u32,
    #[serde(rename = "vaultId")]
    vault_id: String,
    #[serde(rename = "createdUtc")]
    created_utc: String,
    kdf: Kdf,
    canary: Sealed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Kdf {
    algorithm: String,
    #[serde(rename = "memoryKiB")]
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
    salt: String,
}

/// One AES-GCM payload: the header's canary, or with its version,
/// algorithm and time, a secret's record.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Sealed {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    algorithm: Option<String>,
    nonce: String,
    ciphertext: String,
    tag: String,
    #[serde(rename = "updatedUtc", default, skip_serializing_if = "Option::is_none")]
    updated_utc: Option<String>,
}

/// A vault folder, locked or unlocked.
pub struct Vault {
    dir: PathBuf,
    header: Header,
    key: Option<Zeroizing<[u8; KEY_LEN]>>,
}

impl Vault {
    /// Whether `dir` holds a vault.
    pub fn exists(dir: impl AsRef<Path>) -> bool {
        dir.as_ref().join(HEADER_FILE).is_file()
    }

    /// Makes a new vault in `dir` and gives it back unlocked. A vault that is
    /// there already is never overwritten.
    pub fn create(dir: impl AsRef<Path>, password: &str, params: Params) -> Result<Vault> {
        let dir = dir.as_ref().to_path_buf();
        if Vault::exists(&dir) {
            return Err(VaultError::Exists);
        }
        std::fs::create_dir_all(dir.join(SECRETS_DIR))?;
        let salt: [u8; SALT_LEN] = random()?;
        let vault_id = uuid::Uuid::new_v4().simple().to_string();
        let key = derive(password, &salt, params)?;
        let canary = seal(&key, &vault_id, CANARY_SLOT, CANARY)?;
        let header = Header {
            version: FORMAT_VERSION,
            vault_id,
            created_utc: now(),
            kdf: Kdf {
                algorithm: ARGON2ID.into(),
                memory_kib: params.memory_kib,
                iterations: params.iterations,
                parallelism: params.parallelism,
                salt: BASE64.encode(salt),
            },
            canary,
        };
        write_json(&dir.join(HEADER_FILE), &header)?;
        Ok(Vault { dir, header, key: Some(key) })
    }

    /// Opens the vault in `dir`, locked.
    pub fn open(dir: impl AsRef<Path>) -> Result<Vault> {
        let dir = dir.as_ref().to_path_buf();
        if !Vault::exists(&dir) {
            return Err(VaultError::NotFound);
        }
        let header: Header = read_json(&dir.join(HEADER_FILE))?;
        if header.vault_id.is_empty() {
            return Err(VaultError::Corrupt("the header has no vault id".into()));
        }
        Ok(Vault { dir, header, key: None })
    }

    /// Unlocks with the master password, checked against the canary.
    pub fn unlock(&mut self, password: &str) -> Result<()> {
        if self.key.is_some() {
            return Ok(());
        }
        let kdf = &self.header.kdf;
        if kdf.algorithm != ARGON2ID {
            return Err(VaultError::Corrupt(format!("the header names the key derivation {}", kdf.algorithm)));
        }
        let salt = BASE64.decode(&kdf.salt).map_err(|_| VaultError::Corrupt("the salt is no base64".into()))?;
        let params = Params { memory_kib: kdf.memory_kib, iterations: kdf.iterations, parallelism: kdf.parallelism };
        let key = derive(password, &salt, params)?;
        match open(&key, &self.header.vault_id, CANARY_SLOT, &self.header.canary)? {
            Some(_) => {
                self.key = Some(key);
                Ok(())
            }
            None => Err(VaultError::InvalidPassword),
        }
    }

    /// Forgets the key; nothing reads or writes until the vault is unlocked again.
    pub fn lock(&mut self) {
        self.key = None;
    }

    pub fn is_locked(&self) -> bool {
        self.key.is_none()
    }

    /// The folder the vault is in.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The value stored under `name`.
    pub fn get(&self, name: &str) -> Result<String> {
        let key = self.key()?;
        check_name(name)?;
        let path = self.secret_path(name);
        let sealed: Sealed = match read_json(&path) {
            Err(VaultError::Io(_)) if !path.exists() => return Err(VaultError::SecretNotFound),
            other => other?,
        };
        if sealed.algorithm.as_deref().is_some_and(|a| a != AES_GCM) {
            return Err(VaultError::Corrupt(format!("{name} is sealed with another algorithm")));
        }
        let plain = open(key, &self.header.vault_id, name, &sealed)?
            .ok_or_else(|| VaultError::Corrupt(format!("{name} does not belong to this vault")))?;
        String::from_utf8(plain.to_vec()).map_err(|_| VaultError::Corrupt(format!("{name} is no text")))
    }

    /// Stores `value` under `name`, replacing what was there.
    pub fn set(&self, name: &str, value: &str) -> Result<()> {
        let key = self.key()?;
        check_name(name)?;
        let mut sealed = seal(key, &self.header.vault_id, name, value.as_bytes())?;
        sealed.version = Some(FORMAT_VERSION);
        sealed.algorithm = Some(AES_GCM.into());
        sealed.updated_utc = Some(now());
        write_json(&self.secret_path(name), &sealed)
    }

    /// Removes what is stored under `name`.
    pub fn delete(&self, name: &str) -> Result<()> {
        self.key()?;
        check_name(name)?;
        match std::fs::remove_file(self.secret_path(name)) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(VaultError::SecretNotFound),
            other => Ok(other?),
        }
    }

    /// The names stored, sorted. Names are file names, so this needs no key.
    pub fn list(&self) -> Result<Vec<String>> {
        let folder = self.dir.join(SECRETS_DIR);
        let mut names = Vec::new();
        let entries = match std::fs::read_dir(&folder) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(names),
            other => other?,
        };
        for entry in entries {
            let name = entry?.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.strip_suffix(SECRET_EXT) {
                names.push(stem.to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    /// Removes the vault: its header and every record. Other files in the folder stay.
    pub fn destroy(dir: impl AsRef<Path>) -> Result<()> {
        let dir = dir.as_ref();
        if !Vault::exists(dir) {
            return Err(VaultError::NotFound);
        }
        let folder = dir.join(SECRETS_DIR);
        if folder.is_dir() {
            for entry in std::fs::read_dir(&folder)? {
                let path = entry?.path();
                if path.extension().is_some_and(|e| e == "secret") {
                    std::fs::remove_file(path)?;
                }
            }
            let _ = std::fs::remove_dir(&folder);
        }
        std::fs::remove_file(dir.join(HEADER_FILE))?;
        Ok(())
    }

    fn key(&self) -> Result<&[u8; KEY_LEN]> {
        self.key.as_deref().ok_or(VaultError::Locked)
    }

    fn secret_path(&self, name: &str) -> PathBuf {
        self.dir.join(SECRETS_DIR).join(format!("{name}{SECRET_EXT}"))
    }
}

/// A name usable as a file name everywhere: letters, digits, `-`, `_` and `.`, not only dots.
fn check_name(name: &str) -> Result<()> {
    let fine = !name.trim().is_empty()
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !name.chars().all(|c| c == '.');
    if fine { Ok(()) } else { Err(VaultError::InvalidName) }
}

fn random<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|err| VaultError::Io(format!("no random bytes: {err}")))?;
    Ok(bytes)
}

/// The key from the password, normalised to NFC first so the same
/// characters typed on any system give the same key.
fn derive(password: &str, salt: &[u8], params: Params) -> Result<Zeroizing<[u8; KEY_LEN]>> {
    let normalised = Zeroizing::new(password.nfc().collect::<String>());
    let argon_params = argon2::Params::new(params.memory_kib, params.iterations, params.parallelism.max(1), Some(KEY_LEN))
        .map_err(|err| VaultError::Corrupt(format!("the work factors do not fit: {err}")))?;
    let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, argon_params);
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon
        .hash_password_into(normalised.as_bytes(), salt, key.as_mut())
        .map_err(|err| VaultError::Corrupt(format!("the key cannot be derived: {err}")))?;
    Ok(key)
}

fn additional_data(vault_id: &str, slot: &str) -> Vec<u8> {
    [DOMAIN.as_bytes(), vault_id.as_bytes(), slot.as_bytes()].concat()
}

/// Seals `plain`; the format keeps the tag apart from the ciphertext.
fn seal(key: &[u8; KEY_LEN], vault_id: &str, slot: &str, plain: &[u8]) -> Result<Sealed> {
    let nonce: [u8; NONCE_LEN] = random()?;
    let cipher = Aes256Gcm::new(key.into());
    let aad = additional_data(vault_id, slot);
    let out = cipher
        .encrypt(Nonce::from_slice(&nonce), Payload { msg: plain, aad: &aad })
        .map_err(|_| VaultError::Corrupt("the value cannot be sealed".into()))?;
    let (ciphertext, tag) = out.split_at(out.len() - TAG_LEN);
    Ok(Sealed {
        nonce: BASE64.encode(nonce),
        ciphertext: BASE64.encode(ciphertext),
        tag: BASE64.encode(tag),
        ..Sealed::default()
    })
}

/// Opens a sealed value; None when it does not authenticate (a wrong key,
/// or a record of another vault or name).
fn open(key: &[u8; KEY_LEN], vault_id: &str, slot: &str, sealed: &Sealed) -> Result<Option<Zeroizing<Vec<u8>>>> {
    let field = |text: &str, what: &str, len: Option<usize>| {
        let bytes = BASE64.decode(text).map_err(|_| VaultError::Corrupt(format!("the {what} of {slot} is no base64")))?;
        match len {
            Some(n) if bytes.len() != n => Err(VaultError::Corrupt(format!("the {what} of {slot} has the wrong length"))),
            _ => Ok(bytes),
        }
    };
    let nonce = field(&sealed.nonce, "nonce", Some(NONCE_LEN))?;
    let tag = field(&sealed.tag, "tag", Some(TAG_LEN))?;
    let mut message = field(&sealed.ciphertext, "ciphertext", None)?;
    message.extend_from_slice(&tag);
    let aad = additional_data(vault_id, slot);
    let cipher = Aes256Gcm::new(key.into());
    Ok(cipher.decrypt(Nonce::from_slice(&nonce), Payload { msg: &message, aad: &aad }).ok().map(Zeroizing::new))
}

/// The time now, in UTC, as .NET and Go both read it.
pub(crate) fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}

pub(crate) fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|err| VaultError::Corrupt(format!("{} is no valid JSON: {err}", path.display())))
}

/// Writes indented JSON through a file beside it and a rename, so a reader
/// never sees half a record; the file is for its owner only.
pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder)?;
    let text = serde_json::to_string_pretty(value).map_err(|err| VaultError::Io(err.to_string()))?;
    let temp = folder.join(format!(
        "{}.{}.tmp",
        path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::write(&temp, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&temp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })?;
    Ok(())
}
