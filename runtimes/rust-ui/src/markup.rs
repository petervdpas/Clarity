//! A window's markup read as the parts Clarity.UI draws (DESIGN 7.8, 7.9):
//! the file the generator writes for each window (`windows/Main.cxaml`) and
//! for each app its frame. Each element is a part with what differs from its
//! kind's defaults (the catalog, part-defaults.json), each value read by its
//! field's type (part-fields.json), as every stack reads it.

use std::fmt;
use std::sync::OnceLock;

use serde_json::{Map, Value};

use crate::grid::GridItem;
use crate::xml::{self, XmlElement};

const PART_DEFAULTS: &str = include_str!("../catalog/part-defaults.json");
const PART_FIELDS: &str = include_str!("../catalog/part-fields.json");
const COMMAND_DEFAULTS: &str = include_str!("../catalog/command-defaults.json");
const COMMAND_FIELDS: &str = include_str!("../catalog/command-fields.json");
const SIDEBAR_DEFAULTS: &str = include_str!("../catalog/sidebar-defaults.json");
const SIDEBAR_FIELDS: &str = include_str!("../catalog/sidebar-fields.json");

/// The words Clarity writes for a field of a part, where they differ from its name.
const PART_WORDS: [(&str, &str); 4] = [("key", "shortcut"), ("taborder", "tabOrder"), ("childList", "children"), ("opens", "onClick")];

/// The words Clarity writes for a field of a menu's item, where they differ from its name.
const COMMAND_WORDS: [(&str, &str); 3] = [("key", "shortcut"), ("opens", "onClick"), ("onclick", "handler")];

/// The elements in a part that are its pages, and the kind each is.
const PAGE_KINDS: [(&str, &str); 4] = [("Tab", "tab"), ("Step", "step"), ("Section", "section"), ("Pane", "pane")];

/// The elements of a window that are its frame's regions.
const REGIONS: [&str; 5] = ["Menu", "Toolbar", "Ribbon", "Sidebar", "Footer"];

/// The events a part's `on...` attribute may name.
const EVENTS: [&str; 21] = [
    "click", "change", "enter", "leave", "select", "open", "pointer", "key", "close", "return", "start", "submit", "draw", "drag", "drop", "bend", "next", "finish", "reorder", "tick", "unconnected",
];

fn catalog(cell: &'static OnceLock<Map<String, Value>>, text: &str) -> &'static Map<String, Value> {
    cell.get_or_init(|| serde_json::from_str(text).unwrap_or_default())
}

fn part_defaults() -> &'static Map<String, Value> {
    static CELL: OnceLock<Map<String, Value>> = OnceLock::new();
    catalog(&CELL, PART_DEFAULTS)
}

fn part_fields() -> &'static Map<String, Value> {
    static CELL: OnceLock<Map<String, Value>> = OnceLock::new();
    catalog(&CELL, PART_FIELDS)
}

fn command_defaults() -> &'static Map<String, Value> {
    static CELL: OnceLock<Map<String, Value>> = OnceLock::new();
    catalog(&CELL, COMMAND_DEFAULTS)
}

fn command_fields() -> &'static Map<String, Value> {
    static CELL: OnceLock<Map<String, Value>> = OnceLock::new();
    catalog(&CELL, COMMAND_FIELDS)
}

fn sidebar_defaults_by_side() -> &'static Map<String, Value> {
    static CELL: OnceLock<Map<String, Value>> = OnceLock::new();
    catalog(&CELL, SIDEBAR_DEFAULTS)
}

fn sidebar_fields() -> &'static Map<String, Value> {
    static CELL: OnceLock<Map<String, Value>> = OnceLock::new();
    catalog(&CELL, SIDEBAR_FIELDS)
}

/// Why a window's markup could not be read.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkupError(pub String);

impl fmt::Display for MarkupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MarkupError {}

impl From<xml::XmlError> for MarkupError {
    fn from(err: xml::XmlError) -> Self {
        MarkupError(err.0)
    }
}

/// A handler a part runs on an event: `onclick="digit(7)"` is click, digit and the value 7, as written.
#[derive(Debug, Clone, PartialEq)]
pub struct HandlerCall {
    pub event: String,
    pub handler: String,
    pub args: Vec<String>,
}

impl HandlerCall {
    pub fn new(event: &str, handler: &str, args: Vec<String>) -> HandlerCall {
        HandlerCall { event: event.to_string(), handler: handler.to_string(), args }
    }

