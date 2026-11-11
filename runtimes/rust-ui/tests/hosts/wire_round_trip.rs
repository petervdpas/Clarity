//! The wire both ways (round B): what Clarity.UI's `program_wire` sends for
//! each call of the program, the runtime's `wire::answer` reads back into the
//! same call on a Rust web server's program, and its answer reads back as
//! the program gave it.

use std::sync::{Arc, Mutex};

use clarity_runtime::canvas::CanvasArgs;
use clarity_runtime::list_fields::ItemKind;
use clarity_runtime::program::{Answer, CanvasEdit, Edit, ItemEdit, Program, RangeEdit, Reorder, StepPress, Started, Unconnected};
use clarity_runtime::ranges::{Range, RangeKey, RangeMove};
use clarity_runtime::reorder::{DropAt, Move};
use clarity_runtime::screen::{Drawn, Hears, ScreenInput, ScreenTell};
use clarity_runtime::steps::StepButton;
use clarity_runtime::ui::{RefOption, Reply, Text, WindowState};
use clarity_ui::program_wire::{Asks, WireProgram};
use serde_json::{json, Map, Value};

/// The program on the server: it notes each call as Debug text and answers plainly.
#[derive(Default)]
struct Noted(Mutex<Vec<String>>);

impl Noted {
    fn note(&self, call: String) -> Answer<Reply> {
        self.0.lock().unwrap().push(call);
        Ok(super::core_windows_fake::reply(WindowState::new("Main", Map::new())))
    }

    fn last(&self) -> String {
        self.0.lock().unwrap().last().cloned().unwrap_or_default()
    }
}

impl Program for Noted {
    fn started(&self) -> Started {
        Started { main: "Shop".into(), unconnected: Some(Unconnected { handler: Some("offline".into()), window: None, problem: Text::plain("no database") }), signin: Some("SignIn".into()) }
    }
    fn start_app(&self) -> Answer<Reply> {
        self.note("start_app".into())
    }
    fn open_window(&self, window: &str, with: Map<String, Value>) -> Answer<Reply> {
        self.note(format!("open_window {window} {with:?}"))
    }
    fn close_window(&self, state: WindowState) -> Answer<Reply> {
        self.note(format!("close_window {state:?}"))
    }
    fn return_to(&self, state: WindowState) -> Answer<Reply> {
        self.note(format!("return_to {state:?}"))
    }
    fn dispatch_event(&self, state: WindowState, handler: &str) -> Answer<Reply> {
        self.note(format!("dispatch_event {state:?} {handler}"))
    }
    fn edit_event(&self, state: WindowState, edit: Edit) -> Answer<Reply> {
        self.note(format!("edit_event {state:?} {edit:?}"))
    }
    fn undo_edit(&self, state: WindowState, scope: &str, back: bool) -> Answer<Reply> {
        self.note(format!("undo_edit {state:?} {scope} {back}"))
    }
    fn reorder_event(&self, state: WindowState, reorder: Reorder) -> Answer<Reply> {
        self.note(format!("reorder_event {state:?} {reorder:?}"))
    }
    fn item_event(&self, state: WindowState, item: ItemEdit) -> Answer<Reply> {
        self.note(format!("item_event {state:?} {item:?}"))
    }
    fn range_event(&self, state: WindowState, range: RangeEdit) -> Answer<Reply> {
        self.note(format!("range_event {state:?} {range:?}"))
    }
    fn canvas_event(&self, state: WindowState, canvas: CanvasEdit) -> Answer<Reply> {
        self.note(format!("canvas_event {state:?} {canvas:?}"))
    }
    fn step_event(&self, state: WindowState, step: StepPress) -> Answer<Reply> {
        self.note(format!("step_event {state:?} {step:?}"))
    }
    fn view_event(&self, state: WindowState, part: &str, action: Map<String, Value>) -> Answer<Reply> {
        self.note(format!("view_event {state:?} {part} {action:?}"))
    }
    fn answer_event(&self, state: WindowState, handler: &str, values: Vec<Value>) -> Answer<Reply> {
        self.note(format!("answer_event {state:?} {handler} {values:?}"))
    }
    fn ref_options(&self, entity: &str) -> Answer<Vec<RefOption>> {
        Err(Text::plain(format!("no {entity}")))
    }
    fn field_values(&self, entity: &str, field: &str) -> Answer<Vec<String>> {
        Ok(vec![entity.into(), field.into()])
    }
    fn find_options(&self, kind: &str, entity: &str, field: Option<&str>, show: &[String], typed: &str) -> Answer<Value> {
        Ok(json!([kind, entity, field, show, typed]))
    }
    fn field_bytes(&self, _entity: &str, id: i64, _field: &str) -> Answer<Option<Vec<u8>>> {
        Ok((id > 0).then(|| vec![0, 1, 2, 255]))
    }
    fn store_file(&self, _store: &str, _path: &str) -> Answer<Vec<u8>> {
        Ok(b"PNG".to_vec())
    }
    fn read_settings(&self) -> Map<String, Value> {
        Map::new()
    }
    fn keep_settings(&self, settings: Map<String, Value>) -> Answer<()> {
        self.0.lock().unwrap().push(format!("keep_settings {settings:?}"));
        Ok(())
    }
    fn facts_json(&self) -> Value {
        json!({ "fields": { "price": { "label": "Price" } } })
    }
    fn signin(&self, strategy: &str, inputs: Map<String, Value>) -> Answer<()> {
        self.0.lock().unwrap().push(format!("signin {strategy} {inputs:?}"));
        if inputs.get("password") == Some(&json!("right")) { Ok(()) } else { Err(Text::keyed("signinRefused", "Signing in did not work.")) }
    }
    fn signout(&self) -> Answer<()> {
        self.0.lock().unwrap().push("signout".into());
        Ok(())
    }
    fn watches(&self) -> bool {
        true
    }
    fn start_watch(&self, id: u64) {
        self.0.lock().unwrap().push(format!("start_watch {id}"));
    }
    fn end_watch(&self, id: u64) {
        self.0.lock().unwrap().push(format!("end_watch {id}"));
    }
    fn watch_progress(&self, id: u64) -> Option<(i64, i64)> {
        Some((id as i64, 10))
    }
    fn engine_open(&self, name: &str) -> Option<Result<u64, String>> {
        match name {
            "sky" => Some(Ok(7)),
            "broken" => Some(Err("the engine does not load".into())),
            _ => None,
        }
    }
    fn engine_hears(&self, _id: u64) -> Hears {
        Hears { pointer: true, keys: false }
    }
    fn engine_frame(&self, id: u64, ms: f64, input: Vec<ScreenInput>) -> Result<Drawn, String> {
        self.0.lock().unwrap().push(format!("engine_frame {id} {ms} {input:?}"));
        Ok(Drawn { shapes: vec![json!({ "rect": [0, 0, 4, 4] })], tells: vec![ScreenTell { tell: "hit".into(), values: vec![json!(3)] }] })
    }
}

