//! The edit bus (DESIGN 5.29): per open window, the record of the person's
//! edits, kept here in the runtime, not in the screen. Each edit is a message
//! with the part, its scope (its form or panel, or the part itself), what it
//! changed (each name of the window's data, or `#parts.<name>` for a part's
//! own text or tick, with its value before and after) and the database steps
//! its handler made. The screen sends an edit when the person ends one, and
//! Ctrl+Z or Ctrl+Y as an undo; the runtime takes the steps back or does them
//! again and answers with the window's state as it was. Every runtime keeps
//! the record alike (conformance/edit-bus.json).

use crate::ui::{Reply, WindowState};
use crate::{Db, Result};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// One name an edit changed, with its value before and after; none when the
/// name was not there.
#[derive(Debug, Clone)]
pub struct Changed {
    pub name: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// One edit of the person's.
#[derive(Debug, Clone)]
pub struct Edit {
    pub part: String,
    pub scope: String,
    pub changes: Vec<Changed>,
    pub steps: Vec<u64>,
}

/// How many edits a window keeps; the oldest go first.
const MAX_EDITS: usize = 500;

/// The record of one window's edits.
#[derive(Debug, Default)]
pub struct EditBus {
    /// Where the next edit starts; none before the window's first reply.
    baseline: Option<Map<String, Value>>,
    going: Option<Edit>,
    done: Vec<Edit>,
    undone: Vec<Edit>,
}

impl EditBus {
    /// The window's data as a reply that is no edit left it: the edits start
    /// from here. While an edit goes on (a drag), it waits for its end.
    pub fn reset(&mut self, data: &Map<String, Value>) {
        if self.going.is_none() {
            self.baseline = Some(data.clone());
        }
    }

    /// The window's data where the next edit starts: what a change the
    /// runtime refuses is taken back to; none before any reply.
    pub fn baseline(&self) -> Option<&Map<String, Value>> {
        self.baseline.as_ref()
    }

    /// An edit that goes on: `steps` its handler made; it ends with `edit`.
    pub fn drag(&mut self, part: &str, scope: &str, steps: &[u64]) {
        self.going.get_or_insert_with(|| Edit { part: part.into(), scope: scope.into(), changes: vec![], steps: vec![] }).steps.extend_from_slice(steps);
    }

    /// The end of an edit of `part`: `data` is the window's data now, with
    /// what its handler changed, `steps` the database steps it made. One that
    /// changed nothing and made no step is no message; a new edit ends what
    /// could be done again in its scope.
    pub fn edit(&mut self, part: &str, scope: &str, data: &Map<String, Value>, steps: &[u64]) {
        let mut all = self.going.take().map(|g| g.steps).unwrap_or_default();
        all.extend_from_slice(steps);
        let changes = between(self.baseline.as_ref().unwrap_or(&Map::new()), data);
        self.baseline = Some(data.clone());
        if changes.is_empty() && all.is_empty() {
            return;
        }
        self.done.push(Edit { part: part.into(), scope: scope.into(), changes, steps: all });
        if self.done.len() > MAX_EDITS {
            self.done.remove(0);
        }
        self.undone.retain(|e| e.scope != scope);
    }

