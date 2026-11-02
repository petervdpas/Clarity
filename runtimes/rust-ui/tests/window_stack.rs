//! The windows of a program on Clarity.UI over a program's `Program`: the
//! main window opens with its parts, a click runs its handler through the
//! program and the reply's state redraws, an edit of an input is one edit,
//! messages show (an info note, a warning in Clarity's dialog), windows open
//! in place and close back to the one below, and Ctrl+Z asks the program to
//! take back the edit of the focused part's scope.

use std::sync::{Arc, Mutex};

use clarity_runtime::program::{self, Answer, Program, Started};
use clarity_runtime::ui::{Level, Message, OpenRequest, RefOption, Reply, Text, WindowState};
use clarity_ui::element;
use clarity_ui::input::{Key, KeyInput, Modifiers};
use clarity_ui::parts::label::LabelPart;
use clarity_ui::testing::{part_named, Shot};
use clarity_ui::window::{SaidDialog, WindowStack};
use clarity_ui::{AppFiles, Screen};
use serde_json::{json, Map, Value};

const MAIN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Window name="Main">
  <Heading name="title" at="1,1" span="12" caption="Orders" />
  <Label name="count" at="1,2" span="6" caption="@countCaption" />
  <Button name="save" at="1,3" span="3" caption="Save" key="Ctrl+S" default="true" onclick="save" />
  <Button name="users" at="4,3" span="3" caption="Users" opens="Users" />
  <Button name="warn" at="7,3" span="3" caption="Warn" onclick="warn" />
  <Button name="fail" at="10,3" span="3" caption="Fail" onclick="fail" />
  <Button name="digit" at="1,4" span="3" caption="7" onclick="digit(7)" />
</Window>
"#;

const USERS: &str = r#"<Window name="Users">
  <Heading name="usersTitle" at="1,1" span="12" caption="Users" />
  <Button name="back" at="1,2" span="3" caption="Back" cancel="true" onclick="done" />
</Window>
"#;

const MODEL: &str = r#"{"project": {"locale": {"language": "en"}, "translations": {"en": {"countCaption": "How many"}}}}"#;

const FILES: AppFiles = AppFiles { windows: &[("Main", MAIN), ("Users", USERS)], apps: &[], themes: &[], model: MODEL, resources: &[], title: "Orders", width: 800.0, height: 500.0 };

/// A program that logs every call and answers by the handler.
#[derive(Default)]
struct Fake {
    log: Mutex<Vec<String>>,
}

fn reply(state: WindowState) -> Reply {
    Reply { state, messages: Vec::new(), shown: Default::default(), open: Vec::new(), close: false, asks: Vec::new(), result: None, hears: Vec::new(), copy: None, step: 0, refused: false }
}

impl Fake {
    fn said(&self, what: String) {
        self.log.lock().unwrap().push(what);
    }

    fn calls(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.lock().unwrap())
    }
}

