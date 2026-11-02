//! How windows open and close, against the cases every stack shares
//! (conformance/windows.json, C# WindowCaseTests), run through WindowStack
//! as the desktop host runs it: a dialog or a target marked `in new window`
//! gets a desktop window of its own (`take_aparts`), a stack of its own
//! whose opener is told when it ends (`take_closed`, `apart_closed`). A
//! dialog's desktop window is listed on top of the desktop window it was
//! opened from, as the cases expect.

use std::sync::{Arc, Mutex};

use clarity_runtime::program::{self, Answer, Program, Started};
use clarity_runtime::ui::{OpenRequest, RefOption, Reply, WindowState};
use clarity_ui::markup::HandlerCall;
use clarity_ui::parts::NEW_WINDOW_CLICK;
use clarity_ui::window::{Apart, WindowStack};
use clarity_ui::{AppFiles, PartHost, Screen};
use serde_json::{Map, Value};

const CASE: &str = r#"<Window name="Case" />"#;

const MODEL: &str = r#"{"project": {"locale": {"language": "en"}, "files": [{"pages": [{"name": {"name": "Ask"}, "kind": "dialog", "title": "Ask"}]}]}}"#;

const FILES: AppFiles = AppFiles {
    windows: &[("Card", CASE), ("Customers", CASE), ("Customer", CASE), ("Ask", CASE), ("List", CASE), ("Form", CASE), ("Orders", CASE)],
    apps: &[],
    themes: &[],
    model: MODEL,
    resources: &[],
    title: "Cases",
    width: 800.0,
    height: 600.0,
};

/// A program whose windows log their events and whose handler `do:` does a step's actions.
struct Cases {
    start: String,
    log: Mutex<Vec<String>>,
}

fn reply(state: WindowState) -> Reply {
    Reply { state, messages: Vec::new(), shown: Default::default(), open: Vec::new(), close: false, asks: Vec::new(), result: None, hears: Vec::new(), copy: None, step: 0, refused: false }
}

