//! An app's commands for agents while it runs (docs/ledger/app-mcp.md):
//! Clarity.UI listens for the app; each grant and each call goes to the
//! screen's thread as work, answered there; a command's question goes back
//! to the agent while the screen's thread waits.

use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

use clarity_app_mcp::{Decision, Kind, Lines, relay};
use clarity_runtime::program::{self, AgentRan, AgentsOffered, Answer, Program, Started};
use clarity_runtime::ui::{RefOption, Reply, WindowState};
use clarity_ui::agents::{self, AgentWork, GrantAsked};
use clarity_ui::element::Element;
use serde_json::{Map, Value, json};

const TOOLS: &str = r#"{"name":"shop","instructions":"Keeps customers.","tools":[
{"name":"customer-add","description":"Adds a customer","inputSchema":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":false},"changes":{"data":true,"files":false,"settings":false,"net":false},"allow":false,"line":["customer","add"],"arguments":["name"]},
{"name":"customer-remove","description":"Removes a customer","inputSchema":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":false},"changes":{"data":true,"files":false,"settings":false,"net":false},"allow":false,"line":["customer","remove"],"arguments":["name"]}]}"#;

/// An app offering two commands to agents; it keeps the lines it ran.
#[derive(Default)]
struct Offering {
    ran: Mutex<Vec<Vec<String>>>,
    /// How often a window came back on top (`on return`), as after an agent's change.
    returned: Mutex<u32>,
    /// The steps an Undo took back.
    undone: Mutex<Vec<u64>>,
}

fn reply(state: WindowState) -> Reply {
    Reply { state, messages: Vec::new(), shown: Default::default(), open: Vec::new(), close: false, asks: Vec::new(), result: None, hears: Vec::new(), copy: None, step: 0, refused: false, quit: false, keys: false }
}

impl Program for Offering {
    fn started(&self) -> Started {
        Started { main: "Main".into(), unconnected: None, signin: None }
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
        *self.returned.lock().unwrap() += 1;
        Ok(reply(state))
    }
    fn dispatch_event(&self, state: WindowState, _handler: &str) -> Answer<Reply> {
        Ok(reply(state))
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
    fn agent_undo(&self, step: u64) -> Answer<()> {
        self.undone.lock().unwrap().push(step);
        Ok(())
    }
    fn agents(&self) -> Option<AgentsOffered> {
        Some(AgentsOffered { tools: TOOLS.into(), app: "Shop".into(), program: "shop".into(), version: "1.0".into() })
    }
    fn agent_call(&self, line: &[String], ask: &mut dyn FnMut(&str, &str) -> Option<String>) -> AgentRan {
        self.ran.lock().unwrap().push(line.to_vec());
        if line[1] == "remove" {
            return match ask("Remove Ada?", r#"{"kind":"confirm","yes":"Remove","no":"Keep"}"#).as_deref() {
                Some("true") => AgentRan { text: "Removed Ada".into(), refused: false, step: 0 },
                _ => AgentRan { text: "Kept Ada".into(), refused: true, step: 0 },
            };
        }
        AgentRan { text: format!("Added {}", line.last().unwrap()), refused: false, step: 7 }
    }
}

/// The app listening, its screen's thread a thread that answers each grant
/// with the next of `decisions` and runs each call; what it was asked.
fn listening(dir: &std::path::Path, decisions: Vec<Decision>) -> (relay::Listening, Arc<Offering>, Arc<Mutex<Vec<GrantAsked>>>) {
    let app = Arc::new(Offering::default());
    let program: Arc<dyn Program> = app.clone();
    let asked: Arc<Mutex<Vec<GrantAsked>>> = Arc::default();
    let (post, work) = channel::<AgentWork>();
    let seen = asked.clone();
    std::thread::spawn(move || {
        let mut decisions = decisions.into_iter();
        for each in work {
            match each {
                AgentWork::Grant { asked, reply, .. } => {
                    seen.lock().unwrap().push(asked);
                    let _ = reply.send(decisions.next().unwrap_or(Decision::Refuse));
                }
                AgentWork::Call { line, said, .. } => {
                    agents::run_call(program.as_ref(), &line, &said);
                }
                AgentWork::Changed | AgentWork::TurnedAway(_) => {}
            }
        }
    });
    let post = Mutex::new(post);
    let offered = app.agents().unwrap();
    let file = clarity_app_mcp::discovery::file_in(dir);
    let listening = agents::listen_at(&offered, &file, Arc::new(move |work| {
        let _ = post.lock().unwrap().send(work);
    }))
    .expect("it listens");
    (listening, app, asked)
}

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-ui-agents-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn said(reply: &str) -> (String, bool) {
    let reply: Value = serde_json::from_str(reply).unwrap();
    (reply["result"]["content"][0]["text"].as_str().unwrap_or_else(|| panic!("{reply}")).to_string(), reply["result"]["isError"].as_bool().unwrap())
}

#[test]
fn a_change_is_asked_of_the_person_and_runs_on_the_screen_s_thread() {
    let dir = folder("granted");
    let (listening, app, asked) = listening(&dir, vec![Decision::Once, Decision::Refuse]);
    let mut lines = relay::connect(&clarity_app_mcp::discovery::file_in(&dir)).expect("the app answers");
    lines.write(&json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "clientInfo": { "name": "Claude" }, "capabilities": {} } }).to_string());
    let init: Value = serde_json::from_str(&lines.read().unwrap()).unwrap();
    assert_eq!(init["result"]["instructions"], json!("Keeps customers."));
    lines.write(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "customer-add", "arguments": { "name": "Ada" } } }).to_string());
    assert_eq!(said(&lines.read().unwrap()), ("Added Ada".to_string(), false));
    lines.write(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "customer-add", "arguments": { "name": "Bob" } } }).to_string());
    assert_eq!(said(&lines.read().unwrap()), ("The person using Shop refused this.".to_string(), true));
    let asked = asked.lock().unwrap().clone();
    assert_eq!(asked.len(), 2);
    assert_eq!(asked[0].title("Shop"), "Claude wants to change Shop");
    assert_eq!((asked[0].kind, asked[0].description.as_str(), asked[0].arguments.clone()), (Kind::Change, "Adds a customer", vec![("name".to_string(), "Ada".to_string())]));
    assert_eq!(*app.ran.lock().unwrap(), vec![vec!["customer".to_string(), "add".into(), "--".into(), "Ada".into()]], "the refused call ran nothing");
    drop(listening);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_command_s_question_goes_to_the_agent_while_the_screen_waits() {
    let dir = folder("asked");
    let (listening, _app, _asked) = listening(&dir, vec![Decision::Session]);
    let mut lines = relay::connect(&clarity_app_mcp::discovery::file_in(&dir)).expect("the app answers");
    lines.write(&json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "capabilities": { "elicitation": {} } } }).to_string());
    let _ = lines.read();
    lines.write(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "customer-remove", "arguments": { "name": "Ada" } } }).to_string());
    let question: Value = serde_json::from_str(&lines.read().unwrap()).unwrap();
    assert_eq!((question["method"].clone(), question["params"]["message"].clone()), (json!("elicitation/create"), json!("Remove Ada?")));
    lines.write(&json!({ "jsonrpc": "2.0", "id": question["id"], "result": { "action": "accept", "content": { "answer": true } } }).to_string());
    assert_eq!(said(&lines.read().unwrap()), ("Removed Ada".to_string(), false));
    drop(listening);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_unnamed_agent_is_called_an_agent() {
    let grant = clarity_app_mcp::Grant { agent: "", tool: "customer-count", description: "Counts", kind: Kind::Read, arguments: &json!({}), changes: &json!({}) };
    assert_eq!(GrantAsked::of(&grant).title("Shop"), "An agent wants to read Shop");
}

