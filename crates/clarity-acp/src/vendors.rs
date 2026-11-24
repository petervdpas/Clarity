//! The agent programs Clarity's chat panel starts (docs/ledger/acp.md):
//! where each is found, the arguments that make it speak ACP over stdio, how
//! it is kept asking before it runs anything, how it signs in, and where the
//! person installs it. Clarity never installs them. A scan finds the ones
//! installed, so the panel lists them with no vault connection; the person
//! may choose another program in Clarity settings > Environment.
//!
//! Every fact here is from the vendor's own pages, cited beside it and read
//! 2026-11-09. What those pages do not say is marked OPEN.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::Program;

/// An agent program the panel can talk to with the person's own login.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    /// OpenAI's Codex, through the ACP adapter `codex-acp`.
    Codex,
    /// GitHub Copilot CLI, which is an ACP server itself.
    Copilot,
}

impl Vendor {
    /// Every vendor, in the order the panel and the Environment settings list them.
    pub const ALL: [Vendor; 2] = [Vendor::Codex, Vendor::Copilot];

    /// The key the vendor goes by: the panel's name for the agent a scan
    /// found, and the key its chosen program is kept under.
    pub fn key(self) -> &'static str {
        match self {
            Vendor::Codex => "codex",
            Vendor::Copilot => "copilot",
        }
    }

    /// The vendor of key `key`, exactly as `key` gives it.
    pub fn of_key(key: &str) -> Option<Vendor> {
        Vendor::ALL.into_iter().find(|v| v.key() == key)
    }

    /// The vendor a vault connection of kind `kind` names: `codex` or `copilot`, in any case.
    /// Such connections are read still; new ones are not needed (docs/ledger/acp.md, 2026-11-23).
    pub fn of_kind(kind: &str) -> Option<Vendor> {
        match kind.trim().to_ascii_lowercase().as_str() {
            "codex" => Some(Vendor::Codex),
            "copilot" => Some(Vendor::Copilot),
            _ => None,
        }
    }

    /// The vendor's name as people know it.
    pub fn name(self) -> &'static str {
        match self {
            Vendor::Codex => "Codex",
            Vendor::Copilot => "GitHub Copilot",
        }
    }

    /// Where the person installs the program, on the maintainers' own page.
    pub fn install_page(self) -> &'static str {
        match self {
            // codex-acp: "npm install -g @agentclientprotocol/codex-acp"; the package
            // "includes a compatible @openai/codex dependency". The adapter is kept by the
            // ACP project (moved from zed-industries/codex-acp, archived 2026-07-22), not by OpenAI.
            Vendor::Codex => "https://github.com/agentclientprotocol/codex-acp",
            // npm, WinGet (`winget install GitHub.Copilot`), Homebrew and an install script.
            Vendor::Copilot => "https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli",
        }
    }

    /// The arguments that make the program speak ACP on its stdin and stdout.
    pub fn acp_args(self) -> &'static [&'static str] {
        match self {
            // "codex-acp is a stdio ACP agent server": no arguments.
            // https://github.com/agentclientprotocol/codex-acp
            Vendor::Codex => &[],
            // "copilot --acp --stdio"; "stdio mode is inferred by default".
            // https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server
            Vendor::Copilot => &["--acp", "--stdio"],
        }
    }

    /// The program's own setting that keeps it asking before it changes or
    /// runs anything, so such a step reaches the panel as a permission request.
    ///
    /// Codex: the ACP mode `read-only` (given at start as `INITIAL_AGENT_MODE`, see
    /// `environment`, or chosen with ACP's session mode). Codex's own docs: in the
    /// read-only sandbox "Codex can read files and run commands within the read-only
    /// sandbox"; edits and anything the sandbox does not allow need approval
    /// (https://learn.chatgpt.com/docs/agent-approvals-security). OPEN: Codex has no
    /// mode that asks before every command; commands that only read run inside its
    /// sandbox without asking. The adapter's default mode is `agent` ("Auto review"),
    /// which is not used here (https://github.com/agentclientprotocol/codex-acp,
    /// docs/permission-profiles.md).
    ///
    /// Copilot: none needed. "In manual approval mode, Copilot asks for your approval
    /// when a tool request isn't already allowed", for tools that "modify or execute
    /// files"; only `--allow-all-tools` and `--allow-tool` lift it, and Clarity passes
    /// neither (https://docs.github.com/en/copilot/concepts/agents/copilot-cli/about-copilot-cli).
    /// OPEN: the page does not say in so many words that manual approval is the default
    /// of a fresh `--acp` session; to check by hand against the real program.
    pub fn ask_first_mode(self) -> Option<&'static str> {
        match self {
            Vendor::Codex => Some("read-only"),
            Vendor::Copilot => None,
        }
    }

    /// The program's choices the person may make per chat, with the values
    /// allowed (None: every value it offers). Only choices that keep it asking
    /// before it changes anything, so every AI changes the project through
    /// Clarity's tools (the user: "all ai must use tools!"). Codex's `mode`
    /// stays read-only (workspace-write, agent and full access write by
    /// themselves); Copilot's autopilot and `allow_all` stop it asking. Names
    /// and values as the programs gave them over ACP (codex-acp 2.6.0, GitHub
    /// Copilot CLI 1.0.108).
    fn choosable(self) -> &'static [(&'static str, Option<&'static [&'static str]>)] {
        const COPILOT_MODES: &[&str] = &["https://agentclientprotocol.com/protocol/session-modes#agent", "https://agentclientprotocol.com/protocol/session-modes#plan"];
        match self {
            Vendor::Codex => &[("collaboration_mode", None), ("model", None), ("reasoning_effort", None)],
            Vendor::Copilot => &[("mode", Some(COPILOT_MODES)), ("model", None)],
        }
    }

    /// The choices of `all` the person may make per chat, in the program's
    /// order: only the ones `choosable` names, with only allowed values, each
    /// value once. A choice whose current value is not allowed is left out.
    pub fn safe_choices(self, all: &[crate::Choice]) -> Vec<crate::Choice> {
        let mut out = Vec::new();
        for choice in all {
            let Some((_, allowed)) = self.choosable().iter().find(|(id, _)| *id == choice.id) else { continue };
            let ok = |value: &str| allowed.is_none_or(|list| list.contains(&value));
            if !ok(&choice.current) {
                continue;
            }
            let mut values: Vec<crate::ChoiceValue> = Vec::new();
            for v in &choice.values {
                if ok(&v.value) && !values.iter().any(|seen| seen.value == v.value) {
                    values.push(v.clone());
                }
            }
            out.push(crate::Choice { values, ..choice.clone() });
        }
        out
    }

    /// Environment the program starts with, beside its arguments.
    pub fn environment(self) -> &'static [(&'static str, &'static str)] {
        match self {
            // "INITIAL_AGENT_MODE: initial mode id: read-only, workspace-write, agent,
            // agent-full-access" (https://github.com/agentclientprotocol/codex-acp).
            Vendor::Codex => &[("INITIAL_AGENT_MODE", "read-only")],
            Vendor::Copilot => &[],
        }
    }

    /// How the person signs in with the vendor's own login.
    ///
    /// Codex: through ACP, with the method `chat-gpt` ("ChatGPT", "Use ChatGPT to
    /// authenticate"), by its id: the adapter lists `api-key` first, which wants
    /// `CODEX_API_KEY` or `OPENAI_API_KEY`, and the chat-gpt method only without
    /// `NO_BROWSER` (src/CodexAuthMethod.ts of https://github.com/agentclientprotocol/codex-acp).
    ///
    /// Copilot: before ACP, with the program's own command. `/login` is among the
    /// commands that do not work over ACP
    /// (https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server);
    /// `copilot login [--web-flow | --device-code]` signs in
    /// (https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference),
    /// or a token in `COPILOT_GITHUB_TOKEN`, `GH_TOKEN` or `GITHUB_TOKEN`
    /// (https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli).
    /// OPEN: whether `copilot --acp` lists an ACP authentication method at all.
    pub fn sign_in(self) -> SignIn {
        match self {
            Vendor::Codex => SignIn::Acp("chat-gpt"),
            Vendor::Copilot => SignIn::Command(&["login", "--web-flow"]),
        }
    }
}