impl Program for Fake {
    fn started(&self) -> Started {
        Started { main: "Main".into(), unconnected: None }
    }
    fn start_app(&self) -> Answer<Reply> {
        self.said("start".into());
        Ok(reply(WindowState::new("", Map::new())))
    }
    fn open_window(&self, window: &str, with: Map<String, Value>) -> Answer<Reply> {
        self.said(format!("open {window}"));
        let mut state = WindowState::new(window, with);
        state.data.insert("saved".into(), json!(0));
        Ok(reply(state))
    }
    fn close_window(&self, state: WindowState) -> Answer<Reply> {
        self.said(format!("close {}", state.window));
        let mut r = reply(state);
        r.messages.push(Message { level: Level::Info, text: Text::plain("Users closed") });
        Ok(r)
    }
    fn return_to(&self, state: WindowState) -> Answer<Reply> {
        self.said(format!("return {}", state.window));
        Ok(reply(state))
    }
    fn dispatch_event(&self, mut state: WindowState, handler: &str) -> Answer<Reply> {
        self.said(format!("dispatch {handler}"));
        match handler {
            "save" => {
                let saved = state.data["saved"].as_i64().unwrap_or(0) + 1;
                state.data.insert("saved".into(), json!(saved));
                state.parts.entry("count".into()).or_default().caption = Some(Text::plain(format!("Saved {saved}")));
                let mut r = reply(state);
                r.messages.push(Message { level: Level::Info, text: Text::plain("Saved") });
                Ok(r)
            }
            "warn" => {
                let mut r = reply(state);
                r.messages.push(Message { level: Level::Warning, text: Text::plain("Careful") });
                Ok(r)
            }
            "fail" => Err(Text::plain("That did not work")),
            "done" => {
                let mut r = reply(state);
                r.close = true;
                Ok(r)
            }
            "openUsers" => {
                let mut r = reply(state);
                r.open.push(OpenRequest { window: "Users".into(), with: Map::new() });
                Ok(r)
            }
            _ => Ok(reply(state)),
        }
    }
    fn edit_event(&self, state: WindowState, edit: program::Edit) -> Answer<Reply> {
        self.said(format!("edit {} {} {:?}", edit.part, edit.scope, edit.handler));
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

fn started() -> (Screen, Arc<Fake>) {
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let stack = WindowStack::start(program, FILES);
    let mut screen = Screen::new(Box::new(stack), FILES.look(false));
    screen.resize(800.0, 500.0, 1.0);
    (screen, fake)
}

fn stack(screen: &Screen) -> &WindowStack {
    screen.root().as_any().downcast_ref::<WindowStack>().unwrap()
}

fn click(screen: &mut Screen, name: &str) {
    let id = part_named(screen, name);
    let b = screen.element(id).unwrap().node().bounds;
    screen.click(b.x + b.width / 2.0, b.y + b.height / 2.0);
}

#[test]
fn the_program_starts_and_its_main_window_shows_its_parts() {
    let (mut screen, fake) = started();
    assert_eq!(vec!["start", "open Main"], fake.calls());
    assert_eq!(vec!["Main"], stack(&screen).windows());
    assert_eq!("Orders", stack(&screen).title());
    assert_eq!(Some((800.0, 500.0)), stack(&screen).wanted_size());
    // A text written as a key shows in the language loaded from the model.
    let count = part_named(&screen, "count");
    assert_eq!("How many", screen.element(count).unwrap().as_any().downcast_ref::<LabelPart>().unwrap().text());
    let mut shot = Shot::of(&mut screen, 800, 500, 1.0);
    shot.save("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/window-stack.png").unwrap();
    // The window keeps its padding: the parts start 12 in.
    let heading = screen.element(part_named(&screen, "title")).unwrap().node().bounds;
    assert_eq!((12.0, 12.0), (heading.x, heading.y));
}

#[test]
fn a_click_runs_its_handler_through_the_program_and_the_reply_redraws() {
    let (mut screen, fake) = started();
    fake.calls();
    click(&mut screen, "save");
    assert_eq!(vec!["dispatch save"], fake.calls());
    let host = stack(&screen).current().unwrap().clone();
    assert_eq!(json!(1), host.state_now().data["saved"]);
    // What the handler set on a part's state shows: the label's caption.
    let count = part_named(&screen, "count");
    assert_eq!("Saved 1", screen.element(count).unwrap().as_any().downcast_ref::<LabelPart>().unwrap().text());
    // Its info message shows as a note.
    assert_eq!(vec!["Saved".to_string()], stack(&screen).notes().iter().map(|m| m.text.show()).collect::<Vec<_>>());
    // A call with values is sent as Clarity writes it.
    click(&mut screen, "digit");
    assert_eq!(vec!["dispatch digit(7)"], fake.calls());
}

#[test]
fn a_shortcut_and_enter_press_their_buttons() {
    let (mut screen, fake) = started();
    fake.calls();
    screen.key_pressed(KeyInput::new(Key::S, Modifiers::CONTROL));
    assert_eq!(vec!["dispatch save"], fake.calls());
    // Enter presses the default button.
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(vec!["dispatch save"], fake.calls());
}

#[test]
fn a_warning_shows_in_clarity_s_dialog_and_ok_takes_it_away() {
    let (mut screen, _) = started();
    click(&mut screen, "warn");
    assert_eq!(1, screen.overlays().len());
    assert!(screen.find::<SaidDialog>().is_some());
    let mut shot = Shot::of(&mut screen, 800, 500, 1.0);
    shot.save("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/window-warning.png").unwrap();
    // The OK has the focus: Enter closes the dialog, and the window's keys wait while it is open.
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert!(screen.overlays().is_empty());
    assert!(stack(&screen).said().is_empty());
}

#[test]
fn a_handler_that_fails_says_why_as_an_error() {
    let (mut screen, _) = started();
    click(&mut screen, "fail");
    let said = stack(&screen).said();
    assert_eq!(1, said.len());
    assert_eq!((Level::Error, "That did not work".to_string()), (said[0].level, said[0].text.show()));
}

#[test]
fn a_window_opens_in_place_and_closes_back_to_the_one_below() {
    let (mut screen, fake) = started();
    fake.calls();
    click(&mut screen, "users");
    assert_eq!(vec!["open Users"], fake.calls());
    assert_eq!(vec!["Main", "Users"], stack(&screen).windows());
    assert!(element::find_type::<clarity_ui::parts::heading::HeadingPart>(screen.root(), |h| h.text() == "Users").is_some());
    // Escape presses the cancel button, whose handler closes the window.
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert_eq!(vec!["dispatch done", "close Users", "return Main"], fake.calls());
    assert_eq!(vec!["Main"], stack(&screen).windows());
    part_named(&screen, "save");
    // What the closing window said shows in the window below.
    assert_eq!(vec!["Users closed".to_string()], stack(&screen).notes().iter().map(|m| m.text.show()).collect::<Vec<_>>());
}

#[test]
fn closing_the_last_window_ends_the_program() {
    let (mut screen, _) = started();
    click(&mut screen, "users");
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(!stack(&screen).ended());
    screen.act_root::<WindowStack, _>(|_, _| ());
    let host = stack(&screen).current().unwrap().clone();
    // A reply that closes the main window.
    let mut r = reply(host.state_now());
    r.close = true;
    host.apply(r);
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert!(stack(&screen).ended());
}

#[test]
fn ctrl_z_asks_the_program_to_take_back_the_focused_part_s_scope() {
    let (mut screen, fake) = started();
    let save = part_named(&screen, "save");
    screen.focus(Some(save));
    fake.calls();
    screen.key_pressed(KeyInput::new(Key::Z, Modifiers::CONTROL));
    screen.key_pressed(KeyInput::new(Key::Y, Modifiers::CONTROL));
    screen.key_pressed(KeyInput::new(Key::Z, Modifiers::CONTROL | Modifiers::SHIFT));
    assert_eq!(vec!["undo save true", "undo save false", "undo save false"], fake.calls());
}

#[test]
fn the_person_s_edit_of_an_unbound_part_is_one_edit_on_its_scope() {
    let (mut screen, fake) = started();
    fake.calls();
    let host = stack(&screen).current().unwrap().clone();
    use clarity_ui::PartHost;
    host.change_state("save", &mut |s| s.text = Some("x".into()));
    host.edited("save");
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!(vec!["edit save save None"], fake.calls());
}
