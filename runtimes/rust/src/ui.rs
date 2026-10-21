//! The binding between a window on the screen and its handlers in the
//! backend. The screen sends a window's state with each event; the handler
//! runs on it and the reply carries the new state, the messages to show and
//! the windows to open or close. The screen holds no behavior of its own.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Decimal, Error, NaiveDate, NaiveDateTime, Result};

/// Text to show, with values in slots: `Saved {customer.name}`. The screen
/// fills each slot in the format of the person using the application, so
/// the backend never formats a value for a person. Text without slots
/// travels as a plain string.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Text {
    /// The text as written in Clarity, slots and all; `{{` and `}}` are braces.
    pub text: String,
    /// The value of each slot, by its path as written: `customer.name`.
    pub values: BTreeMap<String, Slot>,
}

/// The value of one slot, with what the screen needs to format it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub kind: SlotKind,
    pub value: Value,
    /// The decimals of a decimal field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<u32>,
    /// The named format of the field the value comes from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// The enum of an enum value, for its label.
    #[serde(default, rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SlotKind {
    Text,
    Integer,
    Decimal,
    Money,
    Date,
    DateTime,
    /// Whole seconds, shown as 3:45.
    Duration,
    Bool,
    Enum,
}

fn slot(kind: SlotKind, value: Value) -> Slot {
    Slot {
        kind,
        value,
        scale: None,
        format: None,
        enum_name: None,
    }
}

fn format_of(format: Option<&str>) -> Option<String> {
    format.map(str::to_string)
}

impl Slot {
    pub fn text(value: impl Into<String>) -> Slot {
        slot(SlotKind::Text, Value::String(value.into()))
    }

    /// A duration, in whole seconds.
    pub fn duration(seconds: i64) -> Slot {
        slot(SlotKind::Duration, Value::from(seconds))
    }

    pub fn integer(value: i64, format: Option<&str>) -> Slot {
        Slot {
            format: format_of(format),
            ..slot(SlotKind::Integer, Value::from(value))
        }
    }

    pub fn decimal(value: Decimal, scale: Option<u32>, format: Option<&str>) -> Slot {
        Slot {
            scale,
            format: format_of(format),
            ..slot(SlotKind::Decimal, Value::String(value.to_string()))
        }
    }

    pub fn money(value: Decimal, format: Option<&str>) -> Slot {
        Slot {
            format: format_of(format),
            ..slot(SlotKind::Money, Value::String(value.to_string()))
        }
    }

    pub fn date(value: Option<NaiveDate>, format: Option<&str>) -> Slot {
        Slot {
            format: format_of(format),
            ..slot(SlotKind::Date, value.map_or(Value::Null, |d| Value::String(d.to_string())))
        }
    }

    pub fn date_time(value: Option<NaiveDateTime>, format: Option<&str>) -> Slot {
        let text = value.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string());
        Slot {
            format: format_of(format),
            ..slot(SlotKind::DateTime, text.map_or(Value::Null, Value::String))
        }
    }

    pub fn boolean(value: bool) -> Slot {
        slot(SlotKind::Bool, Value::Bool(value))
    }

    /// An enum value, by its name; the screen shows its label.
    pub fn name<E: ToString>(value: &E, enum_name: &str) -> Slot {
        Slot {
            enum_name: Some(enum_name.to_string()),
            ..slot(SlotKind::Enum, Value::String(value.to_string()))
        }
    }

    /// The value as plain text, for places that are not a screen.
    fn plain(&self) -> String {
        match (&self.kind, &self.value) {
            (_, Value::Null) => String::new(),
            (SlotKind::Bool, Value::Bool(b)) => if *b { "yes" } else { "no" }.to_string(),
            (_, Value::String(s)) => s.clone(),
            (_, other) => other.to_string(),
        }
    }
}

impl Text {
    /// Text as written in Clarity, before its slots get values.
    pub fn new(text: &str) -> Text {
        Text {
            text: text.to_string(),
            values: BTreeMap::new(),
        }
    }

    /// Text that is shown as it is: a brace in it is no slot.
    pub fn plain(text: impl Into<String>) -> Text {
        Text::new(&text.into().replace('{', "{{").replace('}', "}}"))
    }

