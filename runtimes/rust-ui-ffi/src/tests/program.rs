//! The program as callbacks: each call of `Program` is a command with its
//! arguments as JSON, answered `{"value"}` or `{"error"}`; a program started
//! on a screen without a window is driven step by step.

use std::ffi::{c_char, c_void};
use std::ptr::null_mut;
use std::sync::Mutex;

use clarity_runtime::program::{Edit, Program, RangeEdit, Reorder};
use clarity_runtime::ranges::{Range, RangeKey, RangeMove};
use clarity_runtime::reorder::{DropAt, Move};
use clarity_runtime::ui::WindowState;
use serde_json::{json, Map, Value};

use super::{answer, given, last_error, ok, t, taken, GLOBAL};
use crate::desktop::*;
use crate::files::*;
use crate::program::*;
use crate::{ClarityAnswer, CLARITY_FAILED};

/// A program as a C# program answers the library: its calls logged, its
/// window's data kept, and what it was asked last.
#[derive(Default)]
struct Fake {
    log: Mutex<Vec<String>>,
    args: Mutex<Vec<(String, Value)>>,
    released: Mutex<bool>,
}

impl Fake {
    fn of<'a>(user: *mut c_void) -> &'a Fake {
        unsafe { &*(user as *const Fake) }
    }

    fn calls(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.lock().unwrap())
    }

    fn asked(&self, command: &str) -> Option<Value> {
        self.args.lock().unwrap().iter().rev().find(|(c, _)| c == command).map(|(_, a)| a.clone())
    }
}

fn reply(window: &str, data: Value, messages: Value) -> Value {
    json!({ "value": { "state": { "window": window, "instance": 7, "data": data }, "messages": messages, "open": [], "close": false } })
}

unsafe extern "C" fn call(user: *mut c_void, command: *const c_char, command_len: usize, args: *const c_char, args_len: usize, out: *mut ClarityAnswer) {
    let fake = Fake::of(user);
    let command = unsafe { given(command, command_len) }.unwrap();
    let args: Value = serde_json::from_str(&unsafe { given(args, args_len) }.unwrap()).unwrap();
    fake.args.lock().unwrap().push((command.clone(), args.clone()));
    let state_data = || args["state"]["data"].clone();
    let answered = match command.as_str() {
        "started" => json!({ "value": { "main": "Main" } }),
        "start_app" => reply("", json!({}), json!([])),
        "open_window" => reply(args["window"].as_str().unwrap(), json!({ "name": "Ann" }), json!([])),
        "dispatch_event" if args["handler"] == "openOther" => json!({ "value": {
            "state": { "window": "Main", "instance": 7, "data": state_data() }, "messages": [],
            "open": [{ "window": "Other", "with": { "$window": 41, "note": "hi" } }], "close": false } }),
        // Code that quits, opens the keys window, or opens a window in a window of its own.
        "dispatch_event" if args["handler"] == "quitIt" => json!({ "value": {
            "state": { "window": "Other", "instance": 8, "data": state_data() }, "messages": [], "open": [], "close": false, "quit": true } }),
        "dispatch_event" if args["handler"] == "keysIt" => json!({ "value": {
            "state": { "window": "Main", "instance": 7, "data": state_data() }, "messages": [], "open": [], "close": false, "keys": true } }),
        "dispatch_event" if args["handler"] == "openApart" => json!({ "value": {
            "state": { "window": "Main", "instance": 7, "data": state_data() }, "messages": [],
            "open": [{ "window": "Other", "with": { "$window": 42 }, "new_window": true }], "close": false } }),
        "dispatch_event" if args["handler"] == "askIt" => json!({ "value": {
            "state": { "window": "Main", "instance": 7, "data": state_data() }, "messages": [],
            "open": [], "close": false,
            "asks": [{ "question": "Sure?", "form": { "kind": "confirm", "yes": null, "no": null }, "handler": "question-17", "values": [] }] } }),
        "answer_event" => {
            fake.log.lock().unwrap().push(format!("answer {} {}", args["handler"].as_str().unwrap(), args["values"]));
            reply("Main", state_data(), json!([]))
        }
        "dispatch_event" => {
            fake.log.lock().unwrap().push(format!("dispatch {}", args["handler"].as_str().unwrap()));
            reply("Main", json!({ "name": "Saved" }), json!([{ "level": "info", "text": "Saved" }]))
        }
        "edit_event" => {
            fake.log.lock().unwrap().push(format!("edit {} {}", args["part"].as_str().unwrap(), args["state"]["data"]["name"]));
            reply("Main", state_data(), json!([]))
        }
        "refused" => json!({ "error": { "key": "recordNotThere", "values": {} } }),
        "field_bytes" => json!({ "value": "AQID" }),
        "watch_progress" => json!({ "value": [3, 10] }),
        "engine_open" if args["name"] == "broken" => json!({ "error": "no such file" }),
        "read_settings" => json!({ "value": { "zoom": 2 } }),
        // Every other command is one this program does not answer.
        _ => return,
    };
    unsafe { answer(out, &answered.to_string()) };
}

