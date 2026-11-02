//! A program that watches its long calls runs a click's handler off the
//! screen's thread; every other event runs in place: a wizard's Next, Back
//! and Finish, a form's submit (so a refusal keeps the wizard on its step),
//! a timer's tick (so it does not bring back an older window).

use super::core_windows_fake;

use std::rc::Rc;
use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_runtime::ui::WindowState;
use clarity_ui::markup::HandlerCall;
use clarity_ui::window::{ProgramHost, Running};
use clarity_ui::PartHost;
use core_windows_fake::*;
use serde_json::Map;

#[test]
fn a_refusal_is_heard_by_the_part_that_asked_even_when_long_calls_are_watched() {
    let fake = Arc::new(Fake { watches: true, ..Fake::default() });
    let program: Arc<dyn Program> = fake.clone();
    let host = Rc::new(ProgramHost::new(program.clone(), FILES, WindowState::new("Main", Map::new()), &[]));
    host.set_running(Running::new(program, FILES));
    for event in ["next", "back", "finish", "submit", "tick"] {
        assert!(!host.run(Some("wizard"), &HandlerCall::new(event, "refuse", Vec::new())), "{event}");
    }
}
