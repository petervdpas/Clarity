//! The Environment settings (Clarity settings, for every project): the build
//! tools of the five stacks on this computer, every copy a scan finds with
//! its version, the one each build uses, and the person's choice. Kept in
//! Clarity's own folder; see clarity_gen::tools::environment. Beside the
//! stacks, what engines need, with Install for Rust's WebAssembly target.
//! After the tools, the outside agents' programs (Codex, Copilot) the Agent
//! panel starts: every copy a scan finds and the person's choice, kept with
//! the tools' choices and never in the vault (docs/ledger/acp.md).

use std::path::{Path, PathBuf};
use std::process::Stdio;

use clarity_gen::tools::environment::{self, Environment, TOOLS};
use clarity_acp::vendors::{self, Vendor};
use clarity_gen::tools::{self, Probe, System};
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

/// Something a stack misses: what to install, and the heading of the
/// manual's setup page (`clarity_gen::tools::SETUP_TOPIC`) that tells how.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct MissingTool {
    pub what: String,
    pub guide: String,
}

/// A stack: whether it can build here, with what, and what is missing.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct StackView {
    pub name: String,
    pub ready: bool,
    pub found: Vec<String>,
    pub missing: Vec<MissingTool>,
}

/// Engines (Rust built to WebAssembly), beside the stacks: whether they can
/// build here, what is missing, and the command Install runs when Clarity
/// can add what is missing itself.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct EnginesView {
    pub ready: bool,
    pub missing: Vec<MissingTool>,
    /// What Install runs, as people read it: `rustup target add wasm32-unknown-unknown`.
    pub install: Option<String>,
}

/// An outside agent's program as the Environment settings show it. Never
/// run to ask its version: an agent program answers ACP on its own.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentView {
    /// The agent's key: `codex`, `copilot`.
    pub key: String,
    pub name: String,
    /// The program the person chose, if any.
    pub chosen: Option<String>,
    /// The program the Agent panel starts: the chosen one while it is there, else the first found.
    pub used: Option<String>,
    /// Every copy a scan found, the chosen one too.
    pub copies: Vec<String>,
    /// Where the person installs it.
    pub install: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct EnvironmentView {
    /// Where the choices are kept.
    pub file: String,
    pub stacks: Vec<StackView>,
    pub engines: EnginesView,
    pub tools: Vec<ToolView>,
    pub agents: Vec<AgentView>,
}

/// What a copy says it is: the first line it prints for its version.
fn version_of(path: &Path, program: &str) -> Option<String> {
    // Visual Studio's cl has no --version: run alone, it names itself.
    let cl = path.file_stem().is_some_and(|s| s.eq_ignore_ascii_case("cl"));
    let args: &[&str] = if cl {
        &[]
    } else if program == "go" {
        &["version"]
    } else {
        &["--version"]
    };
    let out = clarity_project::background_command(path).args(args).stdin(Stdio::null()).output().ok()?;
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
        .map(|s| StackView {
            name: tools::stack_name(s.backend, s.ui),
            ready: s.missing.is_empty(),
            found: s.found,
            missing: s.missing.into_iter().zip(s.guides).map(|(what, guide)| MissingTool { what, guide: guide.to_string() }).collect(),
        })
        .collect();
    let engines = tools::engine_check(&System);
    let engines = EnginesView {
        ready: engines.missing.is_empty(),
        install: engines.install.map(|command| command.join(" ")),
        missing: engines.missing.into_iter().zip(engines.guides).map(|(what, guide)| MissingTool { what, guide: guide.to_string() }).collect(),
    };
    EnvironmentView { file: environment::file().display().to_string(), stacks, engines, tools, agents: agents(&env) }
}

/// The outside agents' programs: every copy a scan finds, the chosen one first when the scan did not find it.
fn agents(env: &Environment) -> Vec<AgentView> {
    let folders = vendors::search_folders();
    let installed = vendors::installed_in(&|vendor| env.chosen_agent(vendor.key()).map(Path::to_path_buf), &folders);
    Vendor::ALL
        .into_iter()
        .map(|vendor| {
            let mut copies = vendors::copies_in(vendor, &folders);
            if let Some(chosen) = env.chosen_agent(vendor.key()).filter(|c| !copies.iter().any(|p| p == c)) {
                copies.insert(0, chosen.to_path_buf());
            }
            AgentView {
                key: vendor.key().to_string(),
                name: vendor.name().to_string(),
                chosen: env.chosen_agent(vendor.key()).map(|p| p.display().to_string()),
                used: installed.iter().find(|f| f.vendor == vendor).map(|f| f.program.display().to_string()),
                copies: copies.iter().map(|p| p.display().to_string()).collect(),
                install: vendor.install_page().to_string(),
            }
        })
        .collect()
}

/// Installs Rust's WebAssembly target for engines: the one install Clarity
/// runs itself, once the person agreed, and always the same command
/// (`tools::ENGINE_INSTALL`), never one given from outside. Gives the
/// environment as it is then.
#[tauri::command(async)]
pub fn install_engine_target() -> Result<EnvironmentView, String> {
    let [program, args @ ..] = tools::ENGINE_INSTALL;
    let rustup = System.find(program).ok_or_else(|| "rustup is not on this computer: install Rust from https://rustup.rs first".to_string())?;
    let out = clarity_project::background_command(rustup).args(args).stdin(Stdio::null()).output().map_err(|err| format!("cannot start rustup: {err}"))?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr);
        return Err(format!("rustup could not add the target: {}", said.trim()));
    }
    Ok(view())
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

/// Chooses the program at `path` for the outside agent `key` (`codex`,
/// `copilot`), or forgets the choice without a path, so the Agent panel
/// starts the one a scan finds; gives the environment as it is then.
#[tauri::command(async)]
pub fn choose_agent(key: String, path: Option<String>) -> Result<EnvironmentView, String> {
    let vendor = Vendor::of_key(&key).ok_or_else(|| format!("{key} is not an agent Clarity starts"))?;
    let path = path.map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
    if let Some(path) = &path
        && !tools::is_program(Path::new(path))
    {
        return Err(format!("{path} is not a program that can be started; pick the {} program itself", vendor.name()));
    }
    let mut env = Environment::current();
    env.choose_agent(vendor.key(), path.map(PathBuf::from));
    env.save(&environment::file()).map_err(|err| format!("cannot write {}: {err}", environment::file().display()))?;
    Ok(view())
}
