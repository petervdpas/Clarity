//! Files in records (U1c): a `file` field holds no file, the one the record
//! has stored, a new one chosen but not yet saved, or a file of its file
//! store; always with its name. Like a picture, reading records never
//! carries the file itself; the screen fetches it on its own to open it or
//! save a copy. In the database the bytes are in the field's column and the
//! name in the column beside it; in a file store the file keeps its name in
//! a folder named by its contents (conformance/record-files.json).

use std::path::PathBuf;

use base64::Engine;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::picture::Picture;
use crate::{Error, Result};

/// The file of a `file` field that has one, and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFile {
    pub name: String,
    pub content: Picture,
}

/// The name a chosen file is kept under: the last part of what was chosen,
/// trimmed; `file` when nothing is left.
pub fn clean_name(chosen: &str) -> String {
    let last = chosen.rsplit(['/', '\\']).next().unwrap_or("").trim();
    if last.is_empty() { "file".to_string() } else { last.to_string() }
}

/// The path of a file in a file store: a folder named by the first 32 hex
/// digits of the SHA-256 of its bytes, and in it the file under its name.
pub fn store_path(bytes: &[u8], name: &str) -> String {
    format!("{}/{}", contents(bytes), clean_name(name))
}

/// The first 32 hex digits of the SHA-256 of `bytes`: the folder a file is kept in.
fn contents(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().take(16).map(|b| format!("{b:02x}")).collect()
}

/// The name a path in a store shows: its last part.
pub fn name_of(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// The folder a copy is saved in on system `os`, with `env` giving the
/// environment; none when it names no home.
pub fn downloads_in(os: &str, env: impl Fn(&str) -> Option<String>) -> Option<String> {
    let home = match os {
        "windows" => env("USERPROFILE"),
        _ => env("HOME"),
    }
    .filter(|h| !h.is_empty())?;
    Some(format!("{home}/Downloads"))
}

/// The name a copy gets where `taken` names are in use: the name itself,
/// else ` (2)`, ` (3)` and on before its extension.
pub fn free_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(name) {
        return name.to_string();
    }
    // A leading dot starts a name, not an extension.
    let (stem, extension) = match name.rfind('.') {
        Some(at) if at > 0 => (&name[..at], &name[at..]),
        _ => (name, ""),
    };
    (2..).map(|n| format!("{stem} ({n}){extension}")).find(|candidate| !taken(candidate)).expect("a free name")
}

fn system() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// Saves a copy of a file in the person's Downloads folder, under a name
/// free there; the file it wrote.
pub fn save_copy(name: &str, bytes: &[u8]) -> Result<PathBuf> {
    let folder = downloads_in(system(), |v| std::env::var(v).ok()).ok_or_else(|| Error::Invalid("this system names no Downloads folder".into()))?;
    let folder = PathBuf::from(folder);
    std::fs::create_dir_all(&folder).map_err(|e| Error::Storage(format!("cannot make {}: {e}", folder.display())))?;
    let free = free_name(&clean_name(name), |n| folder.join(n).exists());
    let file = folder.join(free);
    std::fs::write(&file, bytes).map_err(|e| Error::Storage(format!("cannot write {}: {e}", file.display())))?;
    Ok(file)
}

/// Opens a file in the program the system uses for its kind, from a copy in
/// a folder of its own under the system's temporary folder.
pub fn open(name: &str, bytes: &[u8]) -> Result<()> {
    let folder = std::env::temp_dir().join(format!("clarity-open-{}", contents(bytes)));
    std::fs::create_dir_all(&folder).map_err(|e| Error::Storage(format!("cannot make {}: {e}", folder.display())))?;
    let file = folder.join(clean_name(name));
    std::fs::write(&file, bytes).map_err(|e| Error::Storage(format!("cannot write {}: {e}", file.display())))?;
    #[cfg(target_os = "windows")]
    let opened = std::process::Command::new("rundll32").arg("url.dll,FileProtocolHandler").arg(&file).spawn();
    #[cfg(target_os = "macos")]
    let opened = std::process::Command::new("open").arg(&file).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let opened = std::process::Command::new("xdg-open").arg(&file).spawn();
    opened.map(|_| ()).map_err(|e| Error::Storage(format!("cannot open {}: {e}", file.display())))
}

/// A file as the page holds it.
pub fn data(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// The bytes of a file the page sends.
pub fn from_data(data: &str) -> Result<Vec<u8>> {
    base64::engine::general_purpose::STANDARD.decode(data).map_err(|e| Error::Invalid(format!("a file that is no base64: {e}")))
}

#[derive(Serialize, Deserialize)]
struct Wire {
    #[serde(default)]
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stored: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    file: Option<String>,
}

/// `{"name", "stored": true}`, `{"name", "data": base64}` for a new file,
/// or `{"name", "file": path}` for a file of the field's own store.
impl Serialize for RecordFile {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let name = self.name.clone();
        let wire = match &self.content {
            Picture::Stored => Wire { name, stored: Some(true), data: None, file: None },
            Picture::New(bytes) => Wire { name, stored: None, data: Some(data(bytes)), file: None },
            Picture::InStore(path) => Wire { name, stored: None, data: None, file: Some(path.clone()) },
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RecordFile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let wire = Wire::deserialize(deserializer)?;
        let content = match (wire.data, wire.file) {
            (Some(data), _) => Picture::New(base64::engine::general_purpose::STANDARD.decode(data).map_err(D::Error::custom)?),
            (None, Some(path)) => Picture::InStore(path),
            (None, None) => Picture::Stored,
        };
        Ok(RecordFile { name: clean_name(&wire.name), content })
    }
}
