//! The resources of a project, under i18n in the project tree: the files of
//! its `resources` folder, each by its own name. A Markdown file is made
//! and written here; a picture is copied in. Which file each language shows
//! is up to the keys in the language files (`gameRules resource "rules-nl.md"`).

use std::path::{Path, PathBuf};

use clarity_spec::Preview;
use clarity_spec::resources::{RESOURCES_DIR, is_markdown, is_picture};
use tauri::State;

use super::{FileChange, HistoryState, WatchState, load, open_project_of, record};

/// The file of resource `name` in the module folder `dir`; only a plain file name.
fn resource_file(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\']) {
        return Err(format!("'{name}' is not a file name for the resources folder"));
    }
    Ok(dir.join(RESOURCES_DIR).join(name))
}

/// A new Markdown file `name` (`.md` added when it has none), starting with a heading.
fn new_markdown_file(dir: &Path, name: &str) -> Result<FileChange, String> {
    let name = name.trim();
    let name = if is_markdown(name) { name.to_string() } else { format!("{name}.md") };
    let file = resource_file(dir, &name)?;
    if file.exists() {
        return Err(format!("the resources have {name} already"));
    }
    let title = name[..name.len() - 3].replace(['-', '_'], " ");
    let mut letters = title.chars();
    let heading: String = letters.next().map(|first| first.to_uppercase().chain(letters).collect()).unwrap_or_default();
    Ok(FileChange { file, before: None, after: format!("# {heading}\n") })
}

/// Copies the picture at `from` into the resources folder, under its own name.
fn copy_picture(dir: &Path, from: &Path) -> Result<PathBuf, String> {
    let name = from.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let to = resource_file(dir, &name)?;
    if !is_picture(&name) {
        return Err(format!("'{name}' is not a picture every stack can show; use a .png, .jpg, .gif, .bmp or .webp file"));
    }
    if to.exists() {
        return Err(format!("the resources have {name} already"));
    }
    std::fs::create_dir_all(dir.join(RESOURCES_DIR)).map_err(|err| format!("cannot make the resources folder: {err}"))?;
    std::fs::copy(from, &to).map_err(|err| format!("cannot copy {}: {err}", from.display()))?;
    Ok(to)
}

/// Writes `files`, so one Undo takes them all back.
fn write(files: Vec<FileChange>, history: &State<HistoryState>) -> Result<(), String> {
    for change in &files {
        if let Some(folder) = change.file.parent() {
            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
        }
        std::fs::write(&change.file, &change.after).map_err(|err| format!("cannot write {}: {err}", change.file.display()))?;
    }
    record(history, files)
}

/// Makes Markdown file `name` in the resources.
#[tauri::command]
pub fn new_markdown(name: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    write(vec![new_markdown_file(&dir, &name)?], &history)?;
    load(&dir)
}

/// Writes the text of Markdown file `name`.
#[tauri::command]
pub fn save_markdown(name: String, text: String, state: State<WatchState>, history: State<HistoryState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    let file = resource_file(&dir, &name)?;
    if !is_markdown(&name) {
        return Err(format!("'{name}' is not a Markdown file"));
    }
    let before = std::fs::read_to_string(&file).ok();
    if before.as_deref() != Some(text.as_str()) {
        write(vec![FileChange { file, before, after: text }], &history)?;
    }
    load(&dir)
}

/// Adds a picture, picked from a file, to the resources.
#[tauri::command]
pub fn add_picture(path: String, state: State<WatchState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    copy_picture(&dir, Path::new(&path))?;
    load(&dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("clarity-resources-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_resource_is_a_plain_file_of_the_resources_folder() {
        let dir = folder("names");
        assert_eq!(resource_file(&dir, "rules-nl.md").unwrap(), dir.join("resources/rules-nl.md"));
        for bad in ["../rules.md", "a/b.md", ".hidden", ""] {
            assert!(resource_file(&dir, bad).is_err(), "{bad}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_new_markdown_file_starts_with_its_heading() {
        let dir = folder("markdown");
        let made = new_markdown_file(&dir, "how-to").unwrap();
        assert_eq!(made.file, dir.join("resources/how-to.md"));
        assert_eq!((made.before, made.after.as_str()), (None, "# How to\n"));
        assert_eq!(new_markdown_file(&dir, "rules-nl.md").unwrap().after, "# Rules nl\n");
        std::fs::create_dir_all(dir.join("resources")).unwrap();
        std::fs::write(dir.join("resources/how-to.md"), "x").unwrap();
        assert!(new_markdown_file(&dir, "how-to.md").unwrap_err().contains("already"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_picture_is_copied_in_under_its_name() {
        let dir = folder("picture");
        let from = dir.join("logo.png");
        std::fs::write(&from, [1, 2, 3]).unwrap();
        assert_eq!(copy_picture(&dir, &from).unwrap(), dir.join("resources/logo.png"));
        assert_eq!(std::fs::read(dir.join("resources/logo.png")).unwrap(), [1, 2, 3]);
        assert!(copy_picture(&dir, &from).unwrap_err().contains("already"));
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        assert!(copy_picture(&dir, &dir.join("notes.txt")).unwrap_err().contains("not a picture"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
