//! The system's file dialogs as the parts use them (manual/start.how-it-works.md), shaped
//! after Avalonia's storage provider (docs/ledger/files-pictures.md): open a
//! file, choose where one is saved, open a folder, each with one set of
//! options (`PickerOptions`, as `FilePickerOpenOptions`), each giving a
//! storage item (`StorageFile`, `StorageFolder`, as `IStorageFile`): its
//! name, read and written, and a path only where the system has one. And a
//! file opened in the program the system uses for its kind. The desktop
//! host gives the system's own (`desktop::files`), a browser the page's, a
//! test one of its own, and a host without any `NoStorageProvider`, which picks
//! nothing.
//!
//! A dialog waits for the person, so a part never asks on the screen's
//! thread: it asks through `Cx::later`, and what was chosen comes back to
//! it in `Element::on_later`:
//!
//! ```ignore
//! let picker = crate::files::system();
//! cx.later(self.node().id(), move || picker.open_file(&options));
//! // ... later, in on_later: done.downcast::<Option<StorageFile>>()
//! ```

use std::cell::RefCell;
use std::sync::Arc;

/// The kinds of file a dialog offers: their name in the person's language and their patterns (`*.png`).
#[derive(Debug, Clone, PartialEq)]
pub struct FileFilter {
    pub name: String,
    pub patterns: Vec<String>,
}

/// What a dialog shows: its title, the kinds of file it offers (none: any),
/// the folder it opens in (none: where the system opens it) and the name it
/// suggests for a file saved.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PickerOptions {
    pub title: String,
    pub file_type: Option<FileFilter>,
    pub start: Option<String>,
    pub suggested_name: Option<String>,
    /// Hidden files and folders show too, where the system's dialog can show them.
    pub hidden: bool,
}

impl PickerOptions {
    /// A dialog titled `title`, with nothing else asked.
    pub fn titled(title: &str) -> PickerOptions {
        PickerOptions { title: title.to_string(), ..PickerOptions::default() }
    }
}

/// A file the person chose: its name, its path where the system has one (a
/// browser gives none), and its bytes where the host gave them already.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageFile {
    pub name: String,
    pub path: Option<String>,
    pub bytes: Option<Vec<u8>>,
}

impl StorageFile {
    /// A file on the disk, by its path; its name is the path's last part.
    pub fn at(path: &str) -> StorageFile {
        let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        StorageFile { name, path: Some(path.to_string()), bytes: None }
    }

    /// A file the host gave with its bytes and no path (a browser's).
    pub fn given(name: &str, bytes: Vec<u8>) -> StorageFile {
        StorageFile { name: name.to_string(), path: None, bytes: Some(bytes) }
    }

    /// Its bytes: those given, else read from its path; none when it cannot be read.
    pub fn read(&self) -> Option<Vec<u8>> {
        if let Some(bytes) = &self.bytes {
            return Some(bytes.clone());
        }
        let path = self.path.as_deref()?;
        std::fs::read(path).map_err(|e| eprintln!("{path}: {e}")).ok()
    }
}

/// A folder the person chose: its name and its path where the system has one.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageFolder {
    pub name: String,
    pub path: Option<String>,
}

impl StorageFolder {
    /// A folder on the disk, by its path.
    pub fn at(path: &str) -> StorageFolder {
        let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string());
        StorageFolder { name, path: Some(path.to_string()) }
    }
}

/// The system's file dialogs, and the program that opens a file. Each call
/// but `show` waits until the person chose, so it runs off the screen's
/// thread (`Cx::later`).
pub trait StorageProvider: Send + Sync {
    /// Whether it can choose a folder; a browser cannot, and a folder is typed there.
    fn picks_folders(&self) -> bool;

    /// The file the person chose to open; none when they chose none.
    fn open_file(&self, options: &PickerOptions) -> Option<StorageFile>;

    /// Where the person chose to save a file; nothing is written yet (`write`). None when they chose nowhere.
    fn save_file(&self, options: &PickerOptions) -> Option<StorageFile>;

