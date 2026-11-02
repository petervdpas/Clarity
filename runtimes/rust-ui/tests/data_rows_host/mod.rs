//! What the data parts' tests give their parts: a window's values as JSON
//! (lists of records, written back by their paths as a window does), what
//! each column offers, the facts and types of fields, and a log of what the
//! parts did (each write and each handler run) and of the events alone.

#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use clarity_runtime::ui::PartState;
use clarity_ui::geometry::Rect;
use clarity_ui::input::PointerButton;
use clarity_ui::markup::HandlerCall;
use clarity_ui::parts::{Choice, FieldFacts, PartHost};
use clarity_ui::screen::Screen;
use clarity_ui::testing::Shot;
use clarity_ui::window::paths;
use serde_json::{Map, Value};

#[derive(Default)]
pub struct DataHost {
    pub data: RefCell<Map<String, Value>>,
    pub states: RefCell<BTreeMap<String, PartState>>,
    pub offered: RefCell<BTreeMap<String, Vec<Choice>>>,
    pub fields: RefCell<BTreeMap<String, FieldFacts>>,
    pub types: RefCell<BTreeMap<String, String>>,
    pub log: RefCell<Vec<String>>,
    pub events: RefCell<Vec<String>>,
    /// Each event with the row chosen when it ran: `select 1`.
    pub events_at: RefCell<Vec<String>>,
    /// The chosen row of each list, table and tree, as the window keeps it.
    pub selected: RefCell<BTreeMap<String, usize>>,
    pub changed: Cell<u64>,
}

impl DataHost {
    pub fn new() -> Rc<DataHost> {
        Rc::new(DataHost::default())
    }

    /// Sets a value as a handler does: the parts read it again.
    pub fn set(&self, path: &str, value: Value) {
        paths::write(&mut self.data.borrow_mut(), path, value);
        self.change();
    }

    pub fn get(&self, path: &str) -> Value {
        paths::read(&self.data.borrow(), path).unwrap_or(Value::Null)
    }

    pub fn change(&self) {
        self.changed.set(self.changed.get() + 1);
    }

    pub fn offer(&self, path: &str, choices: &[(Value, &str)]) {
        self.offered.borrow_mut().insert(path.to_string(), choices.iter().map(|(v, l)| Choice { value: v.clone(), label: l.to_string(), cells: None }).collect());
    }

    /// Chooses a row as code does.
    pub fn choose(&self, part: &str, place: Option<usize>) {
        PartHost::select(self, part, place);
    }

    /// The rows read again, as a handler reads them: no row of the part is chosen any more (as the runtime forgets it).
    pub fn reload(&self, part: &str, path: &str, rows: Value) {
        self.selected.borrow_mut().remove(part);
        self.set(path, rows);
    }

    pub fn events_at(&self) -> Vec<String> {
        self.events_at.borrow().clone()
    }

    pub fn events(&self) -> Vec<String> {
        self.events.borrow().clone()
    }

    pub fn logged(&self) -> Vec<String> {
        self.log.borrow().clone()
    }
}

impl PartHost for DataHost {
    fn read(&self, path: &str) -> Option<Value> {
        paths::read(&self.data.borrow(), path)
    }

    fn write(&self, part: &str, path: &str, value: Value) {
        self.log.borrow_mut().push(format!("write {part} {path}"));
        paths::write(&mut self.data.borrow_mut(), path, value);
        self.change();
    }

    fn edited(&self, _part: &str) {
        self.change();
    }

    fn state(&self, name: Option<&str>) -> Option<PartState> {
        let name = name?;
        Some(self.states.borrow_mut().entry(name.to_string()).or_default().clone())
    }

    fn change_state(&self, name: &str, change: &mut dyn FnMut(&mut PartState)) {
        change(self.states.borrow_mut().entry(name.to_string()).or_default());
        self.change();
    }

    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.log.borrow_mut().push(format!("run {} {} {}", part.unwrap_or(""), call.event, call.written()));
        self.events.borrow_mut().push(call.event.clone());
        let at = part.and_then(|p| self.selected.borrow().get(p).copied()).map_or("-1".to_string(), |p| p.to_string());
        self.events_at.borrow_mut().push(format!("{} {at}", call.event));
        true
    }

    fn selected(&self, part: &str) -> Option<usize> {
        self.selected.borrow().get(part).copied()
    }

    fn select(&self, part: &str, place: Option<usize>) {
        match place {
            Some(place) => self.selected.borrow_mut().insert(part.to_string(), place),
            None => self.selected.borrow_mut().remove(part),
        };
        self.change();
    }

    fn text(&self, written: Option<&str>) -> String {
        written.unwrap_or("").trim_start_matches('@').to_string()
    }

    fn type_of(&self, path: &str) -> Option<String> {
        self.types.borrow().get(path).cloned()
    }

    fn choices(&self, path: &str) -> Vec<Choice> {
        self.offered.borrow().get(path).cloned().unwrap_or_default()
    }

    fn field(&self, path: &str) -> Option<FieldFacts> {
        self.fields.borrow().get(path).cloned()
    }

    fn changes(&self) -> u64 {
        self.changed.get()
    }
}

/// Draws the screen at a size, which lays it out.
pub fn laid(screen: &mut Screen, width: i32, height: i32) -> Shot {
    Shot::of(screen, width, height, 1.0)
}

pub fn click(screen: &mut Screen, b: Rect) {
    click_at(screen, b.x + b.width / 2.0, b.y + b.height / 2.0, 1);
}

pub fn click_at(screen: &mut Screen, x: f64, y: f64, clicks: u32) {
    screen.pointer_pressed(x, y, PointerButton::Left, clicks);
    screen.pointer_released(x, y, PointerButton::Left);
}

/// The JSON of a conformance file.
pub fn case_file(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance").join(name);
    serde_json::from_str(&std::fs::read_to_string(path).expect("the conformance file reads")).expect("it is JSON")
}

/// The scratchpad's shots folder, where the tests keep pictures to look at; none when it is not there.
pub fn shot_path(name: &str) -> Option<std::path::PathBuf> {
    let dir = std::env::var("CLARITY_SHOTS").ok()?;
    Some(std::path::Path::new(&dir).join(name))
}
