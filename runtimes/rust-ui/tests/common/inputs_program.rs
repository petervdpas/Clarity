//! A program as a generated one is, for the inputs' window tests: one
//! window of the test's markup over the test's data, the record of the
//! person's edits kept with the runtime's own edit bus (DESIGN 5.29: the
//! runtime holds it), the handlers it ran, and the values a typed dropdown's
//! field holds.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use clarity_runtime::edits::EditBus;
use clarity_runtime::program::{self, Answer, Program, Started};
use clarity_runtime::ui::{RefOption, Reply, WindowState};
use clarity_ui::window::WindowStack;
use clarity_ui::{AppFiles, Screen};
use serde_json::{Map, Value};

/// A model with English only.
pub const MODEL: &str = r#"{"project": {"locale": {"language": "en"}}}"#;

fn reply(state: WindowState) -> Reply {
    Reply { state, messages: Vec::new(), shown: Default::default(), open: Vec::new(), close: false, asks: Vec::new(), result: None, hears: Vec::new(), copy: None, step: 0, refused: false }
}

pub struct Typed {
    window: String,
    data: Map<String, Value>,
    bus: Mutex<EditBus>,
    /// Every edit it was sent, as `part scope`.
    pub edits: Mutex<Vec<String>>,
    /// Every handler it ran.
    pub ran: Mutex<Vec<String>>,
    /// The languages it was told.
    pub languages: Mutex<Vec<Option<String>>>,
}

impl Typed {
    fn answer(&self, state: WindowState) -> Answer<Reply> {
        self.bus.lock().unwrap().reset(&state.data);
        Ok(reply(state))
    }
}

impl Program for Typed {
    fn started(&self) -> Started {
        Started { main: self.window.clone(), unconnected: None }
    }
    fn start_app(&self) -> Answer<Reply> {
        Ok(reply(WindowState::new("", Map::new())))
    }
    fn open_window(&self, window: &str, _with: Map<String, Value>) -> Answer<Reply> {
        self.answer(WindowState::new(window, self.data.clone()))
    }
    fn close_window(&self, state: WindowState) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn return_to(&self, state: WindowState) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn dispatch_event(&self, state: WindowState, handler: &str) -> Answer<Reply> {
        self.ran.lock().unwrap().push(handler.to_string());
        self.answer(state)
    }
    fn edit_event(&self, state: WindowState, edit: program::Edit) -> Answer<Reply> {
        self.edits.lock().unwrap().push(format!("{} {}", edit.part, edit.scope));
        if let Some(handler) = &edit.handler {
            self.ran.lock().unwrap().push(handler.clone());
        }
        self.bus.lock().unwrap().edit(&edit.part, &edit.scope, &state.data, &[]);
        Ok(reply(state))
    }
    fn undo_edit(&self, mut state: WindowState, scope: &str, back: bool) -> Answer<Reply> {
        let after = self.bus.lock().unwrap().undo(scope, &state.data, back, |_, _| Ok(())).unwrap_or(None);
        if let Some(data) = after {
            state.data = data;
        }
        Ok(reply(state))
    }
    fn reorder_event(&self, state: WindowState, _reorder: program::Reorder) -> Answer<Reply> {
        self.answer(state)
    }
    fn item_event(&self, state: WindowState, _item: program::ItemEdit) -> Answer<Reply> {
        self.answer(state)
    }
    fn range_event(&self, state: WindowState, _range: program::RangeEdit) -> Answer<Reply> {
        self.answer(state)
    }
    fn canvas_event(&self, state: WindowState, _canvas: program::CanvasEdit) -> Answer<Reply> {
        self.answer(state)
    }
    fn step_event(&self, state: WindowState, _step: program::StepPress) -> Answer<Reply> {
        self.answer(state)
    }
    fn view_event(&self, state: WindowState, _part: &str, _action: Map<String, Value>) -> Answer<Reply> {
        self.answer(state)
    }
    fn answer_event(&self, state: WindowState, _handler: &str, _values: Vec<Value>) -> Answer<Reply> {
        self.answer(state)
    }
    fn ref_options(&self, _entity: &str) -> Answer<Vec<RefOption>> {
        Ok(Vec::new())
    }
    // A typed dropdown's values of its field, as the generator writes them.
    fn field_values(&self, _entity: &str, _field: &str) -> Answer<Vec<String>> {
        Ok(vec!["Lima".into(), "Oslo".into(), "Rome".into()])
    }
    fn find_options(&self, _kind: &str, _entity: &str, _field: Option<&str>, _show: &[String], _typed: &str) -> Answer<Value> {
        Ok(Value::Null)
    }
    fn field_bytes(&self, _entity: &str, _id: i64, _field: &str) -> Answer<Option<Vec<u8>>> {
        Ok(None)
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
    fn set_language(&self, language: Option<&str>) {
        self.languages.lock().unwrap().push(language.map(str::to_string));
        clarity_runtime::i18n::set_language(language);
    }
}

/// The program's files: one window named `window` holding `parts`, and `model`.
pub fn files(window: &str, parts: &str, model: &'static str) -> AppFiles {
    let name: &'static str = Box::leak(window.to_string().into_boxed_str());
    let markup: &'static str = Box::leak(format!(r#"<Window name="{window}">{parts}</Window>"#).into_boxed_str());
    let windows: &'static [(&'static str, &'static str)] = Box::leak(vec![(name, markup)].into_boxed_slice());
    AppFiles { windows, apps: &[], themes: &[], model, resources: &[], title: name, width: 800.0, height: 400.0 }
}

/// The window `window` with `parts` over `data`, shown on a screen of 800 by 400.
pub fn shown(window: &str, parts: &str, model: &'static str, data: Map<String, Value>) -> (Screen, Arc<Typed>) {
    shown_from(files(window, parts, model), data)
}

/// The program of `files` over `data`, shown on a screen of 800 by 400.
pub fn shown_from(files: AppFiles, data: Map<String, Value>) -> (Screen, Arc<Typed>) {
    let window = files.windows[0].0.to_string();
    let program = Arc::new(Typed { window, data, bus: Mutex::new(EditBus::default()), edits: Mutex::new(Vec::new()), ran: Mutex::new(Vec::new()), languages: Mutex::new(Vec::new()) });
    let as_program: Arc<dyn Program> = program.clone();
    let look = files.look(false);
    let stack = WindowStack::start(as_program, files);
    let mut screen = Screen::new(Box::new(stack), look);
    screen.resize(800.0, 400.0, 1.0);
    clarity_ui::testing::Shot::of(&mut screen, 800, 400, 1.0);
    (screen, program)
}