    /// How Clarity writes it (conformance/calls.json): `digit(7)`, or `approve` without values.
    pub fn written(&self) -> String {
        if self.args.is_empty() { self.handler.clone() } else { format!("{}({})", self.handler, self.args.join(", ")) }
    }
}

/// A page of a container: a tab, a step, a section, a pane (with its columns when it has a span).
#[derive(Debug, Clone, PartialEq)]
pub struct PageSpec {
    pub kind: String,
    pub title: Option<String>,
    pub columns: Option<i64>,
    pub children: Vec<PartSpec>,
    /// What it is in the design (`designItem`, see `DESIGN_ITEM`); none in a program's own markup.
    pub item: Option<String>,
}

/// An item of a menu: its fields (caption, name, key, handler or window it opens) and its items.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandSpec {
    pub props: Map<String, Value>,
    pub items: Vec<CommandSpec>,
}

impl CommandSpec {
    pub fn text(&self, field: &str) -> Option<&str> {
        self.props.get(field).and_then(Value::as_str)
    }

    pub fn flag(&self, field: &str) -> bool {
        self.props.get(field).and_then(Value::as_bool).unwrap_or(false)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuSpec {
    pub name: Option<String>,
    pub items: Vec<CommandSpec>,
}

/// A part as its window's markup gives it: its kind, its fields (its kind's
/// defaults with what the markup changes), its handlers, its parts, pages and menu.
#[derive(Debug, Clone, PartialEq)]
pub struct PartSpec {
    /// Its kind, as the catalog names it: `button`, `textbox`, `treeTable`.
    pub kind: String,
    /// Every field of the part, its kind's defaults with what the markup changes.
    pub props: Map<String, Value>,
    pub handlers: Vec<HandlerCall>,
    pub children: Vec<PartSpec>,
    pub pages: Vec<PageSpec>,
    pub menu: Option<MenuSpec>,
}

impl PartSpec {
    /// A part of `kind` with only the fields given, as a test or a host makes one.
    pub fn new(kind: &str, props: Map<String, Value>) -> PartSpec {
        PartSpec { kind: kind.to_string(), props, handlers: Vec::new(), children: Vec::new(), pages: Vec::new(), menu: None }
    }

    /// Its name, by which code and its part state know it; none for a part without one.
    pub fn name(&self) -> Option<&str> {
        self.text("name")
    }

    /// Where it is in its grid: column and row from 1, columns wide and rows high.
    pub fn place(&self, grows: bool) -> GridItem {
        let n = |field: &str| self.number(field).map(|v| v as i64).unwrap_or(1);
        GridItem::new(n("col"), n("row"), n("width"), n("height"), grows)
    }

    /// A text field; none when it has none.
    pub fn text(&self, field: &str) -> Option<&str> {
        self.props.get(field).and_then(Value::as_str)
    }

    /// A number field; none when it has none.
    pub fn number(&self, field: &str) -> Option<f64> {
        self.props.get(field).and_then(Value::as_f64)
    }

    /// A yes or no field; false when it has none.
    pub fn flag(&self, field: &str) -> bool {
        self.props.get(field).and_then(Value::as_bool).unwrap_or(false)
    }

    /// A field of names: `show="title, kind"`.
    pub fn names(&self, field: &str) -> Vec<String> {
        match self.props.get(field) {
            Some(Value::Array(items)) => items.iter().map(|n| n.as_str().map(str::to_string).unwrap_or_else(|| n.to_string())).collect(),
            _ => Vec::new(),
        }
    }

    /// A field as JSON: a list, a record, or a name of the model's.
    pub fn json(&self, field: &str) -> Option<&Value> {
        self.props.get(field)
    }

    /// The handlers it runs on `event` (click, change, select...).
    pub fn on<'a>(&'a self, event: &'a str) -> impl Iterator<Item = &'a HandlerCall> + 'a {
        self.handlers.iter().filter(move |h| h.event == event)
    }
}

/// A window's own frame (its regions as written), its parts, its name (`Main`) and the columns of its grid.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowSpec {
    pub frame: Vec<XmlElement>,
    pub parts: Vec<PartSpec>,
    pub name: Option<String>,
    pub columns: i64,
    /// What it is in the design (`designItem`, see `DESIGN_ITEM`); none in a program's own markup.
    pub item: Option<String>,
    /// How high a row of its grid is: the project's `row height`, which the
    /// root says where it is not the standard (`rowHeight="30"`), within `grid::ROW_HEIGHTS`.
    pub row_height: f64,
}

