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
  <Button name="save" at="1,3" span="3" caption="Save" key="Ctrl+S" default="true" undoScope="save" onclick="save" />
  <Button name="users" at="4,3" span="3" caption="Users" opens="Users" />
  <Button name="warn" at="7,3" span="3" caption="Warn" onclick="warn" />
  <Button name="fail" at="10,3" span="3" caption="Fail" onclick="fail" />
  <Button name="digit" at="1,4" span="3" caption="7" onclick="digit(7)" />
  <Button name="twoErrors" at="4,4" span="3" caption="Errors" onclick="twoErrors" />
  <Button name="toasts" at="7,4" span="3" caption="Toasts" onclick="toasts" />
</Window>
"#;

const USERS: &str = r#"<Window name="Users">
  <Heading name="usersTitle" at="1,1" span="12" caption="Users" />
  <Button name="back" at="1,2" span="3" caption="Back" cancel="true" onclick="done" />
</Window>
"#;

const MODEL: &str = r#"{"project": {"locale": {"language": "en"}, "translations": {"en": {"countCaption": "How many"}}}, "icons": {"check": "M 20 6 L 9 17 L 4 12"}}"#;

const FILES: AppFiles = AppFiles { windows: &[("Main", MAIN), ("Users", USERS)], apps: &[], themes: &[], model: MODEL, resources: &[], title: "Orders", width: 800.0, height: 500.0 };

/// A program that logs every call and answers by the handler.
#[derive(Default)]
struct Fake {
    log: Mutex<Vec<String>>,
}