    /// Writes `bytes` to a file `save_file` gave; where it went (a path, or a
    /// browser's download by its name), none when it could not be written.
    fn write(&self, file: &StorageFile, bytes: &[u8]) -> Option<String>;

    /// The folder the person chose; none when they chose none.
    fn open_folder(&self, options: &PickerOptions) -> Option<StorageFolder>;

    /// Opens a file in the program the system uses for its kind; it does not wait.
    fn show(&self, name: &str, bytes: &[u8]);
}

/// Writes `bytes` to a file on the disk by its path: where it went, none when it could not be written.
pub fn write_at(file: &StorageFile, bytes: &[u8]) -> Option<String> {
    let path = file.path.as_deref()?;
    match std::fs::write(path, bytes) {
        Ok(()) => Some(path.to_string()),
        Err(e) => {
            eprintln!("{path}: {e}");
            None
        }
    }
}

/// The picker of a host that has none: it chooses nothing and opens nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoStorageProvider;

impl StorageProvider for NoStorageProvider {
    fn picks_folders(&self) -> bool {
        false
    }

    fn open_file(&self, _options: &PickerOptions) -> Option<StorageFile> {
        None
    }

    fn save_file(&self, _options: &PickerOptions) -> Option<StorageFile> {
        None
    }

    fn write(&self, _file: &StorageFile, _bytes: &[u8]) -> Option<String> {
        None
    }

    fn open_folder(&self, _options: &PickerOptions) -> Option<StorageFolder> {
        None
    }

    fn show(&self, _name: &str, _bytes: &[u8]) {}
}

thread_local! {
    static SYSTEM: RefCell<Option<Arc<dyn StorageProvider>>> = const { RefCell::new(None) };
}

/// The host's file dialogs, which every window gives its parts: the desktop host sets the system's.
pub fn set_system(picker: Arc<dyn StorageProvider>) {
    SYSTEM.with(|s| *s.borrow_mut() = Some(picker));
}

/// The file dialogs the host gave; `NoStorageProvider` when it gave none.
pub fn system() -> Arc<dyn StorageProvider> {
    SYSTEM.with(|s| s.borrow().clone()).unwrap_or_else(|| Arc::new(NoStorageProvider))
}

/// The folder a part's dialog opens in from its `startFolder`: a relative
/// one inside the program's folder (where it runs), an absolute one as it
/// is; none without one.
pub fn start_folder(written: Option<&str>) -> Option<String> {
    let written = written.map(str::trim).filter(|w| !w.is_empty())?;
    let path = std::path::Path::new(written);
    if path.is_absolute() || path.has_root() {
        return Some(written.to_string());
    }
    let here = std::env::current_dir().ok()?;
    Some(here.join(path).to_string_lossy().into_owned())
}

/// The patterns of a part's `extensions` as written: `.png, .jpg, .gif` (or
/// `*.png *.jpg`), separated by commas or spaces, or `*.*` for any file;
/// `.png` is the pattern `*.png`. None written: none.
pub fn patterns_of(written: Option<&str>) -> Vec<String> {
    written
        .unwrap_or_default()
        .split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(|w| match w.strip_prefix('.') {
            Some(ext) if !ext.is_empty() => format!("*.{ext}"),
            _ => w.to_string(),
        })
        .collect()
}

/// The kinds of file a dialog offers for `patterns`, named by them; none
/// without them or with `*.*`, which is any file.
pub fn filter_of(patterns: &[String]) -> Option<FileFilter> {
    if patterns.is_empty() || patterns.iter().any(|p| p == "*.*") {
        return None;
    }
    Some(FileFilter { name: patterns.join(" "), patterns: patterns.to_vec() })
}

/// A file's name as given, without folders or characters a file name cannot have.
pub fn safe_name(name: &str) -> String {
    let normal = name.replace('\\', "/");
    let bare = normal.rsplit('/').next().unwrap_or("");
    let safe: String = bare.chars().map(|c| if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c }).collect();
    if safe.is_empty() || safe == "." || safe == ".." { "file".to_string() } else { safe }
}
