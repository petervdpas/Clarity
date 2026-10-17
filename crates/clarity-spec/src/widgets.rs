//! The widget catalog: every widget a page can hold, with its palette
//! category, default size, what it can bind to and which extra properties
//! it takes. The parser, the checker, the edit engine and the designer's
//! palette all read this catalog, so a new widget is one entry here.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::lexer::quote;
use crate::model::{FieldType, Orientation, Surface, WidgetProps};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum WidgetKind {
    Label,
    Help,
    Heading,
    Text,
    Textbox,
    Memo,
    Password,
    Number,
    Checkbox,
    Switch,
    Dropdown,
    Radio,
    Date,
    Button,
    Form,
    Panel,
    Tabs,
    Splitter,
    Table,
    List,
    Image,
    Canvas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Category {
    Text,
    BasicInput,
    Containers,
    Data,
    Graphics,
}

/// Something that happens to a part, which a handler in the code behind
/// can respond to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Event {
    Click,
    Change,
    /// Focus moves into the part.
    Enter,
    /// Focus moves out of the part.
    Leave,
    /// A row or item is selected.
    Select,
    /// A row or item is opened (double-click or Enter); for a window, the
    /// window has opened with its data loaded.
    Open,
    Pointer,
    Key,
    /// A window is about to close.
    Close,
    /// A window is on top again, after a window it opened has closed.
    Return,
    /// The app starts, before its main window opens.
    Start,
    /// Enter is pressed in an input of a form.
    Submit,
}

/// The events a window has, in the order the designer lists them.
pub const WINDOW_EVENTS: [Event; 3] = [Event::Open, Event::Close, Event::Return];

/// The events the app has.
pub const APP_EVENTS: [Event; 1] = [Event::Start];

impl Event {
    pub fn keyword(self) -> &'static str {
        match self {
            Event::Click => "click",
            Event::Change => "change",
            Event::Enter => "enter",
            Event::Leave => "leave",
            Event::Select => "select",
            Event::Open => "open",
            Event::Pointer => "pointer",
            Event::Key => "key",
            Event::Close => "close",
            Event::Return => "return",
            Event::Start => "start",
            Event::Submit => "submit",
        }
    }

    pub fn by_keyword(keyword: &str) -> Option<Event> {
        match keyword {
            "click" => Some(Event::Click),
            "change" => Some(Event::Change),
            "enter" => Some(Event::Enter),
            "leave" => Some(Event::Leave),
            "select" => Some(Event::Select),
            "open" => Some(Event::Open),
            "pointer" => Some(Event::Pointer),
            "key" => Some(Event::Key),
            "close" => Some(Event::Close),
            "return" => Some(Event::Return),
            "start" => Some(Event::Start),
            "submit" => Some(Event::Submit),
            _ => None,
        }
    }
}

/// What a widget's `bind` refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BindTarget {
    /// The widget cannot be bound.
    Nothing,
    /// A field of a single-record data declaration: `bind customer.name`.
    Field,
    /// A collection data declaration: `bind orders`.
    Collection,
    /// A single-record data declaration: `bind customer`.
    Record,
}

/// How a container organizes its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ContainerKind {
    /// Children directly inside, on the container's own grid.
    Plain,
    /// Children inside `tab "Title"` blocks.
    Tabs,
    /// Children inside exactly two `pane` blocks.
    Splitter,
}

/// Extra properties beyond position, caption, binding and events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum WidgetProperty {
    Columns,
    Show,
    Orientation,
    Surface,
    Renderer,
    Source,
    For,
    Placeholder,
    Readonly,
    TabOrder,
    Shortcut,
    Default,
    Cancel,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct WidgetInfo {
    pub kind: WidgetKind,
    /// The keyword in spec files.
    pub keyword: String,
    /// The name shown in the palette.
    pub name: String,
    pub category: Category,
    pub width: u32,
    pub height: u32,
    /// Caption given to a new widget; `null` for widgets without one.
    pub caption: Option<String>,
    pub bind: BindTarget,
    /// Field types the widget can bind to (`FieldType` kinds); empty means any.
    #[serde(rename = "fieldTypes")]
    #[ts(rename = "fieldTypes")]
    pub field_types: Vec<String>,
    /// Input widgets are reported while they are not bound to data.
    pub input: bool,
    pub container: Option<ContainerKind>,
    pub clickable: bool,
    /// Whether the part takes extra height when the window grows (tables,
    /// lists, containers, canvases); other parts keep their row height.
    pub grows: bool,
    /// Events a handler in the code behind can be wired to.
    pub events: Vec<Event>,
    pub properties: Vec<WidgetProperty>,
}

