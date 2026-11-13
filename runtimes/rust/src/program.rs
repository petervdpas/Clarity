//! The program a generated application is (DESIGN 7.9): everything its screen
//! asks of it, as one trait in plain Rust types. The generated code
//! implements it once, on a struct over its databases. Clarity.UI calls it in
//! the program's own process on the desktop, with no JSON between them; the
//! web server's commands in a browser are thin calls into it.
//!
//! What goes wrong is [`Text`]: a refusal keeps its slots, which the screen
//! fills in the format of the person using the application; any other error
//! or a panic is its message as plain text. On the web server's wire it is
//! JSON.

use std::sync::Mutex;

use serde_json::{Map, Value};

use crate::canvas::CanvasArgs;
use crate::list_fields::ItemKind;
use crate::ranges::{Range, RangeMove};
use crate::reorder::Move;
use crate::steps::StepButton;
use crate::ui::{RefOption, Reply, Text, WindowState};
use crate::{Db, Error, Result};

/// What a call of the program answers: its result, or what went wrong as
/// the screen shows it.
pub type Answer<T> = std::result::Result<T, Text>;

/// What the screen starts from: the window the application opens with, and,
/// when the databases did not open, what to show first and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Started {
    /// The name of the main window.
    pub main: String,
    /// Set while the databases are not open (DESIGN 5.27).
    pub unconnected: Option<Unconnected>,
    /// The sign-in window, when people sign in before the main window
    /// opens (docs/APP-CAPABILITIES-PLAN.md step 4; `Db::signin_first`).
    pub signin: Option<String>,
}

/// What the screen shows when the databases did not open (DESIGN 5.27).
#[derive(Debug, Clone, PartialEq)]
pub struct Unconnected {
    /// The app's `on unconnected` handler, which runs with why.
    pub handler: Option<String>,
    /// Else the window that keeps a secret connection.
    pub window: Option<String>,
    /// Why the databases did not open, in Clarity's own words.
    pub problem: Text,
}

/// The person ended an edit of a part (DESIGN 5.29).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Edit {
    /// The part's `on change` handler, if any.
    pub handler: Option<String>,
    pub part: String,
    /// The undo scope the edit goes on.
    pub scope: String,
    /// False while a drag goes on; true at its drop and for any other edit.
    pub ends: bool,
    /// A readonly part's change is taken back and refused.
    pub readonly: bool,
    /// A table's list, whose changes outside `editable` are refused.
    pub path: Option<String>,
    pub editable: Option<Vec<String>>,
}

/// A row moved by a drag or a key (DESIGN 1.7).
#[derive(Debug, Clone, PartialEq)]
pub struct Reorder {
    pub part: String,
    pub scope: Option<String>,
    /// The data path of the list.
    pub list: String,
    /// The field that keeps the order.
    pub by: String,
    /// The field that keeps a tree row's parent.
    pub parent: Option<String>,
    pub moved: Move,
    pub handler: Option<String>,
    pub readonly: bool,
}

/// An item typed for a list field (DESIGN 1.7).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemEdit {
    pub part: String,
    pub scope: Option<String>,
    /// The data path of the list field.
    pub path: String,
    pub item: ItemKind,
    /// An enum's values, which an item must be one of.
    pub values: Vec<String>,
    pub typed: String,
    pub handler: Option<String>,
    pub readonly: bool,
}

/// A key, a drag or typed text on a slider or spin box (DESIGN 1.7).
#[derive(Debug, Clone, PartialEq)]
pub struct RangeEdit {
    pub part: String,
    pub scope: Option<String>,
    /// The data path of the value.
    pub path: String,
    pub range: Range,
    /// Whether the value is a whole number.
    pub whole: bool,
    pub moved: RangeMove,
    pub handler: Option<String>,
    pub readonly: bool,
}

/// The person's action on a canvas (DESIGN 5.31).
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasEdit {
    pub canvas: CanvasArgs,
    /// What the person did, as the canvas mode reads it.
    pub action: Map<String, Value>,
    pub scope: Option<String>,
    pub handler: Option<String>,
    pub readonly: bool,
}

/// Next, Back or Finish on a wizard (DESIGN 1.7).
#[derive(Debug, Clone, PartialEq)]
pub struct StepPress {
    pub part: String,
    pub button: StepButton,
    /// How many steps the wizard has.
    pub steps: i64,
    /// The handlers of Next and Finish.
    pub next: Option<String>,
    pub finish: Option<String>,
}

/// What an app offers agents while it runs (`Program::agents`).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentsOffered {
    /// The tool list, as clarity-gen writes it.
    pub tools: String,
    /// The app's title, as refusals say it.
    pub app: String,
    /// The name its settings are kept under: running.json goes beside them.
    pub program: String,
    pub version: String,
}

/// What a command an agent called did (`Program::agent_call`).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentRan {
    pub text: String,
    pub refused: bool,
    /// The database step the call's changes are, which `agent_undo` takes
    /// back; 0 when it changed nothing (or was refused, and its changes went).
    #[serde(default)]
    pub step: u64,
}

