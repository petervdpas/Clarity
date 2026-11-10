//! What Clarity's own file browser lists of a folder (docs/PROJECT-BROWSER-PLAN.md).
//! The backend lists and the screen shows: folders first, then files, by
//! name; hidden ones only when asked; only the kinds of file a use can pick;
//! and Clarity's projects told exactly, with the reader opening uses: a
//! project file (a `.clarity` file whose first line, leaving out blank lines
//! and comments, is `project`), the folder of its modules beside it, and a
//! folder that holds one. No guessing: a module is never taken for a project.

use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::{Stack, Target};

/// What a use of the browser can pick, and so what it lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Show {
    /// Every folder and file (opening a project).
    Everything,
    /// Folders only (choosing a folder).
    Folders,
    /// Folders to go into, and files of these kinds, by extension without the dot, in any case.
    Files(Vec<String>),
}

/// A folder as the browser lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Listing {
    /// The folder, as it was asked for.
    pub folder: PathBuf,
    /// The folder above it; none at a root.
    pub up: Option<PathBuf>,
    pub entries: Vec<Entry>,
    /// Why it lists nothing: it is not there, not a folder, or cannot be read.
    pub problem: Option<String>,
}

/// One folder or file in a listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub folder: bool,
    /// A file's size in bytes; 0 for a folder.
    #[ts(type = "number")]
    pub size: u64,
    /// A project file: what it is.
    pub project: Option<ProjectInfo>,
    /// A folder beside a project file of its name (`shop/` beside `shop.clarity`): that project's name.
    pub modules_of: Option<String>,
    /// A folder that holds project files: what they are.
    pub projects: Vec<ProjectInfo>,
}

/// A project file, as the browser tells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectInfo {
    pub file: PathBuf,
    /// The name people see: its title, else its name.
    pub title: String,
    /// Its stack in words (`C#, Clarity.UI, Desktop and CLI`); empty when it does not read.
    pub stack: String,
    /// Why it does not read, when it does not.
    pub problem: Option<String>,
    /// The opening of its README (the first paragraph after the heading, a couple of lines), when it has one.
    pub readme: Option<String>,
}

/// The folder `folder` as `show` asks, with hidden folders and files when `hidden`.
pub fn list(folder: &Path, show: &Show, hidden: bool) -> Listing {
    let up = folder.parent().map(Path::to_path_buf);
    let empty = |problem: String| Listing { folder: folder.to_path_buf(), up: up.clone(), entries: Vec::new(), problem: Some(problem) };
    if folder.exists() && !folder.is_dir() {
        return empty(format!("{} is not a folder", folder.display()));
    }
    let read = match std::fs::read_dir(folder) {
        Ok(read) => read,
        Err(err) => return empty(format!("cannot read {}: {err}", folder.display())),
    };
    let mut entries: Vec<Entry> = Vec::new();
    for item in read.flatten() {
        let path = item.path();
        let name = item.file_name().to_string_lossy().into_owned();
        let Ok(meta) = std::fs::metadata(&path) else { continue };
        if !hidden && is_hidden(&name, &item) {
            continue;
        }
        let is_folder = meta.is_dir();
        if !is_folder && !wanted(show, &name) {
            continue;
        }
        let project = (!is_folder).then(|| project_info(&path)).flatten();
        let (modules_of, projects) = if is_folder {
            let beside = folder.join(format!("{name}.{}", clarity_spec::SPEC_EXTENSION));
            (project_info(&beside).map(|p| p.title), projects_in(&path))
        } else {
            (None, Vec::new())
        };
        entries.push(Entry { name, path, folder: is_folder, size: if is_folder { 0 } else { meta.len() }, project, modules_of, projects });
    }
    entries.sort_by(|a, b| b.folder.cmp(&a.folder).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())).then_with(|| a.name.cmp(&b.name)));
    Listing { folder: folder.to_path_buf(), up, entries, problem: None }
}

/// The project files directly in `folder`, by name.
fn projects_in(folder: &Path) -> Vec<ProjectInfo> {
    let Ok(read) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut files: Vec<PathBuf> = read.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
    files.sort();
    files.iter().filter_map(|p| project_info(p)).collect()
}

