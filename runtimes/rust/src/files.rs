//! The files of a file store, which code reaches through `files f in
//! store`, `read text of f` and `write t to store "path"`, and of any other
//! folder (`files f in folder category.folder`), by its path after
//! `folder:`, with the extensions of the files it holds and whether its
//! hidden ones count. A file found can become a record's picture: in
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

/// The store of a folder that is no store: `folder:` and its path.
pub const FOLDER: &str = "folder:";

/// Whether a file's name (`Notes.MD`, `.env`) is one a folder takes: by its
/// extensions (`*.md`, ignoring case; none or `*.*`: any file), a hidden one
/// (its name starting with a dot) only with `hidden` (conformance/files.json).
pub fn takes(patterns: &[String], hidden: bool, name: &str) -> bool {
    if name.starts_with('.') && !hidden {
        return false;
    }
    if patterns.is_empty() || patterns.iter().any(|p| p == "*.*") {
        return true;
    }
    let extension = extension_of(name);
    patterns.iter().any(|pattern| pattern.strip_prefix("*.").is_some_and(|ext| !extension.is_empty() && extension == ext.to_lowercase()))
}

/// What a field or folder holds, as its refusal says it: its extensions (any for none), or `no hidden` for a hidden name it does not take.
fn holds(patterns: &[String], hidden: bool, name: &str) -> String {
    if name.starts_with('.') && !hidden {
        return "no hidden".to_string();
    }
    if patterns.is_empty() { "any".to_string() } else { patterns.join(" ") }
}

/// Whether a blob or file field that says its extensions takes the file
/// `name` (a name or a path): its last part matched as a folder matches, a
/// hidden one only with `hidden`; no file (an empty name) is always taken. One of
/// another type is refused in Clarity's own words, `fileTypeRefused`
/// (conformance/files.json, fieldFiles).
pub fn check_file_type(field: &str, patterns: &[String], hidden: bool, name: &str) -> Result<()> {
    let last = name.rsplit(['/', '\\']).next().unwrap_or(name);
    if last.is_empty() || takes(patterns, hidden, last) {
        return Ok(());
    }
    use crate::ui::{Slot, Text};
    let text = Text::keyed("fileTypeRefused", "{field} takes {extensions} files, not {name}")
        .with("field", Slot::text(field))
        .with("extensions", Slot::text(&holds(patterns, hidden, last)))
        .with("name", Slot::text(last));
    Err(Error::Refused(text))
}

/// The files under `folder`, whose path inside the store starts with `prefix`;
/// hidden ones (a name starting with a dot) only with `hidden`.
fn walk(store: &str, folder: &Path, prefix: &str, out: &mut Vec<FileEntry>) -> Result<()> {
    walk_hidden(store, folder, prefix, false, out)
}