struct Entry {
    kind: WidgetKind,
    keyword: &'static str,
    name: &'static str,
    category: Category,
    size: (u32, u32),
    caption: Option<&'static str>,
    bind: BindTarget,
    field_types: &'static [&'static str],
    input: bool,
    container: Option<ContainerKind>,
    clickable: bool,
    events: &'static [Event],
    properties: &'static [WidgetProperty],
}

const INPUT_EVENTS: &[Event] = &[Event::Change, Event::Enter, Event::Leave];
const TYPED: &[WidgetProperty] = &[
    WidgetProperty::Placeholder,
    WidgetProperty::Readonly,
    WidgetProperty::TabOrder,
];
const PICKED: &[WidgetProperty] = &[WidgetProperty::Readonly, WidgetProperty::TabOrder];

/// A part that shows text and takes no input.
const fn text_part(
    kind: WidgetKind,
    keyword: &'static str,
    name: &'static str,
    size: (u32, u32),
    caption: Option<&'static str>,
    bind: BindTarget,
    properties: &'static [WidgetProperty],
) -> Entry {
    Entry {
        kind,
        keyword,
        name,
        category: Category::Text,
        size,
        caption,
        bind,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        properties,
    }
}

/// An input bound to a field of one of `field_types`, without a caption.
const fn input(
    kind: WidgetKind,
    keyword: &'static str,
    name: &'static str,
    size: (u32, u32),
    field_types: &'static [&'static str],
    events: &'static [Event],
    properties: &'static [WidgetProperty],
) -> Entry {
    Entry {
        kind,
        keyword,
        name,
        category: Category::BasicInput,
        size,
        caption: None,
        bind: BindTarget::Field,
        field_types,
        input: true,
        container: None,
        clickable: false,
        events,
        properties,
    }
}

