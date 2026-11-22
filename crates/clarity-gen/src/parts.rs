//! Custom parts in the output (manual/design.custom-parts.md), the same for every
//! stack: what code reads of a part, and the files of Clarity's and the
//! starter that declaring a part writes into its folder. Each stack writes
//! its own part files (c/parts.rs, cpp/parts.rs, go/parts.rs, rust/parts.rs,
//! csharp/parts.rs).

pub use clarity_gen_core::parts::*;

use clarity_spec::code::{PartDecl, PartLibrary};
use clarity_spec::{Project, native};

/// Writes each native part's header from its declaration into the part's
/// folder in the project in `dir` (when it changed), and the starter into a
/// folder that has no build file yet. Gives the files written.
pub fn write_part_folders(dir: &std::path::Path, project: &Project) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut written = Vec::new();
    // A .NET part's adapter, a control of Clarity.UI (manual/start.how-it-works.md), written once.
    for part in crate::csharp::parts::dotnet_parts(project) {
        let n = &part.name.name;
        let adapter = dir.join(native::adapter_folder(n)).join(format!("{n}.cs"));
        if !adapter.exists() {
            if let Some(parent) = adapter.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&adapter, crate::csharp::parts::adapter_starter(project, part))?;
            written.push(adapter);
        }
    }
    // A native, Rust or Go part: its starter where the folder has none, and
    // the file of Clarity's (the header, `src/clarity.rs`, `clarity.go`) always.
    // A C project's native part is a kind of part of Clarity.UI, in C (manual/start.how-it-works.md).
    let c_project = project_is_c(dir);
    for part in project.files.iter().flat_map(|f| &f.parts) {
        let (folder, files, manifest, own): (&String, Vec<(String, String)>, &str, &str) = match &part.library {
            Some(PartLibrary::Native { folder }) if c_project => (folder, crate::c::parts::starter(part), native::BUILD_FILE, "clarity/"),
            // Native parts are C or C++; a project of no stack yet gets C++'s.
            Some(PartLibrary::Native { folder }) => (folder, crate::cpp::parts::starter(part), native::BUILD_FILE, "clarity/"),
            Some(PartLibrary::Crate { folder }) => (folder, crate::rust::parts::ui_starter(part), "Cargo.toml", "src/clarity.rs"),
            Some(PartLibrary::Go { folder }) => (folder, crate::go::parts::starter(part), "go.mod", "clarity.go"),
            _ => continue,
        };
        let root = dir.join(folder);
        let fresh = !root.join(manifest).exists();
        // A Rust part's clarity-runtime line, where it has one, follows the runtime the program is built on.
        if matches!(part.library, Some(PartLibrary::Crate { .. })) && !fresh {
            let cargo = root.join(manifest);
            if let Some(text) = std::fs::read_to_string(&cargo).ok().and_then(|t| crate::rust::parts::with_runtime_version(&t)) {
                std::fs::write(&cargo, text)?;
                written.push(cargo);
            }
        }
        for (path, text) in files {
            let own = path.starts_with(own);
            if !own && !fresh {
                continue;
            }
            let target = root.join(&path);
            if std::fs::read_to_string(&target).ok().as_deref() == Some(text.as_str()) {
                continue;
            }
            // A file of the developer's is never written over.
            if !own && target.exists() {
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, text)?;
            written.push(target);
        }
    }
    // A library's adapter, written once, and the file of Clarity's always (manual/code.libraries.md).
    let cpp = project_is_cpp(dir);
    for library in crate::libraries::libraries(project) {
        let root = dir.join(native::library_folder(&library.name.name));
        let Some(source) = &library.source else { continue };
        let fresh = !root.join(native::library_manifest(&library.name.name, source)).exists();
        let own = crate::libraries::own_file(library, cpp);
        for (path, text) in crate::libraries::starter(project, library, cpp, crate::go::runtime_version()) {
            let mine = path == own;
            let target = root.join(&path);
            if (!mine && (!fresh || target.exists())) || std::fs::read_to_string(&target).ok().as_deref() == Some(text.as_str()) {
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, text)?;
            written.push(target);
        }
        // A C# library's generated half lives in the project too, for the editor.
        if matches!(source, clarity_spec::code::LibrarySource::Nuget { .. } | clarity_spec::code::LibrarySource::Dll { .. }) {
            let target = root.join(&own);
            let text = crate::libraries::csharp_half(project, library);
            if std::fs::read_to_string(&target).ok().as_deref() != Some(text.as_str()) {
                std::fs::create_dir_all(&root)?;
                std::fs::write(&target, text)?;
                written.push(target);
            }
        }
    }
    Ok(written)
}

/// The interface file of Rust part `part` on Clarity.UI (`src/clarity.rs`),
/// which the program carries and the part's folder keeps.
pub fn rust_ui_interface(part: &PartDecl) -> String {
    crate::rust::parts::ui_interface(part)
}

/// What declaring a native part writes into its folder in a C project: a
/// kind of part of Clarity.UI, the functions of `clarity/<Name>.h` (manual/start.how-it-works.md).
pub fn c_starter(part: &PartDecl) -> Vec<(String, String)> {
    crate::c::parts::starter(part)
}

/// What declaring a native part writes into its folder in a C++ project: a
/// control of Clarity.UI, the class of `clarity/<Name>.h` (manual/start.how-it-works.md).
pub fn cpp_starter(part: &PartDecl) -> Vec<(String, String)> {
    crate::cpp::parts::starter(part)
}

/// The starter adapter of .NET part `name`, as declaring it writes it: a
/// control of Clarity.UI (manual/start.how-it-works.md).
pub fn dotnet_adapter(project: &Project, name: &str) -> Option<String> {
    let part = crate::csharp::parts::dotnet_parts(project).into_iter().find(|p| p.name.name == name)?;
    Some(crate::csharp::parts::adapter_starter(project, part))
}

/// The generated half of .NET part `part` (`parts/<Name>/<Name>.g.cs`), as the C# project carries it.
pub fn dotnet_half(project: &Project, part: &PartDecl) -> String {
    crate::csharp::parts::generated_half(project, part)
}

/// What declaring a part writes into its folder, by its library: C++ with
/// CMake (`c_starter` gives C's), a Rust crate, or a Go module.
pub fn starter_for(part: &PartDecl) -> Vec<(String, String)> {
    match &part.library {
        Some(PartLibrary::Native { .. }) => crate::cpp::parts::starter(part),
        Some(PartLibrary::Crate { .. }) => crate::rust::parts::ui_starter(part),
        Some(PartLibrary::Go { .. }) => crate::go::parts::starter(part),
        _ => Vec::new(),
    }
}

/// Whether the project in `dir` is on the C++ stack, which a CMake library's
/// adapter is then written for (else C).
fn project_is_cpp(dir: &std::path::Path) -> bool {
    let file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
    clarity_project::load(&file).settings.is_some_and(|s| s.stack.backend == clarity_project::Backend::Cpp)
}

/// Whether the project in `dir` is on the C stack.
fn project_is_c(dir: &std::path::Path) -> bool {
    let file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
    clarity_project::load(&file).settings.is_some_and(|s| s.stack.backend == clarity_project::Backend::C)
}
