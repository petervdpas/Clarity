//! A program for the window tests: it logs every call, answers by the
//! handler, asks questions, opens a dialog for a value, runs a long call
//! with progress and Stop, keeps its settings in memory, and can start with
//! its databases not open.

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clarity_runtime::program::{self, Answer, Program, Started, Unconnected, Watch};
use clarity_runtime::ui::{Ask, AskForm, Level, Message, OpenRequest, RefOption, Reply, Text, WindowState};
use clarity_ui::element::{self, ElementId};
use clarity_ui::window::WindowStack;
use clarity_ui::{AppFiles, Screen};
use serde_json::{json, Map, Value};

pub const SHOTS: &str = "/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots";

pub const MAIN: &str = r#"<Window name="Main">
  <Heading name="title" at="1,1" span="12" caption="Orders" />
  <Label name="greeting" at="1,2" span="6" caption="@hello" />
  <Button name="save" at="1,3" span="3" caption="Save" key="Ctrl+S" onclick="save" />
  <Button name="warn" at="4,3" span="3" caption="Warn" key="Ctrl+W" onclick="warn" />
  <Button name="openPick" at="7,3" span="3" caption="Pick" opens="Pick" />
  <Button name="pick" at="10,3" span="3" caption="Ask Pick" onclick="pick" />
  <Button name="ask" at="1,4" span="3" caption="Ask" onclick="ask" />
  <Button name="askNumber" at="4,4" span="3" caption="Number" onclick="askNumber" />
  <Button name="askColor" at="7,4" span="3" caption="Color" onclick="askColor" />
  <Button name="keys" at="10,4" span="3" caption="Keys" opens="keys" />
  <Button name="long" at="1,5" span="3" caption="Long" onclick="long" />
  <Button name="quit" at="4,5" span="3" caption="Quit" opens="quit" />
</Window>
"#;

pub const PICK: &str = r#"<Window name="Pick">
  <Label name="pickText" at="1,1" span="12" caption="Pick a color" />
  <Button name="choose" at="1,2" span="6" caption="Red" default="true" onclick="choose" />
  <Button name="cancelPick" at="7,2" span="6" caption="Cancel" cancel="true" onclick="done" />
</Window>
"#;

pub const CONNECTION: &str = r#"<Window name="Connection">
  <Heading name="connectionTitle" at="1,1" span="12" caption="Connection" />
  <Button name="keep" at="1,2" span="6" caption="Keep" onclick="keep" />
  <Button name="cancelConnection" at="7,2" span="6" caption="Close" cancel="true" onclick="done" />
</Window>
"#;

pub const MODEL: &str = r#"{
  "icons": {"list": "M 8 5 h 13"},
  "project": {
    "locale": {"language": "en"},
    "keysEditable": true,
    "translations": {"en": {"hello": "Hello"}, "nl": {"hello": "Hallo"}},
    "files": [{
      "pages": [
        {"name": {"name": "Main"}, "kind": "window", "title": "Orders"},
        {"name": {"name": "Pick"}, "kind": "dialog", "title": "Pick a color"},
        {"name": {"name": "Connection"}, "kind": "window", "title": "Connection"}
      ],
      "enums": [{"name": {"name": "Color"}, "variants": [
        {"name": {"name": "red"}, "label": "Red"},
        {"name": {"name": "green"}, "label": "Green"},
        {"name": {"name": "blue"}, "label": "Blue"}
      ]}]
    }]
  },
  "keys": [
    {"id": "", "title": null, "commands": [{"id": "quit", "name": "quit", "caption": "Quit", "keys": ["Ctrl+Q"]}]},
    {"id": "Main", "title": "Orders", "commands": [
      {"id": "Main.save", "name": "save", "caption": "Save", "keys": ["Ctrl+S"]},
      {"id": "Main.warn", "name": "warn", "caption": "Warn", "keys": ["Ctrl+W"]}
    ]}
  ]
}"#;

pub const FILES: AppFiles = AppFiles { windows: &[("Main", MAIN), ("Pick", PICK), ("Connection", CONNECTION)], apps: &[], themes: &[], model: MODEL, title: "Orders", width: 800.0, height: 500.0 };

