//! Every window that opens has a number of its own, and what belongs to one
//! open window (its edits, an engine's state per window) goes when it closes.

use clarity_runtime::ui::{Reply, Ui, WindowState};
use serde_json::{Map, json};

#[test]
fn every_open_window_has_its_own_number() {
    let (first, second) = (WindowState::new("A", Map::new()), WindowState::new("A", Map::new()));
    assert!(first.instance > 0 && second.instance > 0 && first.instance != second.instance);
}

#[test]
fn a_closed_window_forgets_its_edits() {
    let db = clarity_runtime::Db::memory().unwrap();
    let reply = |state: &WindowState| -> Reply { Ui::from_state(state).reply(&state.window, state.data.clone()) };
    let mut state = WindowState::new("Notes", json!({ "text": "a" }).as_object().unwrap().clone());
    clarity_runtime::edits::replied(&reply(&state));
    state.data["text"] = json!("ab");
    clarity_runtime::edits::edit(&db, state.clone(), "textBox", "textBox", true, |s| Ok(reply(&s))).unwrap();
    let back = clarity_runtime::edits::undo(&db, state.clone(), "textBox", true, |s| Ok(reply(&s))).unwrap();
    assert_eq!(back.state.data["text"], json!("a"));
    // Closed, the window has nothing left to take back.
    clarity_runtime::edits::closed(state.instance);
    let after = clarity_runtime::edits::undo(&db, state.clone(), "textBox", true, |s| Ok(reply(&s))).unwrap();
    assert_eq!(after.state.data["text"], json!("ab"));
}
