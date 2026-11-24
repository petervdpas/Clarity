//! The project's custom parts and libraries that name their library: their
//! folders, languages and files, and which file in each is Clarity's.

use std::path::{Path, PathBuf};

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

/// The file of a library's folder that is Clarity's (manual/code.libraries.md).
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

/// `path` in the folder of the part or library `folder`, relative to that
/// folder, checked the one way every front end checks it (the IDE's part
/// files, the agents' tools): plain names split by `/`, none empty, `.`,
/// `..` or hidden, no backslash, and nothing in the build's output.
pub fn part_path(folder: &PartFolder, path: &str) -> Result<PathBuf, String> {
    let pieces: Vec<&str> = path.split('/').collect();
    if path.is_empty() || pieces.iter().any(|p| p.is_empty() || *p == "." || *p == ".." || p.starts_with('.')) || path.contains('\\') {
        return Err(format!("'{}/{path}' is not a file of {} {}: write a path like {}/src/lib.rs", folder.folder, folder.kind, folder.name, folder.folder));
    }
    if pieces.len() > 1 && BUILD_FOLDERS.contains(&pieces[0]) {
        return Err(format!("'{}/{path}' is in the build's output of {} {}, which is not written by hand", folder.folder, folder.kind, folder.name));
    }
    Ok(pieces.iter().collect())
}

/// The file `path` of the part's or library's folder `folder`, in the
/// project's folder `dir`, to write: the one rule of the IDE's part files and
/// the agents' tools ([`part_path`]), and never Clarity's own file there.
pub fn part_file(dir: &Path, folder: &PartFolder, path: &str) -> Result<PathBuf, String> {
    if folder.own.as_deref() == Some(path) {
        return Err(format!("{}/{path} is Clarity's: Clarity writes it from the declaration, so change the declaration instead", folder.folder));
    }
    Ok(dir.join(&folder.folder).join(part_path(folder, path)?))
}

/// Whether `path` in a part's folder is Clarity's file there, or a folder that holds it.
pub fn holds_own(folder: &PartFolder, path: &str) -> bool {
    folder.own.as_deref().is_some_and(|own| own == path || own.starts_with(&format!("{path}/")))
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

    /// One rule for a path in a part's folder, the strict one, for the IDE and the agents alike.
    #[test]
    fn a_path_in_a_part_s_folder_is_plain_and_outside_the_build() {
        let arena = PartFolder { kind: "part", name: "Arena".into(), language: "Rust", folder: "parts/Arena".into(), own: Some("src/clarity.rs".into()) };
        assert_eq!(part_path(&arena, "src/lib.rs").unwrap(), Path::new("src").join("lib.rs"));
        assert_eq!(part_path(&arena, "build").unwrap(), Path::new("build"), "a file named like a build folder");
        for wrong in ["", "/etc/passwd", "../escape.rs", "src/../../x", "./src/lib.rs", "src//lib.rs", ".hidden", "src/.env", "src\\lib.rs", "src/"] {
            let why = part_path(&arena, wrong).unwrap_err();
            assert!(why.contains("is not a file of part Arena"), "{wrong}: {why}");
        }
        for output in ["target/debug/x", "build/a.o", "bin/x", "obj/x", "node_modules/x"] {
            assert!(part_path(&arena, output).unwrap_err().contains("build's output"), "{output}");
        }
        assert!(holds_own(&arena, "src/clarity.rs") && holds_own(&arena, "src"));
        assert!(!holds_own(&arena, "src/lib.rs") && !holds_own(&arena, "sr"));
    }

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
