//! Custom parts in the IDE (manual/design.custom-parts.md): the files of a part's library,
//! read and written from Clarity, and the header and starter written from
//! its declaration. The `clarity` folder of a native part is Clarity's, so
//! its header is shown and never written from here.

use std::path::{Path, PathBuf};

use clarity_spec::code::PartLibrary;
use tauri::State;

use clarity_tools::files::{FileChange, write_text};
use clarity_tools::parts::{PartFolder, holds_own, part_path};

use crate::{HistoryState, WatchState};

pub use clarity_tools::parts::{PartFileInfo, part_folders, tree};

/// The folder of the part or library `name` in the project's folder `dir`.
fn part_folder_in(dir: &Path, project: &clarity_spec::Project, name: &str) -> Result<PartFolder, String> {
    if let Some(found) = part_folders(dir, project).into_iter().find(|f| f.name == name) {
        return Ok(found);
    }
    match project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == name) {
        Some(_) => Err(format!("part {name} names no library yet")),
        None => Err(format!("there is no part or library {name}")),
    }
}

fn folder_in(dir: &Path, project: &clarity_spec::Project, name: &str) -> Result<PathBuf, String> {
    Ok(dir.join(part_folder_in(dir, project, name)?.folder))
}

/// The open project's folder, and the folder of its part or library `name`.
fn folder_of(state: &WatchState, name: &str) -> Result<(PathBuf, PartFolder), String> {
    let dir = crate::open_dir_of(state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let folder = part_folder_in(&dir, &project, name)?;
    Ok((dir, folder))
}

/// A path inside the part's folder, by the one rule of part files (clarity_tools::parts::part_path).
fn inside(dir: &Path, folder: &PartFolder, path: &str) -> Result<PathBuf, String> {
    Ok(dir.join(&folder.folder).join(part_path(folder, path)?))
}

/// The part's files, the developer's first: what its library is made of.
#[tauri::command]
pub fn part_files(name: String, state: State<WatchState>) -> Result<Vec<PartFileInfo>, String> {
    let (dir, folder) = folder_of(&state, &name)?;
    Ok(tree(&dir.join(&folder.folder), folder.own.as_deref()))
}

/// Makes an empty file, or a folder, in the part's folder.
#[tauri::command]
pub fn new_part_file(name: String, path: String, folder: bool, state: State<WatchState>, history: State<HistoryState>) -> Result<(), String> {
    let (dir, part) = folder_of(&state, &name)?;
    let target = inside(&dir, &part, &path)?;
    if target.exists() {
        return Err(format!("{path} is there already"));
    }
    if folder {
        return std::fs::create_dir_all(&target).map_err(|err| format!("cannot make {}: {err}", target.display()));
    }
    // A file is written the one way, for Undo (clarity_tools::files::write_text).
    let change = write_text(&clarity_tools::parts::part_file(&dir, &part, &path)?, "")?;
    crate::record(&history, vec![change])
}

/// Renames or moves a file or folder of the part's folder; Clarity's file
/// stays. One Undo brings the old file back and takes the new one away.
#[tauri::command]
pub fn rename_part_file(name: String, from: String, to: String, state: State<WatchState>, history: State<HistoryState>) -> Result<(), String> {
    let (dir, folder) = folder_of(&state, &name)?;
    recorded(&history, rename_in(&dir, &folder, &from, &to)?)
}

/// Deletes a file, or a folder with what is in it; Clarity's file stays.
/// One Undo writes back what was deleted.
#[tauri::command]
pub fn delete_part_file(name: String, path: String, state: State<WatchState>, history: State<HistoryState>) -> Result<(), String> {
    let (dir, folder) = folder_of(&state, &name)?;
    recorded(&history, delete_in(&dir, &folder, &path)?)
}

/// Keeps the changes as one Undo; a change of no text file (an empty folder) has none.
fn recorded(history: &HistoryState, changes: Vec<FileChange>) -> Result<(), String> {
    if changes.is_empty() { Ok(()) } else { crate::record(history, changes) }
}

/// The text files at `path`: the file itself, or every file under the
/// folder, as (path below `path`, text), in name order. A file that is not
/// text is not among them.
fn texts_at(path: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(below) = stack.pop() {
        let at = under(path, &below);
        if at.is_dir() {
            let Ok(entries) = std::fs::read_dir(&at) else { continue };
            let mut names: Vec<_> = entries.flatten().map(|e| e.file_name()).collect();
            names.sort();
            stack.extend(names.into_iter().rev().map(|n| below.join(n)));
        } else if let Ok(text) = std::fs::read_to_string(&at) {
            found.push((below, text));
        }
    }
    found
}

/// `below` under `path`; `path` itself when `below` is empty (a file, not a folder).
fn under(path: &Path, below: &Path) -> PathBuf {
    if below.as_os_str().is_empty() { path.to_path_buf() } else { path.join(below) }
}

/// Renames `from` to `to` in the part's folder, by the rule of part files:
/// each text file's old place removed and its new one written.
fn rename_in(dir: &Path, folder: &PartFolder, from: &str, to: &str) -> Result<Vec<FileChange>, String> {
    if holds_own(folder, from) {
        return Err(format!("{from} is Clarity's: it follows the declaration"));
    }
    let (source, target) = (inside(dir, folder, from)?, inside(dir, folder, to)?);
    if target.exists() {
        return Err(format!("{to} is there already"));
    }
    let texts = texts_at(&source);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|err| format!("cannot make {}: {err}", parent.display()))?;
    }
    std::fs::rename(&source, &target).map_err(|err| format!("cannot rename {from}: {err}"))?;
    let removed = texts.iter().map(|(below, text)| FileChange { file: under(&source, below), before: Some(text.clone()), after: None });
    let written = texts.iter().map(|(below, text)| FileChange { file: under(&target, below), before: None, after: Some(text.clone()) });
    Ok(removed.chain(written).collect())
}

