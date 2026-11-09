//! The project's custom parts and libraries that name their library: their
//! folders, languages and files, and which file in each is Clarity's.

use std::path::Path;

use clarity_spec::code::PartLibrary;
use serde::Serialize;

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
pub const BUILD_FOLDERS: [&str; 5] = ["build", "target", "bin", "obj", "node_modules"];

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
pub fn own_of(dir: &Path, project: &clarity_spec::Project, name: &str) -> Option<String> {
    let part = project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == name);
    match part.and_then(|p| p.library.as_ref()) {
        Some(library) => own_file(library, name),
        None => library_own(dir, project, name),
    }
}

/// A custom part's or a library's folder in the project: what it is, its
/// language, the folder (relative to the project's folder) and the file in
/// it that is Clarity's.
#[derive(Debug, Clone, PartialEq)]
pub struct PartFolder {
    /// `part` or `library`.
    pub kind: &'static str,
    pub name: String,
    pub language: &'static str,
    pub folder: String,
    pub own: Option<String>,
}

/// The project's custom parts and libraries that name their library, with their folders.
pub fn part_folders(dir: &Path, project: &clarity_spec::Project) -> Vec<PartFolder> {
    let mut out = Vec::new();
    for part in project.files.iter().flat_map(|f| &f.parts) {
        let name = &part.name.name;
        let (language, folder) = match &part.library {
            Some(PartLibrary::Native { folder }) => ("C or C++", folder.clone()),
            Some(PartLibrary::Crate { folder }) => ("Rust", folder.clone()),
            Some(PartLibrary::Go { folder }) => ("Go", folder.clone()),
            Some(_) => ("C#", clarity_spec::native::adapter_folder(name)),
            None => continue,
        };
        out.push(PartFolder { kind: "part", name: name.clone(), language, folder, own: own_of(dir, project, name) });
    }
    for library in clarity_gen::libraries::libraries(project) {
        use clarity_spec::code::LibrarySource;
        let language = match &library.source {
            Some(LibrarySource::Nuget { .. } | LibrarySource::Dll { .. }) => "C#",
            Some(LibrarySource::Crate { .. }) => "Rust",
            Some(LibrarySource::Go { .. }) => "Go",
            Some(LibrarySource::Cmake { .. }) => "C or C++",
            None => continue,
        };
        let name = &library.name.name;
        out.push(PartFolder { kind: "library", name: name.clone(), language, folder: clarity_spec::native::library_folder(name), own: own_of(dir, project, name) });
    }
    out
}

/// Whether the part or library `name` builds on its own (see `build_steps`).
pub fn builds_alone(project: &clarity_spec::Project, name: &str) -> bool {
    let part = project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == name);
    matches!(part.and_then(|p| p.library.as_ref()), Some(PartLibrary::Native { .. } | PartLibrary::Crate { .. } | PartLibrary::Go { .. }))
        || matches!(
            clarity_gen::libraries::library(project, name).and_then(|l| l.source.as_ref()),
            Some(clarity_spec::code::LibrarySource::Crate { .. } | clarity_spec::code::LibrarySource::Go { .. })
        )
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
}
