//! Custom parts in the IDE (DESIGN 5.19): the files of a part's library,
//! read and written from Clarity, and the header and starter written from
//! its declaration. The `clarity` folder of a native part is Clarity's, so
//! its header is shown and never written from here.

use std::path::{Path, PathBuf};

use clarity_spec::code::PartLibrary;
use serde::Serialize;
use tauri::State;

use crate::WatchState;

/// A file or folder of a part's folder: its path in the folder, whether it
/// is a folder, and whether it is Clarity's (written from the declaration)
/// rather than the developer's.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PartFileInfo {
    pub path: String,
    pub folder: bool,
    pub own: bool,
}

/// Folders a build leaves, which the tree does not show.
const BUILD_FOLDERS: [&str; 5] = ["build", "target", "bin", "obj", "node_modules"];

/// Every file and folder under `folder`, folders first at each level, by
/// name; without build output and hidden entries. `own` is Clarity's file.
pub fn tree(folder: &Path, own: Option<&str>) -> Vec<PartFileInfo> {
    fn walk(folder: &Path, prefix: &str, own: Option<&str>, out: &mut Vec<PartFileInfo>) {
        let Ok(entries) = std::fs::read_dir(folder) else { return };
        let mut entries: Vec<(String, bool)> = entries
            .flatten()
            .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path().is_dir()))
            .filter(|(name, dir)| !name.starts_with('.') && !(*dir && BUILD_FOLDERS.contains(&name.as_str())))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
        for (name, dir) in entries {
            let path = format!("{prefix}{name}");
            if dir {
                out.push(PartFileInfo { path: path.clone(), folder: true, own: false });
                walk(&folder.join(&name), &format!("{path}/"), own, out);
            } else {
                out.push(PartFileInfo { own: Some(path.as_str()) == own, path, folder: false });
            }
        }
    }
    let mut out = Vec::new();
    walk(folder, "", own, &mut out);
    out
}

/// The file of a part's folder that is Clarity's, written from the declaration.
fn own_file(library: &PartLibrary, name: &str) -> Option<String> {
    match library {
        PartLibrary::Native { .. } => Some(format!("{}/{name}.h", clarity_spec::native::OWN_FOLDER)),
        PartLibrary::Crate { .. } => Some("src/clarity.rs".into()),
        PartLibrary::Go { .. } => Some("clarity.go".into()),
        _ => None,
    }
}

/// The file of a library's folder that is Clarity's (DESIGN 5.21).
fn library_own(dir: &Path, project: &clarity_spec::Project, name: &str) -> Option<String> {
    let library = clarity_gen::libraries::library(project, name)?;
    let file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
    let cpp = clarity_project::load(&file).settings.is_some_and(|s| s.stack.backend == clarity_project::Backend::Cpp);
    Some(clarity_gen::libraries::own_file(library, cpp))
}

/// The file of Clarity's in the folder of the part or library `name`.
fn own_of(dir: &Path, project: &clarity_spec::Project, name: &str) -> Option<String> {
    let part = project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == name);
    match part.and_then(|p| p.library.as_ref()) {
        Some(library) => own_file(library, name),
        None => library_own(dir, project, name),
    }
}

/// The folder of the part or library `name` in the project, and the project.
fn folder_of(state: &WatchState, name: &str) -> Result<(PathBuf, clarity_spec::Project), String> {
    let dir = crate::open_dir_of(state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    if clarity_gen::libraries::library(&project, name).is_some() {
        return Ok((dir.join(clarity_spec::native::library_folder(name)), project));
    }
    let part = project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == name).ok_or_else(|| format!("there is no part or library {name}"))?;
    let folder = match &part.library {
        Some(PartLibrary::Native { folder } | PartLibrary::Crate { folder } | PartLibrary::Go { folder }) => folder.clone(),
        Some(_) => clarity_spec::native::adapter_folder(name),
        None => return Err(format!("part {name} names no library yet")),
    };
    Ok((dir.join(folder), project))
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
    let (folder, project) = folder_of(&state, &name)?;
    let dir = crate::open_dir_of(&state)?;
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
    crate::build::start_part(app, name, folder, steps, env)
}

/// Writes every part's header from its declaration, and a starter where a
/// part's folder has none; gives the files written, relative to the project.
#[tauri::command]
pub fn write_part_folders(state: State<WatchState>) -> Result<Vec<String>, String> {
    let dir = crate::open_dir_of(&state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let written = clarity_gen::parts::write_part_folders(&dir, &project).map_err(|err| err.to_string())?;
    Ok(written.iter().map(|p| p.strip_prefix(&dir).unwrap_or(p).display().to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tree_shows_folders_first_without_build_output() {
        let root = std::env::temp_dir().join(format!("clarity-part-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["src", "assets/sounds", "target/debug", ".git"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in ["Cargo.toml", "src/lib.rs", "src/clarity.rs", "assets/sounds/roll.ogg", "target/debug/x", ".git/HEAD"] {
            std::fs::write(root.join(file), "").unwrap();
        }
        let shown: Vec<(String, bool, bool)> = tree(&root, Some("src/clarity.rs")).into_iter().map(|f| (f.path, f.folder, f.own)).collect();
        std::fs::remove_dir_all(&root).unwrap();
        let expected = [
            ("assets", true, false),
            ("assets/sounds", true, false),
            ("assets/sounds/roll.ogg", false, false),
            ("src", true, false),
            ("src/clarity.rs", false, true),
            ("src/lib.rs", false, false),
            ("Cargo.toml", false, false),
        ];
        assert_eq!(shown, expected.map(|(p, d, o)| (p.to_string(), d, o)));
    }

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
