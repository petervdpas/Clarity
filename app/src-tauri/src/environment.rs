//! The Environment settings (Clarity settings, for every project): the build
//! tools of the five stacks on this computer, every copy a scan finds with
//! its version, the one each build uses, and the person's choice. Kept in
//! Clarity's own folder; see clarity_gen::tools::environment.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clarity_gen::tools::environment::{self, Environment, TOOLS};
use clarity_gen::tools::{self, System};
use serde::Serialize;
use ts_rs::TS;

/// A copy of a tool on this computer.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ToolCopy {
    pub path: String,
    /// As it names itself, when it answers: `go version go1.24.2 linux/amd64`.
    pub version: Option<String>,
}

/// A build tool as the Environment settings show it.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ToolView {
    pub program: String,
    pub name: String,
    /// The stacks that build with it, as people read them: `C`, `C++`.
    pub stacks: Vec<String>,
    /// The copy the person chose, if any.
    pub chosen: Option<String>,
    /// The copy builds use: the chosen one, else the first found.
    pub used: Option<String>,
    /// Every copy a scan found, the chosen one too.
    pub copies: Vec<ToolCopy>,
}

/// A stack: whether it can build here, with what, and what is missing.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct StackView {
    pub name: String,
    pub ready: bool,
    pub found: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct EnvironmentView {
    /// Where the choices are kept.
    pub file: String,
    pub stacks: Vec<StackView>,
    pub tools: Vec<ToolView>,
}

/// What a copy says it is: the first line it prints for its version.
fn version_of(path: &Path, program: &str) -> Option<String> {
    let args: &[&str] = if program == "go" { &["version"] } else { &["--version"] };
    let out = Command::new(path).args(args).stdin(Stdio::null()).output().ok()?;
    // Some tools (Visual Studio's cl) say who they are on stderr.
    let text = if out.stdout.is_empty() { out.stderr } else { out.stdout };
    String::from_utf8_lossy(&text).lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string)
}

fn language(backend: clarity_project::Backend) -> String {
    clarity_project::catalog().backends.iter().find(|b| b.id == backend).map_or("?", |b| b.label).to_string()
}

/// Scans the computer and describes its environment.
pub fn view() -> EnvironmentView {
    let env = Environment::current();
    let searched = tools::installers_path();
    let tools = TOOLS
        .iter()
        .filter(|t| t.program != "pkg-config" || cfg!(target_os = "linux"))
        .map(|tool| {
            let mut copies: Vec<PathBuf> = environment::candidates(tool.program, &searched);
            if let Some(chosen) = env.chosen(tool.program).filter(|c| !copies.iter().any(|p| p == c)) {
                copies.insert(0, chosen.to_path_buf());
            }
            ToolView {
                program: tool.program.to_string(),
                name: tool.name.to_string(),
                stacks: tool.stacks.iter().map(|b| language(*b)).collect(),
                chosen: env.chosen(tool.program).map(|p| p.display().to_string()),
                used: environment::find(tool.program, &env, &tools::tool_path()).map(|p| p.display().to_string()),
                copies: copies.iter().map(|p| ToolCopy { path: p.display().to_string(), version: version_of(p, tool.program) }).collect(),
            }
        })
        .collect();
    let stacks = tools::check_all(&System)
        .into_iter()
        .filter(|s| s.generated)
        .map(|s| StackView { name: tools::stack_name(s.backend, s.ui), ready: s.missing.is_empty(), found: s.found, missing: s.missing })
        .collect();
    EnvironmentView { file: environment::file().display().to_string(), stacks, tools }
}

/// The environment, scanned anew.
#[tauri::command(async)]
pub fn environment_view() -> EnvironmentView {
    view()
}

/// Chooses the copy at `path` for `program`, or forgets the choice without
/// a path; gives the environment as it is then.
#[tauri::command(async)]
pub fn choose_tool(program: String, path: Option<String>) -> Result<EnvironmentView, String> {
    if !TOOLS.iter().any(|t| t.program == program) {
        return Err(format!("{program} is not a build tool Clarity uses"));
    }
    let path = path.map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
    if let Some(path) = &path {
        let file = Path::new(path);
        if !file.is_file() {
            return Err(format!("{path} is not a file"));
        }
        if version_of(file, &program).is_none() {
            return Err(format!("{path} does not answer as {program} does; pick the program itself"));
        }
    }
    let mut env = Environment::current();
    env.choose(&program, path.map(PathBuf::from));
    env.save(&environment::file()).map_err(|err| format!("cannot write {}: {err}", environment::file().display()))?;
    Ok(view())
}
