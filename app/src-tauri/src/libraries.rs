//! The Libraries settings (Clarity settings, for every project): what of
//! Clarity each build uses, Clarity.UI's library and browser files and each
//! stack's packages, where it comes from now, and the folder the person
//! chose in its place. A factory reset forgets every choice, so builds take
//! what belongs to this Clarity. Kept in environment.json beside the tools;
//! builds and the preview find the choices through
//! clarity_gen::tools::environment::use_libraries.

use std::path::{Path, PathBuf};

use clarity_gen::clarity_ui::{self, Library};
use clarity_gen::tools::environment::{self, Choosable, Environment};
use serde::Serialize;
use ts_rs::TS;

/// Where a build takes a library or package set from now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum UsedFrom {
    /// The folder chosen here.
    Chosen,
    /// The folder an environment variable names (`variable`).
    Environment,
    /// The folder the installed Clarity carries.
    Carried,
    /// Clarity's repository's own build (a Clarity run from its source).
    Repository,
    /// Fetched before into the cache folder.
    Fetched,
    /// Fetched into the cache folder on the first build that needs it.
    ToFetch,
    /// Neither carried nor fetchable on this system.
    Unavailable,
    /// The published packages of this Clarity's version, which the stack's own tool fetches.
    Published,
    /// Copied into each generated program's source.
    InProgram,
}

/// A library or package set as the Libraries settings show it.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LibraryRow {
    /// The choice's key (clarity_gen::tools::environment::Choosable), or
    /// the row's own for one without a choice: `goModule`, `nativeRuntime`.
    pub key: String,
    pub name: String,
    /// What uses it, as people read it.
    pub uses: String,
    /// The version that belongs to this Clarity, where it has its own.
    pub version: Option<String>,
    /// The folder the person chose, if any.
    pub chosen: Option<String>,
    pub from: UsedFrom,
    /// The folder builds use, where there is one.
    pub folder: Option<String>,
    /// The environment variable that names a folder, for `from` environment.
    pub variable: Option<String>,
    /// Whether a folder can be chosen: false where the generator cannot point a program at one.
    pub choosable: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LibrariesView {
    /// Where the choices are kept.
    pub file: String,
    pub rows: Vec<LibraryRow>,
}

fn shown(path: &Path) -> String {
    path.display().to_string()
}

/// Where Clarity.UI's library or browser files come from, as `library` found them.
fn library_row(which: Choosable, name: &str, uses: &str, library: Library, variable: &str, env: &Environment, repo: Option<&Path>) -> LibraryRow {
    let (from, folder) = match library {
        Library::Chosen(folder) => (UsedFrom::Chosen, Some(folder)),
        Library::Named(folder) => (UsedFrom::Environment, Some(folder)),
        Library::Bundled(folder) if repo.is_some_and(|r| folder.starts_with(r)) => (UsedFrom::Repository, Some(folder)),
        Library::Bundled(folder) => (UsedFrom::Carried, Some(folder)),
        Library::Fetched(folder) => (UsedFrom::Fetched, Some(folder)),
        Library::ToFetch { folder, .. } => (UsedFrom::ToFetch, Some(folder)),
        Library::Unavailable => (UsedFrom::Unavailable, None),
    };
    LibraryRow {
        key: which.key().to_string(),
        name: name.to_string(),
        uses: uses.to_string(),
        version: Some(clarity_ui::version().to_string()),
        chosen: env.library(which).map(shown),
        from,
        folder: folder.as_deref().map(shown),
        variable: (from == UsedFrom::Environment).then(|| variable.to_string()),
        choosable: true,
    }
}

