//! Secrets by category and key, on top of the vault: each secret sits under an opaque name of 32 hex digits, with its
//! category, key, value and description inside the sealed record, so the
//! folder says nothing about what it holds. The list of categories is a
//! record too, under a reserved name. Matching ignores case and the space
//! around a name.

use serde::{Deserialize, Serialize};

use crate::error::{Result, VaultError};
use crate::vault::{Params, Vault, now};

const SCHEMA_VERSION: u32 = 1;
/// Where the list of categories is kept; it is no secret.
const CATEGORY_SLOT: &str = "00000000000000000000000000000001";

/// One secret's identity and description, without its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretEntry {
    /// The name of its record in the vault.
    pub slot: String,
    pub category: String,
    pub key: String,
    pub description: String,
    pub created_utc: String,
    pub updated_utc: String,
}

/// A secret's record: PascalCase names, and an
/// explicit null for no description.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Envelope {
    schema_version: u32,
    category: String,
    key: String,
    value: String,
    description: Option<String>,
    #[serde(rename = "CreatedUtc")]
    created_utc: String,
    #[serde(rename = "UpdatedUtc")]
    updated_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CategoryList {
    schema_version: u32,
    categories: Vec<String>,
    #[serde(rename = "UpdatedUtc")]
    updated_utc: String,
}

/// The secrets of a vault.
pub struct Secrets {
    vault: Vault,
}

impl Secrets {
    pub fn new(vault: Vault) -> Secrets {
        Secrets { vault }
    }

    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    pub fn vault_mut(&mut self) -> &mut Vault {
        &mut self.vault
    }

    /// Stores `value` under (category, key), replacing a secret already
    /// there and keeping its record and the time it was made.
    pub fn put(&mut self, category: &str, key: &str, value: &str, description: &str) -> Result<SecretEntry> {
        check(category, key)?;
        let found = self.find(category, key)?;
        let time = now();
        let (slot, created) = match found {
            Some((slot, old)) => (slot, old.created_utc),
            None => (uuid::Uuid::new_v4().simple().to_string(), time.clone()),
        };
        let envelope = Envelope {
            schema_version: SCHEMA_VERSION,
            category: category.trim().into(),
            key: key.trim().into(),
            value: value.into(),
            description: (!description.trim().is_empty()).then(|| description.into()),
            created_utc: created,
            updated_utc: time,
        };
        let raw = serde_json::to_string(&envelope).map_err(|err| VaultError::Io(err.to_string()))?;
        self.vault.set(&slot, &raw)?;
        Ok(entry(slot, &envelope))
    }

    /// The value stored under (category, key).
    pub fn get(&self, category: &str, key: &str) -> Result<String> {
        check(category, key)?;
        self.find(category, key)?.map(|(_, e)| e.value).ok_or(VaultError::SecretNotFound)
    }

    pub fn delete(&mut self, category: &str, key: &str) -> Result<()> {
        check(category, key)?;
        let (slot, _) = self.find(category, key)?.ok_or(VaultError::SecretNotFound)?;
        self.vault.delete(&slot)
    }

    /// Every secret, sorted by category and key. Records of another kind
    /// (another tool sharing the vault) are left out, not refused.
    pub fn list(&self) -> Result<Vec<SecretEntry>> {
        let mut out: Vec<SecretEntry> = self.envelopes()?.into_iter().map(|(slot, e)| entry(slot, &e)).collect();
        out.sort_by(|a, b| a.category.cmp(&b.category).then_with(|| a.key.cmp(&b.key)));
        Ok(out)
    }

    /// The categories to offer: the kept list and every category in use. A
    /// vault without a list (or with a damaged one) gets this as its list,
    /// on first reading.
    pub fn categories(&mut self) -> Result<Vec<String>> {
        let kept = self.kept_categories()?;
        let mut names = kept.clone().unwrap_or_default();
        names.extend(self.list()?.into_iter().map(|e| e.category));
        let all = normalise(names);
        if kept.is_none() && !all.is_empty() {
            self.set_categories(&all)?;
        }
        Ok(all)
    }

    /// Keeps exactly `names` as the list (trimmed, one of each, sorted).
    pub fn set_categories(&mut self, names: &[String]) -> Result<Vec<String>> {
        let categories = normalise(names.to_vec());
        let list = CategoryList { schema_version: SCHEMA_VERSION, categories: categories.clone(), updated_utc: now() };
        let raw = serde_json::to_string(&list).map_err(|err| VaultError::Io(err.to_string()))?;
        self.vault.set(CATEGORY_SLOT, &raw)?;
        Ok(categories)
    }

