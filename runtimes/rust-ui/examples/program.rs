//! A small program on Clarity.UI's desktop host: one window, no handlers and
//! no database, run through `desktop::run` as a generated program is.
//!
//! Run: `cargo run -p clarity-ui --example program`

use std::process::ExitCode;

use clarity_runtime::program::{self, Answer, Program, Started};
use clarity_runtime::ui::{RefOption, Reply, Ui, WindowState};
use clarity_ui::{desktop, AppFiles};
use serde_json::{Map, Value};

const MAIN: &str = r#"<Window name="Main">
  <Heading name="title" at="1,1" span="12" caption="A small program" />
  <Label name="nameLabel" at="1,2" span="3" caption="Name" />
  <Textbox name="name" at="4,2" span="6" />
  <Button name="close" at="1,3" span="3" caption="Close" onclick="close" />
</Window>"#;

const MODEL: &str = r#"{"project": {"locale": {"language": "en"}, "translations": {}}}"#;

const FILES: AppFiles = AppFiles {
    windows: &[("Main", MAIN)],
    apps: &[],
    themes: &[],
    model: MODEL,
    resources: &[],
    title: "Small program",
    width: 720.0,
    height: 420.0,
};

/// A program whose every answer is its window as it was.
struct Small;

impl Small {
    fn drawn(&self, state: WindowState) -> Answer<Reply> {
        let data = state.data.clone();
        Ok(Ui::from_state(&state).reply(&state.window, data))
    }
}

impl Program for Small {
    fn started(&self) -> Started {
        Started { main: "Main".into(), unconnected: None }
    }
    fn start_app(&self) -> Answer<Reply> {
        self.drawn(WindowState::new("Main", Map::new()))
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
        Ok(Vec::new())
    }
    fn find_options(&self, _kind: &str, _entity: &str, _field: Option<&str>, _show: &[String], _typed: &str) -> Answer<Value> {
        Ok(Value::Array(Vec::new()))
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
}

fn main() -> ExitCode {
    desktop::run(Small, FILES)
}