/// `path` as a project file, when it is one: a `.clarity` file whose first
/// line (blank lines and `//` comments left out) is `project ...`, read with
/// the reader opening uses.
pub fn project_info(path: &Path) -> Option<ProjectInfo> {
    if path.extension().and_then(|e| e.to_str()) != Some(clarity_spec::SPEC_EXTENSION) || !path.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let first = text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with("//"))?;
    let named = first.strip_prefix("project")?;
    if !named.is_empty() && !named.starts_with(char::is_whitespace) {
        return None;
    }
    let loaded = crate::load(path);
    let fallback = || named.trim().to_string();
    Some(match &loaded.settings {
        Some(settings) => ProjectInfo {
            file: path.to_path_buf(),
            title: if loaded.title.is_empty() { fallback() } else { loaded.title.clone() },
            stack: stack_words(&settings.stack),
            problem: loaded.problems.first().cloned(),
            readme: readme_opening(path),
        },
        None => ProjectInfo {
            file: path.to_path_buf(),
            title: fallback(),
            stack: String::new(),
            problem: Some(loaded.problems.first().cloned().unwrap_or_else(|| "the project file does not read".into())),
            readme: readme_opening(path),
        },
    })
}

/// How long a card's README lines are at most.
const README_LENGTH: usize = 200;

/// The opening of the README beside the project file `file` (in the folder of
/// its modules, as the project keeps it): the first paragraph that is not a
/// heading, its lines joined, cut at a word to a couple of lines.
fn readme_opening(file: &Path) -> Option<String> {
    let text = std::fs::read_to_string(file.with_extension("").join(clarity_spec::README_FILE)).ok()?;
    let mut paragraph: Vec<&str> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        paragraph.push(line);
    }
    let joined = paragraph.join(" ");
    if joined.is_empty() {
        return None;
    }
    if joined.len() <= README_LENGTH {
        return Some(joined);
    }
    let mut cut = README_LENGTH;
    while !joined.is_char_boundary(cut) {
        cut -= 1;
    }
    let head = &joined[..cut];
    let head = head.rfind(' ').map_or(head, |space| &head[..space]);
    Some(format!("{}...", head.trim_end_matches([',', ';', ':', '.'])))
}

/// A stack in words, as the status bar says it: the backend, the UI when
/// there is a window, and the targets (`C#, Clarity.UI, Desktop and CLI`).
pub fn stack_words(stack: &Stack) -> String {
    let catalog = crate::catalog();
    let backend = catalog.backends.iter().find(|b| b.id == stack.backend).map(|b| b.label).unwrap_or_default();
    let ui = catalog.uis.iter().find(|u| u.id == stack.ui).map(|u| u.label).unwrap_or_default();
    let targets: Vec<&str> = stack.targets.iter().filter_map(|t| catalog.targets.iter().find(|c| c.id == *t).map(|c| c.label)).collect();
    let mut parts = vec![backend];
    if stack.targets.contains(&Target::Desktop) {
        parts.push(ui);
    }
    format!("{}, {}", parts.join(", "), joined(&targets))
}

/// `a`, `a and b`, `a, b and c`.
fn joined(items: &[&str]) -> String {
    match items.split_last() {
        Some((last, [])) => last.to_string(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

fn wanted(show: &Show, name: &str) -> bool {
    match show {
        Show::Everything => true,
        Show::Folders => false,
        Show::Files(kinds) => {
            let extension = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or_default().to_lowercase();
            kinds.iter().any(|k| k.trim_start_matches('.').to_lowercase() == extension)
        }
    }
}

/// Hidden: a name starting with a dot, or marked hidden on Windows.
fn is_hidden(name: &str, item: &std::fs::DirEntry) -> bool {
    if name.starts_with('.') {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        if item.metadata().is_ok_and(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0) {
            return true;
        }
    }
    let _ = item;
    false
}

/// Where browsing can start above everything: `/` on Linux and macOS, the drives on Windows.
pub fn roots() -> Vec<PathBuf> {
    if cfg!(windows) {
        (b'A'..=b'Z').map(|d| PathBuf::from(format!("{}:\\", d as char))).filter(|p| p.is_dir()).collect()
    } else {
        vec![PathBuf::from("/")]
    }
}

/// The person's home folder.
pub fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}