    pub fn with(mut self, path: &str, value: Slot) -> Text {
        self.values.insert(path.to_string(), value);
        self
    }

    /// The same text in the language the screen picked (crate::i18n), its values kept.
    pub fn translated(self) -> Text {
        Text { text: crate::i18n::translate(&self.text), values: self.values }
    }

    /// The text with its slots filled plainly, without a person's format:
    /// for logs, and for code that reads a caption back.
    pub fn show(&self) -> String {
        let mut out = String::new();
        let mut rest = self.text.as_str();
        while let Some(at) = rest.find(['{', '}']) {
            out.push_str(&rest[..at]);
            let brace = &rest[at..at + 1];
            let after = &rest[at + 1..];
            // `{{` and `}}` are braces.
            if let Some(stripped) = after.strip_prefix(brace) {
                out.push_str(brace);
                rest = stripped;
                continue;
            }
            if brace == "}" {
                out.push('}');
                rest = after;
                continue;
            }
            let end = after.find('}').unwrap_or(after.len());
            let path = &after[..end];
            out.push_str(&self.values.get(path).map(Slot::plain).unwrap_or_default());
            rest = after.get(end + 1..).unwrap_or("");
        }
        out.push_str(rest);
        out
    }
}

impl From<&str> for Text {
    fn from(text: &str) -> Text {
        Text::plain(text)
    }
}

impl From<String> for Text {
    fn from(text: String) -> Text {
        Text::plain(text)
    }
}

/// Text on the wire: a plain string, or the text with its slot values.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Wire {
    Plain(String),
    Slots { text: String, values: BTreeMap<String, Slot> },
}

impl Serialize for Text {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        if self.values.is_empty() {
            Wire::Plain(self.show()).serialize(serializer)
        } else {
            Wire::Slots {
                text: self.text.clone(),
                values: self.values.clone(),
            }
            .serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for Text {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Text, D::Error> {
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Plain(text) => Text::plain(text),
            Wire::Slots { text, values } => Text { text, values },
        })
    }
}

/// What a handler set on a part. Anything unset keeps the design.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PartState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<Text>,
    /// A message at an input, such as why its value is not accepted;
    /// empty clears it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Text>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,
    /// Whether a foldable panel or form is folded, as the person or a
    /// handler left it; none keeps the design (conformance/folding.json).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folded: Option<bool>,
    /// Whether a collapsible sidebar is folded to its button, as the person or
    /// a handler left it; none keeps the design (conformance/sidebars.json).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    /// Whether a maximizable sidebar fills the window; none is not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximized: Option<bool>,
    /// The open tab of a ribbon, counted from 1; none is the first
    /// (conformance/ribbons.json).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab: Option<i64>,
    /// What an unbound text input holds; the screen sends it with each
    /// event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Whether an unbound checkbox or switch is on; sent like `text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// A canvas's drawing: its shapes, in order (conformance/drawing.json).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawing: Option<Vec<Value>>,
    /// Where a canvas was last clicked, in its units: `x` and `y`, and
    /// whatever else the screen tells about the pointer, kept as it came.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointer: Option<Map<String, Value>>,
}

/// One open window: its data by name, its parts' properties and the
/// selected row of each table or list.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub window: String,
    /// The number of this open window, its own while it is open: what
    /// belongs to it, such as an engine's state per window, goes by it.
    #[serde(default)]
    pub instance: u64,
    pub data: Map<String, Value>,
    #[serde(default)]
    pub parts: BTreeMap<String, PartState>,
    #[serde(default)]
    pub selected: BTreeMap<String, usize>,
}

/// The number the next window that opens gets.
static NEXT_INSTANCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl WindowState {
    /// A window that opens, with a number of its own.
    pub fn new(window: &str, data: Map<String, Value>) -> Self {
        WindowState {
            window: window.to_string(),
            instance: NEXT_INSTANCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            data,
            ..WindowState::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub level: Level,
    pub text: Text,
}

/// `open Window with name = value, ...`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenRequest {
    pub window: String,
    pub with: Map<String, Value>,
}

/// What the screen does after a handler ran.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reply {
    pub state: WindowState,
    pub messages: Vec<Message>,
    /// The texts the handler put in text values, by the value's name: the
    /// screen fills them in the person's language and format.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub shown: BTreeMap<String, Text>,
    pub open: Vec<OpenRequest>,
    pub close: bool,
}