/// Where a stack's packages come from: the chosen folder, else the one
/// CLARITY_RUNTIME names (this stack's runtime folder beside it), else the published ones.
fn runtime_row(which: Choosable, name: &str, uses: &str, version: &str, env: &Environment) -> LibraryRow {
    let named = std::env::var_os("CLARITY_RUNTIME").filter(|v| !v.is_empty()).map(PathBuf::from);
    let (from, folder) = match (environment::library_in_use(which), named) {
        (Some(folder), _) => (UsedFrom::Chosen, Some(folder)),
        (None, Some(named)) => (UsedFrom::Environment, Some(which.folder_in(&named, std::env::consts::OS).unwrap_or(named))),
        (None, None) => (UsedFrom::Published, None),
    };
    LibraryRow {
        key: which.key().to_string(),
        name: name.to_string(),
        uses: uses.to_string(),
        version: Some(version.to_string()),
        chosen: env.library(which).map(shown),
        from,
        folder: folder.as_deref().map(shown),
        variable: (from == UsedFrom::Environment).then(|| "CLARITY_RUNTIME".to_string()),
        choosable: true,
    }
}

/// What each build uses of Clarity, and where from.
pub fn view() -> LibrariesView {
    let env = Environment::current();
    let repo = crate::repository();
    let rows = vec![
        library_row(Choosable::ClarityUi, "Clarity.UI library", "Go, C, C++ and Rust desktop programs", clarity_ui::library(), "CLARITY_UI", &env, repo.as_deref()),
        library_row(Choosable::ClarityUiBrowser, "Clarity.UI browser files", "web programs, and Clarity's design and preview", clarity_ui::browser_library(), "CLARITY_UI_BROWSER", &env, repo.as_deref()),
        runtime_row(Choosable::RustCrates, "Rust crates", "Rust programs (clarity-runtime and the crates it takes)", clarity_gen::rust_runtime_version(), &env),
        runtime_row(Choosable::NugetPackages, "NuGet packages", "C# programs (Clarity.Runtime, Clarity.Ui and the rest)", clarity_gen::csharp_runtime_version(), &env),
        LibraryRow {
            key: "goModule".to_string(),
            name: "Go module".to_string(),
            uses: "Go programs (github.com/petervdpas/Clarity/runtimes/go)".to_string(),
            version: Some(clarity_gen::go_runtime_version().to_string()),
            chosen: None,
            from: UsedFrom::Published,
            folder: None,
            variable: None,
            choosable: false,
        },
        LibraryRow {
            key: "nativeRuntime".to_string(),
            name: "C and C++ runtime".to_string(),
            uses: "C and C++ programs".to_string(),
            version: None,
            chosen: None,
            from: UsedFrom::InProgram,
            folder: None,
            variable: None,
            choosable: false,
        },
    ];
    LibrariesView { file: shown(&environment::file()), rows }
}

/// Keeps `env` and builds with it from now on.
fn keep(env: &Environment) -> Result<LibrariesView, String> {
    env.save(&environment::file()).map_err(|err| format!("cannot write {}: {err}", environment::file().display()))?;
    environment::use_libraries(env);
    Ok(view())
}

/// What builds use of Clarity, looked at anew.
#[tauri::command(async)]
pub fn libraries_view() -> LibrariesView {
    view()
}

/// Chooses `folder` for the library or packages `key`, or forgets the
/// choice without a folder; gives the libraries as they are then. A folder
/// that does not hold what it is chosen for is refused, saying what it lacks.
#[tauri::command(async)]
pub fn choose_library(key: String, folder: Option<String>) -> Result<LibrariesView, String> {
    let which = Choosable::from_key(&key).ok_or_else(|| format!("{key} has no folder to choose"))?;
    let folder = match folder.map(|f| f.trim().to_string()).filter(|f| !f.is_empty()) {
        Some(picked) => Some(which.folder_in(Path::new(&picked), std::env::consts::OS)?),
        None => None,
    };
    let mut env = Environment::current();
    env.choose_library(which, folder);
    keep(&env)
}

/// The factory reset: every library choice forgotten, so builds take what
/// belongs to this Clarity. The tools chosen stay.
#[tauri::command(async)]
pub fn reset_libraries() -> Result<LibrariesView, String> {
    let mut env = Environment::current();
    env.reset_libraries();
    keep(&env)
}
