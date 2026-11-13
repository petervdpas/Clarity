//! An app's commands for AI agents while it runs (docs/ledger/app-mcp.md):
//! Clarity.UI listens for the app (`relay`, running.json beside its
//! settings) and serves each connection on a thread of its own; a grant the
//! person is asked for and a command to run go to the screen's thread as
//! work (`AgentWork`), as a click does, and their answers come back. A
//! command's question goes to the agent while the screen's thread waits.

use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};

use clarity_app_mcp::{Ask, Decision, Grant, Host, Kind, Options, Ran, Server, Tool, ToolList, relay};
use clarity_runtime::program::{AgentRan, AgentsOffered, Program};
use serde_json::Value;

use crate::locale::translate;

/// What the person is asked to allow, in their words.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantAsked {
    /// The agent's name, or Clarity's word for an agent when its client gives none.
    pub agent: String,
    pub kind: Kind,
    /// What the command does (its help).
    pub description: String,
    /// Each argument and option given, by name, as text.
    pub arguments: Vec<(String, String)>,
}

impl GrantAsked {
    pub fn of(grant: &Grant) -> GrantAsked {
        let agent = if grant.agent.trim().is_empty() { translate("agentSomeone", "An agent") } else { grant.agent.to_string() };
        let arguments = grant
            .arguments
            .as_object()
            .map(|given| given.iter().map(|(name, value)| (name.clone(), value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string()))).collect())
            .unwrap_or_default();
        GrantAsked { agent, kind: grant.kind, description: grant.description.to_string(), arguments }
    }

    /// The dialog's lines: what the command does, then each value given.
    pub fn lines(&self) -> Vec<String> {
        std::iter::once(self.description.clone()).filter(|d| !d.is_empty()).chain(self.arguments.iter().map(|(name, value)| format!("{name}: {value}"))).collect()
    }

    /// The dialog's title: who wants to change or read the app.
    pub fn title(&self, app: &str) -> String {
        let (key, english) = match self.kind {
            Kind::Change => ("agentAsksChange", "{agent} wants to change {app}"),
            Kind::Read => ("agentAsksRead", "{agent} wants to read {app}"),
        };
        translate(key, english).replace("{agent}", &self.agent).replace("{app}", app)
    }
}

/// The core's decision for the person's choice in the grant dialog.
pub fn decision(choice: crate::dialogs::GrantChoice) -> Decision {
    match choice {
        crate::dialogs::GrantChoice::Once => Decision::Once,
        crate::dialogs::GrantChoice::Session => Decision::Session,
        crate::dialogs::GrantChoice::Refuse => Decision::Refuse,
    }
}

/// What a running command says back to the connection: a question for the agent, or its end.
pub enum CallSaid {
    Ask { question: String, form: String, answer: Sender<Option<String>> },
    Done(AgentRan),
}

/// Work for the screen's thread.
pub enum AgentWork {
    /// Ask the person; the answer goes to `reply` (a dialog closed without one is a refusal).
    Grant { asked: GrantAsked, app: String, reply: Sender<Decision> },
    /// Run the command line `line`; what it says goes to `said`. `change` names
    /// the agent and the command when it changes the app: the windows read
    /// their data again after it, and a toast says what the agent did.
    Call { line: Vec<String>, said: Sender<CallSaid>, change: Option<(String, String)> },
    /// What agents do changed (`agent_activity`): the windows draw it again.
    Changed,
    /// An agent was turned away, as the app serves so many at once already
    /// (`agents at most`): the windows say so with this toast.
    TurnedAway(String),
}

impl std::fmt::Debug for AgentWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentWork::Grant { asked, .. } => write!(f, "Grant({:?})", asked.title("")),
            AgentWork::Call { line, .. } => write!(f, "Call({line:?})"),
            AgentWork::Changed => write!(f, "Changed"),
            AgentWork::TurnedAway(toast) => write!(f, "TurnedAway({toast})"),
        }
    }
}

