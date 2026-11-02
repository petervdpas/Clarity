//! Escape closes a dialog that has no cancel button as its close does
//! (DESIGN 5.14: Escape is a cancel, which runs nothing): over the window,
//! and in a desktop window of its own. A window that is no dialog keeps
//! Escape for its parts.

mod core_windows_fake;

use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_ui::dialogs::DialogOverlay;
use clarity_ui::input::{Key, KeyInput};
use clarity_ui::window::{Apart, WindowStack};
use clarity_ui::{AppFiles, Screen};
use core_windows_fake::*;

const HOME: &str = r#"<Window name="Main">
  <Button name="openNote" at="1,1" span="3" caption="Note" opens="Note" />
</Window>
"#;

const NOTE: &str = r#"<Window name="Note">
  <Label name="noteText" at="1,1" span="12" caption="Saved" />
  <Button name="fine" at="1,2" span="6" caption="Fine" default="true" onclick="done" />
</Window>
"#;

const NOTE_MODEL: &str = r#"{
  "project": {
    "locale": {"language": "en"},
    "files": [{"pages": [
      {"name": {"name": "Main"}, "kind": "window", "title": "Home"},
      {"name": {"name": "Note"}, "kind": "dialog", "title": "A note"}
    ]}]
  },
  "keys": []
}"#;

const NOTE_FILES: AppFiles = AppFiles { windows: &[("Main", HOME), ("Note", NOTE)], apps: &[], themes: &[], model: NOTE_MODEL, resources: &[], title: "Home", width: 800.0, height: 500.0 };

fn started_home() -> (Screen, Arc<Fake>) {
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let stack = WindowStack::start(program, NOTE_FILES);
    let mut screen = Screen::new(Box::new(stack), NOTE_FILES.look(false));
    screen.resize(800.0, 500.0, 1.0);
    (screen, fake)
}

#[test]
fn escape_closes_a_dialog_without_a_cancel_button_as_its_close_does() {
    let (mut screen, fake) = started_home();
    click(&mut screen, "openNote");
    assert!(screen.find::<DialogOverlay>().is_some());
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(screen.find::<DialogOverlay>().is_none(), "Escape closes the dialog");
    // As its close does: its close is heard, nothing it would answer runs.
    assert_eq!(vec!["close Note", "return Main"], fake.calls());
    assert_eq!(vec!["Main"], stack(&screen).windows());
}

#[test]
fn escape_closes_a_dialog_in_a_desktop_window_of_its_own() {
    let (mut screen, fake) = started_home();
    stack(&screen).running().set_apart(true);
    click(&mut screen, "openNote");
    let aparts = screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap();
    let Some(Apart::Window(request)) = aparts.into_iter().next() else { panic!("a window of its own is asked for") };
    let apart = WindowStack::apart(stack(&screen).running().clone(), request);
    let mut own = Screen::new(Box::new(apart), NOTE_FILES.look(false));
    own.resize(480.0, 160.0, 1.0);
    fake.calls();
    own.key_pressed(KeyInput::plain(Key::Escape));
    let (ended, closed) = own.act_root::<WindowStack, _>(|s, _| (s.ended(), s.take_closed())).unwrap();
    assert!(ended, "Escape ends the dialog's window");
    let closed = closed.expect("the window that opened it is told");
    assert_eq!(None, closed.result);
    assert_eq!(vec!["close Note"], fake.calls());
}

#[test]
fn escape_in_a_window_that_is_no_dialog_closes_nothing() {
    let (mut screen, fake) = started_home();
    fake.calls();
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert_eq!(vec!["Main"], stack(&screen).windows());
    assert!(fake.calls().is_empty());
}