/// Deletes `path` in the part's folder, by the rule of part files, with the
/// text of each file deleted.
fn delete_in(dir: &Path, folder: &PartFolder, path: &str) -> Result<Vec<FileChange>, String> {
    if holds_own(folder, path) {
        return Err(format!("{path} holds Clarity's file: it follows the declaration"));
    }
    let target = inside(dir, folder, path)?;
    let texts = texts_at(&target);
    let removed = if target.is_dir() { std::fs::remove_dir_all(&target) } else { std::fs::remove_file(&target) };
    removed.map_err(|err| format!("cannot delete {path}: {err}"))?;
    Ok(texts.into_iter().map(|(below, text)| FileChange { file: under(&target, &below), before: Some(text), after: None }).collect())
}

#[tauri::command]
pub fn part_file_text(name: String, path: String, state: State<WatchState>) -> Result<String, String> {
    let (dir, folder) = folder_of(&state, &name)?;
    let file = inside(&dir, &folder, &path)?;
    std::fs::read_to_string(&file).map_err(|err| format!("cannot read {}: {err}", file.display()))
}

/// Saves a file of the part's folder the one way the agents' tools write
/// one (clarity_tools::parts::part_file, clarity_tools::files::write_text),
/// kept for Undo; Clarity's own file is never written.
#[tauri::command]
pub fn save_part_file(name: String, path: String, text: String, state: State<WatchState>, history: State<HistoryState>) -> Result<(), String> {
    let (dir, folder) = folder_of(&state, &name)?;
    let change = write_text(&clarity_tools::parts::part_file(&dir, &folder, &path)?, &text)?;
    crate::record(&history, vec![change])
}

/// How a part or library builds on its own, by its language: the tools to
/// run in its folder. None when it is built only with the program (a .NET
/// adapter, a C or C++ library, which links Clarity's runtime).
pub fn build_steps(kind: &str) -> Option<Vec<Vec<String>>> {
    let step = |words: &[&str]| words.iter().map(|w| w.to_string()).collect::<Vec<_>>();
    match kind {
        "native" => Some(vec![step(&["cmake", "-S", ".", "-B", "build"]), step(&["cmake", "--build", "build"])]),
        "crate" => Some(vec![step(&["cargo", "build", "--color", "never"])]),
        "go" => Some(vec![step(&["go", "build", "./..."])]),
        _ => None,
    }
}

/// Builds the part or library `name` on its own (see `build_steps`). A
/// crate builds into the program's target folder, so what the program
/// built (Bevy, say) is not built twice.
#[tauri::command]
pub fn build_part(name: String, state: State<WatchState>, app: tauri::AppHandle) -> Result<(), String> {
    let dir = crate::open_dir_of(&state)?;
    start_part_build(app, &dir, &name)
}

/// Starts building the part or library `name` of the project in `dir` on its
/// own; its output goes to the Output panel, as Build in its view does.
pub fn start_part_build(app: tauri::AppHandle, dir: &Path, name: &str) -> Result<(), String> {
    let name = name.to_string();
    let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let folder = folder_in(dir, &project, &name)?;
    let part = project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == name);
    let kind = match (part.and_then(|p| p.library.as_ref()), clarity_gen::libraries::library(&project, &name).and_then(|l| l.source.as_ref())) {
        (Some(PartLibrary::Native { .. }), _) => "native",
        (Some(PartLibrary::Crate { .. }), _) | (_, Some(clarity_spec::code::LibrarySource::Crate { .. })) => "crate",
        (Some(PartLibrary::Go { .. }), _) | (_, Some(clarity_spec::code::LibrarySource::Go { .. })) => "go",
        _ => "program",
    };
    let steps = build_steps(kind).ok_or_else(|| format!("{name} is built with the program: use Build in the toolbar"))?;
    let mut env = Vec::new();
    if kind == "crate" {
        let file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
        if let Some(settings) = clarity_project::load(&file).settings {
            env.push(("CARGO_TARGET_DIR".to_string(), clarity_project::out_dir(&file, &settings).join("target").display().to_string()));
        }
    }
    // A C or C++ part includes clarity_ui.h, which its CMakeLists finds through CLARITY_UI.
    if kind == "native" {
        env.push(("CLARITY_UI".to_string(), clarity_project::plain(crate::build::library_folder(&app)?).display().to_string()));
    }
    crate::build::start_part(app, name, folder, steps, env)
}

