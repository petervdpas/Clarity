//! The files of a file store, which code reaches through `files f in
//! store`, `read text of f` and `write t to store "path"`: only folders the
//! project settings name. A file found can become a record's picture: in
//! the field's own store by its path, elsewhere as a copy. The other
//! runtimes do the same (conformance/files.json).

use std::path::Path;

use chrono::{DateTime, NaiveDateTime, Timelike, Utc};

use crate::picture::{self, Picture};
use crate::{Db, Error, Result};

/// A file of a file store.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileEntry {
    pub store: String,
    /// The path inside the store, with `/` between folders.
    pub path: String,
    pub name: String,
    /// In lower case, without the dot; empty without one.
    pub extension: String,
    /// In bytes.
    pub size: i64,
    /// When the file last changed, in UTC to the second.
    pub modified: Option<NaiveDateTime>,
}

fn failed(what: &str, path: &Path, err: std::io::Error) -> Error {
    Error::Storage(format!("cannot {what} {}: {err}", path.display()))
}

/// The files under `folder`, whose path inside the store starts with `prefix`.
fn walk(store: &str, folder: &Path, prefix: &str, out: &mut Vec<FileEntry>) -> Result<()> {
    for entry in std::fs::read_dir(folder).map_err(|e| failed("read", folder, e))? {
        let entry = entry.map_err(|e| failed("read", folder, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let inside = format!("{prefix}{name}");
        let meta = entry.metadata().map_err(|e| failed("read", &path, e))?;
        if meta.is_dir() {
            walk(store, &path, &format!("{inside}/"), out)?;
        } else if meta.is_file() {
            let extension = Path::new(&name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            let modified = meta.modified().ok().map(|t| DateTime::<Utc>::from(t).naive_utc().with_nanosecond(0).unwrap_or_default());
            out.push(FileEntry { store: store.to_string(), path: inside, name, extension, size: meta.len() as i64, modified });
        }
    }
    Ok(())
}


impl Db {
    /// Every file of a store and its folders, but those whose name or
    /// folder starts with a dot, sorted by path.
    pub fn files(&self, store: &str) -> Result<Vec<FileEntry>> {
        let root = self.store_folder(store)?;
        let mut out = Vec::new();
        walk(store, root, "", &mut out)?;
        out.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        Ok(out)
    }

    /// The bytes of the file `path` of a store: a picture linked from a file
    /// before its record is saved.
    pub fn store_file(&self, store: &str, path: &str) -> Result<Vec<u8>> {
        let file = picture::in_store(self.store_folder(store)?, path)?;
        std::fs::read(&file).map_err(|e| failed("read", &file, e))
    }

    /// The text a file holds.
    pub fn read_text(&self, file: &FileEntry) -> Result<String> {
        let path = picture::in_store(self.store_folder(&file.store)?, &file.path)?;
        let bytes = std::fs::read(&path).map_err(|e| failed("read", &path, e))?;
        String::from_utf8(bytes).map_err(|_| Error::Invalid(format!("{} is not text", file.path)))
    }

    /// Writes `text` as the file `path` of a store, making its folders.
    pub fn write_text(&self, store: &str, path: &str, text: &str) -> Result<()> {
        let file = picture::in_store(self.store_folder(store)?, path)?;
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder).map_err(|e| failed("make", folder, e))?;
        }
        std::fs::write(&file, text).map_err(|e| failed("write", &file, e))
    }

    /// A file as the picture of a field kept in `field_store` (none: in the
    /// database): the file itself when it is in that store, else a copy.
    pub fn picture_from(&self, file: &FileEntry, field_store: Option<&str>) -> Result<Picture> {
        let path = picture::in_store(self.store_folder(&file.store)?, &file.path)?;
        let bytes = std::fs::read(&path).map_err(|e| failed("read", &path, e))?;
        picture::checked(&bytes)?;
        Ok(match field_store {
            Some(store) if store == file.store => Picture::InStore(file.path.clone()),
            _ => Picture::New(bytes),
        })
    }
}