/// Sends work to the screen's thread and wakes it.
pub type Post = Arc<dyn Fn(AgentWork) + Send + Sync>;

/// How the open app serves: the person is asked; refusals in their words.
pub fn options(offered: &AgentsOffered) -> Options {
    let say = |key: &str, english: &str| translate(key, english).replace("{app}", &offered.app);
    Options {
        version: offered.version.clone(),
        open: true,
        closed_reads: false,
        closed_changes: false,
        refused: say("agentRefused", "The person using {app} refused this."),
        closed: say("agentClosed", "{app} is not open; open it to allow this."),
    }
}

/// Starts listening for the app `program` runs, when it offers commands to
/// agents: each connection is served on its own thread and posts its work.
/// None when it offers none, or another copy of it listens already.
pub fn listen(program: &Arc<dyn Program>, post: Post) -> Option<relay::Listening> {
    let offered = program.agents()?;
    let folder = clarity_runtime::settings::file(&offered.program)?.parent()?.to_path_buf();
    listen_at(&offered, &clarity_app_mcp::discovery::file_in(&folder), post)
}

/// What a refused agent reads when the app serves `at_most` agents already.
pub fn full_text(app: &str, at_most: usize) -> String {
    let text = if at_most == 1 {
        translate("agentsFullOne", "{app} serves 1 AI agent at once, and that many are connected; try again when one is done.")
    } else {
        translate("agentsFull", "{app} serves {count} AI agents at once, and that many are connected; try again when one is done.")
    };
    text.replace("{app}", app).replace("{count}", &at_most.to_string())
}

/// What the open app's window says when an agent was turned away.
pub fn turned_away_note(agent: Option<&str>, app: &str, at_most: usize) -> String {
    let agent = agent.filter(|a| !a.trim().is_empty()).map(str::to_string).unwrap_or_else(|| translate("agentSomeone", "An agent"));
    let text = if at_most == 1 {
        translate("agentTurnedAwayOne", "{agent} was turned away: {app} serves 1 AI agent at once.")
    } else {
        translate("agentTurnedAway", "{agent} was turned away: {app} serves {count} AI agents at once.")
    };
    text.replace("{agent}", &agent).replace("{app}", app).replace("{count}", &at_most.to_string())
}

/// As `listen`, with running.json at `file`; at most as many agents at once as the tool list says.
pub fn listen_at(offered: &AgentsOffered, file: &std::path::Path, post: Post) -> Option<relay::Listening> {
    let list = ToolList::from_json(&offered.tools).ok()?;
    let options = options(offered);
    let app = offered.app.clone();
    let at_most = list.at_most;
    let (told, said_app) = (post.clone(), app.clone());
    let turned_away = move |agent: Option<String>| (told)(AgentWork::TurnedAway(turned_away_note(agent.as_deref(), &said_app, at_most)));
    let listening = relay::listen_limited(&offered.app, file, at_most, &full_text(&offered.app, at_most), turned_away, move |lines, close| {
        let activity = crate::agent_activity::activity();
        let id = activity.connected(close);
        (post)(AgentWork::Changed);
        let mut server = Server::with(list.clone(), options.clone());
        server.serve(lines, &mut Relayed { post: post.clone(), app: app.clone(), refused: options.refused.clone(), id, granted: None });
        activity.left(id);
        (post)(AgentWork::Changed);
    })
    .ok()?;
    Some(listening)
}

/// A connection's host: grants and calls go to the screen's thread.
struct Relayed {
    post: Post,
    app: String,
    refused: String,
    /// The connection's number in the activity, and the grant of the call about to run.
    id: u64,
    granted: Option<crate::agent_activity::Granted>,
}