impl Vendor {
    /// The arguments that give the program Clarity's tools at its start,
    /// or None when they go with the session (ACP's `session/new`).
    ///
    /// Copilot over ACP takes MCP servers only through HTTP: its log says
    /// "Rejecting non-http/sse MCP server" for a local one. So it gets them
    /// as `--additional-mcp-config` ("Additional MCP servers configuration as
    /// JSON string ... augments config from ~/.copilot/mcp-config.json for
    /// this session", `copilot --help`), in its own config's shape
    /// (`"type": "local"`, command, args, env, `"tools": ["*"]`,
    /// https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers),
    /// and allowed as a whole with `--allow-tool=<server>` (`copilot --allow-tool='MyMCP'`
    /// in `copilot --help`): Clarity's tools design and generate, and run nothing.
    pub fn tools_at_start(self, server: &crate::ToolServer) -> Option<Vec<String>> {
        match self {
            Vendor::Codex => None,
            Vendor::Copilot => {
                // A local server gets only PATH from Copilot ("all other variables must be set here");
                // Clarity's tools find the project open in the IDE through Clarity's folder.
                let env: serde_json::Map<String, serde_json::Value> = ["CLARITY_HOME", "HOME", "USERPROFILE"]
                    .iter()
                    .filter_map(|name| std::env::var(name).ok().map(|value| (name.to_string(), serde_json::Value::String(value))))
                    .collect();
                let config = serde_json::json!({
                    "mcpServers": {
                        server.name.clone(): {
                            "type": "local",
                            "command": server.command.display().to_string(),
                            "args": server.args,
                            "env": env,
                            "tools": ["*"],
                        }
                    }
                });
                Some(vec!["--additional-mcp-config".into(), config.to_string(), format!("--allow-tool={}", server.name)])
            }
        }
    }
}

