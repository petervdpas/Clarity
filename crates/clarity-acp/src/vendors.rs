//! The agent programs Clarity's chat panel starts (docs/ACP-PLAN.md, step 4):
//! where each is found, the arguments that make it speak ACP over stdio, how
//! it is kept asking before it runs anything, how it signs in, and where the
//! person installs it. Clarity never installs them.
//!
//! Every fact here is from the vendor's own pages, cited beside it and read
//! 2026-11-09. What those pages do not say is marked OPEN.

use std::ffi::OsStr;
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
    /// The vendor a vault connection of kind `kind` names: `codex` or `copilot`, in any case.
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

/// The vendor's program to start: the path the person set in Clarity's
/// settings when there is one, else the first on the PATH.
pub fn find(vendor: Vendor, from_settings: Option<&Path>) -> Result<Program, String> {
    find_in(vendor, from_settings, std::env::var_os("PATH").as_deref())
}

/// Like `find`, with `path` in place of the PATH.
pub fn find_in(vendor: Vendor, from_settings: Option<&Path>, path: Option<&OsStr>) -> Result<Program, String> {
    let program = |command: PathBuf| Program {
        command,
        args: vendor.acp_args().iter().map(|a| a.to_string()).collect(),
        env: vendor.environment().iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    };
    if let Some(chosen) = from_settings {
        if startable(chosen) {
            return Ok(program(chosen.to_path_buf()));
        }
        return Err(format!(
            "{} cannot be started from {}, the command path of its connection in the vault; install it ({}) or set the path again",
            vendor.name(),
            chosen.display(),
            vendor.install_page()
        ));
    }
    let folders: Vec<PathBuf> = path.map(|p| std::env::split_paths(p).collect()).unwrap_or_default();
    folders
        .iter()
        .flat_map(|folder| program_names(vendor).iter().map(move |name| folder.join(name)))
        .find(|file| startable(file))
        .map(program)
        .ok_or_else(|| {
            format!(
                "{} is not installed: its program ({}) is not on the PATH. Install it from {}, or give its full path as the field command of its connection in the vault",
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
