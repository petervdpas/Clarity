//! A command-line program's commands for AI agents (docs/ledger/app-mcp.md):
//! `<program> mcp` serves them over MCP through Clarity.AppMcp while the app
//! is closed; while it runs, `mcp` forwards to it (running.json beside the
//! settings), and the app grants each change. A call becomes the command line the program reads with its own
//! parser (`clarity_app_mcp::cli`), runs as if typed, and what it prints is
//! the call's answer; its questions go to the agent. Nobody is there to ask
//! for a grant: the person's settings `agentReadsWhileClosed` and
//! `agentChangesWhileClosed` allow, else the call is refused.

use clarity_app_mcp::{Ask, Decision, Grant, Host, Lines, Options, Ran, Server, Tool, ToolList};
use serde_json::{Map, Value};

use crate::cli::{self, Program, Session};

/// Clarity's words of an app for agents, in English (conformance/app-mcp.json).
pub const WORDS: [(&str, &str); 3] = cli::AGENT_WORDS;

/// The settings, kept in the person's settings.json, that allow agents
/// while the app is closed.
pub const READS_SETTING: &str = "agentReadsWhileClosed";
pub const CHANGES_SETTING: &str = "agentChangesWhileClosed";

/// What a program serves: its tool list (as clarity-gen writes it), the
/// app's title, the name its settings are kept under, and its version.
#[derive(Debug, Clone, Copy)]
pub struct Agents<'a> {
    pub tools: &'a str,
    pub app: &'a str,
    pub program: &'a str,
    pub version: &'a str,
}

/// A word of Clarity's in the language shown, with the app's title.
fn say(key: &str, app: &str) -> String {
    let english = WORDS.iter().find(|(k, _)| *k == key).map_or(key, |(_, v)| *v);
    crate::i18n::value(key).filter(|v| !v.is_empty()).unwrap_or_else(|| english.to_string()).replace("{app}", app)
}

/// How the program serves while the app is closed, with what the person's
/// settings `kept` allow.
/// A switch the person never set is what the project allows (the tool list's `whileClosed`).
pub fn options(agents: &Agents, kept: &Map<String, Value>) -> Options {
    let set = |setting: &str| kept.get(setting).and_then(Value::as_bool);
    let list = ToolList::from_json(agents.tools).ok();
    let (closed_reads, closed_changes) = match &list {
        Some(list) => list.closed_allows(set(READS_SETTING), set(CHANGES_SETTING)),
        None => (set(READS_SETTING).unwrap_or(false), set(CHANGES_SETTING).unwrap_or(false)),
    };
    Options {
        version: agents.version.to_string(),
        open: false,
        closed_reads,
        closed_changes,
        refused: say("agentRefused", agents.app),
        closed: say("agentClosed", agents.app),
    }
}

type Run<'r> = &'r mut dyn FnMut(&mut Session<'_>, usize, Vec<Option<String>>) -> crate::Result<i32>;

/// The program's commands as the service's host.
struct Commands<'p, 'r> {
    program: &'p Program,
    run: Run<'r>,
}

impl Host for Commands<'_, '_> {
    fn run(&mut self, tool: &Tool, arguments: &Value, ask: &mut dyn Ask) -> Ran {
        let line = clarity_app_mcp::cli::command_line(tool, arguments);
        let mut asking = |question: &str, form: &str| clarity_app_mcp::cli::ask(ask, question, &serde_json::from_str::<Value>(form).unwrap_or(Value::Null));
        let run = &mut self.run;
        let ran = cli::run_line(self.program, &line, &mut asking, |session, command, values| run(session, command, values));
        if ran.refused { Ran::Refused(ran.text) } else { Ran::Printed(ran.text) }
    }

    /// While the app is closed nobody is there to ask.
    fn grant(&mut self, _grant: &Grant) -> Decision {
        Decision::Refuse
    }
}

/// Serves the program's commands over `lines` with the person's settings `kept`.
pub fn serve(program: &Program, agents: &Agents, kept: &Map<String, Value>, lines: &mut dyn Lines, run: Run) -> crate::Result<()> {
    let list = ToolList::from_json(agents.tools).map_err(crate::Error::Invalid)?;
    let mut server = Server::with(list, options(agents, kept));
    server.serve(lines, &mut Commands { program, run });
    Ok(())
}

/// Runs a command-line program whose commands agents may call: `mcp` as the
/// only word serves them over standard input and output; anything else is
/// the command line as `cli::run_program` reads it.
pub fn run_program(program: &Program, agents: &Agents, run: impl FnMut(&mut Session<'_>, usize, Vec<Option<String>>) -> crate::Result<i32>) -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args != ["mcp"] {
        return cli::run_program(program, run);
    }
    mcp(program, agents, run)
}

/// `<program> mcp`, of the command-line program or the desktop one: forwards
/// standard input and output to the running app when one answers, else
/// serves the commands for agents as the closed app.
pub fn mcp(program: &Program, agents: &Agents, mut run: impl FnMut(&mut Session<'_>, usize, Vec<Option<String>>) -> crate::Result<i32>) -> std::process::ExitCode {
    // The running app answers agents itself, granting each change (docs/ledger/app-mcp.md).
    if let Some(folder) = crate::settings::file(agents.program).and_then(|f| f.parent().map(std::path::Path::to_path_buf))
        && clarity_app_mcp::relay::forward(&clarity_app_mcp::discovery::file_in(&folder))
    {
        return std::process::ExitCode::SUCCESS;
    }
    let kept = crate::settings::read(agents.program);
    match serve(program, agents, &kept, &mut clarity_app_mcp::stdio::Stdio, &mut run) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            std::process::ExitCode::from(1)
        }
    }
}