    /// Moves every secret of category `old` (ignoring case) to `new`, and
    /// says how many moved.
    pub fn rename_category(&mut self, old: &str, new: &str) -> Result<usize> {
        let (old, new) = (old.trim(), new.trim());
        if old.is_empty() || new.is_empty() {
            return Err(VaultError::InvalidName);
        }
        if old == new {
            return Ok(0);
        }
        let mut moved = 0;
        for (slot, mut envelope) in self.envelopes()? {
            if fold(&envelope.category) != fold(old) {
                continue;
            }
            envelope.category = new.to_string();
            envelope.updated_utc = now();
            let raw = serde_json::to_string(&envelope).map_err(|err| VaultError::Io(err.to_string()))?;
            self.vault.set(&slot, &raw)?;
            moved += 1;
        }
        Ok(moved)
    }

    /// Seals every record again under a new master password (and work
    /// factors). The old vault is kept aside until the new one holds
    /// everything, and put back when anything fails.
    pub fn change_password(&mut self, password: &str, params: Params) -> Result<()> {
        let mut records = Vec::new();
        for slot in self.vault.list()? {
            records.push((slot.clone(), zeroize::Zeroizing::new(self.vault.get(&slot)?)));
        }
        let dir = self.vault.dir().to_path_buf();
        let name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "vault".into());
        let aside = dir.with_file_name(format!("{name}.bak.{}", chrono::Utc::now().format("%Y%m%d%H%M%S")));
        std::fs::rename(&dir, &aside)?;
        let fresh = Vault::create(&dir, password, params).and_then(|vault| {
            for (slot, value) in &records {
                vault.set(slot, value)?;
            }
            Ok(vault)
        });
        match fresh {
            Ok(vault) => {
                // Anything else that was in the folder comes along.
                for entry in std::fs::read_dir(&aside)? {
                    let entry = entry?;
                    let file = entry.file_name();
                    if file != "vault.json" && file != "secrets" {
                        std::fs::rename(entry.path(), dir.join(file))?;
                    }
                }
                std::fs::remove_dir_all(&aside)?;
                self.vault = vault;
                Ok(())
            }
            Err(err) => {
                let _ = std::fs::remove_dir_all(&dir);
                std::fs::rename(&aside, &dir)?;
                Err(err)
            }
        }
    }

    /// The kept list; none when there is none yet, or it is damaged (it is then rebuilt).
    fn kept_categories(&self) -> Result<Option<Vec<String>>> {
        match self.vault.get(CATEGORY_SLOT) {
            Ok(raw) => Ok(serde_json::from_str::<CategoryList>(&raw)
                .ok()
                .filter(|list| list.schema_version == SCHEMA_VERSION)
                .map(|list| normalise(list.categories))),
            Err(VaultError::SecretNotFound) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Every record that is a secret, with its name.
    fn envelopes(&self) -> Result<Vec<(String, Envelope)>> {
        let mut out = Vec::new();
        for slot in self.vault.list()? {
            if slot == CATEGORY_SLOT {
                continue;
            }
            let raw = match self.vault.get(&slot) {
                Ok(raw) => raw,
                // Locked fails the whole call; a record that does not open is another tool's.
                Err(VaultError::Locked) => return Err(VaultError::Locked),
                Err(_) => continue,
            };
            if let Some(envelope) = parse(&raw) {
                out.push((slot, envelope));
            }
        }
        Ok(out)
    }

    fn find(&self, category: &str, key: &str) -> Result<Option<(String, Envelope)>> {
        let (category, key) = (fold(category), fold(key));
        Ok(self.envelopes()?.into_iter().find(|(_, e)| fold(&e.category) == category && fold(&e.key) == key))
    }
}

/// A record that is a secret of this schema.
fn parse(raw: &str) -> Option<Envelope> {
    let envelope: Envelope = serde_json::from_str(raw).ok()?;
    let fine = envelope.schema_version == SCHEMA_VERSION && !envelope.category.trim().is_empty() && !envelope.key.trim().is_empty();
    fine.then_some(envelope)
}

fn entry(slot: String, e: &Envelope) -> SecretEntry {
    SecretEntry {
        slot,
        category: e.category.clone(),
        key: e.key.clone(),
        description: e.description.clone().unwrap_or_default(),
        created_utc: e.created_utc.clone(),
        updated_utc: e.updated_utc.clone(),
    }
}

fn check(category: &str, key: &str) -> Result<()> {
    if category.trim().is_empty() || key.trim().is_empty() { Err(VaultError::InvalidName) } else { Ok(()) }
}

fn fold(text: &str) -> String {
    text.trim().to_lowercase()
}

/// Trimmed, without blanks, one of each ignoring case (the first spelling
/// kept), sorted ignoring case.
fn normalise(names: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<String> = names
        .into_iter()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty() && seen.insert(n.to_lowercase()))
        .collect();
    out.sort_by_key(|n| n.to_lowercase());
    out
}