const ENTRIES: &[Entry] = &[
    text_part(WidgetKind::Label, "label", "Label", (3, 1), Some("Label"), BindTarget::Nothing, &[WidgetProperty::For]),
    text_part(WidgetKind::Help, "help", "Help text", (4, 1), None, BindTarget::Nothing, &[WidgetProperty::For]),
    text_part(WidgetKind::Heading, "heading", "Heading", (6, 1), Some("Heading"), BindTarget::Nothing, &[]),
    text_part(WidgetKind::Text, "text", "Text", (4, 1), None, BindTarget::Field, &[]),
    input(WidgetKind::Textbox, "textbox", "Text box", (4, 1), &["text", "email", "url"], INPUT_EVENTS, TYPED),
    input(WidgetKind::Memo, "memo", "Memo", (6, 3), &["text"], INPUT_EVENTS, TYPED),
    input(WidgetKind::Password, "password", "Password", (4, 1), &["text"], INPUT_EVENTS, TYPED),
    input(WidgetKind::Number, "number", "Number", (3, 1), &["integer", "decimal", "money", "id"], INPUT_EVENTS, TYPED),
    Entry {
        caption: Some("Check box"),
        ..input(WidgetKind::Checkbox, "checkbox", "Check box", (3, 1), &["bool"], &[Event::Change], PICKED)
    },
    Entry {
        caption: Some("Switch"),
        ..input(WidgetKind::Switch, "switch", "Switch", (3, 1), &["bool"], &[Event::Change], PICKED)
    },
    input(WidgetKind::Dropdown, "dropdown", "Dropdown", (4, 1), &["enum", "ref"], &[Event::Change], PICKED),
    input(
        WidgetKind::Radio,
        "radio",
        "Radio buttons",
        (6, 1),
        &["enum"],
        &[Event::Change],
        &[WidgetProperty::Orientation, WidgetProperty::Readonly, WidgetProperty::TabOrder],
    ),
    input(WidgetKind::Date, "date", "Date", (3, 1), &["date", "dateTime"], &[Event::Change], PICKED),
    Entry {
        kind: WidgetKind::Button,
        keyword: "button",
        name: "Button",
        category: Category::BasicInput,
        size: (2, 1),
        caption: Some("Button"),
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: true,
        events: &[Event::Click],
        properties: &[
            WidgetProperty::TabOrder,
            WidgetProperty::Shortcut,
            WidgetProperty::Default,
            WidgetProperty::Cancel,
        ],
    },
    Entry {
        kind: WidgetKind::Form,
        keyword: "form",
        name: "Form",
        category: Category::Containers,
        size: (12, 4),
        caption: None,
        bind: BindTarget::Record,
        field_types: &[],
        input: false,
        container: Some(ContainerKind::Plain),
        clickable: false,
        events: &[Event::Submit],
        properties: &[],
    },
    Entry {
        kind: WidgetKind::Panel,
        keyword: "panel",
        name: "Panel",
        category: Category::Containers,
        size: (6, 4),
        caption: Some("Panel"),
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: Some(ContainerKind::Plain),
        clickable: false,
        events: &[],
        properties: &[],
    },
    Entry {
        kind: WidgetKind::Tabs,
        keyword: "tabs",
        name: "Tabs",
        category: Category::Containers,
        size: (8, 5),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: Some(ContainerKind::Tabs),
        clickable: false,
        events: &[Event::Change],
        properties: &[],
    },
    Entry {
        kind: WidgetKind::Splitter,
        keyword: "splitter",
        name: "Splitter",
        category: Category::Containers,
        size: (12, 6),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: Some(ContainerKind::Splitter),
        clickable: false,
        events: &[],
        properties: &[WidgetProperty::Orientation],
    },
    Entry {
        kind: WidgetKind::Table,
        keyword: "table",
        name: "Table",
        category: Category::Data,
        size: (12, 5),
        caption: None,
        bind: BindTarget::Collection,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::Select, Event::Open],
        properties: &[WidgetProperty::Columns],
    },
    Entry {
        kind: WidgetKind::List,
        keyword: "list",
        name: "List",
        category: Category::Data,
        size: (4, 5),
        caption: None,
        bind: BindTarget::Collection,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::Select, Event::Open],
        properties: &[WidgetProperty::Show],
    },
    Entry {
        kind: WidgetKind::Image,
        keyword: "image",
        name: "Image",
        category: Category::Graphics,
        size: (3, 3),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: true,
        events: &[Event::Click],
        properties: &[WidgetProperty::Source],
    },
    Entry {
        kind: WidgetKind::Canvas,
        keyword: "canvas",
        name: "Canvas",
        category: Category::Graphics,
        size: (6, 4),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::Pointer, Event::Key],
        properties: &[WidgetProperty::Surface, WidgetProperty::Renderer],
    },
];

fn entry(kind: WidgetKind) -> &'static Entry {
    ENTRIES
        .iter()
        .find(|e| e.kind == kind)
        .expect("every widget kind has a catalog entry")
}

/// The full catalog, in palette order.
pub fn catalog() -> Vec<WidgetInfo> {
    ENTRIES.iter().map(|e| info(e.kind)).collect()
}

pub fn info(kind: WidgetKind) -> WidgetInfo {
    let e = entry(kind);
    WidgetInfo {
        kind: e.kind,
        keyword: e.keyword.to_string(),
        name: e.name.to_string(),
        category: e.category,
        width: e.size.0,
        height: e.size.1,
        caption: e.caption.map(str::to_string),
        bind: e.bind,
        field_types: e.field_types.iter().map(|t| t.to_string()).collect(),
        input: e.input,
        container: e.container,
        clickable: e.clickable,
        grows: grows(e.kind),
        events: e.events.to_vec(),
        properties: e.properties.to_vec(),
    }
}

