//! Clarity's vault of secrets and its connections.
//!
//! The vault is a folder: `vault.json` (the vault's id, the Argon2id work
//! factors and salt, and a canary that proves the master password) and one
//! AES-256-GCM record per secret in `secrets/`, each holding its category,
//! key and value.
//!
//! A connection is a named set of fields, each plain text or a pointer to a
//! secret by category and key, kept in `connections.json` beside the vault.
//! Clarity's own vault and connections are Clarity's; an application
//! Clarity generates has its own, only when its design adds them.

mod connections;
mod error;
mod secrets;
mod vault;

pub use connections::{Connection, ConnectionField, Connections, VaultRef};
pub use error::{Result, VaultError};
pub use secrets::{SecretEntry, Secrets};
pub use vault::{DEFAULT_PARAMS, Params, Vault};