const MAIN: &str = r#"<Window name="Main">
  <Label name="hello" at="1,1" span="3" caption="Hello" />
</Window>
"#;

const FILES: clarity_ui::AppFiles = clarity_ui::AppFiles { windows: &[("Main", MAIN)], apps: &[], themes: &[], model: "{}", resources: &[], title: "Shop", width: 800.0, height: 500.0 };

/// A screen with the app's window, and the grant `title` asked over it; what the person chooses goes to the cell.
fn granting(title: &str) -> (clarity_ui::Screen, std::rc::Rc<std::cell::Cell<Option<clarity_ui::dialogs::GrantChoice>>>) {
    granting_lines(title, vec!["Adds a customer".into(), "name: Ada".into()])
}

/// As `granting`, with the grant's own lines (what the command does, then each value given).
fn granting_lines(title: &str, lines: Vec<String>) -> (clarity_ui::Screen, std::rc::Rc<std::cell::Cell<Option<clarity_ui::dialogs::GrantChoice>>>) {
    let program: Arc<dyn Program> = Arc::new(Offering::default());
    let mut screen = clarity_ui::Screen::new(Box::new(clarity_ui::window::WindowStack::start(program, FILES)), FILES.look(false));
    screen.resize(800.0, 500.0, 1.0);
    let chosen = std::rc::Rc::new(std::cell::Cell::new(None));
    let given = chosen.clone();
    let title = title.to_string();
    screen.act_root::<clarity_ui::window::WindowStack, _>(|stack, cx| stack.ask_grant(title, lines, Box::new(move |choice| given.set(Some(choice))), cx));
    (screen, chosen)
}