/// How a vendor's program signs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignIn {
    /// With ACP's own authenticate method of this id, never another the program offers.
    Acp(&'static str),
    /// With the program run once with these arguments, before ACP starts.
    Command(&'static [&'static str]),
}

/// The file names the vendor's program has on this system, most likely first.
/// npm installs a `.cmd` beside a program on Windows; WinGet an `.exe`.
pub fn program_names(vendor: Vendor) -> &'static [&'static str] {
    match (vendor, cfg!(windows)) {
        (Vendor::Codex, false) => &["codex-acp"],
        (Vendor::Codex, true) => &["codex-acp.exe", "codex-acp.cmd"],
        (Vendor::Copilot, false) => &["copilot"],
        (Vendor::Copilot, true) => &["copilot.exe", "copilot.cmd"],
    }
}

/// An agent installed on this machine, as a scan finds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub vendor: Vendor,
    pub program: PathBuf,
}

/// The agents installed on this machine, each with the first copy of its
/// program in the folders `search_folders` gives.
pub fn scan() -> Vec<Found> {
    scan_in(&search_folders())
}

/// Like `scan`, in `folders` alone: the vendors with a copy in them, in `Vendor::ALL`'s order.
pub fn scan_in(folders: &[PathBuf]) -> Vec<Found> {
    Vendor::ALL.into_iter().filter_map(|vendor| copies_in(vendor, folders).into_iter().next().map(|program| Found { vendor, program })).collect()
}

/// Every copy of the vendor's program in `folders`, in their order, each once.
pub fn copies_in(vendor: Vendor, folders: &[PathBuf]) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    for file in folders.iter().flat_map(|folder| program_names(vendor).iter().map(move |name| folder.join(name))) {
        if startable(&file) && !found.contains(&file) {
            found.push(file);
        }
    }
    found
}

/// Where a scan looks: where the build tools check looks (the PATH and the
/// official installers' folders, `clarity_gen_core::tools::installers_path`),
/// then the folders the agents' programs are installed in (`install_folders`).
pub fn search_folders() -> Vec<PathBuf> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    let mut folders: Vec<PathBuf> = std::env::split_paths(&clarity_gen_core::tools::installers_path()).collect();
    for folder in install_folders(std::env::consts::OS, &|name| std::env::var_os(name), home.as_deref()) {
        if !folders.contains(&folder) {
            folders.push(folder);
        }
    }
    folders
}