fn reply(state: WindowState) -> Reply {
    Reply { state, messages: Vec::new(), shown: Default::default(), open: Vec::new(), close: false, asks: Vec::new(), result: None, hears: Vec::new(), copy: None, step: 0, refused: false, quit: false, keys: false }
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
        Started { main: "Main".into(), unconnected: None, signin: None }
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
        r.messages.push(Message { level: Level::Info, text: Text::plain("Users closed"), toast: None });
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
                r.messages.push(Message { level: Level::Info, text: Text::plain("Saved"), toast: None });
                Ok(r)
            }
            "warn" => {
                let mut r = reply(state);
                r.messages.push(Message { level: Level::Warning, text: Text::plain("Careful"), toast: None });
                Ok(r)
            }
            "fail" => Err(Text::plain("That did not work")),
            // Two toasts the project declares, each with its own look and lifespan.
            "toasts" => {
                let mut r = reply(state);
                let saved = clarity_runtime::ui::Toast { name: "saved".into(), level: "success".into(), lifespan: Some(3000), icon: Some("check".into()), corner: "bottomRight".into() };
                let imported = clarity_runtime::ui::Toast { name: "imported".into(), level: "warning".into(), lifespan: Some(8000), icon: None, corner: "topLeft".into() };
                r.messages.push(Message { level: Level::Info, text: Text::plain("Saved Ada"), toast: Some(saved) });
                r.messages.push(Message { level: Level::Info, text: Text::plain("Imported 3"), toast: Some(imported) });
                Ok(r)
            }
            "twoErrors" => {
                let mut r = reply(state);
                r.messages.push(Message { level: Level::Error, text: Text::plain("one"), toast: None });
                r.messages.push(Message { level: Level::Error, text: Text::plain("two"), toast: None });
                Ok(r)
            }
            "done" => {
                let mut r = reply(state);
                r.close = true;
                Ok(r)
            }
            "openUsers" => {
                let mut r = reply(state);
                r.open.push(OpenRequest { window: "Users".into(), with: Map::new(), new_window: false });
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
    assert_eq!(vec!["Saved".to_string()], stack(&screen).toasts().iter().map(|m| m.text.show()).collect::<Vec<_>>());
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
fn an_info_message_is_a_note_that_goes_by_itself() {
    let (mut screen, _) = started();
    // The screen's clock, which the test moves on.
    let now = std::time::Instant::now();
    let clock = std::rc::Rc::new(std::cell::Cell::new(now));
    let read = clock.clone();
    screen.now = Box::new(move || read.get());
    click(&mut screen, "save");
    assert_eq!(vec!["Saved".to_string()], stack(&screen).toasts().iter().map(|m| m.text.show()).collect::<Vec<_>>());
    clock.set(now + std::time::Duration::from_secs(4));
    screen.wake_due(clock.get());
    assert_eq!(1, stack(&screen).toasts().len());
    clock.set(now + clarity_ui::window::messages::INFO_DURATION);
    screen.wake_due(clock.get());
    assert!(stack(&screen).toasts().is_empty());
    assert!(stack(&screen).said().is_empty());
}

/// Toasts the project declares: each in its own corner, with its own
/// lifespan, its level's color and its icon; a warning toast is a toast,
/// not a dialog.
#[test]
fn a_declared_toast_shows_in_its_corner_for_its_lifespan() {
    let (mut screen, _) = started();
    let now = std::time::Instant::now();
    let clock = std::rc::Rc::new(std::cell::Cell::new(now));
    let read = clock.clone();
    screen.now = Box::new(move || read.get());
    click(&mut screen, "toasts");
    assert_eq!(stack(&screen).toasts().iter().map(|m| m.text.show()).collect::<Vec<_>>(), ["Saved Ada", "Imported 3"]);
    assert!(stack(&screen).said().is_empty(), "a warning toast shows as a toast");
    screen.resize(800.0, 500.0, 1.0);
    let list = screen.find::<clarity_ui::window::ToastList>().unwrap();
    let places = list.places();
    let (saved, imported) = (places[0], places[1]);
    assert!(saved.right() > 700.0 && saved.bottom() > 400.0, "saved shows at the bottom right: {saved:?}");
    assert!(imported.x < 100.0 && imported.y < 100.0, "imported shows at the top left: {imported:?}");
    assert_eq!(list.colors(), ["success", "warning"]);
    assert_eq!(list.icons(), [true, false], "saved shows its icon");
    clock.set(now + std::time::Duration::from_millis(3000));
    screen.wake_due(clock.get());
    assert_eq!(stack(&screen).toasts().iter().map(|m| m.text.show()).collect::<Vec<_>>(), ["Imported 3"], "saved goes after 3 seconds");
    clock.set(now + std::time::Duration::from_millis(8000));
    screen.wake_due(clock.get());
    assert!(stack(&screen).toasts().is_empty(), "imported after 8");
}

#[test]
fn a_note_closes_with_its_x() {
    let (mut screen, _) = started();
    click(&mut screen, "save");
    Shot::of(&mut screen, 800, 500, 1.0);
    let list = clarity_ui::Element::node(screen.find::<clarity_ui::window::messages::ToastList>().unwrap()).bounds;
    // The x sits at the note's top right: 30 in from its right edge, 6 down, 24 square.
    let (x, y) = (list.right() - 30.0 + 12.0, list.y + 6.0 + 12.0);
    screen.pointer_pressed(x, y, clarity_ui::input::PointerButton::Left, 1);
    screen.pointer_released(x, y, clarity_ui::input::PointerButton::Left);
    assert!(stack(&screen).toasts().is_empty());
}

#[test]
fn errors_are_said_one_at_a_time() {
    let (mut screen, _) = started();
    click(&mut screen, "twoErrors");
    let says = |screen: &Screen| screen.find::<SaidDialog>().map(|d| d.message.text.show());
    assert_eq!(1, screen.overlays().len());
    assert_eq!(Some("one".to_string()), says(&screen));
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(1, screen.overlays().len());
    assert_eq!(Some("two".to_string()), says(&screen));
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert!(screen.overlays().is_empty());
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
    assert_eq!(vec!["Users closed".to_string()], stack(&screen).toasts().iter().map(|m| m.text.show()).collect::<Vec<_>>());
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
    clarity_ui::parts::change_state(&*host, "save", |s| s.text = Some("x".into()));
    host.edited("save");
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!(vec!["edit save save None"], fake.calls());
}
