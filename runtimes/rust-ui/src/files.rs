//! The system's file dialogs as the parts use them (DESIGN 7.9): choose a
//! file to open, choose where a copy of a file is saved, choose a folder,
//! and open a file in the program the system uses for its kind. The desktop
//! host gives the system's own (`desktop::files`), a test gives one of its
//! own, and a host without any gives `NoFilePicker`, which picks nothing.
//!
//! A dialog waits for the person, so a part never asks on the screen's
//! thread: it asks through `Cx::later`, and what was chosen comes back to
//! it in `Element::on_later`:
//!
//! ```ignore
//! let picker = self.base.host.files();
//! cx.later(self.node().id(), move || picker.open_file(&title, filter.as_ref()));
//! // ... later, in on_later: done.downcast::<Option<ChosenFile>>()
//! ```

use std::cell::RefCell;
use std::sync::Arc;

/// A file the person chose: its name as chosen and its bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct ChosenFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// The kinds of file a dialog offers: their name in the person's language and their patterns (`*.png`).
#[derive(Debug, Clone, PartialEq)]
pub struct FileFilter {
    pub name: String,
    pub patterns: Vec<String>,
}

/// The system's file dialogs, and the program that opens a file. Each call
/// but `show` waits until the person chose, so it runs off the screen's
/// thread (`Cx::later`).
pub trait FilePicker: Send + Sync {
    /// Whether it can choose a folder; a browser cannot, and a folder is typed there.
    fn picks_folders(&self) -> bool;

    /// The file the person chose to open, titled `title`, of the kinds of `filter`; none when they chose none.
    fn open_file(&self, title: &str, filter: Option<&FileFilter>) -> Option<ChosenFile>;

    /// Saves `bytes` where the person chose, `name` suggested; where it went, or none when they chose nowhere.
    fn save_file(&self, title: &str, name: &str, bytes: &[u8]) -> Option<String>;

    /// The folder the person chose, titled `title`; none when they chose none.
    fn choose_folder(&self, title: &str) -> Option<String>;

    /// Opens a file in the program the system uses for its kind; it does not wait.
    fn show(&self, name: &str, bytes: &[u8]);
}

/// The picker of a host that has none: it chooses nothing and opens nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoFilePicker;

impl FilePicker for NoFilePicker {
    fn picks_folders(&self) -> bool {
        false
    }

    fn open_file(&self, _title: &str, _filter: Option<&FileFilter>) -> Option<ChosenFile> {
        None
    }

    fn save_file(&self, _title: &str, _name: &str, _bytes: &[u8]) -> Option<String> {
        None
    }

    fn choose_folder(&self, _title: &str) -> Option<String> {
        None
    }

    fn show(&self, _name: &str, _bytes: &[u8]) {}
}

thread_local! {
    static SYSTEM: RefCell<Option<Arc<dyn FilePicker>>> = const { RefCell::new(None) };
}

/// The host's file dialogs, which every window gives its parts: the desktop host sets the system's.
pub fn set_system(picker: Arc<dyn FilePicker>) {
    SYSTEM.with(|s| *s.borrow_mut() = Some(picker));
}

/// The file dialogs the host gave; `NoFilePicker` when it gave none.
pub fn system() -> Arc<dyn FilePicker> {
    SYSTEM.with(|s| s.borrow().clone()).unwrap_or_else(|| Arc::new(NoFilePicker))
}

/// A file's name as given, without folders or characters a file name cannot have.
pub fn safe_name(name: &str) -> String {
    let normal = name.replace('\\', "/");
    let bare = normal.rsplit('/').next().unwrap_or("");
    let safe: String = bare.chars().map(|c| if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c }).collect();
    if safe.is_empty() || safe == "." || safe == ".." { "file".to_string() } else { safe }
}