/// The folders the agents' programs get installed in on system `os`
/// (`linux`, `macos`, `windows`), by the environment variables `var` gives
/// and the home folder `home`. Codex's ACP adapter comes from npm
/// (`npm install -g @agentclientprotocol/codex-acp`); Copilot CLI from npm,
/// WinGet, Homebrew or its install script (`install_page`). So: npm's global
/// folder (on Windows `%APPDATA%\npm` or Node's own folder, elsewhere
/// `<prefix>/bin`), a prefix of the person's (`NPM_CONFIG_PREFIX`,
/// `~/.npm-global`), nvm's versions (newest first), Volta, pnpm and Bun;
/// WinGet's and Scoop's links on Windows; the install script's
/// `~/.local/bin` and `/usr/local/bin`, and Homebrew's folders, elsewhere.
pub fn install_folders(os: &str, var: &dyn Fn(&str) -> Option<OsString>, home: Option<&Path>) -> Vec<PathBuf> {
    let windows = os == "windows";
    let mut out: Vec<PathBuf> = Vec::new();
    let from = |name: &str| var(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(prefix) = from("NPM_CONFIG_PREFIX") {
        out.push(if windows { prefix } else { prefix.join("bin") });
    }
    if windows {
        if let Some(roaming) = from("APPDATA") {
            out.push(roaming.join("npm"));
        }
        // nvm for Windows links the Node in use here.
        out.extend(from("NVM_SYMLINK"));
        for program_files in ["ProgramFiles", "ProgramFiles(x86)"].iter().filter_map(|v| from(v)) {
            out.push(program_files.join("nodejs"));
        }
        if let Some(local) = from("LOCALAPPDATA") {
            out.push(local.join("Microsoft").join("WinGet").join("Links"));
            out.push(from("PNPM_HOME").unwrap_or_else(|| local.join("pnpm")));
        } else {
            out.extend(from("PNPM_HOME"));
        }
        if let Some(home) = home {
            out.push(home.join("scoop").join("shims"));
        }
    } else {
        out.extend(from("NVM_BIN"));
        if let Some(home) = home {
            out.push(home.join(".npm-global").join("bin"));
            out.extend(nvm_versions(&home.join(".nvm").join("versions").join("node")));
            out.push(home.join(".local").join("bin"));
            let pnpm = if os == "macos" { home.join("Library").join("pnpm") } else { home.join(".local").join("share").join("pnpm") };
            out.push(from("PNPM_HOME").unwrap_or(pnpm));
        } else {
            out.extend(from("PNPM_HOME"));
        }
        out.extend(["/usr/local/bin", "/opt/homebrew/bin"].map(PathBuf::from));
    }
    match (from("VOLTA_HOME"), home) {
        (Some(volta), _) => out.push(volta.join("bin")),
        (None, Some(home)) => out.push(home.join(".volta").join("bin")),
        (None, None) => {}
    }
    if let Some(home) = home {
        out.push(home.join(".bun").join("bin"));
    }
    let mut once: Vec<PathBuf> = Vec::new();
    for folder in out {
        if !once.contains(&folder) {
            once.push(folder);
        }
    }
    once
}

/// The `bin` folders of the Node versions nvm keeps in `versions`, newest first.
fn nvm_versions(versions: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(versions) else { return Vec::new() };
    let number = |name: &str| name.trim_start_matches('v').split('.').map(|n| n.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    let mut named: Vec<(Vec<u32>, PathBuf)> = entries.flatten().filter(|e| e.path().is_dir()).map(|e| (number(&e.file_name().to_string_lossy()), e.path().join("bin"))).collect();
    named.sort();
    named.reverse();
    named.into_iter().map(|(_, bin)| bin).collect()
}

/// The agents the panel lists, each with the program it starts: the one
/// chosen in Clarity settings > Environment while it is there, else the
/// first a scan finds. An agent with neither is not installed.
pub fn installed() -> Vec<Found> {
    let env = clarity_gen_core::tools::environment::Environment::current();
    installed_in(&|vendor| env.chosen_agent(vendor.key()).map(Path::to_path_buf), &search_folders())
}

/// Like `installed`, with the choices `chosen` gives and `folders` in place of the scan's.
pub fn installed_in(chosen: &dyn Fn(Vendor) -> Option<PathBuf>, folders: &[PathBuf]) -> Vec<Found> {
    Vendor::ALL
        .into_iter()
        .filter_map(|vendor| {
            let program = chosen(vendor).filter(|p| startable(p)).or_else(|| copies_in(vendor, folders).into_iter().next())?;
            Some(Found { vendor, program })
        })
        .collect()
}

/// The vendor's program to start: the command of an old vault connection
/// (`from_vault`) when it has one, else the program chosen in Clarity
/// settings > Environment while it is there, else the first a scan finds.
pub fn find(vendor: Vendor, from_vault: Option<&Path>) -> Result<Program, String> {
    let env = clarity_gen_core::tools::environment::Environment::current();
    find_in(vendor, from_vault, env.chosen_agent(vendor.key()), &search_folders())
}

/// Like `find`, with the choice from the settings given and `folders` in place of the scan's.
pub fn find_in(vendor: Vendor, from_vault: Option<&Path>, from_settings: Option<&Path>, folders: &[PathBuf]) -> Result<Program, String> {
    let program = |command: PathBuf| Program {
        command,
        args: vendor.acp_args().iter().map(|a| a.to_string()).collect(),
        env: vendor.environment().iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    };
    if let Some(command) = from_vault {
        if startable(command) {
            return Ok(program(command.to_path_buf()));
        }
        return Err(format!(
            "{} cannot be started from {}, the command of its connection in the vault; install it ({}) or remove the command, so Clarity finds the program itself",
            vendor.name(),
            command.display(),
            vendor.install_page()
        ));
    }
    let chosen = |of: Vendor| if of == vendor { from_settings.map(Path::to_path_buf) } else { None };
    installed_in(&chosen, folders).into_iter().find(|f| f.vendor == vendor).map(|f| program(f.program)).ok_or_else(|| {
        format!(
            "{} is not installed: its program ({}) is not on the PATH or in the folders npm and the installers use. Install it from {}, or choose its program in Clarity settings > Environment",
            vendor.name(),
            program_names(vendor)[0],
            vendor.install_page()
        )
    })
}

/// Whether `file` is a program that can be started: a file, and on Linux and macOS one marked to run.
fn startable(file: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(file) else { return false };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}