pub use clarity_tools::parts::builds_alone;

/// Writes every part's header from its declaration, and a starter where a
/// part's folder has none; gives the files written, relative to the project.
#[tauri::command]
pub fn write_part_folders(state: State<WatchState>) -> Result<Vec<String>, String> {
    let dir = crate::open_dir_of(&state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let written = clarity_gen::parts::write_part_folders(&dir, &project).map_err(|err| err.to_string())?;
    Ok(written.iter().map(|p| clarity_project::slashed(p.strip_prefix(&dir).unwrap_or(p))).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_language_builds_with_its_own_tool() {
        assert_eq!(build_steps("native").unwrap()[0][0], "cmake");
        assert_eq!(build_steps("crate").unwrap(), [["cargo", "build", "--color", "never"]]);
        assert_eq!(build_steps("go").unwrap(), [["go", "build", "./..."]]);
        assert_eq!(build_steps("program"), None);
    }

    /// A project's folder with the native part gauge's folder in it: two
    /// files of the developer's and Clarity's own header.
    fn gauge(name: &str) -> (PathBuf, PartFolder) {
        let dir = std::env::temp_dir().join(format!("clarity-ide-parts-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("parts/gauge/src/inner")).unwrap();
        std::fs::create_dir_all(dir.join("parts/gauge/clarity")).unwrap();
        std::fs::write(dir.join("parts/gauge/src/gauge.c"), "int gauge;\n").unwrap();
        std::fs::write(dir.join("parts/gauge/src/inner/dial.h"), "int dial;\n").unwrap();
        std::fs::write(dir.join("parts/gauge/clarity/gauge.h"), "// Clarity's\n").unwrap();
        let folder = PartFolder { kind: "part", name: "gauge".into(), language: "C or C++", folder: "parts/gauge".into(), own: Some("clarity/gauge.h".into()) };
        (dir, folder)
    }

    /// The changes as (file in the part's folder, before, after), in order.
    fn shown(dir: &Path, changes: &[clarity_tools::files::FileChange]) -> Vec<(String, Option<String>, Option<String>)> {
        changes.iter().map(|c| (clarity_project::slashed(c.file.strip_prefix(dir.join("parts/gauge")).unwrap()), c.before.clone(), c.after.clone())).collect()
    }

    #[test]
    fn deleting_a_file_keeps_its_text_for_undo() {
        let (dir, folder) = gauge("delete-file");
        let changes = delete_in(&dir, &folder, "src/gauge.c").unwrap();
        assert_eq!(shown(&dir, &changes), [("src/gauge.c".to_string(), Some("int gauge;\n".to_string()), None)]);
        assert!(!dir.join("parts/gauge/src/gauge.c").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn deleting_a_folder_keeps_the_text_of_every_file_in_it() {
        let (dir, folder) = gauge("delete-folder");
        let changes = delete_in(&dir, &folder, "src").unwrap();
        assert_eq!(
            shown(&dir, &changes),
            [("src/gauge.c".to_string(), Some("int gauge;\n".to_string()), None), ("src/inner/dial.h".to_string(), Some("int dial;\n".to_string()), None)]
        );
        assert!(!dir.join("parts/gauge/src").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn renaming_removes_the_old_file_and_writes_the_new_in_one_change() {
        let (dir, folder) = gauge("rename");
        let changes = rename_in(&dir, &folder, "src/gauge.c", "src/meter/gauge.c").unwrap();
        assert_eq!(
            shown(&dir, &changes),
            [("src/gauge.c".to_string(), Some("int gauge;\n".to_string()), None), ("src/meter/gauge.c".to_string(), None, Some("int gauge;\n".to_string()))]
        );
        assert!(!dir.join("parts/gauge/src/gauge.c").exists());
        assert_eq!(std::fs::read_to_string(dir.join("parts/gauge/src/meter/gauge.c")).unwrap(), "int gauge;\n");
        let folder_moved = rename_in(&dir, &folder, "src/inner", "src/outer").unwrap();
        assert_eq!(
            shown(&dir, &folder_moved),
            [("src/inner/dial.h".to_string(), Some("int dial;\n".to_string()), None), ("src/outer/dial.h".to_string(), None, Some("int dial;\n".to_string()))]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn renaming_and_deleting_keep_the_rule_of_part_files() {
        let (dir, folder) = gauge("rule");
        assert!(delete_in(&dir, &folder, "clarity/gauge.h").is_err(), "Clarity's file stays");
        assert!(delete_in(&dir, &folder, "clarity").is_err(), "the folder of Clarity's file stays");
        assert!(rename_in(&dir, &folder, "clarity/gauge.h", "src/gauge.h").is_err());
        assert!(delete_in(&dir, &folder, "../gauge").is_err(), "nothing outside the part's folder");
        assert!(rename_in(&dir, &folder, "src/gauge.c", "../gauge.c").is_err());
        assert!(rename_in(&dir, &folder, "src/gauge.c", "src/inner/dial.h").is_err(), "a file there already stays");
        assert!(dir.join("parts/gauge/clarity/gauge.h").exists() && dir.join("parts/gauge/src/gauge.c").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