    /// Takes back (`back`) or does again the last edit of `scope`, on `data`,
    /// the window's data now: `take` takes its steps back or does them again,
    /// in that order. The window's data after it; none when the scope has no
    /// such edit. When `take` refuses, the edit stays where it was.
    pub fn undo(&mut self, scope: &str, data: &Map<String, Value>, back: bool, take: impl FnOnce(&[u64], bool) -> Result<()>) -> Result<Option<Map<String, Value>>> {
        let from = if back { &self.done } else { &self.undone };
        let Some(at) = from.iter().rposition(|e| e.scope == scope) else { return Ok(None) };
        let mut steps = from[at].steps.clone();
        if back {
            steps.reverse();
        }
        if !steps.is_empty() {
            take(&steps, back)?;
        }
        let edit = if back { self.done.remove(at) } else { self.undone.remove(at) };
        let mut result = data.clone();
        let changes: Box<dyn Iterator<Item = &Changed>> = if back { Box::new(edit.changes.iter().rev()) } else { Box::new(edit.changes.iter()) };
        for c in changes {
            match if back { &c.before } else { &c.after } {
                Some(value) => {
                    result.insert(c.name.clone(), value.clone());
                }
                None => {
                    result.remove(&c.name);
                }
            }
        }
        if back { self.undone.push(edit) } else { self.done.push(edit) }
        self.baseline = Some(result.clone());
        Ok(Some(result))
    }
}

/// What changed from `before` to `after`, by name.
fn between(before: &Map<String, Value>, after: &Map<String, Value>) -> Vec<Changed> {
    let mut names: Vec<&String> = before.keys().chain(after.keys()).collect();
    names.sort();
    names.dedup();
    names
        .into_iter()
        .filter(|name| before.get(*name) != after.get(*name))
        .map(|name| Changed { name: name.clone(), before: before.get(name).cloned(), after: after.get(name).cloned() })
        .collect()
}

/// The edit bus of each open window, by its number.
fn buses() -> std::sync::MutexGuard<'static, HashMap<u64, EditBus>> {
    static BUSES: OnceLock<Mutex<HashMap<u64, EditBus>>> = OnceLock::new();
    BUSES.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A part's own text or tick among the data an edit compares: `#parts.<name>`.
const PART_STATE: &str = "#parts.";

/// What an edit of a window compares: its data, and each part's own text and tick.
pub fn edit_data(state: &WindowState) -> Map<String, Value> {
    let mut data = state.data.clone();
    for (name, part) in &state.parts {
        let mut own = Map::new();
        if let Some(text) = &part.text {
            own.insert("text".into(), Value::String(text.clone()));
        }
        if let Some(checked) = part.checked {
            own.insert("checked".into(), Value::Bool(checked));
        }
        if !own.is_empty() {
            data.insert(format!("{PART_STATE}{name}"), Value::Object(own));
        }
    }
    data
}

/// `state` with `data` as an edit put it back: its data, and each part's own text and tick.
pub fn with_edit_data(mut state: WindowState, data: Map<String, Value>) -> WindowState {
    state.data = Map::new();
    for part in state.parts.values_mut() {
        part.text = None;
        part.checked = None;
    }
    for (name, value) in data {
        match name.strip_prefix(PART_STATE) {
            None => {
                state.data.insert(name, value);
            }
            Some(part) => {
                let kept = state.parts.entry(part.to_string()).or_default();
                kept.text = value.get("text").and_then(Value::as_str).map(str::to_string);
                kept.checked = value.get("checked").and_then(Value::as_bool);
            }
        }
    }
    state
}

/// A window's reply is where its edits start: what a handler changed is never
/// the person's edit. Called with every reply to the screen.
pub fn replied(reply: &Reply) {
    buses().entry(reply.state.instance).or_default().reset(&edit_data(&reply.state));
}

/// Forgets the edits of a window that closed.
pub fn closed(instance: u64) {
    buses().remove(&instance);
}

/// The person ended an edit of `part` (`ends`; a drag goes on to its drop):
/// `run` runs its handler, if it has one, as one database step, and the edit
/// goes on the window's record with what it changed.
pub fn edit(db: &Db, state: WindowState, part: &str, scope: &str, ends: bool, run: impl FnOnce(WindowState) -> Result<Reply>) -> Result<Reply> {
    let instance = state.instance;
    let reply = crate::stepped(db, || run(state))?;
    let steps: Vec<u64> = if reply.step > 0 { vec![reply.step] } else { vec![] };
    let mut all = buses();
    let bus = all.entry(instance).or_default();
    if ends {
        bus.edit(part, scope, &edit_data(&reply.state), &steps);
    } else {
        bus.drag(part, scope, &steps);
    }
    Ok(reply)
}

/// The end of a message that changed the window's data (DESIGN 1.7): with a
/// scope, `run` runs as an edit that ends there; without one, as
/// `dispatch_event` runs a handler, as one database step.
pub fn ended(db: &Db, state: WindowState, part: &str, scope: Option<&str>, run: impl FnOnce(WindowState) -> Result<Reply>) -> Result<Reply> {
    match scope.filter(|s| !s.is_empty()) {
        Some(scope) => edit(db, state, part, scope, true, run),
        None => crate::stepped(db, || run(state)),
    }
}

/// Whether a table's edit, its list `before` and `after`, kept to its
/// `editable` fields: the same rows in the same order, every other field as
/// it was (one added or taken away is changed).
pub fn table_edit_fits(before: &Value, after: &Value, editable: &[String]) -> bool {
    let (Some(was), Some(now)) = (before.as_array(), after.as_array()) else { return before == after };
    was.len() == now.len()
        && was.iter().zip(now).all(|(a, b)| match (a.as_object(), b.as_object()) {
            (Some(x), Some(y)) => x.keys().chain(y.keys()).all(|field| editable.contains(field) || x.get(field) == y.get(field)),
            _ => a == b,
        })
}

/// A part's options as the runtime's parameters (DESIGN 1.7): a `readonly`
/// part's change, or a table's change (its list at `path`) outside the
/// fields of `editable`, is taken back to the window's edit baseline and
/// answered `refused` (`draw`), with no handler run and no edit kept. Any
/// other change goes on (`go`). The table is checked only when the baseline
/// has its list.
pub fn guarded(
    state: WindowState,
    readonly: bool,
    path: Option<&str>,
    editable: Option<&[String]>,
    draw: impl FnOnce(WindowState) -> Result<Reply>,
    go: impl FnOnce(WindowState) -> Result<Reply>,
) -> Result<Reply> {
    let baseline = buses().get(&state.instance).and_then(|bus| bus.baseline().cloned());
    let outside = match (path.filter(|p| !p.is_empty()), editable, &baseline) {
        (Some(path), Some(editable), Some(baseline)) => match crate::ui::data_at(baseline, path) {
            Some(before) => !table_edit_fits(before, crate::ui::data_at(&state.data, path).unwrap_or(&Value::Null), editable),
            None => false,
        },
        _ => false,
    };
    if !readonly && !outside {
        return go(state);
    }
    let state = match baseline {
        Some(data) => with_edit_data(state, data),
        None => state,
    };
    let mut reply = draw(state)?;
    reply.refused = true;
    Ok(reply)
}

/// Ctrl+Z (`back`) or Ctrl+Y on `scope`: its last edit taken back or done
/// again, with its database steps; `draw` answers with the window as it is then.
pub fn undo(db: &Db, state: WindowState, scope: &str, back: bool, draw: impl FnOnce(WindowState) -> Result<Reply>) -> Result<Reply> {
    let data = edit_data(&state);
    let after = buses().entry(state.instance).or_default().undo(scope, &data, back, |steps, back| {
        for step in steps {
            if back { db.undo(*step)? } else { db.redo(*step)? };
        }
        Ok(())
    })?;
    match after {
        Some(data) => draw(with_edit_data(state, data)),
        None => draw(state),
    }
}
