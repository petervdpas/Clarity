//! What goes wrong with a vault, each kind its own so a screen can tell
//! "unlock first" from "nothing stored yet".

use std::fmt;

#[derive(Debug)]
pub enum VaultError {
    /// There is no vault in the folder yet.
    NotFound,
    /// The folder holds a vault already; creating one would lose its secrets.
    Exists,
    /// The vault is locked: unlock it with the master password first.
    Locked,
    /// The master password does not open the vault.
    InvalidPassword,
    /// A name or category that cannot be stored: empty, or with characters a file name cannot hold.
    InvalidName,
    /// Nothing is stored under the name asked for.
    SecretNotFound,
    /// A file of the vault cannot be read as what it should be.
    Corrupt(String),
    /// The disk failed.
    Io(String),
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaultError::NotFound => write!(f, "there is no vault yet"),
            VaultError::Exists => write!(f, "a vault exists there already"),
            VaultError::Locked => write!(f, "the vault is locked"),
            VaultError::InvalidPassword => write!(f, "the master password is not right"),
            VaultError::InvalidName => write!(f, "that name cannot be stored in the vault"),
            VaultError::SecretNotFound => write!(f, "no secret is stored under that name"),
            VaultError::Corrupt(what) => write!(f, "the vault is damaged: {what}"),
            VaultError::Io(what) => write!(f, "{what}"),
        }
    }
}

impl std::error::Error for VaultError {}

impl From<std::io::Error> for VaultError {
    fn from(err: std::io::Error) -> Self {
        VaultError::Io(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, VaultError>;