pub fn by_keyword(keyword: &str) -> Option<WidgetKind> {
    ENTRIES
        .iter()
        .find(|e| e.keyword == keyword)
        .map(|e| e.kind)
}

pub fn keyword(kind: WidgetKind) -> &'static str {
    entry(kind).keyword
}

pub fn default_size(kind: WidgetKind) -> (u32, u32) {
    entry(kind).size
}

pub fn container(kind: WidgetKind) -> Option<ContainerKind> {
    entry(kind).container
}

pub fn has_event(kind: WidgetKind, event: Event) -> bool {
    entry(kind).events.contains(&event)
}

pub fn has_property(kind: WidgetKind, property: WidgetProperty) -> bool {
    entry(kind).properties.contains(&property)
}

/// Whether a widget can show a field of this type.
pub fn accepts(kind: WidgetKind, ty: &FieldType) -> bool {
    let types = entry(kind).field_types;
    types.is_empty() || types.contains(&field_type_tag(ty))
}

/// The `kind` tag a field type has in the neutral model.
pub fn field_type_tag(ty: &FieldType) -> &'static str {
    match ty {
        FieldType::Text { .. } => "text",
        FieldType::Integer => "integer",
        FieldType::Decimal { .. } => "decimal",
        FieldType::Money => "money",
        FieldType::Bool => "bool",
        FieldType::Date => "date",
        FieldType::DateTime => "dateTime",
        FieldType::Email => "email",
        FieldType::Url => "url",
        FieldType::Id => "id",
        FieldType::Enum { .. } => "enum",
        FieldType::Ref { .. } => "ref",
        FieldType::List { .. } => "list",
    }
}

/// The widget that shows a field of this type by default, used when a
/// field is dropped onto a page.
pub fn widget_for(ty: &FieldType) -> WidgetKind {
    ENTRIES
        .iter()
        .find(|e| e.input && e.field_types.contains(&field_type_tag(ty)))
        .map(|e| e.kind)
        .unwrap_or(WidgetKind::Label)
}

/// The names of the field types a widget accepts, for messages.
pub fn accepted_types(kind: WidgetKind) -> &'static [&'static str] {
    entry(kind).field_types
}

/// New widget properties at a grid cell, with the catalog defaults.
pub fn new_props(kind: WidgetKind, col: u32, row: u32) -> WidgetProps {
    let e = entry(kind);
    WidgetProps {
        kind,
        caption: e.caption.map(str::to_string),
        name: None,
        col,
        row,
        width: e.size.0,
        height: e.size.1,
        bind: None,
        on_click: None,
        new_window: false,
        handlers: Vec::new(),
        columns: Vec::new(),
        show: Vec::new(),
        orientation: has_property(kind, WidgetProperty::Orientation)
            .then_some(Orientation::Horizontal),
        surface: has_property(kind, WidgetProperty::Surface).then_some(Surface::Svg),
        renderer: None,
        source: None,
        for_part: None,
        placeholder: None,
        tooltip: None,
        style: None,
        readonly: false,
        tab_order: None,
        shortcut: None,
        default_button: false,
        cancel: false,
    }
}

