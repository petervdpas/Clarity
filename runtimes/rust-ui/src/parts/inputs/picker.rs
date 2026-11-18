//! The storage provider is core's: `crate::files` (StorageProvider, PickerOptions, StorageFile, StorageFolder, FileFilter, NoStorageProvider), reached through `files::system()`, the dialogs the host set.
//!
//! What a folder or file part takes is here, shared by both: its own
//! `extensions` and `hidden`, else its field's (docs/ledger/files-pictures.md, step 4).

use clarity_runtime::ui::Level;

use crate::markup::PartSpec;
use crate::parts::PartHost;

/// The files a part takes: its own extensions and hidden where it names
/// them, else those its bound field says; with the name a refusal gives it
/// (the field's label, else the part's name).
pub struct Takes {
    pub patterns: Vec<String>,
    pub hidden: bool,
    pub name: String,
}

impl Takes {
    pub fn of(spec: &PartSpec, host: &dyn PartHost) -> Takes {
        let field = spec.text("bind").filter(|b| !b.is_empty()).and_then(|b| host.field(b));
        let own = crate::files::patterns_of(spec.text("extensions"));
        let patterns = if own.is_empty() { field.as_ref().map(|f| f.extensions.clone()).unwrap_or_default() } else { own };
        let hidden = spec.flag("hidden") || field.as_ref().is_some_and(|f| f.hidden);
        let name = field.map(|f| f.label).filter(|l| !l.is_empty()).or_else(|| spec.name().map(str::to_string)).unwrap_or_default();
        Takes { patterns, hidden, name }
    }

    /// Whether it takes the file `name` (a name or a path); one it does not
    /// take is refused in Clarity's own words, said on the window. Without
    /// extensions it takes any file, as before it had any.
    pub fn take(&self, host: &dyn PartHost, name: &str) -> bool {
        if self.patterns.is_empty() {
            return true;
        }
        match clarity_runtime::files::check_file_type(&self.name, &self.patterns, self.hidden, name) {
            Ok(()) => true,
            Err(err) => {
                host.say(Level::Error, clarity_runtime::program::said(&err));
                false
            }
        }
    }
}
