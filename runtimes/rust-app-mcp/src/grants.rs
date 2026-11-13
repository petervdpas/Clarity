//! Who allows a call. While the app is open the person does: a change is
//! asked at every call unless allowed for the session (that tool, this
//! connection), and the first read of a connection is asked, after which a
//! read allowed for the session goes through. While the app is closed nobody
//! is there to ask: the person's settings allow reads and changes, else they
//! are refused.

use std::collections::HashSet;

use crate::options::Options;

/// What a call does, for its grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Read,
    Change,
}

impl Kind {
    pub fn word(self) -> &'static str {
        match self {
            Kind::Read => "read",
            Kind::Change => "change",
        }
    }
}

/// The person's answer to a grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Once,
    Session,
    Refuse,
}

impl Decision {
    pub fn word(self) -> &'static str {
        match self {
            Decision::Once => "once",
            Decision::Session => "session",
            Decision::Refuse => "refuse",
        }
    }

    pub fn from_word(word: &str) -> Option<Decision> {
        [Decision::Once, Decision::Session, Decision::Refuse].into_iter().find(|d| d.word() == word)
    }
}

/// What happens to a call before it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Run,
    /// The person is asked.
    Ask,
    /// Refused with this text, nobody asked.
    Refuse(String),
}

/// The grants of one connection.
#[derive(Debug, Default)]
pub struct Grants {
    reads: bool,
    tools: HashSet<String>,
}

impl Grants {
    pub fn verdict(&self, options: &Options, tool: &str, kind: Kind) -> Verdict {
        if !options.open {
            let allowed = match kind {
                Kind::Read => options.closed_reads,
                Kind::Change => options.closed_changes,
            };
            return if allowed { Verdict::Run } else { Verdict::Refuse(options.closed.clone()) };
        }
        let granted = match kind {
            Kind::Read => self.reads,
            Kind::Change => self.tools.contains(tool),
        };
        if granted { Verdict::Run } else { Verdict::Ask }
    }

    /// Keeps the person's answer; whether the call runs.
    pub fn decided(&mut self, tool: &str, kind: Kind, decision: Decision) -> bool {
        match (decision, kind) {
            (Decision::Refuse, _) => return false,
            (Decision::Once, _) => {}
            (Decision::Session, Kind::Read) => self.reads = true,
            (Decision::Session, Kind::Change) => {
                self.tools.insert(tool.to_string());
            }
        }
        true
    }
}