unsafe extern "C" fn release(user: *mut c_void) {
    *Fake::of(user).released.lock().unwrap() = true;
}

fn program_of(fake: &Fake) -> *mut ClarityProgram {
    let callbacks = ClarityProgramCallbacks { user: fake as *const Fake as *mut c_void, call: Some(call), release: Some(release) };
    let mut program = null_mut();
    ok(unsafe { clarity_program_new(&callbacks, &mut program) });
    program
}

const MAIN: &str = r#"<Window name="Main"><Textbox name="nameBox" at="1,1" span="4" bind="name" undoScope="nameBox" /><Button name="save" at="1,2" span="3" caption="Save" onclick="save" /><Button name="other" at="4,2" span="3" caption="Other" onclick="openOther" /><Button name="ask" at="7,2" span="3" caption="Ask" onclick="askIt" /><Button name="keys" at="1,3" span="3" caption="Keys" onclick="keysIt" /><Button name="apart" at="4,3" span="3" caption="Apart" onclick="openApart" /></Window>"#;

const OTHER: &str = r#"<Window name="Other"><Label name="hello" at="1,1" span="4" caption="Other" /><Button name="quit" at="1,2" span="3" caption="Quit" onclick="quitIt" /></Window>"#;

/// The model of a project whose keys the person sets, so its keys window opens.
const KEYS_MODEL: &str = r#"{"project": {"locale": {"language": "en"}, "keysEditable": true, "files": []}, "keys": []}"#;

fn files() -> *mut ClarityFiles {
    let mut files = null_mut();
    ok(unsafe { clarity_files_new(&mut files) });
    let ((n, nl), (m, ml)) = (t("Main"), t(MAIN));
    ok(unsafe { clarity_files_window(files, n, nl, m, ml) });
    let ((n, nl), (m, ml)) = (t("Other"), t(OTHER));
    ok(unsafe { clarity_files_window(files, n, nl, m, ml) });
    files
}

fn step(driven: *mut ClarityDriven, step: Value) -> Value {
    let step = step.to_string();
    let mut out = null_mut();
    ok(unsafe { clarity_driven_step(driven, step.as_ptr() as *const c_char, step.len(), &mut out) });
    serde_json::from_str(&unsafe { taken(out) }.unwrap()).unwrap()
}

#[test]
fn a_driven_program_runs_its_handler_and_shows_its_reply() {
    let _one = GLOBAL.lock().unwrap_or_else(|p| p.into_inner());
    let fake = Fake::default();
    let (program, files) = (program_of(&fake), files());
    let mut driven = null_mut();
    ok(unsafe { clarity_driven_start(program, files, &mut driven) });
    assert_eq!(fake.asked("open_window").unwrap()["window"], "Main");
    let read = step(driven, json!({ "do": "read" }));
    assert_eq!(read["data"]["name"], "Ann", "{read}");
    step(driven, json!({ "do": "click", "part": "save" }));
    assert_eq!(fake.calls(), ["dispatch save"]);
    let read = step(driven, json!({ "do": "read" }));
    assert_eq!(read["data"]["name"], "Saved", "{read}");
    // A step that cannot be done fails, and still answers why.
    let bad = json!({ "do": "click", "part": "nothing" }).to_string();
    let mut out = null_mut();
    assert_eq!(unsafe { clarity_driven_step(driven, bad.as_ptr() as *const c_char, bad.len(), &mut out) }, CLARITY_FAILED);
    let answered: Value = serde_json::from_str(&unsafe { taken(out) }.unwrap()).unwrap();
    assert_eq!(answered["ok"], false);
    assert!(last_error().contains("nothing"));
    unsafe {
        clarity_driven_free(driven);
        clarity_program_free(program);
        clarity_files_free(files);
    }
    assert!(*fake.released.lock().unwrap(), "the program is let go once the run and the caller let go");
}