/// The attribute the IDE's designer gives each window, part, page and menu
/// item of the markup it draws: where it is written in the project, so the
/// designer's layer over the drawing knows what each box is (`layout`
/// step). A part keeps it among its fields; a program's markup has none.
pub const DESIGN_ITEM: &str = "designItem";

/// The page a tabs, steps or accordion part shows first in the IDE's
/// designer (`designShown="2"`, from 1): the one the designer switched to.
pub const DESIGN_SHOWN: &str = "designShown";

/// The page a container shows first: the designer's choice, else the first.
pub fn design_shown(spec: &PartSpec) -> i64 {
    spec.number(DESIGN_SHOWN).map(|n| n as i64).filter(|n| *n >= 1).unwrap_or(1)
}

/// The columns of a window that says none.
pub const STANDARD_COLUMNS: i64 = 12;

/// The kinds of part the catalog has.
pub fn kinds() -> impl Iterator<Item = &'static str> {
    part_defaults().keys().map(String::as_str)
}

/// A part kind's defaults, as the catalog gives them; none for a kind it does not have.
pub fn defaults_of(kind: &str) -> Option<&'static Map<String, Value>> {
    part_defaults().get(kind).and_then(Value::as_object)
}

/// A window as its markup writes it; a file that is no well-formed XML, or names a part the catalog does not have, is refused.
pub fn read(markup: &str) -> Result<WindowSpec, MarkupError> {
    let root = xml::parse(markup)?;
    let written = root.attribute("span").or_else(|| root.attribute("columns"));
    let columns = written.and_then(|w| w.trim().parse::<i64>().ok()).filter(|c| *c > 0).unwrap_or(STANDARD_COLUMNS);
    let mut frame = Vec::new();
    let mut parts = Vec::new();
    for element in &root.children {
        if REGIONS.contains(&element.name.as_str()) {
            frame.push(element.clone());
        } else {
            parts.push(part_of(element)?);
        }
    }
    let row_height = root.attribute("rowHeight").and_then(|h| h.trim().parse::<f64>().ok()).filter(|h| h.is_finite()).map(|h| h.clamp(crate::grid::ROW_HEIGHTS.0, crate::grid::ROW_HEIGHTS.1)).unwrap_or(crate::grid::ROW_HEIGHT);
    Ok(WindowSpec { frame, parts, name: root.attribute("name").map(str::to_string), columns, item: root.attribute(DESIGN_ITEM).map(str::to_string), row_height })
}

/// An app's frame as its markup writes it: its regions.
pub fn read_app(markup: &str) -> Result<Vec<XmlElement>, MarkupError> {
    Ok(xml::parse(markup)?.children)
}

fn word<'a>(words: &[(&'a str, &'a str)], written: &'a str) -> &'a str {
    words.iter().find(|(w, _)| *w == written).map(|(_, f)| *f).unwrap_or(written)
}

