//! Transpiles a checked Clarity project into a real application for the
//! stack its project file names (docs/DESIGN.md section 1.2). Generation is
//! deterministic: the same project always gives the same files, byte for
//! byte, so `out/` can be compared, cached and regenerated at any time.

pub mod tools;
mod csharp;
mod ddl;
mod go;
mod print;
mod readme;
mod rust;
mod schema;
mod web;

pub use csharp::avalonia_theme;
pub use rust::migration_sql;
pub use web::{APP_CLASS, base_css, theme_css, theme_file};

use clarity_project::{Backend, Settings};
use clarity_spec::Project;

use ddl::Sql;

/// The databases of an application and the ones Clarity owns.
type Databases = (Vec<(String, Sql, String)>, Vec<(String, Sql)>);

/// The database of every data source that holds entities, with its kind and
/// where it is, and the ones Clarity owns (and so migrates); external ones
/// are opened as they are. `backend` names the backend in messages.
fn databases(project: &Project, settings: &Settings, backend: &str) -> Result<Databases, Vec<String>> {
    if settings.datasources.is_empty() {
        return Err(vec!["the project needs a data source to store its records".to_string()]);
    }
    let mut databases: Vec<(String, Sql, String)> = Vec::new();
    for source in &settings.datasources {
        if project.entities_in(&source.name).next().is_none() {
            continue;
        }
        let Some(sql) = Sql::of(source.kind) else {
            return Err(vec![format!(
                "data source '{}' is a web API; the {backend} backend stores records in SQL databases only for now",
                source.name
            )]);
        };
        if source.url.starts_with("keychain:") {
            return Err(vec![format!(
                "data source '{}': a generated application reads its connection from 'env:NAME' for now, not from the keychain",
                source.name
            )]);
        }
        databases.push((source.name.clone(), sql, source.url.clone()));
    }
    let owned = databases
        .iter()
        .filter(|(name, _, _)| settings.datasources.iter().any(|s| s.name == *name && !s.external))
        .map(|(name, sql, _)| (name.clone(), *sql))
        .collect();
    Ok((databases, owned))
}

/// One generated file, by its path inside `out/`. Most are text; a few, like
/// icons, are bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutFile {
    pub path: String,
    pub text: String,
    /// The contents of a binary file; `text` is empty then.
    pub bytes: Option<&'static [u8]>,
}

/// The files of the application, sorted by path; or why it cannot be
/// generated.
pub fn generate(project: &Project, settings: &Settings) -> Result<Vec<OutFile>, Vec<String>> {
    if project.has_errors() {
        return Err(vec!["the project has errors; fix the errors before generating it".to_string()]);
    }
    let mut files = match settings.stack.backend {
        Backend::Rust => rust::generate(project, settings)?,
        Backend::Csharp if settings.stack.ui == clarity_project::Ui::Avalonia => csharp::generate(project, settings)?,
        Backend::Csharp => {
            return Err(vec!["C# with the web renderer is not generated yet; C# uses Avalonia".to_string()]);
        }
        Backend::Go => go::generate(project, settings)?,
        other => {
            let name = match other {
                Backend::Go => "Go",
                Backend::Csharp => "C#",
                Backend::C => "C",
                Backend::Cpp => "C++",
                Backend::Rust => "Rust",
            };
            return Err(vec![format!("the {name} backend is not generated yet; Rust, Go and C# are")]);
        }
    };
    files.push(OutFile {
        path: "README.md".to_string(),
        text: readme::readme(settings),
        bytes: None,
    });
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Writes `files` into `dir`, which then holds exactly these files: files
/// that changed are rewritten, unchanged ones are left alone (so build
/// tools see no change), and files no longer generated are removed.
/// `keep` names top-level entries that are not generated, such as the
/// build tool's `target` folder.
pub fn write_out(dir: &std::path::Path, files: &[OutFile], keep: &[&str]) -> std::io::Result<usize> {
    // Removing stale files is only safe in a folder Clarity made: refuse a
    // folder that holds anything without the marker.
    // Output from before the marker is recognized by the runtime Clarity copies in.
    let marked = dir.join(OUT_MARKER).exists() || dir.join("clarity-runtime/src/ui.rs").exists();
    let empty = match std::fs::read_dir(dir) {
        Ok(mut entries) => entries.next().is_none(),
        Err(_) => true,
    };
    if !marked && !empty {
        return Err(std::io::Error::other(format!(
            "{} holds files not made by Clarity; choose an empty or new output folder",
            dir.display()
        )));
    }
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(OUT_MARKER), OUT_MARKER_TEXT)?;
    let mut written = 0;
    for file in files {
        let path = dir.join(&file.path);
        let contents = file.bytes.unwrap_or(file.text.as_bytes());
        if std::fs::read(&path).ok().as_deref() == Some(contents) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, contents)?;
        written += 1;
    }
    remove_stale(dir, dir, files, keep)?;
    Ok(written)
}