fn walk_hidden(store: &str, folder: &Path, prefix: &str, hidden: bool, out: &mut Vec<FileEntry>) -> Result<()> {
    for entry in std::fs::read_dir(folder).map_err(|e| failed("read", folder, e))? {
        let entry = entry.map_err(|e| failed("read", folder, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') && !hidden {
            continue;
        }
        let path = entry.path();
        let inside = format!("{prefix}{name}");
        let meta = entry.metadata().map_err(|e| failed("read", &path, e))?;
        if meta.is_dir() {
            walk_hidden(store, &path, &format!("{inside}/"), hidden, out)?;
        } else if meta.is_file() {
            let extension = extension_of(&name);
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
        walk(store, &root, "", &mut out)?;
        out.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        Ok(out)
    }

    /// The files of a folder (`folder:` and its path) its extensions take,
    /// in its folders too, sorted by path; hidden ones only with `hidden`.
    pub fn files_in(&self, folder: &str, patterns: &[String], hidden: bool) -> Result<Vec<FileEntry>> {
        let root = self.store_folder(folder)?;
        let mut out = Vec::new();
        walk_hidden(folder, &root, "", hidden, &mut out)?;
        out.retain(|f| takes(patterns, hidden, &f.name));
        out.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
        Ok(out)
    }

    /// Writes `text` as the file `path` of a folder, when it takes the file.
    pub fn write_text_in(&self, folder: &str, patterns: &[String], hidden: bool, path: &str, text: &str) -> Result<()> {
        self.write_bytes_in(folder, patterns, hidden, path, text.as_bytes())
    }

    /// Writes `bytes` as the file `path` of a folder, when it takes the file.
    pub fn write_bytes_in(&self, folder: &str, patterns: &[String], hidden: bool, path: &str, bytes: &[u8]) -> Result<()> {
        let name = path.rsplit('/').next().unwrap_or(path);
        if !takes(patterns, hidden, name) {
            let holds = if name.starts_with('.') && !hidden { "no hidden".to_string() } else { patterns.join(" ") };
            return Err(Error::Invalid(format!("the folder holds {holds} files, not '{path}'")));
        }
        self.write_bytes(folder, path, bytes)
    }

    /// A file a person picked in a web app, kept in the file store `store`
    /// on the server: in a folder named by its contents, as a blob's is, so
    /// nothing is written over; its path on the server (step 5 of
    /// docs/ledger/files-pictures.md).
    pub fn store_upload(&self, store: &str, name: &str, bytes: &[u8]) -> Result<String> {
        let inside = crate::record_file::store_path(bytes, name);
        self.write_bytes(store, &inside, bytes)?;
        Ok(self.store_folder(store)?.join(&inside).display().to_string())
    }

    /// The file stores and their folders on this machine, by name; one whose
    /// folder cannot be found is left out.
    pub fn stores(&self) -> Vec<(String, String)> {
        let mut names = self.store_names();
        names.sort();
        names.into_iter().filter_map(|name| self.store_folder(&name).ok().map(|f| (name, f.display().to_string()))).collect()
    }

    /// The bytes of the file `path` of a store: a picture linked from a file
    /// before its record is saved.
    pub fn store_file(&self, store: &str, path: &str) -> Result<Vec<u8>> {
        let file = picture::in_store(&self.store_folder(store)?, path)?;
        std::fs::read(&file).map_err(|e| failed("read", &file, e))
    }

    /// The text a file holds.
    pub fn read_text(&self, file: &FileEntry) -> Result<String> {
        let path = picture::in_store(&self.store_folder(&file.store)?, &file.path)?;
        let bytes = std::fs::read(&path).map_err(|e| failed("read", &path, e))?;
        String::from_utf8(bytes).map_err(|_| Error::Invalid(format!("{} is not text", file.path)))
    }

    /// Writes `text` as the file `path` of a store, making its folders.
    pub fn write_text(&self, store: &str, path: &str, text: &str) -> Result<()> {
        self.write_bytes(store, path, text.as_bytes())
    }

    /// The contents of a file as they are.
    pub fn read_bytes(&self, file: &FileEntry) -> Result<Vec<u8>> {
        let path = picture::in_store(&self.store_folder(&file.store)?, &file.path)?;
        std::fs::read(&path).map_err(|e| failed("read", &path, e))
    }

    /// Writes `bytes` as they are as the file `path` of a store, making its folders.
    pub fn write_bytes(&self, store: &str, path: &str, bytes: &[u8]) -> Result<()> {
        let file = picture::in_store(&self.store_folder(store)?, path)?;
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder).map_err(|e| failed("make", folder, e))?;
        }
        std::fs::write(&file, bytes).map_err(|e| failed("write", &file, e))
    }

    /// A file as the picture of a field kept in `field_store` (none: in the
    /// database): the file itself when it is in that store, else a copy.
    pub fn picture_from(&self, file: &FileEntry, field_store: Option<&str>) -> Result<Picture> {
        let path = picture::in_store(&self.store_folder(&file.store)?, &file.path)?;
        let bytes = std::fs::read(&path).map_err(|e| failed("read", &path, e))?;
        picture::checked(&bytes)?;
        Ok(match field_store {
            Some(store) if store == file.store => Picture::InStore(file.path.clone()),
            _ => Picture::New(bytes),
        })
    }

    /// A file of a store as the file of a `file` field, under its name:
    /// linked when it is in the field's own store, else copied.
    pub fn record_file_from(&self, file: &FileEntry, field_store: Option<&str>) -> Result<crate::RecordFile> {
        let content = match field_store {
            Some(store) if store == file.store => Picture::InStore(file.path.clone()),
            _ => {
                let path = picture::in_store(&self.store_folder(&file.store)?, &file.path)?;
                Picture::New(std::fs::read(&path).map_err(|e| failed("read", &path, e))?)
            }
        };
        Ok(crate::RecordFile { name: file.name.clone(), content })
    }
}

/// The extension of a file's name, in lowercase, without its dot; empty
/// when it has none.
pub fn extension_of(name: &str) -> String {
    Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}