impl Relayed {
    /// Notes a call in the activity, and the windows draw it again.
    fn note(&mut self, tool: &str, arguments: &Value, granted: Option<crate::agent_activity::Granted>, answer: &str, refused: bool) {
        let activity = crate::agent_activity::activity();
        let agent = activity.connections().into_iter().find(|c| c.id == self.id).map(|c| c.agent).unwrap_or_default();
        let values = arguments.as_object().map(|given| given.iter().map(|(n, v)| format!("{n}: {}", v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string()))).collect()).unwrap_or_default();
        activity.log(crate::agent_activity::Logged { at: crate::agent_activity::clock(), agent, tool: tool.to_string(), values, granted, answer: answer.to_string(), refused });
        (self.post)(AgentWork::Changed);
    }
}

impl Host for Relayed {
    fn grant(&mut self, grant: &Grant) -> Decision {
        crate::agent_activity::activity().named(self.id, grant.agent);
        let (reply, answer) = channel();
        (self.post)(AgentWork::Grant { asked: GrantAsked::of(grant), app: self.app.clone(), reply });
        let decision = answer.recv().unwrap_or(Decision::Refuse);
        let granted = match decision {
            Decision::Once => crate::agent_activity::Granted::Once,
            Decision::Session => crate::agent_activity::Granted::Session,
            Decision::Refuse => crate::agent_activity::Granted::Refused,
        };
        if decision == Decision::Refuse {
            let refused = self.refused.clone();
            self.note(grant.tool, grant.arguments, Some(granted), &refused, true);
        } else {
            self.granted = Some(granted);
        }
        decision
    }

    fn run(&mut self, tool: &Tool, arguments: &Value, ask: &mut dyn Ask) -> Ran {
        let line = clarity_app_mcp::cli::command_line(tool, arguments);
        let (said, hears) = channel();
        let change = (tool.kind() == Kind::Change).then(|| {
            let agent = crate::agent_activity::activity().connections().into_iter().find(|c| c.id == self.id).map(|c| c.agent).unwrap_or_default();
            (agent, tool.name.clone())
        });
        (self.post)(AgentWork::Call { line, said, change });
        loop {
            match hears.recv() {
                Ok(CallSaid::Ask { question, form, answer }) => {
                    let form = serde_json::from_str::<Value>(&form).unwrap_or(Value::Null);
                    let _ = answer.send(clarity_app_mcp::cli::ask(ask, &question, &form));
                }
                Ok(CallSaid::Done(ran)) => {
                    let granted = self.granted.take();
                    self.note(&tool.name, arguments, granted, &ran.text, ran.refused);
                    return if ran.refused { Ran::Refused(ran.text) } else { Ran::Printed(ran.text) };
                }
                // The app closed while the command ran.
                Err(_) => return Ran::Refused(self.refused.clone()),
            }
        }
    }
}

/// The toast in the app's window after an agent's change: who, and what the command said.
pub fn change_toast(agent: &str, tool: &str, ran: &AgentRan) -> String {
    let agent = if agent.trim().is_empty() { translate("agentSomeone", "An agent") } else { agent.to_string() };
    match ran.text.lines().next().filter(|l| !l.trim().is_empty()) {
        Some(what) => translate("agentDid", "{agent}: {what}").replace("{agent}", &agent).replace("{what}", what),
        None => translate("agentRan", "{agent} ran {tool}").replace("{agent}", &agent).replace("{tool}", tool),
    }
}

/// Runs a call's command on the screen's thread: its questions go back to
/// the connection, which asks the agent, and its end follows them. What it did.
pub fn run_call(program: &dyn Program, line: &[String], said: &Sender<CallSaid>) -> AgentRan {
    let mut ask = |question: &str, form: &str| {
        let (answer, heard) = channel();
        said.send(CallSaid::Ask { question: question.to_string(), form: form.to_string(), answer }).ok()?;
        heard.recv().ok().flatten()
    };
    let ran = program.agent_call(line, &mut ask);
    let _ = said.send(CallSaid::Done(ran.clone()));
    ran
}