/// Writes widget properties as one canonical spec line, in a fixed order.
pub fn widget_line(props: &WidgetProps) -> String {
    let mut parts = vec![keyword(props.kind).to_string()];
    if let Some(caption) = &props.caption {
        parts.push(quote(caption));
    }
    if let Some(name) = &props.name {
        parts.push(format!("name {name}"));
    }
    parts.push(format!("at {},{}", props.col, props.row));
    parts.push(format!("span {}", props.width));
    // `rows` is left out only where the catalog height is also 1, so a
    // one-row panel does not read back at its default height.
    if props.height != 1 || default_size(props.kind).1 != 1 {
        parts.push(format!("rows {}", props.height));
    }
    if let Some(orientation) = props.orientation {
        parts.push(orientation_keyword(orientation).to_string());
    }
    if let Some(bind) = &props.bind {
        parts.push(format!("bind {bind}"));
    }
    if !props.columns.is_empty() {
        parts.push(format!("columns {}", props.columns.join(", ")));
    }
    if !props.show.is_empty() {
        parts.push(format!("show {}", props.show.join(", ")));
    }
    if let Some(surface) = props.surface {
        parts.push(format!("surface {}", surface_keyword(surface)));
    }
    if let Some(renderer) = &props.renderer {
        parts.push(format!("renderer {renderer}"));
    }
    if let Some(source) = &props.source {
        parts.push(format!("source {}", quote(source)));
    }
    if let Some(target) = &props.for_part {
        parts.push(format!("for {target}"));
    }
    if let Some(placeholder) = &props.placeholder {
        parts.push(format!("placeholder {}", quote(placeholder)));
    }
    if let Some(tooltip) = &props.tooltip {
        parts.push(format!("tooltip {}", quote(tooltip)));
    }
    if let Some(style) = &props.style {
        parts.push(format!("style {style}"));
    }
    if props.readonly {
        parts.push("readonly".to_string());
    }
    if let Some(order) = props.tab_order {
        parts.push(format!("taborder {order}"));
    }
    if let Some(shortcut) = &props.shortcut {
        parts.push(format!("key {}", quote(shortcut)));
    }
    if props.default_button {
        parts.push("default".to_string());
    }
    if props.cancel {
        parts.push("cancel".to_string());
    }
    for h in &props.handlers {
        parts.push(format!("on {}: {}", h.event.keyword(), h.handler));
    }
    if let Some(target) = &props.on_click {
        parts.push(format!("on click -> {target}"));
        if props.new_window {
            parts.push("in new window".to_string());
        }
    }
    parts.join(" ")
}

pub fn orientation_keyword(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

pub fn surface_keyword(surface: Surface) -> &'static str {
    match surface {
        Surface::Svg => "svg",
        Surface::Bitmap => "bitmap",
        Surface::Gpu => "gpu",
    }
}

/// Checks properties that come from outside (the designer) before they are
/// written: names must be identifiers, sizes positive, and properties must
/// apply to the widget.
pub fn validate_props(props: &WidgetProps) -> Result<(), String> {
    let name = keyword(props.kind);
    if props.col == 0 || props.row == 0 || props.width == 0 || props.height == 0 {
        return Err("position and size start at 1".to_string());
    }
    let identifier = |what: &str, text: &str| {
        if is_identifier(text) {
            Ok(())
        } else {
            Err(format!("'{text}' is not a valid {what}"))
        }
    };
    if let Some(bind) = &props.bind {
        if entry(props.kind).bind == BindTarget::Nothing {
            return Err(format!("a {name} cannot be bound to data"));
        }
        for part in bind.split('.') {
            identifier("data name or field", part)?;
        }
        if bind.split('.').count() > 2 {
            return Err(format!("'{bind}' has too many parts, use data.field"));
        }
    }
    if let Some(target) = &props.on_click {
        if !entry(props.kind).clickable {
            return Err(format!("a {name} has no click event"));
        }
        identifier("click target", target)?;
    }
    if let Some(part) = &props.name {
        identifier("part name", part)?;
    }
    for (i, h) in props.handlers.iter().enumerate() {
        if !has_event(props.kind, h.event) {
            return Err(format!("a {name} has no {} event", h.event.keyword()));
        }
        identifier("handler name", &h.handler)?;
        let wired_twice = props.handlers[..i].iter().any(|o| o.event == h.event)
            || (h.event == Event::Click && props.on_click.is_some());
        if wired_twice {
            return Err(format!("'{}' is wired twice", h.event.keyword()));
        }
    }
    let checks = [
        (
            WidgetProperty::Columns,
            !props.columns.is_empty(),
            "columns",
        ),
        (WidgetProperty::Show, !props.show.is_empty(), "show"),
        (
            WidgetProperty::Orientation,
            props.orientation.is_some(),
            "orientation",
        ),
        (WidgetProperty::Surface, props.surface.is_some(), "surface"),
        (
            WidgetProperty::Renderer,
            props.renderer.is_some(),
            "renderer",
        ),
        (WidgetProperty::Source, props.source.is_some(), "source"),
        (WidgetProperty::For, props.for_part.is_some(), "for"),
        (
            WidgetProperty::Placeholder,
            props.placeholder.is_some(),
            "placeholder",
        ),
        (WidgetProperty::Readonly, props.readonly, "readonly"),
        (WidgetProperty::TabOrder, props.tab_order.is_some(), "taborder"),
        (WidgetProperty::Shortcut, props.shortcut.is_some(), "key"),
        (WidgetProperty::Default, props.default_button, "default"),
        (WidgetProperty::Cancel, props.cancel, "cancel"),
    ];
    for (property, present, what) in checks {
        if present && !has_property(props.kind, property) {
            return Err(format!("'{what}' does not apply to a {name}"));
        }
    }
    for field in props.columns.iter().chain(&props.show) {
        identifier("field name", field)?;
    }
    if let Some(renderer) = &props.renderer {
        identifier("renderer name", renderer)?;
    }
    if let Some(target) = &props.for_part {
        identifier("part name", target)?;
    }
    if let Some(shortcut) = &props.shortcut {
        check_shortcut(shortcut)?;
    }
    if props.tab_order == Some(0) {
        return Err("tab order starts at 1".to_string());
    }
    Ok(())
}

