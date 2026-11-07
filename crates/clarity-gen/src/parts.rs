//! Custom parts in the output (DESIGN 5.19 and 6.5), the same for every
//! stack: what code reads of a part, and the files of Clarity's and the
//! starter that declaring a part writes into its folder. Each stack writes
//! its own part files (c/parts.rs, cpp/parts.rs, go/parts.rs, rust/parts.rs,
//! csharp/parts.rs).

use clarity_spec::code::{PartDecl, PartLibrary, TypeRefKind};
use clarity_spec::{Project, native};

/// The folder of the output the parts are built in.
pub const FOLDER: &str = "parts";

/// The project's native parts, in the order they are declared.
pub fn native_parts(project: &Project) -> Vec<&PartDecl> {
    project.files.iter().flat_map(|f| &f.parts).filter(|p| matches!(p.library, Some(PartLibrary::Native { .. }))).collect()
}

/// The kind under which a sound part keeps what code reads of it
/// (docs/AUDIO-PLAN.md): lower case, so no declared part has it.
pub(crate) const SOUND_KIND: &str = "sound";

/// The kind under which a video part keeps what code reads of it (docs/VIDEO-PLAN.md).
pub(crate) const VIDEO_KIND: &str = "video";

/// What a part keeps in its values for code to read, as the generators
/// carry it: a custom part's kind and the values on its line, or a sound's.
pub(crate) fn kept_of(props: &clarity_spec::model::WidgetProps) -> Option<(String, std::collections::BTreeMap<String, clarity_spec::model::Literal>)> {
    match props.kind {
        clarity_spec::widgets::WidgetKind::Sound => Some((SOUND_KIND.to_string(), Default::default())),
        clarity_spec::widgets::WidgetKind::Video => Some((VIDEO_KIND.to_string(), Default::default())),
        _ => props.part.clone().map(|kind| (kind, props.values.clone())),
    }
}

/// Whether a part of `kind` keeps `property` in its values: a custom part's own, or what a sound says.
pub(crate) fn keeps(project: &Project, kind: &str, property: &str) -> bool {
    if kind == SOUND_KIND {
        return clarity_spec::sound::read(property).is_some();
    }
    if kind == VIDEO_KIND {
        return clarity_spec::video::read(property).is_some();
    }
    declaration(project, kind).is_some_and(|d| d.properties.iter().any(|p| p.name.name == property))
}

/// The declaration of the custom part of kind `kind`.
pub(crate) fn declaration<'a>(project: &'a Project, kind: &str) -> Option<&'a PartDecl> {
    project.files.iter().flat_map(|f| &f.parts).find(|p| p.name.name == kind)
}

/// A custom part's property as code reads it when no handler set it: the
/// value written on the part's line, else the declared default, else the
/// empty value of its type; as JSON, which each runtime reads as the type.
pub(crate) fn fallback_json(project: &Project, kind: &str, written: &std::collections::BTreeMap<String, clarity_spec::model::Literal>, property: &str) -> String {
    use clarity_spec::code::ExprKind;
    use clarity_spec::model::Literal;
    if kind == SOUND_KIND {
        return clarity_spec::sound::read_fallback(property).to_string();
    }
    if kind == VIDEO_KIND {
        return clarity_spec::video::read_fallback(property).to_string();
    }
    let Some(declared) = declaration(project, kind).and_then(|d| d.properties.iter().find(|p| p.name.name == property)) else {
        return "null".into();
    };
    let whole = matches!(declared.ty.kind, TypeRefKind::Integer | TypeRefKind::Duration);
    let number = |n: &str| if whole { n.to_string() } else { serde_json::Value::String(n.to_string()).to_string() };
    let text = |t: &str| serde_json::Value::String(t.to_string()).to_string();
    if let Some(value) = written.get(property) {
        return match value {
            Literal::Number(n) => number(n),
            Literal::Str(s) | Literal::Ident(s) => text(s),
            Literal::Bool(b) => b.to_string(),
        };
    }
    if let Some(default) = &declared.default {
        match &default.kind {
            ExprKind::Number { value } => return number(value),
            ExprKind::Unary { operand, .. } if matches!(&operand.kind, ExprKind::Number { .. }) => {
                if let ExprKind::Number { value } = &operand.kind {
                    return number(&format!("-{value}"));
                }
            }
            ExprKind::Str { value } => return text(value),
            ExprKind::Bool { value } => return value.to_string(),
            ExprKind::Name { name } => return text(name),
            _ => {}
        }
    }
    match declared.ty.kind {
        TypeRefKind::Integer | TypeRefKind::Duration => "0".into(),
        TypeRefKind::Decimal | TypeRefKind::Money => text("0"),
        TypeRefKind::Bool => "false".into(),
        TypeRefKind::Date | TypeRefKind::DateTime => "null".into(),
        _ => text(""),
    }
}

/// The handlers of `page` wired to a custom part's tells: they run with the
/// tell's values, as the handler an answer goes to does.
pub(crate) fn told_handlers(page: Option<&clarity_spec::model::Page>) -> Vec<String> {
    fn walk(widgets: &[clarity_spec::model::Widget], found: &mut Vec<String>) {
        for widget in widgets {
            for tell in &widget.props.tells {
                if !found.contains(&tell.handler) {
                    found.push(tell.handler.clone());
                }
            }
            walk(&widget.children, found);
            for page in &widget.pages {
                walk(&page.children, found);
            }
        }
    }
    let mut found = Vec::new();
    if let Some(page) = page {
        walk(&page.widgets, &mut found);
        for sidebar in &page.frame.sidebars {
            walk(&sidebar.widgets, &mut found);
        }
    }
    found
}

/// Writes each native part's header from its declaration into the part's
/// folder in the project in `dir` (when it changed), and the starter into a
/// folder that has no build file yet. Gives the files written.
pub fn write_part_folders(dir: &std::path::Path, project: &Project) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut written = Vec::new();
    // A .NET part's adapter, a control of Clarity.UI (DESIGN 7.9), written once.
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
    // A C project's native part is a kind of part of Clarity.UI, in C (DESIGN 7.9).
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
        // A Rust part's clarity-ui line follows the library the program is built on:
        // the one line of its Cargo.toml that names Clarity's own crate.
        if matches!(part.library, Some(PartLibrary::Crate { .. })) && !fresh {
            let cargo = root.join(manifest);
            if let Some(text) = std::fs::read_to_string(&cargo).ok().and_then(|t| crate::rust::parts::with_ui_version(&t)) {
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
    // A library's adapter, written once, and the file of Clarity's always (DESIGN 5.21).
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

/// What declaring a native part writes into its folder in a C project: a
/// kind of part of Clarity.UI, the functions of `clarity/<Name>.h` (DESIGN 7.9).
pub fn c_starter(part: &PartDecl) -> Vec<(String, String)> {
    crate::c::parts::starter(part)
}

/// What declaring a native part writes into its folder in a C++ project: a
/// control of Clarity.UI, the class of `clarity/<Name>.h` (DESIGN 7.9).
pub fn cpp_starter(part: &PartDecl) -> Vec<(String, String)> {
    crate::cpp::parts::starter(part)
}

/// The starter adapter of .NET part `name`, as declaring it writes it: a
/// control of Clarity.UI (DESIGN 7.9).
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