/// The browser's side: each command goes through the JSON text a request carries, to `wire::answer`.
struct Through(Arc<Noted>);

impl Asks for Through {
    fn ask(&self, command: &str, args: Map<String, Value>) -> Option<Value> {
        let sent: Value = serde_json::from_str(&json!({ "command": command, "args": args }).to_string()).unwrap();
        let answered = clarity_runtime::wire::answer(&*self.0, sent["command"].as_str().unwrap(), sent["args"].clone());
        Some(serde_json::from_str(&answered.to_string()).unwrap())
    }
}

fn state() -> WindowState {
    let mut data = Map::new();
    data.insert("price".into(), json!("12.50"));
    WindowState::new("Main", data)
}

#[test]
fn every_call_crosses_the_wire_and_arrives_as_it_was_made() {
    let noted = Arc::new(Noted::default());
    let wire = WireProgram::new(Through(noted.clone()));
    let at = state();

    let started = wire.started();
    assert_eq!(started.main, "Shop");
    assert_eq!(started.unconnected.as_ref().and_then(|u| u.handler.clone()).as_deref(), Some("offline"));
    assert_eq!(started.unconnected.unwrap().problem, Text::plain("no database"));
    assert_eq!(started.signin.as_deref(), Some("SignIn"));

    // Signing in and out, the inputs as the sign-in window's parts had them.
    let mut inputs = Map::new();
    inputs.insert("name".into(), json!("ann"));
    inputs.insert("password".into(), json!("wrong"));
    assert_eq!(wire.signin("password", inputs.clone()).unwrap_err().key.as_deref(), Some("signinRefused"));
    assert_eq!(noted.last(), format!("signin password {inputs:?}"));
    inputs.insert("password".into(), json!("right"));
    wire.signin("password", inputs).unwrap();
    wire.signout().unwrap();
    assert_eq!(noted.last(), "signout");

    let mut with = Map::new();
    with.insert("id".into(), json!(4));
    wire.open_window("Edit", with.clone()).unwrap();
    assert_eq!(noted.last(), format!("open_window Edit {with:?}"));
    wire.dispatch_event(at.clone(), "save").unwrap();
    assert_eq!(noted.last(), format!("dispatch_event {at:?} save"));

    let edit = Edit { handler: Some("changed".into()), part: "price".into(), scope: "form".into(), ends: true, readonly: false, path: Some("lines".into()), editable: Some(vec!["qty".into()]) };
    wire.edit_event(at.clone(), edit.clone()).unwrap();
    assert_eq!(noted.last(), format!("edit_event {at:?} {edit:?}"));
    wire.undo_edit(at.clone(), "form", true).unwrap();
    assert_eq!(noted.last(), format!("undo_edit {at:?} form true"));

    let reorder = Reorder { part: "list".into(), scope: Some("s".into()), list: "rows".into(), by: "rank".into(), parent: None, moved: Move::Drag { row: 2, at: DropAt::Before, target: 0 }, handler: None, readonly: false };
    wire.reorder_event(at.clone(), reorder.clone()).unwrap();
    assert_eq!(noted.last(), format!("reorder_event {at:?} {reorder:?}"));

    let item = ItemEdit { part: "tags".into(), scope: None, path: "tags".into(), item: ItemKind::Enum, values: vec!["a".into(), "b".into()], typed: "a".into(), handler: None, readonly: false };
    wire.item_event(at.clone(), item.clone()).unwrap();
    assert_eq!(noted.last(), format!("item_event {at:?} {item:?}"));

    for moved in [RangeMove::Key(RangeKey::PageUp), RangeMove::At("3.5".into()), RangeMove::Typed("7".into())] {
        let range = RangeEdit { part: "volume".into(), scope: None, path: "volume".into(), range: Range { min: Some("0".into()), max: None, step: "0.5".into() }, whole: false, moved, handler: Some("louder".into()), readonly: false };
        wire.range_event(at.clone(), range.clone()).unwrap();
        assert_eq!(noted.last(), format!("range_event {at:?} {range:?}"));
    }

    let mut action = Map::new();
    action.insert("kind".into(), json!("drop"));
    let canvas = CanvasEdit {
        canvas: CanvasArgs { part: "board".into(), mode: "objects".into(), list: "pieces".into(), at: Some(["x".into(), "y".into()]), path: String::new(), units: Some([800, 600]), grid: 10, snap: true, readonly: false },
        action: action.clone(),
        scope: None,
        handler: Some("moved".into()),
        readonly: false,
    };
    wire.canvas_event(at.clone(), canvas.clone()).unwrap();
    assert_eq!(noted.last(), format!("canvas_event {at:?} {canvas:?}"));

    let step = StepPress { part: "wizard".into(), button: StepButton::Next, steps: 3, next: Some("check".into()), finish: None };
    wire.step_event(at.clone(), step.clone()).unwrap();
    assert_eq!(noted.last(), format!("step_event {at:?} {step:?}"));
    wire.view_event(at.clone(), "table", action.clone()).unwrap();
    assert_eq!(noted.last(), format!("view_event {at:?} table {action:?}"));
    wire.answer_event(at.clone(), "asked", vec![json!(1), json!("yes")]).unwrap();
    assert_eq!(noted.last(), format!("answer_event {at:?} asked {:?}", vec![json!(1), json!("yes")]));

    // Answers read back as the program gave them, errors as their text.
    assert_eq!(wire.ref_options("Friend").unwrap_err(), Text::plain("no Friend"));
    assert_eq!(wire.field_values("Album", "title").unwrap(), ["Album", "title"]);
    assert_eq!(wire.find_options("search", "Album", None, &["title".into()], "ab").unwrap(), json!(["search", "Album", null, ["title"], "ab"]));
    assert_eq!(wire.field_bytes("Album", 1, "cover").unwrap(), Some(vec![0, 1, 2, 255]));
    assert_eq!(wire.field_bytes("Album", 0, "cover").unwrap(), None);
    assert_eq!(wire.store_file("pictures", "a.png").unwrap(), b"PNG");
    let mut kept = Map::new();
    kept.insert("theme".into(), json!("dark"));
    wire.keep_settings(kept.clone()).unwrap();
    assert_eq!(noted.last(), format!("keep_settings {kept:?}"));
    assert_eq!(wire.facts_json(), json!({ "fields": { "price": { "label": "Price" } } }));
    assert!(wire.watches());
    assert_eq!(wire.watch_progress(3), Some((3, 10)));

    assert_eq!(wire.engine_open("sky"), Some(Ok(7)));
    assert_eq!(wire.engine_open("broken"), Some(Err("the engine does not load".into())));
    assert_eq!(wire.engine_open("none"), None);
    assert_eq!(wire.engine_hears(7), Hears { pointer: true, keys: false });
    let input = vec![ScreenInput::Pointer { kind: "down".into(), x: 3, y: 4 }, ScreenInput::Key { kind: "up".into(), name: "Space".into() }, ScreenInput::Hear { name: "start".into(), values: vec![json!(2)] }];
    let frame = wire.engine_frame(7, 16.5, input.clone()).unwrap();
    assert_eq!(noted.last(), format!("engine_frame 7 16.5 {input:?}"));
    assert_eq!(frame, Drawn { shapes: vec![json!({ "rect": [0, 0, 4, 4] })], tells: vec![ScreenTell { tell: "hit".into(), values: vec![json!(3)] }] });
}