fn dialog(screen: &clarity_ui::Screen) -> &clarity_ui::dialogs::GrantDialog {
    screen.find::<clarity_ui::dialogs::GrantDialog>().expect("the grant dialog shows")
}

#[test]
fn the_person_allows_once_in_clarity_s_dialog() {
    let (mut screen, chosen) = granting("Claude wants to change Shop");
    let words: Vec<String> = dialog(&screen).buttons().iter().map(|b| b.text.clone()).collect();
    assert_eq!(words, ["Refuse", "Allow for this session", "Allow once"]);
    assert_eq!(screen.focused(), Some(dialog(&screen).first()), "Refuse has the focus");
    let once = dialog(&screen).buttons()[2].node().bounds;
    screen.click(once.x + once.width / 2.0, once.y + once.height / 2.0);
    assert_eq!(chosen.get(), Some(clarity_ui::dialogs::GrantChoice::Once));
    assert!(screen.find::<clarity_ui::dialogs::GrantDialog>().is_none(), "it closed");
}

#[test]
fn escape_refuses() {
    let (mut screen, chosen) = granting("Claude wants to read Shop");
    screen.key_pressed(clarity_ui::input::KeyInput::plain(clarity_ui::input::Key::Escape));
    assert_eq!(chosen.get(), Some(clarity_ui::dialogs::GrantChoice::Refuse));
}

/// A value as long as a review's notes: the dialog stays as large as a
/// window's dialog is, its buttons on the screen, and the lines scroll in it.
fn long_notes() -> Vec<String> {
    let notes: Vec<String> = (1..=120).map(|n| format!("- note line {n}: what the entry says against what the code does")).collect();
    vec!["Keeps the review of one entry".into(), format!("notes: {}", notes.join("\n"))]
}

#[test]
fn a_long_value_scrolls_inside_a_dialog_that_keeps_its_size() {
    let (screen, _) = granting_lines("Claude wants to change Shop", long_notes());
    let d = dialog(&screen);
    let panel = d.panel();
    assert!(panel.y >= 0.0 && panel.bottom() <= 500.0, "the dialog stays on the screen: {panel:?}");
    assert!(panel.height <= 420.0, "the dialog keeps about a dialog's size: {panel:?}");
    for b in d.buttons() {
        let r = b.node().bounds;
        assert!(r.y >= panel.y && r.bottom() <= panel.bottom(), "{} shows in the dialog: {r:?} in {panel:?}", b.text);
    }
    let text = d.text();
    assert!(text.max_scroll() > 0.0, "the lines are more than the dialog shows");
    assert_eq!(text.scroll_y(), 0.0, "it opens at the top");
    let area = text.node().bounds;
    assert!(area.y >= panel.y && area.bottom() <= panel.bottom(), "the lines' area is inside the dialog");
}

