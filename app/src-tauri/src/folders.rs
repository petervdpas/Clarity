//! The Folders settings (Clarity settings, for every project): where new
//! projects go and Open starts, and where the apps are written. Kept in
//! Clarity's own folder; see clarity_project::folders.

use std::path::{Path, PathBuf};

use clarity_project::{folders, Folders};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct FoldersView {
    /// Where New makes a project and Open starts; none when not set.
    pub projects: Option<String>,
    /// Where the apps are written, a folder per project; none when not set.
    pub output: Option<String>,
    /// Where the choice is kept.
    pub file: String,
}

fn view_of(folders: &Folders, file: &Path) -> FoldersView {
    let shown = |p: &Option<PathBuf>| p.as_ref().map(|p| p.display().to_string());
    FoldersView { projects: shown(&folders.projects), output: shown(&folders.output), file: file.display().to_string() }
}

/// Sets the folder `which` (`projects` or `output`) to `path` in `file`,
/// making the folder when it is not there yet, or forgets it without a path.
fn set(file: &Path, which: &str, path: Option<String>) -> Result<FoldersView, String> {
    let path = path.map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
    if let Some(path) = &path {
        if let Some(problem) = Folders::problem(path) {
            return Err(problem);
        }
        std::fs::create_dir_all(path).map_err(|err| format!("cannot make the folder {path}: {err}"))?;
    }
    let mut folders = Folders::load(file);
    let chosen = path.map(PathBuf::from);
    match which {
        "projects" => folders.projects = chosen,
        "output" => folders.output = chosen,
        other => return Err(format!("'{other}' is not one of Clarity's folders; they are projects and output")),
    }
    folders.save(file).map_err(|err| format!("cannot write {}: {err}", file.display()))?;
    Ok(view_of(&folders, file))
}

/// The folders as they are set.
#[tauri::command]
pub fn folders_view() -> FoldersView {
    let file = folders::file();
    view_of(&Folders::load(&file), &file)
}

/// Sets one folder, or forgets it without a path; gives the folders as they are then.
#[tauri::command]
pub fn set_folder(which: String, path: Option<String>) -> Result<FoldersView, String> {
    set(&folders::file(), &which, path)
}

/// The project file a new project named `name` gets in the projects folder.
#[tauri::command]
pub fn new_project_file(name: String) -> Result<String, String> {
    Ok(Folders::current().new_project_file(&name)?.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_is_made_set_and_forgotten() {
        let dir = std::env::temp_dir().join(format!("clarity-folders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("folders.json");
        let projects = dir.join("projects").display().to_string();
        let view = set(&file, "projects", Some(projects.clone())).unwrap();
        assert_eq!(view.projects.as_deref(), Some(projects.as_str()));
        assert_eq!(view.output, None);
        assert!(Path::new(&projects).is_dir());
        // Forgetting one keeps the other.
        let output = dir.join("apps").display().to_string();
        set(&file, "output", Some(output.clone())).unwrap();
        let view = set(&file, "projects", None).unwrap();
        assert_eq!((view.projects, view.output.as_deref()), (None, Some(output.as_str())));
        assert!(set(&file, "output", Some("apps".into())).unwrap_err().contains("not a full path"));
        assert!(set(&file, "elsewhere", None).unwrap_err().contains("projects and output"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
