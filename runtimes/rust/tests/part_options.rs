//! A part's options are the runtime's parameters (DESIGN 1.7): a readonly
//! part's change, and a table's change outside the fields its cells edit,
//! are taken back to the window's edit baseline and answered refused, with
//! no handler run and no edit kept.

use clarity_runtime::edits;
use clarity_runtime::ui::{Reply, Ui, WindowState};
use clarity_runtime::{Db, Result};
use serde_json::{Map, Value, json};

fn drawn(state: WindowState) -> Result<Reply> {
    let data = state.data.clone();
    Ok(Ui::from_state(&state).reply(&state.window, data))
}

fn window(rows: Value) -> WindowState {
    let mut data = Map::new();
    data.insert("note".into(), json!("kept"));
    data.insert("rows".into(), rows);
    WindowState::new("Grid", data)
}

fn rows() -> Value {
    json!([{ "id": 1, "title": "A", "hours": 2 }, { "id": 2, "title": "B", "hours": 3 }])
}

/// The window as the screen sends it after the person changed it, its
/// reply before that being where its edits start.
fn changed(rows_after: Value) -> WindowState {
    let before = window(rows());
    edits::replied(&drawn(before.clone()).unwrap());
    let mut after = before;
    after.data.insert("rows".into(), rows_after);
    after.data.insert("note".into(), json!("typed"));
    after
}

#[test]
fn a_readonly_part_takes_the_change_back_and_runs_nothing() {
    let db = Db::memory().unwrap();
    let state = changed(json!([]));
    let reply = edits::guarded(state, true, None, None, drawn, |_| panic!("no handler runs on a readonly part")).unwrap();
    assert!(reply.refused);
    assert_eq!(reply.state.data["rows"], rows(), "the data as the baseline had it");
    assert_eq!(reply.state.data["note"], json!("kept"), "the whole baseline");
    // Nothing went on the record: there is nothing to take back.
    let back = edits::undo(&db, reply.state.clone(), "grid", true, drawn).unwrap();
    assert_eq!(back.state.data, reply.state.data);
}

#[test]
fn a_part_that_is_not_readonly_goes_on() {
    let state = changed(json!([]));
    let reply = edits::guarded(state, false, None, None, drawn, drawn).unwrap();
    assert!(!reply.refused);
    assert_eq!(reply.state.data["rows"], json!([]));
}

#[test]
fn a_table_edit_keeps_to_its_editable_fields() {
    let editable = ["hours".to_string()];
    let fits = |after: Value| edits::table_edit_fits(&rows(), &after, &editable);
    assert!(fits(json!([{ "id": 1, "title": "A", "hours": 5 }, { "id": 2, "title": "B", "hours": 3 }])), "an editable field");
    assert!(!fits(json!([{ "id": 1, "title": "Z", "hours": 2 }, { "id": 2, "title": "B", "hours": 3 }])), "another field");
    assert!(!fits(json!([{ "id": 2, "title": "B", "hours": 3 }, { "id": 1, "title": "A", "hours": 2 }])), "another order");
    assert!(!fits(json!([{ "id": 1, "title": "A", "hours": 2 }])), "a row fewer");
    assert!(!fits(json!([{ "id": 1, "title": "A", "hours": 2, "extra": 1 }, { "id": 2, "title": "B", "hours": 3 }])), "a new field");
    assert!(!fits(json!([{ "id": 1, "hours": 2 }, { "id": 2, "title": "B", "hours": 3 }])), "a removed field that is not editable");
    assert!(fits(json!([{ "id": 1, "title": "A" }, { "id": 2, "title": "B", "hours": 3 }])), "an editable field taken away");
}

#[test]
fn a_change_outside_the_editable_fields_is_refused() {
    let editable = vec!["hours".to_string()];
    let state = changed(json!([{ "id": 1, "title": "Z", "hours": 2 }, { "id": 2, "title": "B", "hours": 3 }]));
    let reply = edits::guarded(state, false, Some("rows"), Some(&editable), drawn, |_| panic!("refused")).unwrap();
    assert!(reply.refused);
    assert_eq!(reply.state.data["rows"], rows());
    let state = changed(json!([{ "id": 1, "title": "A", "hours": 9 }, { "id": 2, "title": "B", "hours": 3 }]));
    let reply = edits::guarded(state, false, Some("rows"), Some(&editable), drawn, drawn).unwrap();
    assert!(!reply.refused);
    assert_eq!(reply.state.data["rows"][0]["hours"], json!(9));
}

#[test]
fn without_the_list_in_the_baseline_nothing_is_checked() {
    let editable = vec!["hours".to_string()];
    let state = changed(json!([{ "id": 1, "title": "Z" }]));
    let reply = edits::guarded(state, false, Some("elsewhere"), Some(&editable), drawn, drawn).unwrap();
    assert!(!reply.refused);
    // A window that has had no reply has no baseline.
    let fresh = window(json!([{ "id": 1, "title": "Z" }]));
    let reply = edits::guarded(fresh, false, Some("rows"), Some(&editable), drawn, drawn).unwrap();
    assert!(!reply.refused);
}