/// Keys a shortcut can hold besides its modifiers, by name.
const NAMED_KEYS: [&str; 15] = [
    "Enter", "Escape", "Tab", "Space", "Backspace", "Delete", "Insert", "Home", "End", "PageUp",
    "PageDown", "Up", "Down", "Left", "Right",
];

/// `Ctrl+S`, `Ctrl+Shift+F2`: modifiers, then one key.
pub fn check_shortcut(shortcut: &str) -> Result<(), String> {
    let not = || Err(format!("'{shortcut}' is not a shortcut, as in 'Ctrl+S'"));
    let mut keys: Vec<&str> = shortcut.split('+').collect();
    let Some(key) = keys.pop() else { return not() };
    for (i, modifier) in keys.iter().enumerate() {
        if !["Ctrl", "Alt", "Shift", "Meta"].contains(modifier) || keys[..i].contains(modifier) {
            return not();
        }
    }
    let single = key.len() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric());
    let function = key
        .strip_prefix('F')
        .and_then(|n| n.parse::<u32>().ok())
        .is_some_and(|n| (1..=12).contains(&n));
    if single || function || NAMED_KEYS.contains(&key) {
        Ok(())
    } else {
        not()
    }
}

/// Parts that take extra height in a fluid window.
fn grows(kind: WidgetKind) -> bool {
    matches!(
        kind,
        WidgetKind::Form
            | WidgetKind::Panel
            | WidgetKind::Tabs
            | WidgetKind::Splitter
            | WidgetKind::Table
            | WidgetKind::List
            | WidgetKind::Image
            | WidgetKind::Canvas
    )
}

pub fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_one_entry_and_a_unique_keyword() {
        let mut keywords: Vec<_> = ENTRIES.iter().map(|e| e.keyword).collect();
        keywords.sort();
        keywords.dedup();
        assert_eq!(keywords.len(), ENTRIES.len());
        for e in ENTRIES {
            assert_eq!(by_keyword(e.keyword), Some(e.kind));
        }
    }

    #[test]
    fn field_type_tags_match_the_model_serialization() {
        let ty = FieldType::DateTime;
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], field_type_tag(&ty));
    }

    #[test]
    fn picks_a_widget_per_field_type() {
        assert_eq!(widget_for(&FieldType::Bool), WidgetKind::Checkbox);
        assert_eq!(widget_for(&FieldType::Money), WidgetKind::Number);
        assert_eq!(
            widget_for(&FieldType::Text { max_length: None }),
            WidgetKind::Textbox
        );
        assert_eq!(widget_for(&FieldType::DateTime), WidgetKind::Date);
    }
}