impl Program for Cases {
    fn started(&self) -> Started {
        Started { main: self.start.clone(), unconnected: None }
    }
    fn start_app(&self) -> Answer<Reply> {
        Ok(reply(WindowState::new("", Map::new())))
    }
    fn open_window(&self, window: &str, with: Map<String, Value>) -> Answer<Reply> {
        self.log.lock().unwrap().push(format!("{window}:open"));
        Ok(reply(WindowState::new(window, with)))
    }
    fn close_window(&self, state: WindowState) -> Answer<Reply> {
        self.log.lock().unwrap().push(format!("{}:close", state.window));
        Ok(reply(state))
    }
    fn return_to(&self, state: WindowState) -> Answer<Reply> {
        self.log.lock().unwrap().push(format!("{}:return", state.window));
        Ok(reply(state))
    }
    fn dispatch_event(&self, state: WindowState, handler: &str) -> Answer<Reply> {
        let mut r = reply(state);
        for action in handler.strip_prefix("do:").unwrap_or_default().split('|') {
            match action.strip_prefix("open ") {
                Some(window) => r.open.push(OpenRequest { window: window.to_string(), with: Map::new() }),
                None if action == "close" => r.close = true,
                None => panic!("unknown action {action}"),
            }
        }
        Ok(r)
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

/// One desktop window: its screen, whether it shows a dialog, and the window that opened it.
struct Desktop {
    screen: Screen,
    dialog: bool,
    opener: Option<u64>,
}

fn stack_of(screen: &Screen) -> &WindowStack {
    screen.root().as_any().downcast_ref::<WindowStack>().unwrap()
}

fn on_screen(stack: WindowStack) -> Screen {
    let mut screen = Screen::new(Box::new(stack), FILES.look(false));
    screen.resize(800.0, 600.0, 1.0);
    screen
}

/// The desktop windows of a case's program, as the desktop host keeps them.
struct Desktops {
    open: Vec<Desktop>,
    ended: bool,
}

impl Desktops {
    fn start(program: Arc<dyn Program>) -> Desktops {
        let stack = WindowStack::start(program, FILES);
        stack.running().set_apart(true);
        let mut desktops = Desktops { open: vec![Desktop { screen: on_screen(stack), dialog: false, opener: None }], ended: false };
        desktops.settle();
        desktops
    }

    /// What the stacks asked of the host: windows of their own open, ended ones close and tell their openers.
    fn settle(&mut self) {
        loop {
            let mut changed = false;
            for i in 0..self.open.len() {
                self.open[i].screen.act_root::<WindowStack, _>(|_, _| ());
                let (aparts, running) = self.open[i].screen.act_root::<WindowStack, _>(|s, _| (s.take_aparts(), s.running().clone())).unwrap();
                for apart in aparts {
                    if let Apart::Window(request) = apart {
                        let (dialog, opener) = (request.dialog, Some(request.opener));
                        let stack = WindowStack::apart(running.clone(), request);
                        self.open.push(Desktop { screen: on_screen(stack), dialog, opener });
                        changed = true;
                    }
                }
            }
            let ended: Vec<usize> = (0..self.open.len()).filter(|i| stack_of(&self.open[*i].screen).ended()).collect();
            for i in ended.into_iter().rev() {
                if i == 0 {
                    self.ended = true;
                }
                let mut gone = self.open.remove(i);
                let closed = gone.screen.act_root::<WindowStack, _>(|s, _| s.take_closed()).flatten();
                if let Some(closed) = closed {
                    if let Some(to) = self.open.iter_mut().find(|d| stack_of(&d.screen).holds(closed.opener)) {
                        to.screen.act_root::<WindowStack, _>(move |s, cx| s.apart_closed(closed, cx));
                    }
                }
                changed = true;
            }
            if !changed {
                return;
            }
        }
    }

    /// The window on top of the newest desktop window runs one step.
    fn run(&mut self, actions: &[String]) {
        let newest = self.open.last_mut().expect("a desktop window is open");
        let host = stack_of(&newest.screen).current().unwrap().clone();
        let call = match actions {
            [one] if one.ends_with(" in new window") => {
                let window = one.trim_start_matches("open ").trim_end_matches(" in new window");
                HandlerCall::new(NEW_WINDOW_CLICK, window, Vec::new())
            }
            _ => HandlerCall::new("click", &format!("do:{}", actions.join("|")), Vec::new()),
        };
        host.run(None, &call);
        self.settle();
    }

    /// Each desktop window that is no dialog with its windows, bottom to top, the dialogs opened from it on top.
    fn shown(&self) -> Vec<String> {
        self.open
            .iter()
            .filter(|d| !d.dialog)
            .map(|d| {
                let stack = stack_of(&d.screen);
                let mut windows = stack.windows();
                for over in self.open.iter().filter(|o| o.dialog && o.opener.is_some_and(|op| stack.holds(op))) {
                    windows.extend(stack_of(&over.screen).windows());
                }
                windows.join(", ")
            })
            .collect()
    }
}

fn strings(value: &Value) -> Vec<String> {
    value.as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect()
}

#[test]
fn every_case_opens_and_closes_its_windows_the_same_way() {
    let cases: Value = serde_json::from_str(include_str!("../../../../conformance/windows.json")).unwrap();
    let cases = cases["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let name = case["name"].as_str().unwrap();
        // Every case's dialog is Ask, as the model says.
        assert!(strings(&case["dialogs"]).iter().all(|d| d == "Ask"), "{name}");
        let program = Arc::new(Cases { start: case["start"].as_str().unwrap().to_string(), log: Mutex::new(Vec::new()) });
        let mut desktops = Desktops::start(program.clone());
        for step in case["steps"].as_array().unwrap() {
            desktops.run(&strings(step));
        }
        let expect = &case["expect"];
        let wanted: Vec<String> = expect["desktops"].as_array().unwrap().iter().map(|d| strings(d).join(", ")).collect();
        assert_eq!(wanted, desktops.shown(), "{name}: desktop windows");
        assert_eq!(strings(&expect["events"]), program.log.lock().unwrap().clone(), "{name}: events");
        assert_eq!(expect["ended"].as_bool().unwrap(), desktops.ended, "{name}: ended");
    }
}
