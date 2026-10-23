//! The folders of custom parts (DESIGN 5.19). `native "parts/Arena"` names
//! a folder of the project with the library's sources and its
//! `CMakeLists.txt`, which the generated program builds; its `clarity`
//! folder is Clarity's: the header written from the declaration. A part
//! from a .NET library (`nuget`, `dll`) has its C# adapter in
//! `parts/<Name>`, and a `dll` names its assembly in the project.

use std::path::Path;

use crate::code::PartLibrary;
use crate::diagnostic::Diagnostic;
use crate::model::Project;

/// The folder inside a native part's folder that is Clarity's, written from the declaration.
pub const OWN_FOLDER: &str = "clarity";

/// What a native part's folder must hold for Clarity to build it.
pub const BUILD_FILE: &str = "CMakeLists.txt";

/// A file of a native part's folder: the part's name, its path in the
/// folder (with `/`) and what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartFile {
    pub part: String,
    pub path: String,
    pub bytes: Vec<u8>,
}

/// Folders a part's build leaves, which are not its sources.
const LEFT_OUT: [&str; 4] = ["build", "out", "target", OWN_FOLDER];

/// Where a .NET part's adapter is: `parts/<Name>`.
pub fn adapter_folder(part: &str) -> String {
    format!("parts/{part}")
}

/// The files of every custom part's folder in the project in `dir`; a
/// folder that is not there gives none. A `dll` part's assembly is among
/// its files under its file name.
pub fn native_files(dir: &Path, project: &Project) -> std::io::Result<Vec<PartFile>> {
    let mut found = Vec::new();
    for part in project.files.iter().flat_map(|f| &f.parts) {
        let name = &part.name.name;
        let folder = match &part.library {
            Some(PartLibrary::Native { folder } | PartLibrary::Crate { folder } | PartLibrary::Go { folder }) => folder.clone(),
            Some(PartLibrary::Nuget { .. } | PartLibrary::Dll { .. }) => adapter_folder(name),
            None => continue,
        };
        let root = dir.join(&folder);
        if root.is_dir() {
            read_folder(&root, "", name, &mut found)?;
        }
        if let Some(PartLibrary::Dll { file }) = &part.library {
            let path = dir.join(file);
            let at = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if path.is_file() && !found.iter().any(|f| f.part == *name && f.path == at) {
                found.push(PartFile { part: name.clone(), path: at, bytes: std::fs::read(&path)? });
            }
        }
    }
    found.sort_by(|a, b| (&a.part, &a.path).cmp(&(&b.part, &b.path)));
    Ok(found)
}

fn read_folder(folder: &Path, inside: &str, part: &str, found: &mut Vec<PartFile>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(folder)? {
        let path = entry?.path();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if name.starts_with('.') {
            continue;
        }
        let at = if inside.is_empty() { name.clone() } else { format!("{inside}/{name}") };
        if path.is_dir() {
            if inside.is_empty() && LEFT_OUT.contains(&name.as_str()) {
                continue;
            }
            read_folder(&path, &at, part, found)?;
        } else if path.is_file() {
            found.push(PartFile { part: part.to_string(), path: at, bytes: std::fs::read(&path)? });
        }
    }
    Ok(())
}

/// Gives the project the files of its native parts, and checks each native
/// part has its folder with its build file.
pub fn attach(project: &mut Project, files: Vec<PartFile>) {
    let mut found = Vec::new();
    for file in &project.files {
        for part in &file.parts {
            let name = &part.name.name;
            let own: Vec<&PartFile> = files.iter().filter(|f| f.part == *name).collect();
            let adapter = format!("{name}.cs");
            let problem = match &part.library {
                Some(PartLibrary::Native { folder } | PartLibrary::Crate { folder } | PartLibrary::Go { folder }) if own.is_empty() => {
                    Some(format!("part {name}: its folder '{folder}' is not there or is empty"))
                }
                Some(PartLibrary::Native { folder }) if !own.iter().any(|f| f.path == BUILD_FILE) => {
                    Some(format!("part {name}: '{folder}' has no {BUILD_FILE}, which builds its library"))
                }
                Some(PartLibrary::Crate { folder }) if !own.iter().any(|f| f.path == "Cargo.toml") => Some(format!("part {name}: '{folder}' has no Cargo.toml, which makes it a crate")),
                Some(PartLibrary::Go { folder }) if !own.iter().any(|f| f.path == "go.mod") => Some(format!("part {name}: '{folder}' has no go.mod, which makes it a module")),
                Some(PartLibrary::Nuget { .. } | PartLibrary::Dll { .. }) if !own.iter().any(|f| f.path == adapter) => {
                    Some(format!("part {name}: '{}/{adapter}' is not there; it shows the library's control", adapter_folder(name)))
                }
                Some(PartLibrary::Dll { file: dll }) if !own.iter().any(|f| Some(f.path.as_str()) == Path::new(dll).file_name().and_then(|n| n.to_str())) => {
                    Some(format!("part {name}: its assembly '{dll}' is not there"))
                }
                _ => None,
            };
            if let Some(problem) = problem {
                found.push(Diagnostic::error(problem, part.name.span).in_file(&file.path));
            }
        }
    }
    project.diagnostics.extend(found);
    project.part_files = Some(files);
}
