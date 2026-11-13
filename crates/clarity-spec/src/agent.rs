//! What a command offered to AI agents does to the app (docs/ledger/app-mcp.md):
//! whether it changes anything, and what kind of change, worked out from its
//! statements. The app's MCP service asks the person before a change and once
//! a connection for reads; the grant dialog says what kind of change comes.

use crate::code::check::each_stmt;
use crate::code::{CommandDecl, Expr, ExprKind, FunctionKind, HttpMethod, StmtKind, each_expr_in};
use crate::model::Project;

/// What a command changes; nothing at all for one that only reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct Changes {
    /// Saves or deletes records, itself or through an action with `effects db.write`.
    pub data: bool,
    /// Writes a file to a file store, itself or through an action with `effects files.write`.
    pub files: bool,
    /// Sets one of the person's settings.
    pub settings: bool,
    /// Sends something out: an HTTP call that is no `get`, or an action with `effects net`.
    pub net: bool,
}

impl Changes {
    /// Whether it changes anything.
    pub fn any(&self) -> bool {
        self.data || self.files || self.settings || self.net
    }
}

/// What `command` changes, from its statements and the effects of the actions it calls.
pub fn changes(project: &Project, command: &CommandDecl) -> Changes {
    let mut out = Changes::default();
    each_stmt(&command.body, &mut |stmt| match &stmt.kind {
        StmtKind::Save { .. } | StmtKind::Delete { .. } => out.data = true,
        StmtKind::Write { .. } => out.files = true,
        StmtKind::Assign { target, .. } if sets_a_setting(target) => out.settings = true,
        _ => {}
    });
    each_expr_in(&command.body, &mut |expr| match &expr.kind {
        ExprKind::Http { method, .. } if *method != HttpMethod::Get => out.net = true,
        ExprKind::Call { function, .. } => {
            let action = project.files.iter().flat_map(|f| &f.functions).find(|f| f.name.name == function.name && f.kind == FunctionKind::Action);
            for effect in action.map(|a| a.effects.as_slice()).unwrap_or_default() {
                match effect.name.as_str() {
                    "db.write" => out.data = true,
                    "files.write" => out.files = true,
                    "net" => out.net = true,
                    _ => {}
                }
            }
        }
        _ => {}
    });
    out
}

/// `settings.pageSize = 30`: a command sets one of the person's settings.
fn sets_a_setting(target: &Expr) -> bool {
    matches!(&target.kind, ExprKind::Member { object, .. } if matches!(&object.kind, ExprKind::Name { name } if name == "settings"))
}