#[test]
fn the_wheel_and_the_keys_scroll_the_lines_and_refuse_keeps_the_focus() {
    let (mut screen, chosen) = granting_lines("Claude wants to change Shop", long_notes());
    let area = dialog(&screen).text().node().bounds;
    screen.wheel(area.x + 20.0, area.y + 20.0, 0.0, 60.0, clarity_ui::input::Modifiers::NONE);
    assert_eq!(dialog(&screen).text().scroll_y(), 60.0, "the wheel scrolls the lines");
    screen.key_pressed(clarity_ui::input::KeyInput::plain(clarity_ui::input::Key::End));
    let text = dialog(&screen).text();
    assert_eq!(text.scroll_y(), text.max_scroll(), "End goes to the last line");
    screen.key_pressed(clarity_ui::input::KeyInput::plain(clarity_ui::input::Key::Home));
    assert_eq!(dialog(&screen).text().scroll_y(), 0.0, "Home goes to the first");
    screen.key_pressed(clarity_ui::input::KeyInput::plain(clarity_ui::input::Key::PageDown));
    assert!(dialog(&screen).text().scroll_y() > 0.0, "Page Down scrolls");
    assert_eq!(screen.focused(), Some(dialog(&screen).first()), "Refuse keeps the focus");
    assert_eq!(chosen.get(), None, "no key chose anything");
}

#[test]
fn a_short_grant_shows_all_its_lines_without_scrolling() {
    let (screen, _) = granting("Claude wants to change Shop");
    assert_eq!(dialog(&screen).text().max_scroll(), 0.0);
}

/// The program across the wire, as every stack's desktop program is: Clarity.UI's
/// `program_wire` sends each call, the runtime's `wire::answer` answers it.
struct Across(Arc<Offering>);

impl clarity_ui::program_wire::Asks for Across {
    fn ask(&self, command: &str, args: Map<String, Value>) -> Option<Value> {
        Some(clarity_runtime::wire::answer(self.0.as_ref(), command, Value::Object(args)))
    }
}

