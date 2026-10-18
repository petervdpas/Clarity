//! Clarity's own vault and connections: the secrets Clarity uses while you
//! design (the password of a database, the key of a web API) and the
//! connections that name them. They are Clarity's, in `~/.clarity/vault`
//! and `~/.clarity/connections.json`, never part of a project or of an
//! application Clarity generates. The vault locks itself after a while
//! without use.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use clarity_vault::{Connection, ConnectionField, Connections, DEFAULT_PARAMS, Params, SecretEntry, Secrets, Vault, VaultError};
use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;

/// How long the vault stays unlocked without being used.
const AUTO_LOCK: Duration = Duration::from_secs(15 * 60);

/// Where Clarity keeps its own vault and connections: `~/.clarity`, or
/// the folder the environment variable CLARITY_HOME names.
pub fn clarity_folder() -> PathBuf {
    if let Some(folder) = std::env::var_os("CLARITY_HOME").filter(|f| !f.is_empty()) {
        return PathBuf::from(folder);
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
    home.join(".clarity")
}

/// The vault as the screen shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct VaultInfo {
    /// The vault's folder.
    pub folder: String,
    /// Whether there is a vault yet; without one, the screen offers to make it.
    pub exists: bool,
    pub unlocked: bool,
}

/// One secret without its value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SecretRow {
    pub category: String,
    pub key: String,
    pub description: String,
    /// When it was made and when it last changed, in UTC.
    pub created: String,
    pub updated: String,
}

/// A secret as the screen saves it; `was` names the one it replaces when
/// its category or key changed.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct SecretEdit {
    pub was: Option<SecretName>,
    pub category: String,
    pub key: String,
    pub value: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SecretName {
    pub category: String,
    pub key: String,
}

