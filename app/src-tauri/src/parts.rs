//! Custom parts in the IDE (DESIGN 5.19): the files of a part's library,
//! read and written from Clarity, and the header and starter written from
//! its declaration. The `clarity` folder of a native part is Clarity's, so
//! its header is shown and never written from here.

use std::path::{Path, PathBuf};

use clarity_spec::code::PartLibrary;
use serde::Serialize;
use tauri::State;

use crate::WatchState;

/// A file of a part's folder: its path in the folder, and whether it is
/// Clarity's (written from the declaration) rather than the developer's.
#[derive(Debug, Clone, Serialize)]
pub struct PartFileInfo {
    pub path: String,
    pub own: bool,
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

/// The part's folder in the project, and the project.
fn folder_of(state: &WatchState, name: &str) -> Result<(PathBuf, clarity_spec::Project), String> {
    let dir = crate::open_dir_of(state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let part = project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == name).ok_or_else(|| format!("there is no part {name}"))?;
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
    let own = project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == name).and_then(|p| p.library.as_ref()).and_then(|l| own_file(l, &name));
    let mut files: Vec<PartFileInfo> = clarity_spec::native::native_files(&dir, &project)
        .map_err(|err| err.to_string())?
        .into_iter()
        .filter(|f| f.part == name && folder.join(&f.path).is_file() && Some(&f.path) != own.as_ref())
        .map(|f| PartFileInfo { path: f.path, own: false })
        .collect();
    if let Some(own) = own.filter(|o| folder.join(o).is_file()) {
        files.push(PartFileInfo { path: own, own: true });
    }
    Ok(files)
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
    let own = project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == name).and_then(|p| p.library.as_ref()).and_then(|l| own_file(l, &name));
    if Some(&path) == own.as_ref() {
        return Err(format!("{path} is Clarity's: it follows the part's declaration"));
    }
    let file = inside(&folder, &path)?;
    std::fs::write(&file, text).map_err(|err| format!("cannot write {}: {err}", file.display()))
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
