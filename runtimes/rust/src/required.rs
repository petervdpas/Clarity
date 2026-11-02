//! Why a save refused a record whose required field is empty, shown at the
//! input bound to that field, as a form check shows it, and not as a message
//! (conformance/required.json). The generated program names, per window, each
//! input bound to a required field; a refusal no input shows stays an error
//! the screen says.

use crate::ui::{Reply, WindowState};
use crate::{Error, Result};

/// An input bound to a required field: the part, the window's data that
/// holds the record, the record's table and the field's column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bound {
    pub part: &'static str,
    pub data: &'static str,
    pub table: &'static str,
    pub column: &'static str,
}

/// The key of the words a refused required field is said in.
const REQUIRED_KEY: &str = "fieldRequired";

/// Runs a handler (`run`) on `state`. A save it made that refused an empty
/// required field of a record the window holds shows why at each input of
/// `bound` that shows that field of that record, and the window is drawn as
/// it was (`draw`): the handler's changes are gone with its failure. What an
/// earlier refusal showed at an input is taken away first, unless code set
/// another error there since.
pub fn shown_at_input(
    mut state: WindowState,
    bound: &[Bound],
    run: impl FnOnce(WindowState) -> Result<Reply>,
    draw: impl FnOnce(WindowState) -> Result<Reply>,
) -> Result<Reply> {
    for b in bound {
        if let Some(part) = state.parts.get_mut(b.part)
            && part.error.as_ref().is_some_and(|e| e.key.as_deref() == Some(REQUIRED_KEY))
        {
            part.error = None;
        }
    }
    let before = (!bound.is_empty()).then(|| state.clone());
    let failed = match run(state) {
        Err(Error::Required { table, id, column, text }) => (table, id, column, text),
        other => return other,
    };
    let (table, id, column, text) = failed;
    let Some(mut state) = before else { return Err(Error::Required { table, id, column, text }) };
    let mut shown = false;
    for b in bound.iter().filter(|b| b.table == table && b.column == column) {
        let holds = state.data.get(b.data).and_then(|record| record.get("id")).and_then(serde_json::Value::as_i64) == Some(id);
        if holds {
            state.parts.entry(b.part.to_string()).or_default().error = Some(text.clone());
            shown = true;
        }
    }
    if !shown {
        return Err(Error::Required { table, id, column, text });
    }
    draw(state)
}
