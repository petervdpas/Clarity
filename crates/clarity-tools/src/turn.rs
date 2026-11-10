//! A turn of an outside agent in Clarity's chat panel (docs/ACP-PLAN.md).
//! Such an agent also writes files itself, not only through Clarity's tools,
//! so the IDE takes the project's files when a turn starts and again when it
//! ends: the difference is the turn, one change for Undo however it was
//! written. The turn's migration step follows once at its end
//! (`crate::steps::follow`).
//!
//! What the turn asks Clarity to show (a window in Preview, a diagram, a
//! project it made to open) is
//! written by `clarity mcp --turn <file>` as one JSON line per request, and
//! read back by the IDE when the turn ends.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::{Value, json};

use crate::files::FileChange;

/// The project's files at one moment: the text of each text file, and the
/// size and time of every other one (pictures, sounds), never read as text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    texts: BTreeMap<PathBuf, String>,
    others: BTreeMap<PathBuf, Stamp>,
}

/// A file that is not text, as far as a turn tells it changed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
}

/// What a turn changed.
#[derive(Debug, Default)]
pub struct TurnChanges {
    /// Each text file once: its text before (none when the turn made it) and after (none when it removed it).
    pub files: Vec<FileChange>,
    /// The files that are not text the turn made, changed or removed; Undo cannot put these back.
    pub others: Vec<PathBuf>,
}

/// The files of the project `file` with its folder `dir`: the project file
/// and the folder, less the app folder Clarity generates into, the
/// databases (`data`), the folders the stack's build tools make and the ones
/// the project keeps (in custom parts' folders too), and hidden folders.
pub fn take(file: &Path, dir: &Path) -> Snapshot {
    let mut skipped: Vec<String> = vec!["data".to_string()];
    let mut out = None;
    if let Some(settings) = clarity_project::load(file).settings {
        skipped.extend(clarity_gen::build_output(settings.stack.backend).iter().map(|s| s.to_string()));
        skipped.extend(settings.keeps.iter().cloned());
        out = Some(clarity_project::out_dir(file, &settings));
    }
    let mut snapshot = Snapshot::default();
    snapshot.add(file);
    snapshot.walk(dir, &skipped, out.as_deref());
    snapshot
}

impl Snapshot {
    fn walk(&mut self, folder: &Path, skipped: &[String], out: Option<&Path>) {
        let Ok(entries) = std::fs::read_dir(folder) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                let left_out = name.starts_with('.') || skipped.contains(&name) || out == Some(path.as_path());
                if !left_out {
                    self.walk(&path, skipped, out);
                }
            } else if kind.is_file() && !skipped.contains(&name) {
                self.add(&path);
            }
        }
    }

    /// One file: its text when it is text, else its size and time.
    fn add(&mut self, path: &Path) {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let media = clarity_spec::resources::is_picture(&name) || clarity_spec::resources::is_sound(&name) || clarity_spec::resources::is_video(&name);
        if !media
            && let Ok(bytes) = std::fs::read(path)
        {
            match String::from_utf8(bytes) {
                Ok(text) if !text.contains('\0') => {
                    self.texts.insert(path.to_path_buf(), text);
                    return;
                }
                _ => {}
            }
        }
        if let Ok(meta) = std::fs::metadata(path) {
            self.others.insert(path.to_path_buf(), Stamp { len: meta.len(), modified: meta.modified().ok() });
        }
    }
}

/// What changed from `before` to `after`, each file once.
pub fn changes(before: &Snapshot, after: &Snapshot) -> TurnChanges {
    let mut files = Vec::new();
    for (path, text) in &before.texts {
        match after.texts.get(path) {
            Some(now) if now == text => {}
            now => files.push(FileChange { file: path.clone(), before: Some(text.clone()), after: now.cloned() }),
        }
    }
    for (path, text) in &after.texts {
        if !before.texts.contains_key(path) {
            files.push(FileChange { file: path.clone(), before: None, after: Some(text.clone()) });
        }
    }
    // A file that turned from text into something else, or back, counts among the others.
    files.retain(|c| !(c.after.is_none() && after.others.contains_key(&c.file)) && !(c.before.is_none() && before.others.contains_key(&c.file)));
    let mut others: Vec<PathBuf> = Vec::new();
    for (path, stamp) in &before.others {
        if after.others.get(path) != Some(stamp) {
            others.push(path.clone());
        }
    }
    for path in after.others.keys() {
        if !before.others.contains_key(path) {
            others.push(path.clone());
        }
    }
    files.sort_by(|a, b| a.file.cmp(&b.file));
    others.sort();
    TurnChanges { files, others }
}

/// What a turn asks Clarity to show when it ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    /// A window, by its name, in Preview.
    Window(String),
    /// A written diagram, by its file name in diagrams/ (`billing.mermaid`).
    Diagram(String),
    /// A project the turn made, by its project file, to open.
    Project(PathBuf),
}

const SHOW_WINDOW: &str = "show_window";
const SHOW_DIAGRAM: &str = "show_diagram";
const OPEN_PROJECT: &str = "open_project";

/// Adds `shown` to the turn's file `file`, one JSON line.
pub fn show(file: &Path, shown: &Shown) -> Result<(), String> {
    use std::io::Write;
    let line = match shown {
        Shown::Window(name) => json!({ SHOW_WINDOW: name }),
        Shown::Diagram(name) => json!({ SHOW_DIAGRAM: name }),
        Shown::Project(file) => json!({ OPEN_PROJECT: file.display().to_string() }),
    };
    let mut kept = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .map_err(|err| format!("cannot write {}: {err}", file.display()))?;
    writeln!(kept, "{line}").map_err(|err| format!("cannot write {}: {err}", file.display()))
}

/// What the turn's file `file` asks to show, in order; none when it is not
/// there. A line that cannot be read is skipped.
pub fn shown(file: &Path) -> Vec<Shown> {
    let text = std::fs::read_to_string(file).unwrap_or_default();
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|line| {
            let name = |key: &str| line.get(key).and_then(Value::as_str).map(str::to_string);
            name(SHOW_WINDOW)
                .map(Shown::Window)
                .or_else(|| name(SHOW_DIAGRAM).map(Shown::Diagram))
                .or_else(|| name(OPEN_PROJECT).map(|file| Shown::Project(PathBuf::from(file))))
        })
        .collect()
}