/// Everything the screen of a generated application asks of it. Each call
/// that answers a [`Reply`] gives the window's state as the program leaves
/// it: what each table, tree and chart shows filled in, and its edits on the
/// window's undo record.
pub trait Program: Send + Sync + 'static {
    /// The window the application opens with, and whether the databases opened.
    fn started(&self) -> Started;

    /// Runs the app's `on start`, before its main window opens.
    fn start_app(&self) -> Answer<Reply>;

    /// Opens window `window` with `with` filled in, running its `on open`.
    fn open_window(&self, window: &str, with: Map<String, Value>) -> Answer<Reply>;

    /// Runs a window's `on close` before the screen closes it; what it had of
    /// an engine goes, and its edits.
    fn close_window(&self, state: WindowState) -> Answer<Reply>;

    /// Runs a window's `on return` when it is on top again.
    fn return_to(&self, state: WindowState) -> Answer<Reply>;

    /// Runs a handler of the window or the app, as one undo step.
    fn dispatch_event(&self, state: WindowState, handler: &str) -> Answer<Reply>;

    /// The person ended an edit of a part: its handler runs as one step and
    /// the edit goes on the window's record.
    fn edit_event(&self, state: WindowState, edit: Edit) -> Answer<Reply>;

    /// Ctrl+Z (`back`) or Ctrl+Y on a scope: its last edit taken back or done again.
    fn undo_edit(&self, state: WindowState, scope: &str, back: bool) -> Answer<Reply>;

    /// A row moved by a drag or a key.
    fn reorder_event(&self, state: WindowState, reorder: Reorder) -> Answer<Reply>;

    /// An item typed for a list field.
    fn item_event(&self, state: WindowState, item: ItemEdit) -> Answer<Reply>;

    /// A slider's or spin box's value moved.
    fn range_event(&self, state: WindowState, range: RangeEdit) -> Answer<Reply>;

    /// The person's action on a canvas.
    fn canvas_event(&self, state: WindowState, canvas: CanvasEdit) -> Answer<Reply>;

    /// Next, Back or Finish on a wizard.
    fn step_event(&self, state: WindowState, step: StepPress) -> Answer<Reply>;

    /// The person sorted, turned a page, or opened or closed a tree row. Never an edit.
    fn view_event(&self, state: WindowState, part: &str, action: Map<String, Value>) -> Answer<Reply>;

    /// The answer to a question a handler asked: runs the handler it goes to,
    /// with the values written for it and the answer last.
    fn answer_event(&self, state: WindowState, handler: &str, values: Vec<Value>) -> Answer<Reply>;

    /// The records a reference field of `entity` can point at.
    fn ref_options(&self, entity: &str) -> Answer<Vec<RefOption>>;

    /// The values a text field already has, which a typed dropdown offers.
    fn field_values(&self, entity: &str, field: &str) -> Answer<Vec<String>>;

    /// What a dropdown of `kind` ("typed" or "search") offers for typed text:
    /// a field's values, or the records of an entity with the fields it shows
    /// (crate::dropdown_options::find).
    fn find_options(&self, kind: &str, entity: &str, field: Option<&str>, show: &[String], typed: &str) -> Answer<Value>;

    /// The bytes of an image or file field of a record; none when it has none.
    fn field_bytes(&self, entity: &str, id: i64, field: &str) -> Answer<Option<Vec<u8>>>;

    /// A file of a store: a picture linked from a file before its record is saved.
    fn store_file(&self, store: &str, path: &str) -> Answer<Vec<u8>>;

    /// The settings the person kept (DESIGN 5.17).
    fn read_settings(&self) -> Map<String, Value>;

    /// Keeps the person's settings, as the screen has them now.
    fn keep_settings(&self, settings: Map<String, Value>) -> Answer<()>;

    /// Opens the databases again, as when the person has kept their
    /// connection (DESIGN 5.27). A program whose databases always open has
    /// nothing to do.
    fn connect(&self) -> Answer<()> {
        Ok(())
    }

    /// Signs a person in with `strategy` (`signin password` on a button) and
    /// the values of the sign-in window's parts named as its inputs (`name`,
    /// `password`, `token`): refused in Clarity's own words when it did not
    /// work. A program whose people do not sign in refuses.
    fn signin(&self, strategy: &str, inputs: Map<String, Value>) -> Answer<()> {
        let _ = (strategy, inputs);
        Err(Text::keyed("signinRefused", "Signing in did not work. Check what you typed and try again."))
    }

    /// Signs the person out (`signout` on a button). A program whose people
    /// do not sign in has nothing to do.
    fn signout(&self) -> Answer<()> {
        Ok(())
    }

    /// The language the screen picked, which text code makes is translated to.
    fn set_language(&self, language: Option<&str>) {
        crate::i18n::set_language(language);
    }

    /// Whether the program's engines say how far a long call is, so the
    /// screen runs handlers off its own thread and shows the bar with Stop.
    fn watches(&self) -> bool {
        false
    }

    /// Watches the long engine calls this thread makes from now on as watch
    /// `id` (a number the screen picks), until `end_watch`: the screen calls
    /// it on the thread that runs the handler, just before the handler.
    fn start_watch(&self, id: u64) {
        let _ = id;
    }

    /// What the splash screen says is loading, as code last set it
    /// (`splash.status`); none while it said nothing. The screen asks a few
    /// times a second while the app starts.
    fn splash_status(&self) -> Option<crate::ui::Text> {
        crate::splash::status()
    }

    /// How far the long call of watch `id` is, done of total, as it last
    /// said; none while it said nothing. The screen asks a few times a second.
    fn watch_progress(&self, id: u64) -> Option<(i64, i64)> {
        let _ = id;
        None
    }

    /// The person pressed Stop: the call of watch `id` ends at its next look.
    /// A Stop that comes before `start_watch` holds once it starts.
    fn stop_watch(&self, id: u64) {
        let _ = id;
    }

    /// The handler of watch `id` ended: its calls are watched no more and the
    /// watch is forgotten. Calling it again does nothing.
    fn end_watch(&self, id: u64) {
        let _ = id;
    }

    /// What the program knows of its fields and types (DESIGN 7.9), as
    /// plain JSON (`fields::Facts::to_json`): each field's label, format and
    /// range, and the type of each window's values, which the screen's parts
    /// read beside the values. A program that carries none gives null.
    fn facts_json(&self) -> Value {
        Value::Null
    }

    /// Opens a new instance of the engine beside the screen `name` (DESIGN
    /// 5.12) for one canvas that runs it: its number, or why it could not
    /// start; none when the program has no engine of that name. The program
    /// keeps the instance (`screen::Instances`) until `engine_close`.
    fn engine_open(&self, name: &str) -> Option<std::result::Result<u64, String>> {
        let _ = name;
        None
    }

    /// The pointer and keys instance `id` takes.
    fn engine_hears(&self, id: u64) -> crate::screen::Hears {
        let _ = id;
        crate::screen::Hears::default()
    }

    /// One frame of instance `id`: the input since the last, then `ms`, the
    /// milliseconds since the last frame. It may run on any thread, never on
    /// two at once for one instance.
    fn engine_frame(&self, id: u64, ms: f64, input: Vec<crate::screen::ScreenInput>) -> std::result::Result<crate::screen::Drawn, String> {
        let _ = (id, ms, input);
        Err(crate::screen::GONE.to_string())
    }

    /// Ends instance `id`: the canvas left its screen. Closing one that is gone does nothing.
    fn engine_close(&self, id: u64) {
        let _ = id;
    }

    /// The commands agents may call while the app runs (docs/ledger/app-mcp.md):
    /// its tool list, title, the name its settings are kept under and its
    /// version; none for an app without commands for agents.
    fn agents(&self) -> Option<AgentsOffered> {
        None
    }

    /// Runs a command for an agent as the command line `line` (Clarity.AppMcp's
    /// `cli::command_line`), on the screen's thread: what it printed, and
    /// whether the call is refused. A question it asks goes to `ask` with its
    /// form, which gives the answer as the terminal gives it, or none.
    fn agent_call(&self, line: &[String], ask: &mut dyn FnMut(&str, &str) -> Option<String>) -> AgentRan {
        let _ = (line, ask);
        AgentRan { text: String::new(), refused: true, step: 0 }
    }

    /// Takes back the database step `step` of an agent's call (the person's
    /// Undo on the note it left).
    fn agent_undo(&self, step: u64) -> Answer<()> {
        let _ = step;
        Err(crate::ui::Text::plain("this app keeps no change of an agent's"))
    }
}