fn remove_stale(root: &std::path::Path, dir: &std::path::Path, files: &[OutFile], keep: &[&str]) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        if dir == root && keep.contains(&relative.as_str()) {
            continue;
        }
        if path.is_dir() {
            remove_stale(root, &path, files, keep)?;
            if std::fs::read_dir(&path)?.next().is_none() {
                std::fs::remove_dir(&path)?;
            }
        } else if relative != OUT_MARKER && !files.iter().any(|f| f.path == relative) {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// How the build tools of a stack build its generated project in `out/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolchain {
    /// The command, such as `cargo`.
    pub tool: &'static str,
    pub args: Vec<String>,
    /// Environment the tool needs for plain line output.
    pub env: Vec<(&'static str, &'static str)>,
    /// The program the build makes, relative to `out/`.
    pub program: std::path::PathBuf,
    /// What to say when the tool cannot be started.
    pub missing: &'static str,
}

/// The build of the project `settings` describes, as a debug or (with
/// `release`) a release program; `None` for a stack Clarity cannot generate yet.
pub fn toolchain(settings: &Settings, release: bool) -> Option<Toolchain> {
    let exe = |path: String| std::path::PathBuf::from(if cfg!(windows) { format!("{path}.exe") } else { path });
    match settings.stack.backend {
        Backend::Rust => {
            let mut args = vec!["build".to_string(), "--color".to_string(), "never".to_string()];
            if release {
                args.push("--release".to_string());
            }
            let profile = if release { "release" } else { "debug" };
            Some(Toolchain {
                tool: "cargo",
                args,
                // No progress bars, which only make sense in a terminal.
                env: vec![("CARGO_TERM_PROGRESS_WHEN", "never")],
                program: exe(format!("target/{profile}/{}", rust::crate_name(settings))),
                missing: "is Rust installed?",
            })
        }
        Backend::Csharp if settings.stack.ui == clarity_project::Ui::Avalonia => {
            let configuration = if release { "Release" } else { "Debug" };
            Some(Toolchain {
                tool: "dotnet",
                // `-tl:off`: plain lines, not the terminal logger.
                args: ["build", "--nologo", "-tl:off", "-c", configuration].map(String::from).to_vec(),
                env: vec![("DOTNET_CLI_TELEMETRY_OPTOUT", "1"), ("DOTNET_NOLOGO", "1")],
                program: exe(format!("bin/{configuration}/{}/{}", csharp::TARGET_FRAMEWORK, csharp::project_name(settings))),
                missing: "is the .NET SDK installed?",
            })
        }
        _ => None,
    }
}

/// What the build tools of `backend` put in `out/`, which `write_out`
/// leaves alone, and the application's `data` (its database while it runs
/// from `out/`). Everything else in `out/` is the generated project, so
/// the build output of another stack goes when a project changes stack.
pub fn build_output(backend: Backend) -> &'static [&'static str] {
    match backend {
        // cargo's `target` and lock file, and Tauri's `gen`.
        Backend::Rust => &["target", "data", "gen", "Cargo.lock"],
        // dotnet's `bin` and `obj`.
        Backend::Csharp => &["bin", "obj", "data"],
        Backend::Go | Backend::C | Backend::Cpp => &["data"],
    }
}

/// The file that marks a folder as Clarity's output. `write_out` only
/// removes files in folders that have it.
pub const OUT_MARKER: &str = ".clarity-out";

const OUT_MARKER_TEXT: &str =
    "This folder is generated by Clarity from a project's .clarity files. Clarity replaces its contents on every build.\n";
