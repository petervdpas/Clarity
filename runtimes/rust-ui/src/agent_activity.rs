//! What AI agents do in the running app (docs/ledger/app-mcp.md): who is
//! connected, and each call with the person's grant and its answer. The
//! connections (`agents`, on their own threads) write it; the grant dialog
//! and the note after a change name the agent from it. One per app. No networking here, so a
//! browser build has it too, always empty.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// Closes one connection from the app's side.
pub type Closer = Arc<dyn Fn() + Send + Sync>;

/// A connected agent.
#[derive(Clone)]
pub struct Connected {
    pub id: u64,
    /// The agent's name, once its client gave one.
    pub agent: String,
    /// When it connected, as the clock on the wall says it (`21:30`).
    pub since: String,
    close: Closer,
}

/// What the person decided for a call; none when no grant was asked (allowed for the session already).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granted {
    Once,
    Session,
    Refused,
}

/// One call an agent made.
#[derive(Debug, Clone, PartialEq)]
pub struct Logged {
    pub at: String,
    pub agent: String,
    /// The command, as agents call it (`customer-add`).
    pub tool: String,
    /// Its values, as `name: Ada`.
    pub values: Vec<String>,
    pub granted: Option<Granted>,
    /// What it answered, or why it was refused.
    pub answer: String,
    pub refused: bool,
}

#[derive(Default)]
struct State {
    next: u64,
    connections: Vec<Connected>,
    log: Vec<Logged>,
}

/// How many calls the log keeps, the oldest going first.
const KEPT: usize = 200;

#[derive(Default)]
pub struct Activity {
    state: Mutex<State>,
    changes: AtomicU64,
}

/// The app's activity.
pub fn activity() -> &'static Activity {
    static ONE: OnceLock<Activity> = OnceLock::new();
    ONE.get_or_init(Activity::default)
}

/// The time now as the clock on the wall says it.
pub fn clock() -> String {
    clarity_runtime::now().format("%H:%M:%S").to_string()
}

impl Activity {
    fn change(&self, act: impl FnOnce(&mut State)) {
        act(&mut self.state.lock().unwrap_or_else(|p| p.into_inner()));
        self.changes.fetch_add(1, Ordering::SeqCst);
    }

    fn read<R>(&self, read: impl FnOnce(&State) -> R) -> R {
        read(&self.state.lock().unwrap_or_else(|p| p.into_inner()))
    }

    /// A number that grows with every change: what reads it draws again when it moved.
    pub fn changes(&self) -> u64 {
        self.changes.load(Ordering::SeqCst)
    }

    /// A connection began: its number.
    pub fn connected(&self, close: Closer) -> u64 {
        let mut id = 0;
        self.change(|s| {
            s.next += 1;
            id = s.next;
            s.connections.push(Connected { id, agent: String::new(), since: clock().chars().take(5).collect(), close });
        });
        id
    }

    /// Connection `id`'s agent gave its name.
    pub fn named(&self, id: u64, agent: &str) {
        if agent.trim().is_empty() || self.read(|s| s.connections.iter().any(|c| c.id == id && c.agent == agent)) {
            return;
        }
        self.change(|s| {
            if let Some(c) = s.connections.iter_mut().find(|c| c.id == id) {
                c.agent = agent.to_string();
            }
        });
    }

    /// Connection `id` ended.
    pub fn left(&self, id: u64) {
        self.change(|s| s.connections.retain(|c| c.id != id));
    }

    pub fn log(&self, logged: Logged) {
        self.change(|s| {
            s.log.push(logged);
            let over = s.log.len().saturating_sub(KEPT);
            s.log.drain(..over);
        });
    }

    pub fn connections(&self) -> Vec<Connected> {
        self.read(|s| s.connections.clone())
    }

    /// The calls, oldest first.
    pub fn logged(&self) -> Vec<Logged> {
        self.read(|s| s.log.clone())
    }

    /// Closes connection `id`: its agent's session ends, and its grants with it.
    pub fn disconnect(&self, id: u64) {
        if let Some(close) = self.read(|s| s.connections.iter().find(|c| c.id == id).map(|c| c.close.clone())) {
            close();
        }
    }

    pub fn disconnect_all(&self) {
        for close in self.read(|s| s.connections.iter().map(|c| c.close.clone()).collect::<Vec<_>>()) {
            close();
        }
    }
}