/// Fail safe: runs a call on the database. An error is what the screen
/// shows (a refusal its own text); a panic, as on a list index out of range,
/// becomes an error too, and the database stays usable for the next call.
pub fn guarded<T>(db: &Mutex<Db>, run: impl FnOnce(&Db) -> Result<T>) -> Answer<T> {
    let db = db.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&db))) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(said(&err)),
        Err(panic) => {
            let why = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "the handler stopped".to_string());
            Err(Text::plain(why))
        }
    }
}

/// An error as the screen shows it: a refusal its own text; a value or a
/// record that is not there, or still used, in Clarity's own words by a
/// literal key (translated where the project has them); anything else its
/// message. The message (`to_string`) stays the English of the shared cases.
pub fn said(err: &Error) -> Text {
    use crate::ui::Slot;
    match err {
        Error::Refused(text) | Error::Required { text, .. } => text.clone(),
        Error::Outside { position, count } => Text::keyed("itemOutsideList", "There is no item {position} in a list of {count}")
            .with("position", Slot::integer(*position, None))
            .with("count", Slot::integer(*count as i64, None)),
        Error::NotFound { table, id: Some(id) } => {
            Text::keyed("recordNotThere", "There is no {table} with id {id}").with("table", Slot::text(*table)).with("id", Slot::integer(*id, None))
        }
        Error::InUse { table, .. } => Text::keyed("recordStillUsed", "This {table} is still used by other records").with("table", Slot::text(*table)),
        Error::DividedByZero => Text::keyed("divisionByZero", "Division by zero"),
        other => Text::plain(other.to_string()),
    }
}
