//! Connections: a named set of fields, each plain text or a pointer into
//! the vault, in connections.json. Asking for
//! (category, key) answers from the connection named like the category,
//! and from the vault for any other category.

use clarity_vault::{Connection, ConnectionField, Connections, Params, Secrets, Vault, VaultError};

const FAST: Params = Params { memory_kib: 1024, iterations: 1, parallelism: 1 };

#[test]
fn connections_keep_their_file_and_resolve_through_the_vault() {
    let root = std::env::temp_dir().join(format!("clarity-connections-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let file = root.join("connections.json");

    let mut connections = Connections::load(&file).unwrap();
    connections
        .save(Connection::new(
            "shop-db",
            [
                ("host".to_string(), ConnectionField::plain("localhost")),
                ("password".to_string(), ConnectionField::vault("Postgres", "password")),
            ],
        ))
        .unwrap();

    // The shape of the file.
    let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(
        written,
        serde_json::json!({
            "shop-db": {
                "host": { "value": "localhost" },
                "password": { "fromVault": { "category": "Postgres", "key": "password" } }
            }
        })
    );
    // Names are found ignoring case; a malformed field is dropped on reading.
    std::fs::write(
        &file,
        r#"{ "Shop-DB": { "host": { "value": "db.local" }, "broken": {}, "password": { "fromVault": { "category": "Postgres", "key": "password" } } } }"#,
    )
    .unwrap();
    let connections = Connections::load(&file).unwrap();
    let shop = connections.get("shop-db").unwrap();
    assert_eq!(shop.fields.len(), 2);

    let mut secrets = Secrets::new(Vault::create(root.join("vault"), "pw", FAST).unwrap());
    secrets.put("Postgres", "password", "pg-pass", "").unwrap();
    secrets.put("Anthropic", "apikey", "sk-1", "").unwrap();
    assert_eq!(connections.resolve("shop-db", "host", &secrets).unwrap(), "db.local");
    assert_eq!(connections.resolve("shop-db", "password", &secrets).unwrap(), "pg-pass");
    // A key the connection does not declare is empty.
    assert_eq!(connections.resolve("shop-db", "port", &secrets).unwrap(), "");
    // Any other category goes to the vault itself.
    assert_eq!(connections.resolve("Anthropic", "apikey", &secrets).unwrap(), "sk-1");

    // A plain field never needs the vault; a field from the vault does.
    secrets.vault_mut().lock();
    assert_eq!(connections.resolve("shop-db", "host", &secrets).unwrap(), "db.local");
    assert!(matches!(connections.resolve("shop-db", "password", &secrets), Err(VaultError::Locked)));

    // A file that is not JSON reads as no connections, so a hand edit gone wrong does not lock Clarity out.
    std::fs::write(&file, "{ not json").unwrap();
    assert!(Connections::load(&file).unwrap().list().is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}

/// Fields keep the order they are written in.
#[test]
fn fields_keep_their_order() {
    let root = std::env::temp_dir().join(format!("clarity-connections-order-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let file = root.join("connections.json");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(&file, r#"{ "Anthropic": { "baseUrl": { "value": "u" }, "model": { "value": "m" }, "apikey": { "fromVault": { "category": "Anthropic", "key": "apikey" } }, "kind": { "value": "k" } } }"#).unwrap();
    let mut connections = Connections::load(&file).unwrap();
    let order = |c: &Connections| c.get("Anthropic").unwrap().fields.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>();
    assert_eq!(order(&connections), ["baseUrl", "model", "apikey", "kind"]);
    let again = connections.get("Anthropic").unwrap().clone();
    connections.save(again).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.find("baseUrl").unwrap() < text.find("model").unwrap() && text.find("model").unwrap() < text.find("apikey").unwrap(), "{text}");
    assert_eq!(order(&Connections::load(&file).unwrap()), ["baseUrl", "model", "apikey", "kind"]);
    std::fs::remove_dir_all(&root).unwrap();
}
