//! What a window's host answers its parts beyond values and handlers, over
//! a program's `Program` (C# ViewModelHost.Routes): a custom part's tell
//! runs the handler wired to it, a canvas's pointer runs its handler (a drag
//! as one edit), a canvas with a mode sends what the person did as a canvas
//! event, a canvas that runs an engine gets the program's, and the person's
//! keys answer where they set their own.

mod core_windows_fake;

use std::rc::Rc;
use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_runtime::ui::WindowState;
use clarity_ui::markup;
use clarity_ui::window::{ProgramHost, Running};
use clarity_ui::PartHost;
use core_windows_fake::*;
use serde_json::{json, Map, Value};

const PARTS: &str = r#"<Window name="Main">
  <Custom name="gauge" part="Gauge" at="1,1" span="6" tells="[{&quot;tell&quot;:&quot;reached&quot;,&quot;handler&quot;:&quot;gaugeReached&quot;}]" />
  <Canvas name="pad" at="1,2" span="6" onclick="paint" ondrag="moved" />
  <Canvas name="sketch" at="1,3" span="6" mode="vector" bind="drawing" units="{&quot;width&quot;:800,&quot;height&quot;:600}" onchange="drawn" />
  <Canvas name="board" at="1,4" span="6" mode="objects" objects="{&quot;list&quot;:&quot;pieces&quot;,&quot;x&quot;:&quot;col&quot;,&quot;y&quot;:&quot;row&quot;}" />
</Window>"#;

fn host() -> (Rc<ProgramHost>, Arc<Fake>) {
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let parts = markup::read(PARTS).unwrap().parts;
    let host = Rc::new(ProgramHost::new(program.clone(), FILES, WindowState::new("Main", Map::new()), &parts));
    host.set_running(Running::new(program, FILES));
    (host, fake)
}

fn at(x: i64, y: i64) -> Map<String, Value> {
    let mut at = Map::new();
    at.insert("x".into(), json!(x));
    at.insert("y".into(), json!(y));
    at
}

#[test]
fn a_tell_runs_the_handler_wired_to_it_with_its_values() {
    let (host, fake) = host();
    host.told("gauge", "reached", &[json!(3), json!("high")]);
    assert_eq!(vec![r#"answer gaugeReached [3,"high"]"#], fake.calls());
    // A tell nothing is wired to is let go.
    host.told("gauge", "other", &[]);
    assert!(fake.calls().is_empty());
}

#[test]
fn a_canvas_s_pointer_runs_its_handler_and_a_drag_is_one_edit() {
    let (host, fake) = host();
    assert_eq!(Some(true), host.pointed("pad", "click", &at(10, 20)));
    assert_eq!(vec!["dispatch paint"], fake.calls());
    assert_eq!(Some(at(10, 20)), host.state_now().parts["pad"].pointer);
    assert_eq!(Some(true), host.pointed("pad", "drag", &at(30, 40)));
    assert_eq!(vec![r#"edit pad Some("moved")"#], fake.calls());
    // An event the canvas has no handler for is none of the window's.
    assert_eq!(None, host.pointed("pad", "bend", &at(0, 0)));
}

#[test]
fn a_canvas_with_a_mode_sends_what_the_person_did_as_a_canvas_event() {
    let (host, fake) = host();
    let mut action = Map::new();
    action.insert("add".into(), json!({"op": "line"}));
    assert_eq!(Some(true), host.acted("sketch", &action));
    assert_eq!(vec![r#"canvas sketch vector path=drawing list= at=None units=Some([800, 600]) handler=Some("drawn") action={"add":{"op":"line"}}"#], fake.calls());
    assert_eq!(Some(true), host.acted("board", &Map::new()));
    assert_eq!(vec![r#"canvas board objects path= list=pieces at=Some(["col", "row"]) units=None handler=None action={}"#], fake.calls());
    // A canvas without a mode sends nothing.
    assert_eq!(None, host.acted("pad", &Map::new()));
}

#[test]
fn a_canvas_that_runs_an_engine_gets_the_program_s() {
    let (host, _) = host();
    let make = host.engine("sky").expect("the window answers its engines");
    // The program has none of that name: the canvas says so.
    let why = make().err().unwrap();
    assert!(why.contains("sky"), "{why}");
}

#[test]
fn the_person_s_keys_answer_where_they_set_their_own() {
    let (host, fake) = host();
    assert_eq!(vec!["Ctrl+S".to_string()], host.keys_of("Main.save", Some("Ctrl+S")));
    let (fresh, _) = {
        fake.settings.lock().unwrap().insert("keys".into(), json!({"Main.save": ["Ctrl+K"]}));
        host_over(fake.clone())
    };
    assert_eq!(vec!["Ctrl+K".to_string()], fresh.keys_of("Main.save", Some("Ctrl+S")));
}

fn host_over(fake: Arc<Fake>) -> (Rc<ProgramHost>, Arc<Fake>) {
    let program: Arc<dyn Program> = fake.clone();
    let host = Rc::new(ProgramHost::new(program.clone(), FILES, WindowState::new("Main", Map::new()), &[]));
    host.set_running(Running::new(program, FILES));
    (host, fake)
}

#[test]
fn a_dialog_closing_with_a_value_keeps_it_for_the_handler_that_asked() {
    let (host, _) = host();
    let mut r = reply(host.state_now());
    r.close = true;
    r.result = Some(json!(7));
    host.apply(r);
    assert_eq!(Some(json!(7)), host.take_result());
    assert_eq!(None, host.take_result());
}

#[test]
fn a_cell_edit_whose_row_is_chosen_on_the_way_is_one_edit() {
    // A table writes the cell, chooses its row (`select`), then runs `change`
    // (C# TablePart.Edit): the select may not end the edit, or Ctrl+Z would
    // take back only the change's step and leave the typed value.
    let (host, fake) = host();
    host.write("grid", "tasks[0].title", json!("Edited title"));
    assert!(host.run(Some("grid"), &clarity_ui::markup::HandlerCall::new("select", "rowSelected", Vec::new())));
    assert!(host.run(Some("grid"), &clarity_ui::markup::HandlerCall::new("change", "taskChanged", Vec::new())));
    assert_eq!(vec![r#"edit grid Some("rowSelected") going"#, r#"edit grid Some("taskChanged")"#], fake.calls());
    // A select with no edit waiting is a plain handler.
    host.run(Some("grid"), &clarity_ui::markup::HandlerCall::new("select", "rowSelected", Vec::new()));
    assert_eq!(vec!["dispatch rowSelected"], fake.calls());
}

#[test]
fn the_window_answers_the_icons_its_program_carries() {
    // The ribbon's large buttons draw them (Users icon="list").
    let (host, _) = host();
    assert_eq!(Some("M 8 5 h 13".to_string()), host.icon_path("list"));
    assert_eq!(None, host.icon_path("languages"));
}

#[test]
fn the_undo_and_redo_targets_take_back_the_part_that_had_the_focus_last() {
    // C# UndoFocused: `-> undo` and `-> redo` act on the scope of the part focused last.
    let (host, fake) = host();
    let click = |target: &str| clarity_ui::markup::HandlerCall::new("click", target, Vec::new());
    // Nothing had the focus: nothing to take back.
    assert!(host.run(Some("undoButton"), &click("undo")));
    assert!(fake.calls().is_empty());
    host.set_focused("sketch");
    assert!(host.run(Some("undoButton"), &click("undo")));
    assert!(host.run(Some("redoButton"), &click("redo")));
    assert_eq!(vec!["undo sketch true", "undo sketch false"], fake.calls());
}