#[test]
fn typing_in_a_bound_box_is_an_edit_the_program_hears() {
    let _one = GLOBAL.lock().unwrap_or_else(|p| p.into_inner());
    let fake = Fake::default();
    let (program, files) = (program_of(&fake), files());
    let mut driven = null_mut();
    ok(unsafe { clarity_driven_start(program, files, &mut driven) });
    step(driven, json!({ "do": "focus", "part": "nameBox" }));
    step(driven, json!({ "do": "key", "key": "End" }));
    step(driven, json!({ "do": "type", "text": " B" }));
    step(driven, json!({ "do": "key", "key": "Tab" }));
    assert!(fake.calls().iter().any(|c| c == "edit nameBox \"Ann B\""), "{:?}", fake.args.lock().unwrap().iter().map(|(c, _)| c.clone()).collect::<Vec<_>>());
    unsafe {
        clarity_driven_free(driven);
        clarity_program_free(program);
        clarity_files_free(files);
    }
}

#[test]
fn what_a_reply_opens_with_and_a_questions_handler_come_back_as_given() {
    let _one = GLOBAL.lock().unwrap_or_else(|p| p.into_inner());
    let fake = Fake::default();
    let (program, files) = (program_of(&fake), files());
    let mut driven = null_mut();
    ok(unsafe { clarity_driven_start(program, files, &mut driven) });
    step(driven, json!({ "do": "click", "part": "ask" }));
    fake.calls();
    step(driven, json!({ "do": "click", "text": "OK" }));
    assert_eq!(fake.calls(), ["answer question-17 []"]);
    step(driven, json!({ "do": "click", "part": "other" }));
    assert_eq!(fake.asked("open_window").unwrap()["with"], json!({ "$window": 41, "note": "hi" }));
    assert_eq!(step(driven, json!({ "do": "read" }))["windows"], json!(["Main", "Other"]));
    unsafe {
        clarity_driven_free(driven);
        clarity_program_free(program);
        clarity_files_free(files);
    }
}

/// Code quits (the built-in `quit` run by a handler): the run ends with
/// two windows open, the reply of the one on top saying so.
#[test]
fn a_handler_that_quits_ends_a_driven_run_with_two_windows_open() {
    let _one = GLOBAL.lock().unwrap_or_else(|p| p.into_inner());
    let fake = Fake::default();
    let (program, files) = (program_of(&fake), files());
    let mut driven = null_mut();
    ok(unsafe { clarity_driven_start(program, files, &mut driven) });
    step(driven, json!({ "do": "click", "part": "other" }));
    let read = step(driven, json!({ "do": "read" }));
    assert_eq!((read["windows"].clone(), read["quit"].clone()), (json!(["Main", "Other"]), json!(false)));
    step(driven, json!({ "do": "click", "part": "quit" }));
    let read = step(driven, json!({ "do": "read" }));
    assert_eq!((read["quit"].clone(), read["ended"].clone()), (json!(true), json!(true)), "{read}");
    unsafe {
        clarity_driven_free(driven);
        clarity_program_free(program);
        clarity_files_free(files);
    }
}