/// A command the program does not know is an error, and a panic in a call
/// is the error the screen shows, after which the server goes on.
#[test]
fn an_unknown_command_and_a_panic_are_errors() {
    struct Panics;
    impl Program for Panics {
        fn started(&self) -> Started {
            panic!("the started call fell over")
        }
        fn start_app(&self) -> Answer<Reply> {
            unreachable!()
        }
        fn open_window(&self, _: &str, _: Map<String, Value>) -> Answer<Reply> {
            unreachable!()
        }
        fn close_window(&self, _: WindowState) -> Answer<Reply> {
            unreachable!()
        }
        fn return_to(&self, _: WindowState) -> Answer<Reply> {
            unreachable!()
        }
        fn dispatch_event(&self, _: WindowState, _: &str) -> Answer<Reply> {
            unreachable!()
        }
        fn edit_event(&self, _: WindowState, _: Edit) -> Answer<Reply> {
            unreachable!()
        }
        fn undo_edit(&self, _: WindowState, _: &str, _: bool) -> Answer<Reply> {
            unreachable!()
        }
        fn reorder_event(&self, _: WindowState, _: Reorder) -> Answer<Reply> {
            unreachable!()
        }
        fn item_event(&self, _: WindowState, _: ItemEdit) -> Answer<Reply> {
            unreachable!()
        }
        fn range_event(&self, _: WindowState, _: RangeEdit) -> Answer<Reply> {
            unreachable!()
        }
        fn canvas_event(&self, _: WindowState, _: CanvasEdit) -> Answer<Reply> {
            unreachable!()
        }
        fn step_event(&self, _: WindowState, _: StepPress) -> Answer<Reply> {
            unreachable!()
        }
        fn view_event(&self, _: WindowState, _: &str, _: Map<String, Value>) -> Answer<Reply> {
            unreachable!()
        }
        fn answer_event(&self, _: WindowState, _: &str, _: Vec<Value>) -> Answer<Reply> {
            unreachable!()
        }
        fn ref_options(&self, _: &str) -> Answer<Vec<RefOption>> {
            unreachable!()
        }
        fn field_values(&self, _: &str, _: &str) -> Answer<Vec<String>> {
            unreachable!()
        }
        fn find_options(&self, _: &str, _: &str, _: Option<&str>, _: &[String], _: &str) -> Answer<Value> {
            unreachable!()
        }
        fn field_bytes(&self, _: &str, _: i64, _: &str) -> Answer<Option<Vec<u8>>> {
            unreachable!()
        }
        fn store_file(&self, _: &str, _: &str) -> Answer<Vec<u8>> {
            unreachable!()
        }
        fn read_settings(&self) -> Map<String, Value> {
            Map::new()
        }
        fn keep_settings(&self, _: Map<String, Value>) -> Answer<()> {
            Ok(())
        }
    }
    assert_eq!(clarity_runtime::wire::answer(&Panics, "nonsense", json!({})), json!({ "error": "there is no command nonsense" }));
    let failed = clarity_runtime::wire::answer(&Panics, "started", json!({}));
    assert!(failed["error"].to_string().contains("the started call fell over"), "{failed}");
    // A wrong argument is an error naming it, not a panic.
    let wrong = clarity_runtime::wire::answer(&Panics, "dispatch_event", json!({ "state": 3 }));
    assert!(wrong["error"].to_string().contains("state"), "{wrong}");
}

