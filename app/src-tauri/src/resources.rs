//! The resources of a project, under i18n in the project tree: the files of
//! its `resources` folder, each by its own name and of a type (Markdown, a
//! picture, a PDF, a drawing or another file). A resource is made from a
//! picked file, copied in under its name, or as Markdown to write here; a
//! picked file of its type replaces it. Which file each language shows is
//! up to the keys in the language files (`gameRules resource "rules-nl.md"`).

use std::path::{Path, PathBuf};

use clarity_spec::Preview;
use clarity_spec::resources::{RESOURCES_DIR, ResourceKind, ResourceKindInfo, is_markdown, kinds};
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

/// The extension of a file name, in small letters.
fn extension(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// The type of resource `kind`, with its name and extensions.
fn info(kind: ResourceKind) -> ResourceKindInfo {
    kinds().into_iter().find(|k| k.kind == kind).expect("every type is listed")
}

/// Makes resource `name` of type `kind` in the resources folder: a copy of
/// the file `from`, renamed to `name` with the file's extension, or, for
/// Markdown without a file, a heading to write under.
fn new_resource(dir: &Path, name: &str, kind: ResourceKind, from: Option<&Path>) -> Result<PathBuf, String> {
    let info = info(kind);
    let stem = name.trim();
    let stem = info.extensions.iter().find_map(|e| stem.strip_suffix(&format!(".{e}"))).unwrap_or(stem);
    let extension = match from {
        Some(from) => {
            let found = extension(from);
            if !info.extensions.is_empty() && !info.extensions.contains(&found) {
                let shown = from.file_name().unwrap_or_default().to_string_lossy();
                return Err(format!("{shown} is not a {}: a {} is a .{} file", info.name, info.name, info.extensions.join(", .")));
            }
            found
        }
        None if kind == ResourceKind::Markdown => info.extensions[0].clone(),
        None => return Err(format!("a {} starts from a file: pick the file to copy in", info.name)),
    };
    let file_name = if extension.is_empty() { stem.to_string() } else { format!("{stem}.{extension}") };
    let to = resource_file(dir, &file_name)?;
    if to.exists() {
        return Err(format!("the resources have {file_name} already"));
    }
    std::fs::create_dir_all(dir.join(RESOURCES_DIR)).map_err(|err| format!("cannot make the resources folder: {err}"))?;
    match from {
        Some(from) => {
            std::fs::copy(from, &to).map_err(|err| format!("cannot copy {}: {err}", from.display()))?;
        }
        None => {
            let title = stem.replace(['-', '_'], " ");
            let mut letters = title.chars();
            let heading: String = letters.next().map(|first| first.to_uppercase().chain(letters).collect()).unwrap_or_default();
            std::fs::write(&to, format!("# {heading}\n")).map_err(|err| format!("cannot write {}: {err}", to.display()))?;
        }
    }
    Ok(to)
}

/// Overwrites resource `name` with the file `from`, of the same type and
/// extension; the resource keeps its name.
fn replace_resource(dir: &Path, name: &str, from: &Path) -> Result<PathBuf, String> {
    let to = resource_file(dir, name)?;
    if !to.is_file() {
        return Err(format!("the resources have no {name}"));
    }
    if extension(from) != extension(&to) {
        let shown = from.file_name().unwrap_or_default().to_string_lossy();
        return Err(format!("{name} takes a .{} file, as its name says; {shown} is not one", extension(&to)));
    }
    std::fs::copy(from, &to).map_err(|err| format!("cannot copy {}: {err}", from.display()))?;
    Ok(to)
}

/// The types a resource can have, for the New resource dialog and the file pickers.
#[tauri::command]
pub fn resource_kinds() -> Vec<ResourceKindInfo> {
    kinds()
}

/// Makes resource `name` of type `kind`, from the file `from` when one is picked.
#[tauri::command]
pub fn add_resource(name: String, kind: ResourceKind, from: Option<String>, state: State<WatchState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    new_resource(&dir, &name, kind, from.as_deref().map(Path::new))?;
    load(&dir)
}

/// Overwrites resource `name` with the picked file `from`.
#[tauri::command]
pub fn replace_resource_file(name: String, from: String, state: State<WatchState>) -> Result<Preview, String> {
    let (_, dir) = open_project_of(&state)?;
    replace_resource(&dir, &name, Path::new(&from))?;
    load(&dir)
}

/// Writes `files`, so one Undo takes them all back.
fn write(files: Vec<FileChange>, history: &State<HistoryState>) -> Result<(), String> {
    for change in &files {
        if let Some(folder) = change.file.parent() {
            std::fs::create_dir_all(folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
        }
        change.make()?;
    }
    record(history, files)
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
        write(vec![FileChange { file, before, after: Some(text) }], &history)?;
    }
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

    /// A new resource has a name and a type; it starts from a file, which is
    /// copied in under the name, or, for Markdown, as a heading to write under.
    #[test]
    fn a_new_resource_is_named_typed_and_maybe_copied_in() {
        let dir = folder("new");
        let made = new_resource(&dir, "how-to", ResourceKind::Markdown, None).unwrap();
        assert_eq!(made, dir.join("resources/how-to.md"));
        assert_eq!(std::fs::read_to_string(&made).unwrap(), "# How to\n");

        let picked = dir.join("Scan 0001.PDF");
        std::fs::write(&picked, b"%PDF").unwrap();
        let pdf = new_resource(&dir, "manual", ResourceKind::Pdf, Some(&picked)).unwrap();
        assert_eq!(pdf, dir.join("resources/manual.pdf"));
        assert_eq!(std::fs::read(&pdf).unwrap(), b"%PDF");
        // A picture keeps the extension of its file; an other file too.
        let jpg = dir.join("photo.jpeg");
        std::fs::write(&jpg, [1]).unwrap();
        assert_eq!(new_resource(&dir, "banner", ResourceKind::Picture, Some(&jpg)).unwrap(), dir.join("resources/banner.jpeg"));
        let csv = dir.join("export.csv");
        std::fs::write(&csv, "a,b").unwrap();
        assert_eq!(new_resource(&dir, "prices", ResourceKind::File, Some(&csv)).unwrap(), dir.join("resources/prices.csv"));

        // A file of another type, a type with nothing to start from, a name taken: refused.
        assert!(new_resource(&dir, "plan", ResourceKind::Drawing, Some(&picked)).unwrap_err().contains("not a Drawing"));
        assert!(new_resource(&dir, "plan", ResourceKind::Drawing, None).unwrap_err().contains("pick the file"));
        assert!(new_resource(&dir, "manual", ResourceKind::Pdf, Some(&picked)).unwrap_err().contains("already"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A resource is replaced by a file of its own type; its name stays.
    #[test]
    fn a_resource_is_replaced_by_a_file_of_its_type() {
        let dir = folder("replace");
        std::fs::create_dir_all(dir.join("resources")).unwrap();
        std::fs::write(dir.join("resources/plan.dxf"), "old").unwrap();
        let newer = dir.join("Plan v2.DXF");
        std::fs::write(&newer, "new").unwrap();
        replace_resource(&dir, "plan.dxf", &newer).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("resources/plan.dxf")).unwrap(), "new");
        let svg = dir.join("plan.svg");
        std::fs::write(&svg, "<svg/>").unwrap();
        // An SVG is a drawing too, but the resource is named .dxf: its file keeps its type.
        assert!(replace_resource(&dir, "plan.dxf", &svg).unwrap_err().contains("plan.dxf"));
        assert!(replace_resource(&dir, "missing.dxf", &newer).unwrap_err().contains("no missing.dxf"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