/// Code opens Clarity's keys window, and a window `in new window`: the
/// reply's `keys` and its open's `new_window` (in place on a screen without
/// desktop windows of their own, with the values it was given).
#[test]
fn a_reply_opens_the_keys_window_and_a_window_of_its_own() {
    let _one = GLOBAL.lock().unwrap_or_else(|p| p.into_inner());
    let fake = Fake::default();
    let (program, files) = (program_of(&fake), files());
    let (m, ml) = t(KEYS_MODEL);
    ok(unsafe { clarity_files_model(files, m, ml) });
    let mut driven = null_mut();
    ok(unsafe { clarity_driven_start(program, files, &mut driven) });
    assert_eq!(step(driven, json!({ "do": "read" }))["keys"], json!(false));
    step(driven, json!({ "do": "click", "part": "keys" }));
    assert_eq!(step(driven, json!({ "do": "read" }))["keys"], json!(true));
    step(driven, json!({ "do": "key", "key": "Escape" }));
    step(driven, json!({ "do": "click", "part": "apart" }));
    assert_eq!(fake.asked("open_window").unwrap()["with"], json!({ "$window": 42 }));
    assert_eq!(step(driven, json!({ "do": "read" }))["windows"], json!(["Main", "Other"]));
    unsafe {
        clarity_driven_free(driven);
        clarity_program_free(program);
        clarity_files_free(files);
    }
}

fn direct(fake: &Fake) -> CallbackProgram {
    CallbackProgram::new(ClarityProgramCallbacks { user: fake as *const Fake as *mut c_void, call: Some(call), release: None })
}

#[test]
fn each_call_carries_the_wires_arguments() {
    let fake = Fake::default();
    let program = direct(&fake);
    let state = WindowState { window: "Main".into(), instance: 3, data: Map::new(), ..WindowState::default() };
    let edit = Edit { handler: Some("changed".into()), part: "nameBox".into(), scope: "form".into(), ends: true, ..Edit::default() };
    let _ = program.edit_event(state.clone(), edit);
    let args = fake.asked("edit_event").unwrap();
    assert_eq!(args["state"]["instance"], 3);
    assert_eq!((args["part"].as_str(), args["scope"].as_str(), args["ends"].as_bool(), args["handler"].as_str()), (Some("nameBox"), Some("form"), Some(true), Some("changed")));
    let reorder = Reorder { part: "rows".into(), scope: None, list: "lines".into(), by: "pos".into(), parent: None, moved: Move::Drag { row: 1, at: DropAt::After, target: 3 }, handler: None, readonly: false };
    let _ = program.reorder_event(state.clone(), reorder);
    assert_eq!(fake.asked("reorder_event").unwrap()["move"], json!({ "row": 1, "where": "after", "target": 3 }));
    let range = RangeEdit { part: "zoom".into(), scope: None, path: "zoom".into(), range: Range { min: Some("0".into()), max: None, step: "1".into() }, whole: true, moved: RangeMove::Key(RangeKey::PageUp), handler: None, readonly: false };
    let _ = program.range_event(state, range);
    let args = fake.asked("range_event").unwrap();
    assert_eq!((args["key"].as_str(), args["min"].as_str(), args["max"].is_null()), (Some("pageUp"), Some("0"), true));
}

#[test]
fn answers_are_read_back_and_unanswered_calls_take_the_default() {
    let fake = Fake::default();
    let program = direct(&fake);
    assert_eq!(program.started().main, "Main");
    assert_eq!(program.field_bytes("Task", 1, "file"), Ok(Some(vec![1, 2, 3])));
    assert_eq!(program.watch_progress(4), Some((3, 10)));
    assert_eq!(program.read_settings().get("zoom"), Some(&json!(2)));
    // Unanswered: no watch, no engine, no facts, and a call that needs an answer says so.
    assert!(!program.watches());
    assert_eq!(program.engine_open("sky"), None);
    assert_eq!(program.engine_open("broken"), Some(Err("no such file".to_string())));
    assert_eq!(program.facts_json(), Value::Null);
    assert_eq!(program.connect(), Ok(()));
    assert_eq!(program.keep_settings(Map::new()), Ok(()));
    let refused = program.return_to(WindowState::default()).unwrap_err();
    assert_eq!(refused.show(), "The program did not answer return_to.");
    let keyed = program.answer::<Value>("refused", Map::new()).unwrap_err();
    assert_eq!(keyed.key.as_deref(), Some("recordNotThere"));
}

#[test]
fn a_program_without_a_call_is_invalid() {
    let callbacks = ClarityProgramCallbacks { user: null_mut(), call: None, release: None };
    let mut program = null_mut();
    assert_eq!(unsafe { clarity_program_new(&callbacks, &mut program) }, crate::CLARITY_INVALID);
    assert_eq!(last_error(), "the program has no call");
}
