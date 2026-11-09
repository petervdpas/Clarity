//! Custom parts in the IDE (DESIGN 5.19): the files of a part's library,
//! read and written from Clarity, and the header and starter written from
//! its declaration. The `clarity` folder of a native part is Clarity's, so
//! its header is shown and never written from here.

use std::path::{Path, PathBuf};

use clarity_spec::code::PartLibrary;
use tauri::State;

use crate::WatchState;

pub use clarity_tools::parts::{PartFileInfo, own_of, part_folders, tree};

/// The folder of the part or library `name` in the project's folder `dir`.
fn folder_in(dir: &Path, project: &clarity_spec::Project, name: &str) -> Result<PathBuf, String> {
    if let Some(found) = part_folders(dir, project).into_iter().find(|f| f.name == name) {
        return Ok(dir.join(found.folder));
    }
    match project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == name) {
        Some(_) => Err(format!("part {name} names no library yet")),
        None => Err(format!("there is no part or library {name}")),
    }
}

/// The folder of the part or library `name` in the open project, and the project.
fn folder_of(state: &WatchState, name: &str) -> Result<(PathBuf, clarity_spec::Project), String> {
    let dir = crate::open_dir_of(state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    Ok((folder_in(&dir, &project, name)?, project))
}

/// A path inside the folder: no way out of it.
fn inside(folder: &Path, path: &str) -> Result<PathBuf, String> {
    if path.split('/').any(|piece| piece == ".." || piece.is_empty()) || path.starts_with('/') {
        return Err(format!("'{path}' is not a file of the part"));
    }
    Ok(folder.join(path))
}

/// The part's files, the developer's first: what its library is made of.
#[tauri::command]
pub fn part_files(name: String, state: State<WatchState>) -> Result<Vec<PartFileInfo>, String> {
    let (folder, project) = folder_of(&state, &name)?;
    let dir = crate::open_dir_of(&state)?;
    let own = own_of(&dir, &project, &name);
    Ok(tree(&folder, own.as_deref()))
}

/// Whether `path` is Clarity's file, or a folder that holds it.
fn holds_own(path: &str, own: Option<&str>) -> bool {
    own.is_some_and(|own| own == path || own.starts_with(&format!("{path}/")))
}

/// Makes an empty file, or a folder, in the part's folder.
#[tauri::command]
pub fn new_part_file(name: String, path: String, folder: bool, state: State<WatchState>) -> Result<(), String> {
    let (root, _) = folder_of(&state, &name)?;
    let target = inside(&root, &path)?;
    if target.exists() {
        return Err(format!("{path} is there already"));
    }
    if folder {
        return std::fs::create_dir_all(&target).map_err(|err| format!("cannot make {}: {err}", target.display()));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|err| format!("cannot make {}: {err}", parent.display()))?;
    }
    std::fs::write(&target, "").map_err(|err| format!("cannot make {}: {err}", target.display()))
}

/// Renames or moves a file or folder of the part's folder; Clarity's file stays.
#[tauri::command]
pub fn rename_part_file(name: String, from: String, to: String, state: State<WatchState>) -> Result<(), String> {
    let (root, project) = folder_of(&state, &name)?;
    let dir = crate::open_dir_of(&state)?;
    if holds_own(&from, own_of(&dir, &project, &name).as_deref()) {
        return Err(format!("{from} is Clarity's: it follows the declaration"));
    }
    let (source, target) = (inside(&root, &from)?, inside(&root, &to)?);
    if target.exists() {
        return Err(format!("{to} is there already"));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|err| format!("cannot make {}: {err}", parent.display()))?;
    }
    std::fs::rename(&source, &target).map_err(|err| format!("cannot rename {from}: {err}"))
}

/// Deletes a file, or a folder with what is in it; Clarity's file stays.
#[tauri::command]
pub fn delete_part_file(name: String, path: String, state: State<WatchState>) -> Result<(), String> {
    let (root, project) = folder_of(&state, &name)?;
    let dir = crate::open_dir_of(&state)?;
    if holds_own(&path, own_of(&dir, &project, &name).as_deref()) {
        return Err(format!("{path} holds Clarity's file: it follows the declaration"));
    }
    let target = inside(&root, &path)?;
    let removed = if target.is_dir() { std::fs::remove_dir_all(&target) } else { std::fs::remove_file(&target) };
    removed.map_err(|err| format!("cannot delete {path}: {err}"))
}

#[tauri::command]
pub fn part_file_text(name: String, path: String, state: State<WatchState>) -> Result<String, String> {
    let (folder, _) = folder_of(&state, &name)?;
    let file = inside(&folder, &path)?;
    std::fs::read_to_string(&file).map_err(|err| format!("cannot read {}: {err}", file.display()))
}

#[tauri::command]
pub fn save_part_file(name: String, path: String, text: String, state: State<WatchState>) -> Result<(), String> {
    let (folder, project) = folder_of(&state, &name)?;
    let dir = crate::open_dir_of(&state)?;
    let own = own_of(&dir, &project, &name);
    if Some(&path) == own.as_ref() {
        return Err(format!("{path} is Clarity's: it follows the part's declaration"));
    }
    let file = inside(&folder, &path)?;
    std::fs::write(&file, text).map_err(|err| format!("cannot write {}: {err}", file.display()))
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
        env.push(("CLARITY_UI".to_string(), crate::build::library_folder(&app)?.display().to_string()));
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

    #[test]
    fn clarity_s_file_and_its_folder_stay() {
        assert!(holds_own("clarity", Some("clarity/Orbs.h")));
        assert!(holds_own("src/clarity.rs", Some("src/clarity.rs")));
        assert!(!holds_own("src", Some("srcx/clarity.rs")));
        assert!(!holds_own("src/lib.rs", Some("src/clarity.rs")));
    }
}