/// One field of a connection: plain text, or a secret of the vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum FieldValue {
    Plain { value: String },
    Vault { category: String, key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ConnectionRow {
    pub name: String,
    /// The fields in the order they are written.
    pub fields: Vec<(String, FieldValue)>,
}

/// The vault and the connections, with the time the vault was last used.
pub struct Store {
    folder: PathBuf,
    secrets: Option<Secrets>,
    used: Instant,
}

impl Store {
    pub fn at(folder: impl AsRef<Path>) -> Store {
        Store { folder: folder.as_ref().to_path_buf(), secrets: None, used: Instant::now() }
    }

    fn vault_dir(&self) -> PathBuf {
        self.folder.join("vault")
    }

    fn connections_file(&self) -> PathBuf {
        self.folder.join("connections.json")
    }

    /// The vault's state; a vault unused for longer than AUTO_LOCK locks first.
    pub fn info(&mut self) -> VaultInfo {
        if self.used.elapsed() > AUTO_LOCK
            && let Some(secrets) = &mut self.secrets
        {
            secrets.vault_mut().lock();
        }
        let dir = self.vault_dir();
        VaultInfo {
            folder: dir.display().to_string(),
            exists: Vault::exists(&dir),
            unlocked: self.secrets.as_ref().is_some_and(|s| !s.vault().is_locked()),
        }
    }

    pub fn create(&mut self, password: &str, params: Params) -> Result<VaultInfo, String> {
        if password.is_empty() {
            return Err("a vault needs a master password".into());
        }
        let vault = Vault::create(self.vault_dir(), password, params).map_err(said)?;
        self.secrets = Some(Secrets::new(vault));
        self.used = Instant::now();
        Ok(self.info())
    }

    pub fn unlock(&mut self, password: &str) -> Result<VaultInfo, String> {
        let mut vault = Vault::open(self.vault_dir()).map_err(said)?;
        vault.unlock(password).map_err(said)?;
        self.secrets = Some(Secrets::new(vault));
        self.used = Instant::now();
        Ok(self.info())
    }

    pub fn lock(&mut self) -> VaultInfo {
        self.secrets = None;
        self.info()
    }

    /// The unlocked secrets, counting as a use.
    fn open(&mut self) -> Result<&mut Secrets, String> {
        if !self.info().unlocked {
            return Err(if Vault::exists(self.vault_dir()) { said(VaultError::Locked) } else { said(VaultError::NotFound) });
        }
        self.used = Instant::now();
        self.secrets.as_mut().ok_or_else(|| said(VaultError::Locked))
    }

    pub fn secrets(&mut self) -> Result<Vec<SecretRow>, String> {
        let entries = self.open()?.list().map_err(said)?;
        Ok(entries.into_iter().map(row).collect())
    }

    pub fn value(&mut self, name: &SecretName) -> Result<String, String> {
        self.open()?.get(&name.category, &name.key).map_err(said)
    }

    /// Adds a secret, or changes one: a new category or key moves it. A
    /// secret is never written over another one that is already there.
    pub fn save_secret(&mut self, edit: &SecretEdit) -> Result<Vec<SecretRow>, String> {
        let secrets = self.open()?;
        let same = |a: &str, b: &str| a.trim().eq_ignore_ascii_case(b.trim());
        let moved = edit.was.as_ref().is_none_or(|w| !same(&w.category, &edit.category) || !same(&w.key, &edit.key));
        if moved && secrets.get(&edit.category, &edit.key).is_ok() {
            return Err(format!("there is a secret {} / {} already", edit.category.trim(), edit.key.trim()));
        }
        secrets.put(&edit.category, &edit.key, &edit.value, &edit.description).map_err(said)?;
        if let Some(was) = edit.was.as_ref().filter(|_| moved) {
            secrets.delete(&was.category, &was.key).map_err(said)?;
        }
        self.secrets()
    }

    pub fn delete_secret(&mut self, name: &SecretName) -> Result<Vec<SecretRow>, String> {
        self.open()?.delete(&name.category, &name.key).map_err(said)?;
        self.secrets()
    }

    pub fn categories(&mut self) -> Result<Vec<String>, String> {
        self.open()?.categories().map_err(said)
    }

    /// Saves the list from the Categories dialog: each rename
    /// moves the secrets of the old name first, and a category that secrets
    /// still use cannot leave the list.
    pub fn save_categories(&mut self, names: &[String], renames: &[(String, String)]) -> Result<Vec<String>, String> {
        let secrets = self.open()?;
        let kept: Vec<String> = names.iter().map(|n| n.trim().to_lowercase()).collect();
        let renamed: Vec<String> = renames.iter().map(|(old, _)| old.trim().to_lowercase()).collect();
        let mut used: Vec<(String, usize)> = Vec::new();
        for entry in secrets.list().map_err(said)? {
            match used.iter_mut().find(|(c, _)| c.eq_ignore_ascii_case(&entry.category)) {
                Some((_, n)) => *n += 1,
                None => used.push((entry.category, 1)),
            }
        }
        for (category, count) in &used {
            let folded = category.to_lowercase();
            if !kept.contains(&folded) && !renamed.contains(&folded) {
                return Err(format!(
                    "Can't remove '{category}': {count} secret(s) still use it. Edit those secrets to reassign, then remove the category."
                ));
            }
        }
        for (old, new) in renames {
            secrets.rename_category(old, new).map_err(said)?;
        }
        secrets.set_categories(names).map_err(said)
    }

    /// A new master password; the current one is asked again first.
    pub fn change_password(&mut self, current: &str, new: &str, params: Params) -> Result<VaultInfo, String> {
        if new.is_empty() {
            return Err("a vault needs a master password".into());
        }
        Vault::open(self.vault_dir()).and_then(|mut v| v.unlock(current)).map_err(said)?;
        self.open()?.change_password(new, params).map_err(said)?;
        Ok(self.info())
    }

    /// Removes the vault and every secret in it, after its password is given.
    pub fn destroy(&mut self, password: &str) -> Result<VaultInfo, String> {
        Vault::open(self.vault_dir()).and_then(|mut v| v.unlock(password)).map_err(said)?;
        self.secrets = None;
        Vault::destroy(self.vault_dir()).map_err(said)?;
        Ok(self.info())
    }

    pub fn connections(&self) -> Result<Vec<ConnectionRow>, String> {
        let connections = Connections::load(self.connections_file()).map_err(said)?;
        Ok(connections.list().iter().map(connection_row).collect())
    }

    /// Adds a connection, or replaces one; `was` is its name before a rename.
    pub fn save_connection(&self, was: Option<&str>, row: &ConnectionRow) -> Result<Vec<ConnectionRow>, String> {
        let mut connections = Connections::load(self.connections_file()).map_err(said)?;
        let renamed = was.is_some_and(|w| !w.trim().eq_ignore_ascii_case(row.name.trim()));
        if (was.is_none() || renamed) && connections.get(&row.name).is_some() {
            return Err(format!("there is a connection {} already", row.name.trim()));
        }
        let fields = row.fields.iter().map(|(name, value)| {
            let field = match value {
                FieldValue::Plain { value } => ConnectionField::plain(value),
                FieldValue::Vault { category, key } => ConnectionField::vault(category, key),
            };
            (name.trim().to_string(), field)
        });
        connections.save(Connection::new(row.name.trim(), fields)).map_err(said)?;
        if let Some(was) = was.filter(|_| renamed) {
            connections.remove(was).map_err(said)?;
        }
        self.connections()
    }

    /// The value of field `key` of the connection named `connection`: plain
    /// text as written, a vault field from the vault (which must be unlocked).
    pub fn resolve(&mut self, connection: &str, key: &str) -> Result<String, String> {
        let connections = Connections::load(self.connections_file()).map_err(said)?;
        let found = connections.get(connection).ok_or_else(|| format!("there is no connection {connection} in the vault"))?;
        match found.field(key) {
            None => Ok(String::new()),
            Some(ConnectionField { value: Some(value), .. }) => Ok(value.clone()),
            Some(ConnectionField { from_vault: Some(r), .. }) => {
                let (category, key) = (r.category.clone(), r.key.clone());
                self.open()?.get(&category, &key).map_err(said)
            }
            Some(_) => Ok(String::new()),
        }
    }

    pub fn remove_connection(&self, name: &str) -> Result<Vec<ConnectionRow>, String> {
        Connections::load(self.connections_file()).and_then(|mut c| c.remove(name)).map_err(said)?;
        self.connections()
    }
}

/// The value of field `key` of a connection, for the rest of Clarity.
pub fn resolve(state: &VaultState, connection: &str, key: &str) -> Result<String, String> {
    with(state, |s| s.resolve(connection, key))
}

/// Clarity's vault and connections, for the commands.
pub struct VaultState(pub Mutex<Store>);

impl Default for VaultState {
    fn default() -> Self {
        VaultState(Mutex::new(Store::at(clarity_folder())))
    }
}

fn with<T>(state: &VaultState, run: impl FnOnce(&mut Store) -> Result<T, String>) -> Result<T, String> {
    let mut store = state.0.lock().map_err(|_| "the vault is not available".to_string())?;
    run(&mut store)
}

#[tauri::command]
pub fn vault_info(state: State<VaultState>) -> Result<VaultInfo, String> {
    with(&state, |s| Ok(s.info()))
}

// Deriving the key from the password takes a moment on purpose, so these
// run beside the window rather than in it.

#[tauri::command]
pub async fn vault_create(state: State<'_, VaultState>, password: String) -> Result<VaultInfo, String> {
    with(&state, |s| s.create(&password, DEFAULT_PARAMS))
}

#[tauri::command]
pub async fn vault_unlock(state: State<'_, VaultState>, password: String) -> Result<VaultInfo, String> {
    with(&state, |s| s.unlock(&password))
}

#[tauri::command]
pub async fn vault_change_password(state: State<'_, VaultState>, current: String, new: String) -> Result<VaultInfo, String> {
    with(&state, |s| s.change_password(&current, &new, DEFAULT_PARAMS))
}

#[tauri::command]
pub async fn vault_destroy(state: State<'_, VaultState>, password: String) -> Result<VaultInfo, String> {
    with(&state, |s| s.destroy(&password))
}

#[tauri::command]
pub fn vault_lock(state: State<VaultState>) -> Result<VaultInfo, String> {
    with(&state, |s| Ok(s.lock()))
}

#[tauri::command]
pub fn vault_secrets(state: State<VaultState>) -> Result<Vec<SecretRow>, String> {
    with(&state, |s| s.secrets())
}

#[tauri::command]
pub fn vault_value(state: State<VaultState>, name: SecretName) -> Result<String, String> {
    with(&state, |s| s.value(&name))
}

#[tauri::command]
pub fn vault_save_secret(state: State<VaultState>, edit: SecretEdit) -> Result<Vec<SecretRow>, String> {
    with(&state, |s| s.save_secret(&edit))
}

#[tauri::command]
pub fn vault_delete_secret(state: State<VaultState>, name: SecretName) -> Result<Vec<SecretRow>, String> {
    with(&state, |s| s.delete_secret(&name))
}

#[tauri::command]
pub fn vault_categories(state: State<VaultState>) -> Result<Vec<String>, String> {
    with(&state, |s| s.categories())
}

/// A rename the Categories dialog made: the old name and the new one.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct CategoryRename {
    pub from: String,
    pub to: String,
}

