//! A program's values as parts read and write them: plain JSON, the same
//! from every language's program (conformance/pictures.json and
//! record-files.json). A part reads an `image` or `file` field's value here
//! instead of through a runtime's own types.

use base64::Engine;
use serde_json::{Map, Value};

/// The picture or file content of an `image` or `file` field that has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldContent {
    /// The record has one stored; it is fetched on its own.
    Stored,
    /// One loaded but not yet saved.
    New(Vec<u8>),
    /// A file of the field's own file store, by its path there.
    InStore(String),
}

impl FieldContent {
    /// Reads `{"stored": true}`, `{"data": base64}` or `{"file": path}`; none for anything else.
    pub fn read(value: &Value) -> Option<FieldContent> {
        let object = value.as_object()?;
        match (object.get("data"), object.get("file")) {
            (Some(Value::String(data)), _) => base64::engine::general_purpose::STANDARD.decode(data).ok().map(FieldContent::New),
            (Some(_), _) => None,
            (None, Some(Value::String(path))) => Some(FieldContent::InStore(path.clone())),
            (None, Some(_)) => None,
            (None, None) => Some(FieldContent::Stored),
        }
    }

    /// The content as the program reads it.
    pub fn value(&self) -> Value {
        Value::Object(self.wire())
    }

    fn wire(&self) -> Map<String, Value> {
        let mut wire = Map::new();
        match self {
            FieldContent::Stored => {
                wire.insert("stored".into(), Value::Bool(true));
            }
            FieldContent::New(bytes) => {
                wire.insert("data".into(), Value::String(base64::engine::general_purpose::STANDARD.encode(bytes)));
            }
            FieldContent::InStore(path) => {
                wire.insert("file".into(), Value::String(path.clone()));
            }
        }
        wire
    }
}

/// The file of a `file` field that has one, and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldFile {
    pub name: String,
    pub content: FieldContent,
}

impl FieldFile {
    /// Reads `{"name": ..., and the content}`; none for anything else.
    pub fn read(value: &Value) -> Option<FieldFile> {
        let name = value.get("name").and_then(Value::as_str).unwrap_or_default();
        Some(FieldFile { name: clean_name(name), content: FieldContent::read(value)? })
    }

    /// The file as the program reads it.
    pub fn value(&self) -> Value {
        let mut wire = self.content.wire();
        wire.insert("name".into(), Value::String(self.name.clone()));
        Value::Object(wire)
    }
}

/// The name a chosen file is kept under: the last part of what was chosen,
/// trimmed; `file` when nothing is left.
pub fn clean_name(chosen: &str) -> String {
    let last = chosen.rsplit(['/', '\\']).next().unwrap_or("").trim();
    if last.is_empty() { "file".to_string() } else { last.to_string() }
}

/// Whether `bytes` are a picture of a kind every stack shows (png, jpg, gif, bmp, webp), told by their first bytes.
pub fn is_picture(bytes: &[u8]) -> bool {
    matches!(
        bytes,
        [0x89, b'P', b'N', b'G', ..] | [0xFF, 0xD8, 0xFF, ..] | [b'G', b'I', b'F', b'8', ..] | [b'B', b'M', ..] | [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..]
    )
}
