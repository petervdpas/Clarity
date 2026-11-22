//! Transpiles a checked Clarity project into a real application for the
//! stack its project file names (docs/ledger/project-file.md). Generation is
//! deterministic: the same project always gives the same files, byte for
//! byte, so `out/` can be compared, cached and regenerated at any time.
//! What the stacks share is clarity-gen-core, which this crate gives on.

pub use clarity_gen_core::*;

use clarity_project::{Backend, Settings};
use clarity_spec::Project;

pub mod carried;
use clarity_gen_c as c;
pub mod parts;
use clarity_gen_csharp as csharp;
use clarity_gen_go as go;
use clarity_gen_cpp as cpp;
use clarity_gen_rust as rust;
pub mod preview;

pub use csharp::runtime_version as csharp_runtime_version;
pub use go::runtime_version as go_runtime_version;
pub use rust::runtime_version as rust_runtime_version;
pub use rust::local_patch as rust_local_patch;
pub use rust::pdf_version as rust_pdf_version;
pub use rust::migration_sql;

pub fn generate(project: &Project, settings: &Settings) -> Result<Vec<OutFile>, Vec<String>> {
    let project = &prepared(project, settings)?;
    let files = match settings.stack.backend {
        Backend::Rust => rust::generate(project, settings)?,
        Backend::Csharp => csharp::generate(project, settings)?,
        Backend::Go => go::generate(project, settings)?,
        Backend::C => c::generate(project, settings)?,
        Backend::Cpp => cpp::generate(project, settings)?,
    };
    let readme = match settings.stack.backend {
        Backend::Rust => rust::readme(settings, project),
        Backend::Csharp => csharp::readme(settings, project),
        Backend::Go => go::readme(settings, project),
        Backend::C => c::readme(settings, project),
        Backend::Cpp => cpp::readme(settings, project),
    };
    finished(files, project, settings, Some(readme))
}

/// The files of the command-line program, sorted by path; or why it cannot
/// be generated.
pub fn generate_cli(project: &Project, settings: &Settings) -> Result<Vec<OutFile>, Vec<String>> {
    let project = &prepared_cli(project, settings)?;
    match settings.stack.backend {
        Backend::Rust => rust::generate_cli(project, settings),
        Backend::Go => go::generate_cli(project, settings),
        Backend::Csharp => csharp::generate_cli(project, settings),
        Backend::C => c::generate_cli(project, settings),
        Backend::Cpp => cpp::generate_cli(project, settings),
    }
}

/// The build of the project `settings` describes, as a debug or (with
/// `release`) a release program, as its stack builds it; `None` for a stack
/// Clarity cannot generate yet.
pub fn toolchain(settings: &Settings, release: bool) -> Option<Toolchain> {
    match settings.stack.backend {
        Backend::Rust => rust::toolchain(settings, release),
        Backend::Csharp => csharp::toolchain(settings, release),
        Backend::Go => go::toolchain(settings, release),
        Backend::C => c::toolchain(settings, release),
        Backend::Cpp => cpp::toolchain(settings, release),
    }
}

/// The build of the command-line program (manual/code.commands.md) in `out/cli`, with
/// the stack's own tools; the program is named as it is typed.
pub fn cli_toolchain(settings: &Settings, release: bool) -> Option<Toolchain> {
    match settings.stack.backend {
        Backend::Rust => rust::cli_toolchain(settings, release),
        Backend::Csharp => csharp::cli_toolchain(settings, release),
        Backend::Go => go::cli_toolchain(settings, release),
        Backend::C => c::cli_toolchain(settings, release),
        Backend::Cpp => cpp::cli_toolchain(settings, release),
    }
}

/// The programs of the project: the program with windows when a target
/// shows them (the desktop program, or the web program when it is served
/// to a browser alone), and the command-line program with the `cli` target.
/// C# with both `desktop` and `web` gives both programs, each in its own
/// folder: its desktop program cannot serve a browser.
pub fn programs(project: &Project, settings: &Settings) -> Result<Vec<Program>, Vec<String>> {
    use clarity_project::Target;
    if let Some(why) = pictures_without_library(project, settings) {
        return Err(vec![why]);
    }
    if let Some(why) = pdf_without_library(project, settings) {
        return Err(vec![why]);
    }
    if let Some(why) = report_pdfs_without_desktop(project, settings) {
        return Err(vec![why]);
    }
    let mut programs = Vec::new();
    if settings.stack.backend == Backend::Csharp && settings.stack.has(Target::Desktop) && settings.stack.has(Target::Web) {
        for target in [Target::Desktop, Target::Web] {
            let other = if target == Target::Desktop { Target::Web } else { Target::Desktop };
            let mut alone = settings.clone();
            alone.stack.targets.retain(|t| *t != other);
            programs.push(Program { target, files: generate(project, &alone)? });
        }
    } else if settings.stack.has_ui() {
        programs.push(Program { target: windowed_target(settings), files: generate(project, settings)? });
    }
    if settings.stack.has(Target::Cli) {
        programs.push(Program { target: Target::Cli, files: generate_cli(project, settings)? });
    }
    Ok(programs)
}