/// A handler's request that names a watch (round B: the browser sends no
/// start_watch) runs under it on its own thread and ends it after, also
/// when the handler fails; one without runs unwatched.
#[test]
fn a_handler_runs_under_the_watch_its_request_names() {
    let noted = Noted::default();
    let at = state();
    let answered = clarity_runtime::wire::answer(&noted, "dispatch_event", json!({ "state": at, "handler": "plan", "watch": 4 }));
    assert!(answered["value"].is_object(), "{answered}");
    let notes = noted.0.lock().unwrap().clone();
    assert_eq!(notes, ["start_watch 4".to_string(), format!("dispatch_event {at:?} plan"), "end_watch 4".to_string()]);
    noted.0.lock().unwrap().clear();
    clarity_runtime::wire::answer(&noted, "answer_event", json!({ "state": at, "handler": "asked", "values": [1], "watch": 5 }));
    clarity_runtime::wire::answer(&noted, "dispatch_event", json!({ "state": at, "handler": "plain" }));
    let notes = noted.0.lock().unwrap().clone();
    assert_eq!(notes.iter().filter(|n| n.contains("_watch")).cloned().collect::<Vec<_>>(), ["start_watch 5", "end_watch 5"]);
    assert_eq!(clarity_runtime::wire::answer(&noted, "watches", json!({})), json!({ "value": true }));
}