/// What a handler can do to the screen.
#[derive(Debug, Default)]
pub struct Ui {
    instance: u64,
    parts: BTreeMap<String, PartState>,
    selected: BTreeMap<String, usize>,
    messages: Vec<Message>,
    shown: BTreeMap<String, Text>,
    open: Vec<OpenRequest>,
    pub close: bool,
}

impl Ui {
    /// The number of the open window the handler runs in.
    pub fn instance(&self) -> u64 {
        self.instance
    }

    pub fn from_state(state: &WindowState) -> Ui {
        Ui {
            instance: state.instance,
            parts: state.parts.clone(),
            selected: state.selected.clone(),
            ..Ui::default()
        }
    }

    pub fn visible(&self, part: &str) -> bool {
        self.parts.get(part).and_then(|p| p.visible).unwrap_or(true)
    }

    pub fn set_visible(&mut self, part: &str, value: bool) {
        self.parts.entry(part.to_string()).or_default().visible = Some(value);
    }

    pub fn enabled(&self, part: &str) -> bool {
        self.parts.get(part).and_then(|p| p.enabled).unwrap_or(true)
    }

    pub fn set_enabled(&mut self, part: &str, value: bool) {
        self.parts.entry(part.to_string()).or_default().enabled = Some(value);
    }

    /// Starts an empty drawing on a canvas, before its draw handler runs.
    pub fn begin_drawing(&mut self, part: &str) {
        self.parts.entry(part.to_string()).or_default().drawing = Some(Vec::new());
    }

    /// Adds a shape to a canvas's drawing: `{"op": "line", ...}`.
    pub fn draw(&mut self, part: &str, shape: Value) {
        self.parts.entry(part.to_string()).or_default().drawing.get_or_insert_with(Vec::new).push(shape);
    }

    /// Where a canvas was last clicked, across: `canvas.x`; 0 before any click.
    pub fn x(&self, part: &str) -> i64 {
        self.pointer(part, "x")
    }

    /// Where a canvas was last clicked, down: `canvas.y`.
    pub fn y(&self, part: &str) -> i64 {
        self.pointer(part, "y")
    }

