//! A part bound to a setting (`settings.showToolbar`) keeps it: the program's
//! `keep_settings` hears every setting the window holds over what was kept
//! (the person's keys among them), as the web screen and C# keep them.

use super::core_windows_fake;

use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_runtime::ui::WindowState;
use clarity_ui::window::ProgramHost;
use clarity_ui::PartHost;
use core_windows_fake::*;
use serde_json::{Map, json};

#[test]
fn a_setting_written_by_a_part_is_kept() {
    let fake = Arc::new(Fake::default());
    fake.settings.lock().unwrap().insert("keys".into(), json!({ "Main.save": ["Ctrl+K"] }));
    let program: Arc<dyn Program> = fake.clone();
    let mut data = Map::new();
    data.insert("settings".into(), json!({ "id": 0, "showToolbar": true, "apiToken": "" }));
    let host = ProgramHost::new(program, FILES, WindowState::new("Main", data), &[]);
    fake.calls();
    host.write("toolbarSwitch", "settings.showToolbar", json!(false));
    let calls = fake.calls();
    let kept: Vec<&String> = calls.iter().filter(|c| c.starts_with("keep ")).collect();
    assert_eq!(kept.len(), 1, "{calls:?}");
    let settings = fake.settings.lock().unwrap().clone();
    assert_eq!(settings["showToolbar"], json!(false));
    assert_eq!(settings["keys"], json!({ "Main.save": ["Ctrl+K"] }));
    // The settings record's own number is not kept.
    assert!(!settings.contains_key("id"), "{settings:?}");
    // A value that is no setting keeps nothing.
    host.write("titleBox", "title", json!("x"));
    assert!(!fake.calls().iter().any(|c| c.starts_with("keep ")));
}

/// A handler that changed a setting keeps it, as a part's write does: the
/// program's `keep_settings` hears it over what was kept, whatever language
/// the program is written in. A reply that changed none keeps nothing.
#[test]
fn a_setting_changed_by_a_handler_is_kept() {
    let fake = Arc::new(Fake::default());
    fake.settings.lock().unwrap().insert("keys".into(), json!({ "Main.save": ["Ctrl+K"] }));
    let program: Arc<dyn Program> = fake.clone();
    let mut data = Map::new();
    data.insert("settings".into(), json!({ "id": 0, "showDone": true }));
    let host = ProgramHost::new(program, FILES, WindowState::new("Main", data.clone()), &[]);
    fake.calls();
    // The same settings back: nothing is kept.
    let mut same = WindowState::new("Main", data.clone());
    same.data.insert("title".into(), json!("x"));
    assert!(host.answered(Ok(reply(same))));
    assert!(!fake.calls().iter().any(|c| c.starts_with("keep ")));
    let mut flipped = WindowState::new("Main", data);
    flipped.data.insert("settings".into(), json!({ "id": 0, "showDone": false }));
    assert!(host.answered(Ok(reply(flipped))));
    let calls = fake.calls();
    assert_eq!(calls.iter().filter(|c| c.starts_with("keep ")).count(), 1, "{calls:?}");
    let settings = fake.settings.lock().unwrap().clone();
    assert_eq!(settings["showDone"], json!(false));
    assert_eq!(settings["keys"], json!({ "Main.save": ["Ctrl+K"] }));
    assert!(!settings.contains_key("id"), "{settings:?}");
}
