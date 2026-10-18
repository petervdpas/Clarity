//! Connections: a named set of fields, each plain text (a host, a port, a
//! base URL) or a pointer to a secret by category and key (a password, a
//! token). They are kept in `connections.json` beside the vault, in the
//! this shape:
//!
//! ```json
//! { "shop-db": { "host": { "value": "localhost" },
//!                "password": { "fromVault": { "category": "Postgres", "key": "password" } } } }
//! ```
//!
//! Asking for (category, key) answers from the connection named like the
//! category: a plain field as written, a vault field from the vault, and a
//! key the connection does not have as empty text. Any other category is
//! asked of the vault itself.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{Result, VaultError};
use crate::secrets::Secrets;
use crate::vault::write_json;

/// A secret by category and key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultRef {
    pub category: String,
    pub key: String,
}

/// One field of a connection: plain text, or a pointer into the vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionField {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "fromVault", default, skip_serializing_if = "Option::is_none")]
    pub from_vault: Option<VaultRef>,
}

impl ConnectionField {
    pub fn plain(value: &str) -> ConnectionField {
        ConnectionField { value: Some(value.into()), from_vault: None }
    }

    pub fn vault(category: &str, key: &str) -> ConnectionField {
        ConnectionField { value: None, from_vault: Some(VaultRef { category: category.into(), key: key.into() }) }
    }

    /// Exactly one of the two is set; anything else is dropped on reading.
    fn is_sound(&self) -> bool {
        match (&self.value, &self.from_vault) {
            (Some(_), None) => true,
            (None, Some(r)) => !r.category.trim().is_empty() && !r.key.trim().is_empty(),
            _ => false,
        }
    }
}

/// A named set of fields, in the order they are written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub name: String,
    pub fields: Vec<(String, ConnectionField)>,
}

impl Connection {
    pub fn new(name: &str, fields: impl IntoIterator<Item = (String, ConnectionField)>) -> Connection {
        Connection { name: name.into(), fields: fields.into_iter().collect() }
    }

    /// The field named `key`, as written.
    pub fn field(&self, key: &str) -> Option<&ConnectionField> {
        self.fields.iter().find(|(name, _)| name == key).map(|(_, field)| field)
    }
}

/// A JSON object read and written in the order of its names.
struct Ordered<V>(Vec<(String, V)>);

impl<'de, V: Deserialize<'de>> Deserialize<'de> for Ordered<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct Entries<V>(std::marker::PhantomData<V>);
        impl<'de, V: Deserialize<'de>> Visitor<'de> for Entries<V> {
            type Value = Ordered<V>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Ordered<V>, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) = map.next_entry::<String, V>()? {
                    out.push(entry);
                }
                Ok(Ordered(out))
            }
        }
        deserializer.deserialize_map(Entries(std::marker::PhantomData))
    }
}

/// The connections of one `connections.json`.
pub struct Connections {
    file: PathBuf,
    connections: Vec<Connection>,
}

impl Connections {
    /// Reads the file; a file that is not there, or not JSON, is no
    /// connections, so a hand edit gone wrong never locks anyone out.
    pub fn load(file: impl AsRef<Path>) -> Result<Connections> {
        let file = file.as_ref().to_path_buf();
        let mut connections = Vec::new();
        let text = match std::fs::read_to_string(&file) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(err) => return Err(err.into()),
        };
        let raw: Ordered<Ordered<serde_json::Value>> = serde_json::from_str(&text).unwrap_or(Ordered(Vec::new()));
        for (name, Ordered(fields)) in raw.0 {
            if name.trim().is_empty() {
                continue;
            }
            let fields = fields
                .into_iter()
                .filter(|(key, _)| !key.trim().is_empty())
                .filter_map(|(key, value)| serde_json::from_value::<ConnectionField>(value).ok().filter(|f| f.is_sound()).map(|f| (key, f)))
                .collect();
            connections.push(Connection { name, fields });
        }
        connections.sort_by_key(|c| c.name.to_lowercase());
        Ok(Connections { file, connections })
    }

    /// Every connection, sorted by name.
    pub fn list(&self) -> &[Connection] {
        &self.connections
    }

    /// The connection named `name`, ignoring case.
    pub fn get(&self, name: &str) -> Option<&Connection> {
        let name = name.trim().to_lowercase();
        self.connections.iter().find(|c| c.name.to_lowercase() == name)
    }

    /// Adds a connection, or replaces the one with its name, and writes the file.
    pub fn save(&mut self, connection: Connection) -> Result<()> {
        if connection.name.trim().is_empty() {
            return Err(VaultError::InvalidName);
        }
        let name = connection.name.to_lowercase();
        self.connections.retain(|c| c.name.to_lowercase() != name);
        self.connections.push(connection);
        self.connections.sort_by_key(|c| c.name.to_lowercase());
        self.write()
    }

    /// Removes the connection named `name`, and writes the file.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let name = name.trim().to_lowercase();
        self.connections.retain(|c| c.name.to_lowercase() != name);
        self.write()
    }

    /// The value of (category, key): from the connection named `category`,
    /// or else from the vault.
    pub fn resolve(&self, category: &str, key: &str, secrets: &Secrets) -> Result<String> {
        let Some(connection) = self.get(category) else {
            return secrets.get(category, key);
        };
        match connection.field(key) {
            None => Ok(String::new()),
            Some(ConnectionField { value: Some(value), .. }) => Ok(value.clone()),
            Some(ConnectionField { from_vault: Some(r), .. }) => secrets.get(&r.category, &r.key),
            Some(_) => Ok(String::new()),
        }
    }

    fn write(&self) -> Result<()> {
        write_json(&self.file, &FileView(&self.connections))
    }
}

/// The connections as the file holds them: by name, each with its fields in order.
struct FileView<'a>(&'a [Connection]);

/// The fields of one connection, in order.
struct FieldsView<'a>(&'a [(String, ConnectionField)]);

impl Serialize for FileView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for c in self.0 {
            map.serialize_entry(&c.name, &FieldsView(&c.fields))?;
        }
        map.end()
    }
}

impl Serialize for FieldsView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, field) in self.0 {
            map.serialize_entry(name, field)?;
        }
        map.end()
    }
}
