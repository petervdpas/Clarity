//! Writing project files: a change with its text before and after, for Undo,
//! and writes through a rename so no reader sees half a file.

use std::path::{Path, PathBuf};

/// One file's text before and after a change. A file that did not exist
/// before has no text, and undoing its change removes it.
#[derive(Debug)]
pub struct FileChange {
    pub file: PathBuf,
    /// The text before; None when the change made the file.
    pub before: Option<String>,
    /// The text after; None when the change took the file away.
    pub after: Option<String>,
}

impl FileChange {
    /// Makes the change: writes the text after, or takes the file away.
    pub fn make(&self) -> Result<(), String> {
        match &self.after {
            Some(text) => write_atomically(&self.file, text),
            None => std::fs::remove_file(&self.file).map_err(|err| format!("cannot remove {}: {err}", self.file.display())),
        }
    }
}

/// Whether `path` is a Clarity file (`.clarity`).
pub fn is_spec_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == clarity_spec::SPEC_EXTENSION)
}

/// Writes through a temporary file and a rename, so readers (including the
/// watcher and other editors) never see a half-written spec.
pub fn write_atomically(file: &Path, text: &str) -> Result<(), String> {
    let mut tmp = file.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, text).map_err(|err| format!("cannot write {}: {err}", tmp.display()))?;
    std::fs::rename(&tmp, file).map_err(|err| {
        let _ = std::fs::remove_file(&tmp);
        format!("cannot replace {}: {err}", file.display())
    })
}
