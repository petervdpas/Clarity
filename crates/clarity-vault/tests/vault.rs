//! The vault: a folder of encrypted secrets behind a master password, in
//! a format any program that follows it can open too.

use std::path::PathBuf;

use clarity_vault::{Params, Secrets, Vault, VaultError};

/// Cheap work factors, so the tests run fast; real vaults use the defaults.
const FAST: Params = Params { memory_kib: 1024, iterations: 1, parallelism: 1 };

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-vault-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_vault_keeps_values_behind_its_password() {
    let dir = scratch("basic");
    let mut vault = Vault::create(&dir, "correct horse", FAST).unwrap();
    assert!(!vault.is_locked());
    vault.set("db-password", "s3cret").unwrap();
    assert_eq!(vault.get("db-password").unwrap(), "s3cret");
    assert_eq!(vault.list().unwrap(), ["db-password"]);

    // On disk: the header and one record, neither holding the value.
    let header = std::fs::read_to_string(dir.join("vault.json")).unwrap();
    let header: serde_json::Value = serde_json::from_str(&header).unwrap();
    assert_eq!(header["version"], 1);
    assert_eq!(header["kdf"]["algorithm"], "argon2id");
    assert_eq!(header["kdf"]["memoryKiB"], 1024);
    assert_eq!(header["vaultId"].as_str().unwrap().len(), 32);
    let record = std::fs::read_to_string(dir.join("secrets/db-password.secret")).unwrap();
    assert!(!record.contains("s3cret"));
    let record: serde_json::Value = serde_json::from_str(&record).unwrap();
    assert_eq!(record["algorithm"], "aes-256-gcm");
    assert!(record["updatedUtc"].is_string());

    // Locked, nothing reads; the wrong password does not unlock.
    vault.lock();
    assert!(matches!(vault.get("db-password"), Err(VaultError::Locked)));
    let mut again = Vault::open(&dir).unwrap();
    assert!(again.is_locked());
    assert!(matches!(again.unlock("wrong"), Err(VaultError::InvalidPassword)));
    again.unlock("correct horse").unwrap();
    assert_eq!(again.get("db-password").unwrap(), "s3cret");

    // A record is bound to its own name: copied under another, it does not open.
    std::fs::copy(dir.join("secrets/db-password.secret"), dir.join("secrets/other.secret")).unwrap();
    assert!(matches!(again.get("other"), Err(VaultError::Corrupt(_))));

    again.delete("db-password").unwrap();
    assert!(matches!(again.get("db-password"), Err(VaultError::SecretNotFound)));
    assert!(matches!(again.set("../escape", "x"), Err(VaultError::InvalidName)));
    assert!(matches!(Vault::create(&dir, "x", FAST), Err(VaultError::Exists)));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn secrets_are_found_by_category_and_key() {
    let dir = scratch("secrets");
    let vault = Vault::create(&dir, "pw", FAST).unwrap();
    let mut secrets = Secrets::new(vault);
    secrets.put("Postgres", "password", "pg-pass", "the shop database").unwrap();
    secrets.put("Anthropic", "apikey", "sk-1", "").unwrap();
    // Replacing keeps the same record; matching ignores case.
    secrets.put("postgres", "PASSWORD", "pg-pass-2", "the shop database").unwrap();
    assert_eq!(secrets.get("POSTGRES", "password").unwrap(), "pg-pass-2");

    let listed = secrets.list().unwrap();
    let names: Vec<(&str, &str)> = listed.iter().map(|e| (e.category.as_str(), e.key.as_str())).collect();
    assert_eq!(names, [("Anthropic", "apikey"), ("postgres", "PASSWORD")]);
    assert_eq!(listed[1].description, "the shop database");
    // Identity is inside the ciphertext: file names say nothing.
    for name in secrets.vault().list().unwrap() {
        assert_eq!(name.len(), 32, "{name}");
    }

    // The category list is kept in the vault too; reading offers the categories in use with it.
    secrets.set_categories(&["Azure".into(), "anthropic".into()]).unwrap();
    // The first spelling met is kept.
    assert_eq!(secrets.categories().unwrap(), ["anthropic", "Azure", "postgres"]);
    assert!(!secrets.list().unwrap().iter().any(|e| e.category.is_empty()));

    secrets.delete("Anthropic", "apikey").unwrap();
    assert!(matches!(secrets.get("Anthropic", "apikey"), Err(VaultError::SecretNotFound)));
    assert!(matches!(secrets.put(" ", "k", "v", ""), Err(VaultError::InvalidName)));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_password_changes_and_the_vault_can_be_destroyed() {
    let dir = scratch("rekey");
    let vault = Vault::create(&dir, "old", FAST).unwrap();
    let mut secrets = Secrets::new(vault);
    secrets.put("Mail", "smtp", "mail-pass", "").unwrap();
    secrets.change_password("new", FAST).unwrap();
    assert_eq!(secrets.get("Mail", "smtp").unwrap(), "mail-pass");

    let mut reopened = Vault::open(&dir).unwrap();
    assert!(matches!(reopened.unlock("old"), Err(VaultError::InvalidPassword)));
    reopened.unlock("new").unwrap();
    assert_eq!(Secrets::new(reopened).get("Mail", "smtp").unwrap(), "mail-pass");
    // No backup is left behind once the change worked.
    let parent = dir.parent().unwrap();
    let stem = dir.file_name().unwrap().to_string_lossy().to_string();
    assert!(!std::fs::read_dir(parent).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().starts_with(&format!("{stem}.bak"))));

    Vault::destroy(&dir).unwrap();
    assert!(!Vault::exists(&dir));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The categories: the kept list is what was
/// saved, nothing merged in; reading offers it with every category in use;
/// a vault without a list gets one on first reading; a damaged list is
/// rebuilt; renaming a category moves every secret in it.
#[test]
fn categories_keep_their_list_and_move_with_a_rename() {
    let dir = scratch("categories");
    let mut secrets = Secrets::new(Vault::create(&dir, "pw", FAST).unwrap());
    let kept = |s: &Secrets| -> Option<serde_json::Value> {
        let raw = s.vault().get("00000000000000000000000000000001").ok()?;
        let list: serde_json::Value = serde_json::from_str(&raw).unwrap();
        Some(list["Categories"].clone())
    };
    secrets.put("Postgres", "password", "pg", "").unwrap();
    assert_eq!(kept(&secrets), None);
    // First reading keeps the categories in use as the list.
    assert_eq!(secrets.categories().unwrap(), ["Postgres"]);
    assert_eq!(kept(&secrets), Some(serde_json::json!(["Postgres"])));

    // Saving keeps exactly what is given, sorted, one of each.
    secrets.set_categories(&[" mail ".into(), "Azure".into(), "MAIL".into()]).unwrap();
    assert_eq!(kept(&secrets), Some(serde_json::json!(["Azure", "mail"])));
    // Reading still offers the categories in use.
    assert_eq!(secrets.categories().unwrap(), ["Azure", "mail", "Postgres"]);

    // Renaming moves the secrets; it says how many.
    assert_eq!(secrets.rename_category("postgres", "Database").unwrap(), 1);
    assert_eq!(secrets.get("Database", "password").unwrap(), "pg");
    assert!(secrets.get("Postgres", "password").is_err());

    // A damaged list is rebuilt from the secrets.
    secrets.vault().set("00000000000000000000000000000001", "{ not json").unwrap();
    assert_eq!(secrets.categories().unwrap(), ["Database"]);
    std::fs::remove_dir_all(&dir).unwrap();
}
