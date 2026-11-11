//! A handler that fails in a way nobody foresaw, as the C# HandlerFailsTests
//! check it: the window says why in an error message, in Clarity's dialog,
//! and goes on working: the next handler runs. In Rust a generated program
//! runs each handler fail-safe (clarity_runtime::program::guarded turns a
//! panic into its error, tested with the runtime); the window gets that
//! error as the program's answer, as this program gives it.

use std::sync::{Arc, Mutex};

use clarity_runtime::program::{self, Answer, Program, Started};
use clarity_runtime::ui::{Level, RefOption, Reply, Text, WindowState};
use clarity_ui::input::{Key, KeyInput};
use clarity_ui::testing::part_named;
use clarity_ui::window::{SaidDialog, WindowStack};
use clarity_ui::{AppFiles, Screen};
use serde_json::{Map, Value};

const MAIN: &str = r#"<Window name="Failing">
  <Button name="fail" at="1,1" span="3" caption="Fail" onclick="fail" />
  <Button name="work" at="4,1" span="3" caption="Work" onclick="work" />
</Window>
"#;

const FILES: AppFiles = AppFiles { windows: &[("Failing", MAIN)], apps: &[], themes: &[], model: "{}", resources: &[], title: "Failing", width: 800.0, height: 500.0 };

/// A program whose `fail` handler fails as a panic caught by the program does, and whose `work` counts its runs.
#[derive(Default)]
struct Failing {
    ran: Mutex<u32>,
}

fn reply(state: WindowState) -> Reply {
    Reply { state, messages: Vec::new(), shown: Default::default(), open: Vec::new(), close: false, asks: Vec::new(), result: None, hears: Vec::new(), copy: None, step: 0, refused: false, quit: false, keys: false }
}

impl Program for Failing {
    fn started(&self) -> Started {
        Started { main: "Failing".into(), unconnected: None, signin: None }
    }
    fn start_app(&self) -> Answer<Reply> {
        Ok(reply(WindowState::new("", Map::new())))
    }
    fn open_window(&self, window: &str, with: Map<String, Value>) -> Answer<Reply> {
        Ok(reply(WindowState::new(window, with)))
    }
    fn close_window(&self, state: WindowState) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn return_to(&self, state: WindowState) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn dispatch_event(&self, state: WindowState, handler: &str) -> Answer<Reply> {
        match handler {
            "fail" => Err(Text::plain("the disk is gone")),
            _ => {
                *self.ran.lock().unwrap() += 1;
                Ok(reply(state))
            }
        }
    }
    fn edit_event(&self, state: WindowState, _edit: program::Edit) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn undo_edit(&self, state: WindowState, _scope: &str, _back: bool) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn reorder_event(&self, state: WindowState, _reorder: program::Reorder) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn item_event(&self, state: WindowState, _item: program::ItemEdit) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn range_event(&self, state: WindowState, _range: program::RangeEdit) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn canvas_event(&self, state: WindowState, _canvas: program::CanvasEdit) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn step_event(&self, state: WindowState, _step: program::StepPress) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn view_event(&self, state: WindowState, _part: &str, _action: Map<String, Value>) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn answer_event(&self, state: WindowState, _handler: &str, _values: Vec<Value>) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn ref_options(&self, _entity: &str) -> Answer<Vec<RefOption>> {
        Ok(Vec::new())
    }
    fn field_values(&self, _entity: &str, _field: &str) -> Answer<Vec<String>> {
        Ok(Vec::new())
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
}

fn click(screen: &mut Screen, name: &str) {
    let b = screen.element(part_named(screen, name)).unwrap().node().bounds;
    screen.click(b.x + b.width / 2.0, b.y + b.height / 2.0);
}

#[test]
fn a_handler_that_fails_says_why_and_the_window_keeps_working() {
    let failing = Arc::new(Failing::default());
    let program: Arc<dyn Program> = failing.clone();
    let mut screen = Screen::new(Box::new(WindowStack::start(program, FILES)), FILES.look(false));
    screen.resize(800.0, 500.0, 1.0);
    click(&mut screen, "fail");
    assert_eq!(1, screen.overlays().len());
    let said = screen.find::<SaidDialog>().expect("the error shows in Clarity's dialog");
    assert_eq!(Level::Error, said.message.level);
    assert!(said.message.text.show().contains("the disk is gone"));
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert!(screen.overlays().is_empty());
    click(&mut screen, "work");
    assert_eq!(1, *failing.ran.lock().unwrap());
}