    fn pointer(&self, part: &str, key: &str) -> i64 {
        let value = self.parts.get(part).and_then(|p| p.pointer.as_ref()).and_then(|p| p.get(key));
        value.and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.floor() as i64))).unwrap_or(0)
    }

    /// The caption a handler set, or the one from the design.
    pub fn caption(&self, part: &str, design: &str) -> String {
        self.parts
            .get(part)
            .and_then(|p| p.caption.as_ref().map(Text::show))
            .unwrap_or_else(|| design.to_string())
    }

    pub fn set_caption(&mut self, part: &str, value: Text) {
        self.parts.entry(part.to_string()).or_default().caption = Some(value);
    }

    /// The message a handler set on an input, or an empty text.
    pub fn error(&self, part: &str) -> String {
        self.parts.get(part).and_then(|p| p.error.as_ref().map(Text::show)).unwrap_or_default()
    }

    pub fn set_error(&mut self, part: &str, value: Text) {
        self.parts.entry(part.to_string()).or_default().error = Some(value);
    }

    /// Whether an input is read-only: as a handler set it, or as designed.
    pub fn readonly(&self, part: &str, design: bool) -> bool {
        self.parts.get(part).and_then(|p| p.readonly).unwrap_or(design)
    }

    pub fn set_readonly(&mut self, part: &str, value: bool) {
        self.parts.entry(part.to_string()).or_default().readonly = Some(value);
    }

    /// Whether a foldable part is folded: as the person or a handler left it, or `design`.
    pub fn folded(&self, part: &str, design: bool) -> bool {
        self.parts.get(part).and_then(|p| p.folded).unwrap_or(design)
    }

    pub fn set_folded(&mut self, part: &str, value: bool) {
        self.parts.entry(part.to_string()).or_default().folded = Some(value);
    }

    /// Whether a collapsible sidebar is folded to its button: as left, or `design`.
    pub fn collapsed(&self, part: &str, design: bool) -> bool {
        self.parts.get(part).and_then(|p| p.collapsed).unwrap_or(design)
    }

    pub fn set_collapsed(&mut self, part: &str, value: bool) {
        self.parts.entry(part.to_string()).or_default().collapsed = Some(value);
    }

    /// Whether a maximizable sidebar fills the window.
    pub fn maximized(&self, part: &str) -> bool {
        self.parts.get(part).and_then(|p| p.maximized).unwrap_or(false)
    }

    pub fn set_maximized(&mut self, part: &str, value: bool) {
        self.parts.entry(part.to_string()).or_default().maximized = Some(value);
    }

    /// The open tab of a ribbon, counted from 1.
    pub fn tab(&self, part: &str) -> i64 {
        self.parts.get(part).and_then(|p| p.tab).unwrap_or(1)
    }

    pub fn set_tab(&mut self, part: &str, value: i64) {
        self.parts.entry(part.to_string()).or_default().tab = Some(value);
    }

    /// The text of an unbound text input.
    pub fn text(&self, part: &str) -> String {
        self.parts.get(part).and_then(|p| p.text.clone()).unwrap_or_default()
    }

    pub fn set_text(&mut self, part: &str, value: String) {
        self.parts.entry(part.to_string()).or_default().text = Some(value);
    }

    /// Whether an unbound checkbox or switch is on.
    pub fn checked(&self, part: &str) -> bool {
        self.parts.get(part).and_then(|p| p.checked).unwrap_or(false)
    }

    pub fn set_checked(&mut self, part: &str, value: bool) {
        self.parts.entry(part.to_string()).or_default().checked = Some(value);
    }

    /// Forgets the selected row of a table or list, whose rows were read
    /// again or replaced: they are other records.
    pub fn deselect(&mut self, part: &str) {
        self.selected.remove(part);
    }

    /// The selected record of a table or list showing `rows`.
    pub fn selected<R: Clone>(&self, part: &str, rows: &[R]) -> Result<R> {
        self.selected
            .get(part)
            .and_then(|i| rows.get(*i))
            .cloned()
            .ok_or_else(|| Error::Invalid(format!("no row of {part} is selected")))
    }

    pub fn message(&mut self, level: Level, text: Text) {
        self.messages.push(Message { level, text });
    }

    /// The text value `data` shows `text`, translated and formatted by the screen.
    pub fn set_shown(&mut self, data: &str, text: Text) {
        self.shown.insert(data.to_string(), text);
    }

    pub fn open(&mut self, window: &str, with: Vec<(&str, Value)>) {
        self.open.push(OpenRequest {
            window: window.to_string(),
            with: with.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        });
    }

    /// The reply for the window `window`, with its data after the handler.
    pub fn reply(self, window: &str, data: Map<String, Value>) -> Reply {
        Reply {
            state: WindowState {
                window: window.to_string(),
                instance: self.instance,
                data,
                parts: self.parts,
                selected: self.selected,
            },
            messages: self.messages,
            shown: self.shown,
            open: self.open,
            close: self.close,
        }
    }
}

/// A window's typed data as the map the screen holds.
pub fn to_data<T: Serialize>(window: &T) -> Result<Map<String, Value>> {
    match serde_json::to_value(window) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err(Error::Invalid("window data must be named values".into())),
        Err(err) => Err(Error::Invalid(err.to_string())),
    }
}

/// The screen's data of a window as the window's typed data.
pub fn from_data<T: DeserializeOwned>(data: Map<String, Value>) -> Result<T> {
    serde_json::from_value(Value::Object(data)).map_err(|err| Error::Invalid(err.to_string()))
}

/// A value as the screen holds it, for `open Window with name = value`.
pub fn to_value<T: Serialize>(value: &T) -> Result<Value> {
    serde_json::to_value(value).map_err(|err| Error::Invalid(err.to_string()))
}

