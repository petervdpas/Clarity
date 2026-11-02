// The program a generated application is (DESIGN 7.9): what its screen asks
// of it, in plain Rust types. The webview's commands and Clarity.UI both call
// it; what goes wrong is text the screen shows, the same on both.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clarity_runtime::program::{self, Answer, Program, Started, Watch};
use clarity_runtime::ui::{RefOption, Reply, Text, Ui, WindowState};
use clarity_runtime::{Db, Error};
use serde_json::{Map, Value};

/// A program with one window and no handlers, as small as a generated one gets.
struct Small(Mutex<Db>);

impl Small {
    fn drawn(&self, state: WindowState) -> Answer<Reply> {
        program::guarded(&self.0, |_| {
            let data = state.data.clone();
            Ok(Ui::from_state(&state).reply(&state.window, data))
        })
    }
}

impl Program for Small {
    fn started(&self) -> Started {
        Started { main: "Main".into(), unconnected: None }
    }
    fn start_app(&self) -> Answer<Reply> {
        self.drawn(WindowState::new("", Map::new()))
    }
    fn open_window(&self, window: &str, with: Map<String, Value>) -> Answer<Reply> {
        self.drawn(WindowState::new(window, with))
    }
    fn close_window(&self, state: WindowState) -> Answer<Reply> {
        self.drawn(state)
    }
    fn return_to(&self, state: WindowState) -> Answer<Reply> {
        self.drawn(state)
    }
    fn dispatch_event(&self, state: WindowState, _handler: &str) -> Answer<Reply> {
        self.drawn(state)
    }
    fn edit_event(&self, state: WindowState, _edit: program::Edit) -> Answer<Reply> {
        self.drawn(state)
    }
    fn undo_edit(&self, state: WindowState, _scope: &str, _back: bool) -> Answer<Reply> {
        self.drawn(state)
    }
    fn reorder_event(&self, state: WindowState, _reorder: program::Reorder) -> Answer<Reply> {
        self.drawn(state)
    }
    fn item_event(&self, state: WindowState, _item: program::ItemEdit) -> Answer<Reply> {
        self.drawn(state)
    }
    fn range_event(&self, state: WindowState, _range: program::RangeEdit) -> Answer<Reply> {
        self.drawn(state)
    }
    fn canvas_event(&self, state: WindowState, _canvas: program::CanvasEdit) -> Answer<Reply> {
        self.drawn(state)
    }
    fn step_event(&self, state: WindowState, _step: program::StepPress) -> Answer<Reply> {
        self.drawn(state)
    }
    fn view_event(&self, state: WindowState, _part: &str, _action: Map<String, Value>) -> Answer<Reply> {
        self.drawn(state)
    }
    fn answer_event(&self, state: WindowState, _handler: &str, _values: Vec<Value>) -> Answer<Reply> {
        self.drawn(state)
    }
    fn ref_options(&self, _entity: &str) -> Answer<Vec<RefOption>> {
        Ok(Vec::new())
    }
    fn field_values(&self, _entity: &str, _field: &str) -> Answer<Vec<String>> {
        Ok(vec!["one".into()])
    }
    fn find_options(&self, _kind: &str, _entity: &str, _field: Option<&str>, _show: &[String], _typed: &str) -> Answer<Value> {
        Ok(Value::Array(Vec::new()))
    }
    fn field_bytes(&self, entity: &str, _id: i64, field: &str) -> Answer<Option<Vec<u8>>> {
        Err(Text::plain(format!("{entity} has no picture {field}")))
    }
    fn store_file(&self, _store: &str, _path: &str) -> Answer<Vec<u8>> {
        Ok(Vec::new())
    }
    fn read_settings(&self) -> Map<String, Value> {
        Map::new()
    }
    fn keep_settings(&self, _settings: Map<String, Value>) -> Answer<()> {
        Ok(())
    }
}

#[test]
fn a_program_answers_its_screen_in_plain_types() {
    let small = Small(Mutex::new(Db::memory().unwrap()));
    let program: &dyn Program = &small;
    assert_eq!(program.started().main, "Main");
    let reply = program.open_window("Main", Map::new()).unwrap();
    assert_eq!(reply.state.window, "Main");
    // What a program leaves out has its default: connected, and calls not watched.
    assert!(program.connect().is_ok());
    assert!(!program.watches());
    let mut ran = false;
    program.watched(&Watch { progress: None, stop: Arc::new(AtomicBool::new(false)) }, &mut || ran = true);
    assert!(ran);
    let stop = Arc::new(AtomicBool::new(false));
    stop.store(true, Ordering::SeqCst);
    assert!(Watch { progress: None, stop }.stopped());
}

#[test]
fn what_goes_wrong_is_text_as_the_webview_had_it() {
    let db = Mutex::new(Db::memory().unwrap());
    // Any error is its text, sent as the plain string it always was.
    let refused = program::guarded(&db, |_| -> clarity_runtime::Result<()> { Err(Error::Invalid("not a {number}".into())) }).unwrap_err();
    assert_eq!(refused.show(), "invalid value: not a {number}");
    assert_eq!(serde_json::to_value(&refused).unwrap(), Value::from("invalid value: not a {number}"));
    // A refusal keeps its slots, which the screen fills.
    let said = Text::new("No {name}").with("name", clarity_runtime::ui::Slot::text("Ann"));
    let refused = program::guarded(&db, |_| -> clarity_runtime::Result<()> { Err(Error::Refused(said.clone())) }).unwrap_err();
    assert_eq!(refused, said);
    // A panic is an error too, and the database stays usable.
    std::panic::set_hook(Box::new(|_| {}));
    let failed = program::guarded(&db, |_| -> clarity_runtime::Result<i64> {
        let none: Vec<i64> = Vec::new();
        Ok(none[3])
    });
    let _ = std::panic::take_hook();
    assert!(failed.unwrap_err().show().contains("out of bounds"));
    assert_eq!(program::guarded(&db, |_| Ok(7)).unwrap(), 7);
    // The webview's guard says the same, as JSON.
    let json = clarity_runtime::run_guarded(&db, |_| -> clarity_runtime::Result<()> { Err(Error::Refused(said.clone())) }).unwrap_err();
    assert_eq!(json, serde_json::to_value(&said).unwrap());
}

#[test]
fn why_the_databases_did_not_open_goes_with_the_start() {
    let started = Started {
        main: "Main".into(),
        unconnected: Some(program::Unconnected { handler: None, window: Some("Connection".into()), problem: clarity_runtime::unconnected_reason(&Error::NotKept("password".into())) }),
    };
    let unconnected = started.unconnected.unwrap();
    assert_eq!(unconnected.window.as_deref(), Some("Connection"));
    assert_eq!(serde_json::to_value(&unconnected.problem).unwrap()["key"], "unconnectedNotKept");
}