#[tauri::command]
pub fn vault_save_categories(state: State<VaultState>, names: Vec<String>, renames: Vec<CategoryRename>) -> Result<Vec<String>, String> {
    let renames: Vec<(String, String)> = renames.into_iter().map(|r| (r.from, r.to)).collect();
    with(&state, |s| s.save_categories(&names, &renames))
}

#[tauri::command]
pub fn connections_list(state: State<VaultState>) -> Result<Vec<ConnectionRow>, String> {
    with(&state, |s| s.connections())
}

#[tauri::command]
pub fn connection_save(state: State<VaultState>, was: Option<String>, connection: ConnectionRow) -> Result<Vec<ConnectionRow>, String> {
    with(&state, |s| s.save_connection(was.as_deref(), &connection))
}

#[tauri::command]
pub fn connection_remove(state: State<VaultState>, name: String) -> Result<Vec<ConnectionRow>, String> {
    with(&state, |s| s.remove_connection(&name))
}

fn said(err: VaultError) -> String {
    err.to_string()
}

fn row(entry: SecretEntry) -> SecretRow {
    SecretRow { category: entry.category, key: entry.key, description: entry.description, created: entry.created_utc, updated: entry.updated_utc }
}

fn connection_row(c: &Connection) -> ConnectionRow {
    let fields = c
        .fields
        .iter()
        .map(|(name, field)| {
            let value = match (&field.value, &field.from_vault) {
                (_, Some(r)) => FieldValue::Vault { category: r.category.clone(), key: r.key.clone() },
                (value, None) => FieldValue::Plain { value: value.clone().unwrap_or_default() },
            };
            (name.clone(), value)
        })
        .collect();
    ConnectionRow { name: c.name.clone(), fields }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAST: Params = Params { memory_kib: 1024, iterations: 1, parallelism: 1 };

    fn store(name: &str) -> (Store, PathBuf) {
        let folder = std::env::temp_dir().join(format!("clarity-own-vault-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        (Store::at(&folder), folder)
    }

    fn name(category: &str, key: &str) -> SecretName {
        SecretName { category: category.into(), key: key.into() }
    }

    #[test]
    fn clarity_makes_unlocks_and_locks_its_vault() {
        let (mut store, folder) = store("life");
        let info = store.info();
        assert!(!info.exists && !info.unlocked);
        assert_eq!(info.folder, folder.join("vault").display().to_string());
        assert!(store.secrets().is_err(), "no vault, no secrets");

        assert!(store.create("pw", FAST).unwrap().unlocked);
        assert!(store.create("again", FAST).is_err(), "a vault is never made over another");
        assert!(store.lock().exists);
        assert_eq!(store.unlock("wrong").unwrap_err(), "the master password is not right");
        assert!(store.unlock("pw").unwrap().unlocked);

        // Unused for longer than the auto-lock, it locks itself.
        store.used = Instant::now() - AUTO_LOCK - Duration::from_secs(1);
        assert!(!store.info().unlocked);
        assert_eq!(store.secrets().unwrap_err(), "the vault is locked");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn secrets_are_added_renamed_shown_and_removed() {
        let (mut store, folder) = store("secrets");
        store.create("pw", FAST).unwrap();
        let edit = |was: Option<SecretName>, category: &str, key: &str, value: &str| SecretEdit {
            was,
            category: category.into(),
            key: key.into(),
            value: value.into(),
            description: "the shop database".into(),
        };
        let rows = store.save_secret(&edit(None, "Postgres", "password", "pg")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].description, "the shop database");
        assert!(!rows[0].updated.is_empty());
        // A new category or key moves the secret rather than copying it.
        let rows = store.save_secret(&edit(Some(name("Postgres", "password")), "Shop", "password", "pg2")).unwrap();
        assert_eq!(rows.iter().map(|r| r.category.as_str()).collect::<Vec<_>>(), ["Shop"]);
        assert_eq!(store.value(&name("shop", "password")).unwrap(), "pg2");
        // A secret is never added over another by mistake.
        assert!(store.save_secret(&edit(None, "Shop", "password", "x")).is_err());

        // A category secrets use cannot go,
        let refused = store.save_categories(&["Anthropic".into()], &[]).unwrap_err();
        assert_eq!(refused, "Can't remove 'Shop': 1 secret(s) still use it. Edit those secrets to reassign, then remove the category.");
        // a rename moves its secrets, and the list is kept as given.
        assert_eq!(store.save_categories(&["Anthropic".into(), "Store".into()], &[("Shop".into(), "Store".into())]).unwrap(), ["Anthropic", "Store"]);
        assert_eq!(store.value(&name("Store", "password")).unwrap(), "pg2");
        assert!(store.delete_secret(&name("Store", "password")).unwrap().is_empty());
        // A category in the kept list stays when its last secret goes.
        assert_eq!(store.categories().unwrap(), ["Anthropic", "Store"]);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_password_changes_and_the_vault_goes_only_with_its_password() {
        let (mut store, folder) = store("rekey");
        store.create("old", FAST).unwrap();
        store.save_secret(&SecretEdit { was: None, category: "Mail".into(), key: "smtp".into(), value: "m".into(), description: String::new() }).unwrap();
        assert!(store.change_password("wrong", "new", FAST).is_err());
        store.change_password("old", "new", FAST).unwrap();
        store.lock();
        assert!(store.unlock("old").is_err());
        store.unlock("new").unwrap();
        assert_eq!(store.value(&name("Mail", "smtp")).unwrap(), "m");

        assert!(store.destroy("old").is_err());
        assert!(!store.destroy("new").unwrap().exists);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn connections_are_saved_renamed_and_removed() {
        let (mut store, folder) = store("connections");
        assert!(store.connections().unwrap().is_empty());
        let row = ConnectionRow {
            name: "shop-db".into(),
            fields: vec![
                ("host".into(), FieldValue::Plain { value: "localhost".into() }),
                ("password".into(), FieldValue::Vault { category: "Postgres".into(), key: "password".into() }),
            ],
        };
        let rows = store.save_connection(None, &row).unwrap();
        assert_eq!(rows, std::slice::from_ref(&row));
        // Renaming replaces the connection under its old name.
        let renamed = ConnectionRow { name: "shop".into(), ..row };
        let rows = store.save_connection(Some("shop-db"), &renamed).unwrap();
        assert_eq!(rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["shop"]);
        assert!(store.save_connection(None, &ConnectionRow { name: " ".into(), fields: vec![] }).is_err());
        // Clarity reads a field: plain as written, from the vault when unlocked.
        assert_eq!(store.resolve("shop", "host").unwrap(), "localhost");
        assert_eq!(store.resolve("shop", "port").unwrap(), "");
        assert_eq!(store.resolve("shop", "password").unwrap_err(), "there is no vault yet");
        store.create("pw", FAST).unwrap();
        store.save_secret(&SecretEdit { was: None, category: "Postgres".into(), key: "password".into(), value: "pg".into(), description: String::new() }).unwrap();
        assert_eq!(store.resolve("shop", "password").unwrap(), "pg");
        store.lock();
        assert_eq!(store.resolve("shop", "password").unwrap_err(), "the vault is locked");
        assert!(store.resolve("nowhere", "host").is_err());
        assert!(store.remove_connection("shop").unwrap().is_empty());
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