/// A record a reference field can point at, as a dropdown shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefOption {
    pub id: crate::Id,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn text_travels_as_a_string_or_with_its_slots() {
        let plain = Text::plain("Brace { yourself }");
        assert_eq!(serde_json::to_value(&plain).unwrap(), Value::String("Brace { yourself }".into()));
        assert_eq!(serde_json::from_value::<Text>(Value::String("Brace { yourself }".into())).unwrap(), plain);

        let saved = Text::new("Saved {customer.name} on {when}, {{done}}")
            .with("customer.name", Slot::text("Ada"))
            .with("when", Slot::date(NaiveDate::from_ymd_opt(2026, 10, 17), Some("long")));
        let wire = serde_json::to_value(&saved).unwrap();
        assert_eq!(
            wire,
            serde_json::json!({
                "text": "Saved {customer.name} on {when}, {{done}}",
                "values": {
                    "customer.name": { "kind": "text", "value": "Ada" },
                    "when": { "kind": "date", "value": "2026-10-17", "format": "long" }
                }
            })
        );
        assert_eq!(serde_json::from_value::<Text>(wire).unwrap(), saved);
        // Without a screen, slots are filled plainly.
        assert_eq!(saved.show(), "Saved Ada on 2026-10-17, {done}");
        assert_eq!(Slot::money(Decimal::new(125050, 2), None).value, Value::String("1250.50".into()));
        assert_eq!(Slot::name(&"Business", "CustomerKind").enum_name.as_deref(), Some("CustomerKind"));
    }
    /// Every window that opens has a number of its own, which its handlers'
    /// replies keep, so what belongs to one open window (an engine's state)
    /// is found again and dropped when it closes.
    #[test]
    fn every_open_window_has_its_own_number() {
        let (first, second) = (WindowState::new("Orders", Map::new()), WindowState::new("Orders", Map::new()));
        assert!(first.instance > 0 && second.instance > 0 && first.instance != second.instance);
        let ui = Ui::from_state(&first);
        assert_eq!(ui.instance(), first.instance);
        assert_eq!(ui.reply("Orders", Map::new()).state.instance, first.instance);
        // The number travels with the state to the screen and back.
        let wire = serde_json::to_value(&first).unwrap();
        assert_eq!(wire["instance"], first.instance);
        assert_eq!(serde_json::from_value::<WindowState>(wire).unwrap().instance, first.instance);
    }

    #[test]
    fn unbound_inputs_carry_their_text() {
        let mut state = WindowState::new("Customers", Map::new());
        state.parts.insert("name_filter".into(), PartState { text: Some("Ada".into()), ..PartState::default() });
        let mut ui = Ui::from_state(&state);
        assert_eq!(ui.text("name_filter"), "Ada");
        assert_eq!(ui.text("other"), "");
        ui.set_text("name_filter", String::new());
        assert_eq!(ui.reply("Customers", Map::new()).state.parts["name_filter"].text.as_deref(), Some(""));
    }

    #[test]
    fn inputs_keep_their_design_until_a_handler_sets_them() {
        let mut ui = Ui::default();
        assert_eq!(ui.error("name"), "");
        assert!(ui.readonly("name", true));
        ui.set_error("name", "Name is required".into());
        ui.set_readonly("name", false);
        assert_eq!(ui.error("name"), "Name is required");
        assert!(!ui.readonly("name", true));
        let reply = ui.reply("Card", Map::new());
        let json = serde_json::to_value(&reply.state.parts["name"]).unwrap();
        assert_eq!(json, serde_json::json!({ "error": "Name is required", "readonly": false }));
    }

    #[test]
    fn rows_read_again_have_no_selected_row() {
        let mut state = WindowState::new("Orders", Map::new());
        state.selected.insert("grid".into(), 1);
        state.selected.insert("other".into(), 0);
        let mut ui = Ui::from_state(&state);
        assert_eq!(ui.selected("grid", &["a", "b"]).unwrap(), "b");
        ui.deselect("grid");
        assert!(ui.selected("grid", &["a", "b"]).is_err());
        let reply = ui.reply("Orders", Map::new());
        assert!(!reply.state.selected.contains_key("grid"));
        assert_eq!(reply.state.selected.get("other"), Some(&0));
    }
}