/// A part's kind from its element's name: `TreeTable` is `treeTable`.
fn kind_of(tag: &str) -> String {
    let mut chars = tag.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A part as its element writes it.
pub fn part_of(element: &XmlElement) -> Result<PartSpec, MarkupError> {
    let kind = kind_of(&element.name);
    let Some(defaults) = defaults_of(&kind) else {
        return Err(MarkupError(format!("a window's markup has no part <{}>", element.name)));
    };
    let mut props = defaults.clone();
    // A part one row high writes no `rows`.
    props.insert("height".to_string(), Value::from(1));
    let mut handlers = Vec::new();
    for (written, value) in &element.attributes {
        let number = |text: &str| text.trim().parse::<i64>().map(Value::from).map_err(|_| MarkupError(format!("<{}> has {written}=\"{value}\", which is no number", element.name)));
        match written.as_str() {
            "at" => {
                let mut at = value.split(',');
                props.insert("col".to_string(), number(at.next().unwrap_or(""))?);
                props.insert("row".to_string(), number(at.next().unwrap_or(""))?);
            }
            "span" => {
                props.insert("width".to_string(), number(value)?);
            }
            "rows" => {
                props.insert("height".to_string(), number(value)?);
            }
            // Kept as text, as the designer wrote it.
            item if item == DESIGN_ITEM => {
                props.insert(DESIGN_ITEM.to_string(), Value::from(value.as_str()));
            }
            "none" => {
                for empty in value.split_whitespace() {
                    props.insert(word(&PART_WORDS, empty).to_string(), Value::Null);
                }
            }
            word_written => match word_written.strip_prefix("on").filter(|e| EVENTS.contains(e)) {
                Some(event) => handlers.push(handler_of(event, value)),
                None => {
                    let field = word(&PART_WORDS, word_written);
                    let kind = part_fields().get(field).and_then(Value::as_str);
                    props.insert(field.to_string(), value_of(kind, value));
                }
            },
        }
    }
    let mut children = Vec::new();
    let mut pages = Vec::new();
    let mut menu = None;
    for child in &element.children {
        if child.name == "Menu" {
            menu = Some(menu_of(child));
        } else if let Some((_, page)) = PAGE_KINDS.iter().find(|(tag, _)| *tag == child.name) {
            pages.push(PageSpec {
                kind: page.to_string(),
                title: child.attribute("title").map(str::to_string),
                columns: child.attribute("span").and_then(|s| s.trim().parse().ok()),
                children: child.children.iter().map(part_of).collect::<Result<_, _>>()?,
                item: child.attribute(DESIGN_ITEM).map(str::to_string),
            });
        } else {
            children.push(part_of(child)?);
        }
    }
    Ok(PartSpec { kind, props, handlers, children, pages, menu })
}

pub fn menu_of(element: &XmlElement) -> MenuSpec {
    MenuSpec { name: element.attribute("name").map(str::to_string), items: element.children.iter().map(command_of).collect() }
}

pub fn command_of(element: &XmlElement) -> CommandSpec {
    let mut props = command_defaults().clone();
    for (written, value) in &element.attributes {
        let field = word(&COMMAND_WORDS, written);
        // Kept as text, as the designer wrote it.
        let kind = if field == DESIGN_ITEM { Some("text") } else { command_fields().get(field).and_then(Value::as_str) };
        props.insert(field.to_string(), value_of(kind, value));
    }
    CommandSpec { props, items: element.children.iter().map(command_of).collect() }
}

/// A new sidebar's fields on `side` (`left`, `right`), as the catalog gives them.
pub fn sidebar_defaults(side: &str) -> Map<String, Value> {
    let by_side = sidebar_defaults_by_side();
    by_side.get(side).or_else(|| by_side.get("left")).and_then(Value::as_object).cloned().unwrap_or_default()
}

/// A sidebar's fields: the catalog's for its side, with what its element writes, each read as the catalog says.
pub fn sidebar_of(sidebar: &XmlElement) -> Map<String, Value> {
    let mut props = sidebar_defaults(sidebar.attribute("side").unwrap_or("left"));
    for (name, value) in &sidebar.attributes {
        if value == "null" {
            props.insert(name.clone(), Value::Null);
        } else if let Some(kind) = sidebar_fields().get(name).and_then(Value::as_str) {
            props.insert(name.clone(), value_of(Some(kind), value));
        }
    }
    props
}

/// A value of a field, by its type: text as it is, a number, true or false, names, or JSON.
fn value_of(kind: Option<&str>, text: &str) -> Value {
    match kind {
        Some("text") => Value::from(text),
        Some("number") => text.trim().parse::<f64>().ok().and_then(|n| serde_json::Number::from_f64(n).map(Value::Number)).unwrap_or(Value::Null),
        Some("bool") => Value::Bool(text == "true"),
        Some("texts") => Value::Array(text.split(',').map(str::trim).filter(|t| !t.is_empty()).map(Value::from).collect()),
        _ => serde_json::from_str(text).unwrap_or_else(|_| Value::from(text)),
    }
}

/// `digit(7)` as the handler and the values it runs with, as Clarity writes a handler call.
pub fn handler_of(event: &str, call: &str) -> HandlerCall {
    let Some(open) = call.find('(') else {
        return HandlerCall::new(event, call.trim(), Vec::new());
    };
    let close = call.rfind(')').filter(|c| *c > open).unwrap_or(call.len());
    HandlerCall::new(event, call[..open].trim(), split_values(&call[open + 1..close]))
}

fn split_values(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' && quoted {
            current.push(c);
            if let Some(next) = chars.next() {
                current.push(next);
            }
        } else if c == '"' {
            quoted = !quoted;
            current.push(c);
        } else if c == ',' && !quoted {
            values.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        values.push(current.trim().to_string());
    }
    values
}