#[test]
fn the_commands_for_agents_cross_the_wire_with_their_questions() {
    // The program asks the agent through Clarity.UI, as the C interface's clarity_agent_ask does.
    let _ = clarity_runtime::wire::AGENT_ASK.set(clarity_ui::program_wire::ask_agent);
    let app = Arc::new(Offering::default());
    let program = clarity_ui::program_wire::WireProgram::new(Across(app.clone()));
    assert_eq!(program.agents().map(|o| o.app), Some("Shop".to_string()));
    let mut asked = Vec::new();
    let line: Vec<String> = ["customer", "remove", "--", "Ada"].map(String::from).to_vec();
    let ran = program.agent_call(&line, &mut |question, form| {
        asked.push((question.to_string(), form.to_string()));
        Some("true".to_string())
    });
    assert_eq!((ran.text.as_str(), ran.refused), ("Removed Ada", false));
    assert_eq!(asked, [("Remove Ada?".to_string(), r#"{"kind":"confirm","yes":"Remove","no":"Keep"}"#.to_string())]);
    assert_eq!(clarity_ui::program_wire::ask_agent("Anyone?", "{}"), None, "nobody asks once the call is done");
}

#[test]
fn an_agent_s_change_shows_in_the_open_window_with_a_note() {
    let app = Arc::new(Offering::default());
    let program: Arc<dyn Program> = app.clone();
    let mut screen = clarity_ui::Screen::new(Box::new(clarity_ui::window::WindowStack::start(program, FILES)), FILES.look(false));
    screen.resize(800.0, 500.0, 1.0);
    let before = *app.returned.lock().unwrap();
    screen.act_root::<clarity_ui::window::WindowStack, _>(|stack, cx| stack.agent_changed("Claude: Added the project Grace.".into(), 7, cx));
    assert_eq!(*app.returned.lock().unwrap(), before + 1, "the window read its data again");
    let notes = screen.find::<clarity_ui::window::WindowStack>().unwrap().toasts();
    assert!(notes.iter().any(|n| n.text.show() == "Claude: Added the project Grace."), "{notes:?}");
    // The note's Undo takes the change's step back, and the window reads its data again.
    let list = screen.find::<clarity_ui::window::ToastList>().unwrap();
    assert_eq!(list.actions().last().cloned().flatten().as_deref(), Some("Undo"));
    let index = list.actions().len() - 1;
    let id = screen.find::<clarity_ui::window::ToastList>().map(|n| n.node().id()).unwrap();
    screen.act(id, |element, cx| {
        if let Some(list) = element.as_any_mut().downcast_mut::<clarity_ui::window::ToastList>() {
            list.act(index, cx);
        }
    });
    screen.act_root::<clarity_ui::window::WindowStack, _>(|stack, cx| clarity_ui::element::Element::after_event(stack, cx));
    assert_eq!(*app.undone.lock().unwrap(), vec![7], "the change's step went back");
    assert_eq!(*app.returned.lock().unwrap(), before + 2, "and the window read its data again");
}

/// An agent's toasts have a lifespan: they go by themselves, so a run of
/// changes never fills the window; the Undo goes with the toast.
#[test]
fn an_agent_s_toasts_go_by_themselves() {
    let program: Arc<dyn Program> = Arc::new(Offering::default());
    let mut screen = clarity_ui::Screen::new(Box::new(clarity_ui::window::WindowStack::start(program, FILES)), FILES.look(false));
    screen.resize(800.0, 500.0, 1.0);
    let now = std::time::Instant::now();
    let clock = std::rc::Rc::new(std::cell::Cell::new(now));
    let read = clock.clone();
    screen.now = Box::new(move || read.get());
    screen.act_root::<clarity_ui::window::WindowStack, _>(|stack, cx| stack.agent_changed("Claude: Added the project Grace.".into(), 7, cx));
    screen.act_root::<clarity_ui::window::WindowStack, _>(|stack, cx| stack.agent_said("Codex was turned away.".into(), cx));
    let shown = |screen: &clarity_ui::Screen| screen.find::<clarity_ui::window::WindowStack>().unwrap().toasts().len();
    assert_eq!(shown(&screen), 2);
    clock.set(now + clarity_ui::window::messages::AGENT_TOAST_DURATION - std::time::Duration::from_secs(1));
    screen.wake_due(clock.get());
    assert_eq!(shown(&screen), 2, "they stay a while, to read and to Undo");
    clock.set(now + clarity_ui::window::messages::AGENT_TOAST_DURATION);
    screen.wake_due(clock.get());
    assert_eq!(shown(&screen), 0, "then they go by themselves");
}

#[test]
fn the_note_says_who_did_what() {
    let ran = AgentRan { text: "Added the project Grace.\nmore".into(), refused: false, step: 0 };
    assert_eq!(agents::change_toast("Claude", "project-add", &ran), "Claude: Added the project Grace.");
    let silent = AgentRan { text: String::new(), refused: false, step: 0 };
    assert_eq!(agents::change_toast("", "review-set", &silent), "An agent ran review-set");
}

/// One agent more than the app serves at once is refused, and the open app
/// says so in its window (`agents at most`, docs/ledger/app-mcp.md).
#[test]
fn an_agent_over_the_cap_is_refused_and_the_app_says_so() {
    let dir = folder("cap");
    let mut tools: serde_json::Value = serde_json::from_str(TOOLS).unwrap();
    tools["atMost"] = json!(1);
    let offered = AgentsOffered { tools: tools.to_string(), app: "Shop".into(), program: "shop".into(), version: "1.0".into() };
    let (post, work) = channel::<AgentWork>();
    let post = Mutex::new(post);
    let file = clarity_app_mcp::discovery::file_in(&dir);
    let listening = agents::listen_at(&offered, &file, Arc::new(move |w| {
        let _ = post.lock().unwrap().send(w);
    }))
    .expect("it listens");
    let mut first = relay::connect(&file).expect("the app answers");
    first.write(&json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "clientInfo": { "name": "Claude" }, "capabilities": {} } }).to_string());
    assert!(first.read().unwrap().contains("\"result\""));
    let mut second = relay::connect(&file).expect("the app answers");
    second.write(&json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "clientInfo": { "name": "Codex" }, "capabilities": {} } }).to_string());
    let refused: serde_json::Value = serde_json::from_str(&second.read().unwrap()).unwrap();
    assert_eq!(refused["error"]["message"], "Shop serves 1 AI agent at once, and that many are connected; try again when one is done.");
    let note = work.iter().find_map(|w| match w {
        AgentWork::TurnedAway(note) => Some(note),
        _ => None,
    });
    assert_eq!(note.as_deref(), Some("Codex was turned away: Shop serves 1 AI agent at once."));
    drop(listening);
}