/// The program of the tests.
#[derive(Default)]
pub struct Fake {
    pub log: Mutex<Vec<String>>,
    pub settings: Mutex<Map<String, Value>>,
    pub watches: bool,
    pub unconnected: Option<Unconnected>,
    pub connects: bool,
    pub watch: Mutex<Option<Watch>>,
    pub long_reached: AtomicBool,
}

pub fn reply(state: WindowState) -> Reply {
    Reply { state, messages: Vec::new(), shown: Default::default(), open: Vec::new(), close: false, asks: Vec::new(), result: None, hears: Vec::new(), copy: None, step: 0, refused: false }
}

fn asking(state: WindowState, form: AskForm, handler: &str, values: Vec<Value>) -> Reply {
    let mut r = reply(state);
    r.asks.push(Ask { question: Text::plain("Delete the order for good?"), form, handler: handler.into(), values });
    r
}

impl Fake {
    pub fn said(&self, what: String) {
        self.log.lock().unwrap().push(what);
    }

    pub fn calls(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.lock().unwrap())
    }
}

impl Program for Fake {
    fn started(&self) -> Started {
        Started { main: "Main".into(), unconnected: self.unconnected.clone() }
    }
    fn start_app(&self) -> Answer<Reply> {
        self.said("start".into());
        Ok(reply(WindowState::new("", Map::new())))
    }
    fn open_window(&self, window: &str, with: Map<String, Value>) -> Answer<Reply> {
        self.said(format!("open {window}"));
        Ok(reply(WindowState::new(window, with)))
    }
    fn close_window(&self, state: WindowState) -> Answer<Reply> {
        self.said(format!("close {}", state.window));
        Ok(reply(state))
    }
    fn return_to(&self, state: WindowState) -> Answer<Reply> {
        self.said(format!("return {}", state.window));
        Ok(reply(state))
    }
    fn dispatch_event(&self, mut state: WindowState, handler: &str) -> Answer<Reply> {
        self.said(format!("dispatch {handler}"));
        match handler {
            "warn" => {
                let mut r = reply(state);
                r.messages.push(Message { level: Level::Warning, text: Text::plain("Careful") });
                Ok(r)
            }
            "ask" => Ok(asking(state, AskForm::Confirm { yes: Some(Text::plain("Delete")), no: Some(Text::plain("Keep")) }, "confirmed", vec![json!(1)])),
            "askNumber" => Ok(asking(state, AskForm::Value { ty: "integer".into(), of: None, only: None, default: None }, "numbered", Vec::new())),
            "askColor" => Ok(asking(state, AskForm::Value { ty: "enum".into(), of: Some("Color".into()), only: Some(vec![json!("green"), json!("blue")]), default: None }, "colored", Vec::new())),
            "pick" => Ok(asking(state, AskForm::Window { window: "Pick".into(), with: Map::new() }, "picked", vec![json!("x")])),
            "choose" => {
                let mut r = reply(state);
                r.close = true;
                r.result = Some(json!("red"));
                Ok(r)
            }
            "done" => {
                let mut r = reply(state);
                r.close = true;
                Ok(r)
            }
            "keep" => {
                self.settings.lock().unwrap().insert("connection".into(), json!("kept"));
                Ok(reply(state))
            }
            "long" => {
                let watch = self.watch.lock().unwrap().clone();
                if let Some(progress) = watch.as_ref().and_then(|w| w.progress.clone()) {
                    progress(1, 4);
                }
                self.long_reached.store(true, Ordering::SeqCst);
                let until = Instant::now() + Duration::from_secs(10);
                while Instant::now() < until && !watch.as_ref().is_some_and(|w| w.stopped()) {
                    std::thread::sleep(Duration::from_millis(5));
                }
                state.data.insert("stopped".into(), json!(watch.is_some_and(|w| w.stopped())));
                Ok(reply(state))
            }
            "unconnected" => {
                let mut r = reply(state);
                r.open.push(OpenRequest { window: "Connection".into(), with: Map::new() });
                Ok(r)
            }
            _ => Ok(reply(state)),
        }
    }
    fn edit_event(&self, state: WindowState, edit: program::Edit) -> Answer<Reply> {
        let going = if edit.ends { "" } else { " going" };
        self.said(format!("edit {} {:?}{going}", edit.part, edit.handler));
        Ok(reply(state))
    }
    fn undo_edit(&self, state: WindowState, scope: &str, back: bool) -> Answer<Reply> {
        self.said(format!("undo {scope} {back}"));
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
    fn canvas_event(&self, state: WindowState, edit: program::CanvasEdit) -> Answer<Reply> {
        let c = &edit.canvas;
        self.said(format!("canvas {} {} path={} list={} at={:?} units={:?} handler={:?} action={}", c.part, c.mode, c.path, c.list, c.at, c.units, edit.handler, Value::Object(edit.action.clone())));
        Ok(reply(state))
    }
    fn step_event(&self, state: WindowState, _step: program::StepPress) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn view_event(&self, state: WindowState, _part: &str, _action: Map<String, Value>) -> Answer<Reply> {
        Ok(reply(state))
    }
    fn answer_event(&self, state: WindowState, handler: &str, values: Vec<Value>) -> Answer<Reply> {
        self.said(format!("answer {handler} {}", Value::from(values.clone())));
        if handler == "unconnected" {
            return self.dispatch_event(state, "unconnected");
        }
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
        self.settings.lock().unwrap().clone()
    }
    fn keep_settings(&self, settings: Map<String, Value>) -> Answer<()> {
        self.said(format!("keep {}", Value::Object(settings.clone())));
        *self.settings.lock().unwrap() = settings;
        Ok(())
    }
    fn connect(&self) -> Answer<()> {
        self.said("connect".into());
        if self.connects { Ok(()) } else { Err(Text::plain("still no database")) }
    }
    fn set_language(&self, language: Option<&str>) {
        self.said(format!("language {language:?}"));
        clarity_runtime::i18n::set_language(language);
    }
    fn watches(&self) -> bool {
        self.watches
    }
    fn watched(&self, watch: &Watch, run: &mut dyn FnMut()) {
        *self.watch.lock().unwrap() = Some(watch.clone());
        run();
        *self.watch.lock().unwrap() = None;
    }
}

/// The program started on a screen of 800 by 500.
pub fn started_with(fake: Fake) -> (Screen, Arc<Fake>) {
    let fake = Arc::new(fake);
    let program: Arc<dyn Program> = fake.clone();
    let stack = WindowStack::start(program, FILES);
    let mut screen = Screen::new(Box::new(stack), FILES.look(false));
    screen.resize(800.0, 500.0, 1.0);
    (screen, fake)
}

pub fn started() -> (Screen, Arc<Fake>) {
    started_with(Fake::default())
}

pub fn stack(screen: &Screen) -> &WindowStack {
    screen.root().as_any().downcast_ref::<WindowStack>().unwrap()
}

/// The part of that name, in the window or an overlay over it.
pub fn named(screen: &Screen, name: &str) -> ElementId {
    let in_tree = |root: &dyn clarity_ui::Element| {
        element::in_order(root).into_iter().find(|e| e.as_part().and_then(|p| p.base().spec.name()) == Some(name)).map(|e| e.node().id())
    };
    screen.overlays().iter().rev().find_map(|o| in_tree(o.content.as_ref())).or_else(|| in_tree(screen.root())).unwrap_or_else(|| panic!("no part named {name}"))
}

/// Clicks the middle of an element.
pub fn click_at(screen: &mut Screen, id: ElementId) {
    let b = screen.element(id).unwrap().node().bounds;
    screen.click(b.x + b.width / 2.0, b.y + b.height / 2.0);
}

pub fn click(screen: &mut Screen, name: &str) {
    let id = named(screen, name);
    click_at(screen, id);
}

/// Saves a shot of the screen as `core-windows-<name>.png`.
pub fn shot(screen: &mut Screen, name: &str) {
    let mut shot = clarity_ui::testing::Shot::of(screen, 800, 500, 1.0);
    shot.save(format!("{SHOTS}/core-windows-{name}.png")).unwrap();
}
