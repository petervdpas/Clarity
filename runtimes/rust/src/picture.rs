//! Pictures in records: an `image` field holds no picture, the one the
//! record has stored, or a new one loaded but not yet saved. Reading records
//! never carries the pictures themselves, so a list of thousands stays
//! light; the screen fetches each picture on its own. A picture is kept in
//! the database, or as a file in a file store named by the hash of its
//! bytes, the column holding that name. The other runtimes do the same
//! (conformance/pictures.json).

use std::path::{Component, Path, PathBuf};

use base64::Engine;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::dialect::Dialect;
use crate::{Error, Result};

/// The picture of an `image` field that has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picture {
    /// The record has one stored; it is fetched on its own.
    Stored,
    /// One loaded but not yet saved.
    New(Vec<u8>),
    /// A file of the field's own file store, by its path there: saved as
    /// that path, nothing copied.
    InStore(String),
}

/// An `image` field's column, and the file store it keeps its pictures in;
/// without one they are in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureColumn {
    pub column: &'static str,
    pub store: Option<&'static str>,
}

/// The kinds of picture every stack shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureKind {
    pub extension: &'static str,
    pub media_type: &'static str,
}

/// The kind of picture `bytes` are, told by their first bytes.
pub fn kind_of(bytes: &[u8]) -> Option<PictureKind> {
    let kind = |extension, media_type| Some(PictureKind { extension, media_type });
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => kind("png", "image/png"),
        [0xFF, 0xD8, 0xFF, ..] => kind("jpg", "image/jpeg"),
        [b'G', b'I', b'F', b'8', ..] => kind("gif", "image/gif"),
        [b'B', b'M', ..] => kind("bmp", "image/bmp"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => kind("webp", "image/webp"),
        _ => None,
    }
}

/// The kind of `bytes`, or why they cannot be kept as a picture.
pub fn checked(bytes: &[u8]) -> Result<PictureKind> {
    kind_of(bytes).ok_or_else(|| Error::Invalid("this file is not a picture; use a png, jpg, gif, bmp or webp file".into()))
}

/// The name of a picture's file in a file store: the first 32 hex digits of
/// the SHA-256 of its bytes, and the extension of its kind.
pub fn file_name(bytes: &[u8]) -> Result<String> {
    let kind = checked(bytes)?;
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().take(16).map(|b| format!("{b:02x}")).collect();
    Ok(format!("{hex}.{}", kind.extension))
}

/// A picture as the page shows it.
pub fn data_url(bytes: &[u8]) -> Result<String> {
    let kind = checked(bytes)?;
    Ok(format!("data:{};base64,{}", kind.media_type, base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// What the select list reads of a picture kept in the database: whether
/// there is one, not the picture.
pub fn presence(d: Dialect, column: &str) -> String {
    format!("CASE WHEN {} IS NULL THEN 0 ELSE 1 END", d.quote(column))
}

/// The file `path` (as a column holds it) in the store at `root`; a path
/// that would leave the store is refused.
pub(crate) fn in_store(root: &Path, path: &str) -> Result<PathBuf> {
    let relative = Path::new(path);
    if path.is_empty() || !relative.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(Error::Invalid(format!("'{path}' is not a picture of its file store")));
    }
    Ok(root.join(relative))
}

#[derive(Serialize, Deserialize)]
struct Wire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stored: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    file: Option<String>,
}

/// `{"stored": true}`, `{"data": base64}` for a new picture, or
/// `{"file": path}` for a file of the field's own store.
impl Serialize for Picture {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let wire = match self {
            Picture::Stored => Wire { stored: Some(true), data: None, file: None },
            Picture::New(bytes) => Wire { stored: None, data: Some(base64::engine::general_purpose::STANDARD.encode(bytes)), file: None },
            Picture::InStore(path) => Wire { stored: None, data: None, file: Some(path.clone()) },
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Picture {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let wire = Wire::deserialize(deserializer)?;
        match (wire.data, wire.file) {
            (Some(data), _) => base64::engine::general_purpose::STANDARD.decode(data).map(Picture::New).map_err(D::Error::custom),
            (None, Some(path)) => Ok(Picture::InStore(path)),
            (None, None) => Ok(Picture::Stored),
        }
    }
}
