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
    /// An icon of its own: `icon "triangle-alert"`.
    Icon,
    /// A line between parts, level or `vertical`.
    Separator,
    /// Text that opens an address (`to`), the address it binds, or runs a handler.
    Link,
    /// A short text or number in a pill.
    Badge,
    /// A bar for a number it binds, from 0 to `max`.
    Progress,
    /// A busy indicator, which code shows and hides with `visible`.
    Busy,
    /// A button that stays pressed: a bool, as a checkbox.
    Toggle,
    /// Buttons and toggles joined in one row.
    Buttons,
    /// `section "Title"` blocks that fold, one open at a time.
    Accordion,
    /// Parts on its own grid, scrolling when they reach below its rows.
    Scroll,
    Text,
    Markdown,
    Textbox,
    Memo,
    Password,
    Number,
    Checkbox,
    Switch,
    Dropdown,
    Radio,
    Date,
    /// A color, typed as `#rrggbb` or picked from a palette.
    Color,
    /// A file of a record: chosen, opened, saved as a copy, removed.
    File,
    /// A folder setting's path: typed, or chosen with the system's folder picker.
    Folder,
    /// A list of values as chips: typed or picked, and removed.
    Tags,
    /// A number between a lowest and a highest value, dragged along a bar.
    Slider,
    /// A number typed, or moved a step at a time with its arrows.
    Spinbox,
    Button,
    Form,
    Panel,
    Tabs,
    /// A wizard: `step "Title"` blocks shown one at a time, with Back, Next and Finish (DESIGN 5.15).
    Steps,
    /// Parts on their own grid, outside the window's, opened next to the part in `for` (DESIGN 5.16).
    Popover,
    Splitter,
    Table,
    List,
    /// A list of records as a tree, by the reference to their own kind.
    Tree,
    /// A table whose rows are a tree, its first column indented.
    TreeTable,
    /// Bars, a line or a pie of a list's rows grouped by a field, or of a
    /// list of numbers as they are (DESIGN 5.26).
    Chart,
    /// The last values of one number, taken every so many seconds, as a
    /// line (DESIGN 5.26).
    Trend,
    Image,
    Display,
    Canvas,
    /// Sound files of the resources, tones and melodies, which code tells
    /// it to play; it shows nothing and has no place in the grid (DESIGN 5.22).
    Sound,
    /// Runs its `tick` handler every so many seconds while its window is
    /// open; it shows nothing and has no place in the grid (DESIGN 5.26).
    Timer,
    /// A part an outside library draws, placed by the name it is declared by
    /// (`Arena`, DESIGN 5.19); `props.part` names its declaration.
    Custom,
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
    /// Sound: parts that are heard, not seen.
    Media,
    /// Timers: parts that keep time, not seen.
    Time,
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
    /// A canvas is drawn: when its window opens and after every other
    /// handler of the window.
    Draw,
    /// The pointer moves on a canvas with its button down.
    Drag,
    /// The pointer is let go on a canvas after a drag.
    Drop,
    /// Next is pressed in a wizard, before it moves on; a refusal keeps the step.
    Next,
    /// Finish is pressed on a wizard's last step.
    Finish,
    /// A timer's time has come round again (DESIGN 5.26).
    Tick,
    /// The app's databases did not open, as when a connection kept as a
    /// secret setting is not kept yet (DESIGN 5.27).
    Unconnected,
}

/// The events a window has, in the order the designer lists them.
/// Every event, in the order the language lists them.
pub const ALL_EVENTS: [Event; 19] = [
    Event::Click,
    Event::Change,
    Event::Enter,
    Event::Leave,
    Event::Select,
    Event::Open,
    Event::Pointer,
    Event::Key,
    Event::Close,
    Event::Return,
    Event::Start,
    Event::Submit,
    Event::Draw,
    Event::Drag,
    Event::Drop,
    Event::Next,
    Event::Finish,
    Event::Tick,
    Event::Unconnected,
];

pub const WINDOW_EVENTS: [Event; 3] = [Event::Open, Event::Close, Event::Return];

/// The events the app has: `unconnected` when its databases did not open (DESIGN 5.27).
pub const APP_EVENTS: [Event; 2] = [Event::Start, Event::Unconnected];

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
            Event::Draw => "draw",
            Event::Drag => "drag",
            Event::Drop => "drop",
            Event::Next => "next",
            Event::Finish => "finish",
            Event::Tick => "tick",
            Event::Unconnected => "unconnected",
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
            "draw" => Some(Event::Draw),
            "drag" => Some(Event::Drag),
            "drop" => Some(Event::Drop),
            "next" => Some(Event::Next),
            "finish" => Some(Event::Finish),
            "tick" => Some(Event::Tick),
            "unconnected" => Some(Event::Unconnected),
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
    /// Children inside `step "Title"` blocks, one shown at a time.
    Steps,
    /// Children inside `section "Title"` blocks, which fold, one open at a time.
    Accordion,
    /// Children inside exactly two `pane` blocks.
    Splitter,
}

/// Extra properties beyond position, caption, binding and events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum WidgetProperty {
    Columns,
    Edit,
    Show,
    Orientation,
    Surface,
    Source,
    For,
    Placeholder,
    Readonly,
    TabOrder,
    Shortcut,
    Default,
    Cancel,
    Digits,
    /// `volume 80`: how loud a sound plays, 0 to 100.
    Volume,
    /// `repeat`: a sound plays its source until it is stopped.
    Repeat,
    Units,
    /// `zoom`: a canvas the person zooms and scrolls.
    Zoom,
    /// `grid 20`: a canvas shows a grid.
    Grid,
    /// `snap`: the pointer snaps to a canvas's grid.
    Snap,
    /// `runs Screen`: a canvas an engine beside the screen draws.
    Runs,
    /// `foldable` and `folded`: a panel or form folds by its header.
    Foldable,
    /// `resizable`: a table's header edges are dragged to resize its columns.
    Resizable,
    /// `sortable`: a click on a table's header sorts by that column.
    Sortable,
    /// `order by field [desc]`: the order a table's rows show in first.
    OrderBy,
    /// `icon "save"`: the icon a button shows beside its caption.
    Icon,
    /// `to "https://..."`: the address a link opens.
    To,
    /// `max 250`: the number a progress bar is full at, or the highest value of a slider or a spin box.
    Max,
    /// `min 0`: the lowest value of a slider or a spin box.
    Min,
    /// `step 0.5`: how far a slider or a spin box moves at a time.
    Step,
    /// `split`: a button whose `menu` opens from an arrow beside it.
    Split,
    /// `labels 3`: a form's label column, its inputs' labels in its first columns.
    Labels,
    /// `typed`: a dropdown of text that takes any text, the field's values as suggestions.
    Typed,
    /// `search`: a dropdown of a reference whose records filter as it is typed.
    Search,
    /// `page 50`: a table's rows shown that many at a time.
    Page,
    /// `children children`: the list field a tree's rows are arranged by.
    Children,
    /// `by status`: the field a chart groups its rows by.
    By,
    /// `count`, `sum total`, `average total`: each group's number in a chart.
    Measure,
    /// `points 60`: how many values a trend keeps, or a chart of numbers shows.
    Points,
    /// `every 1`: how many seconds between a timer's ticks, or a trend's values.
    Every,
}

/// The fewest and most rows a table's page holds.
pub const PAGE_SIZES: std::ops::RangeInclusive<u32> = 1..=1000;
pub const PAGE_PROBLEM: &str = "a page holds 1 to 1000 rows";

/// The narrowest and widest a table column can be written, in pixels.
pub const MIN_COLUMN_WIDTH: u32 = 20;
pub const MAX_COLUMN_WIDTH: u32 = 2000;
pub const COLUMN_WIDTH_PROBLEM: &str = "a column is 20 to 2000 pixels wide";

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
    // An icon of its own; what it shows is its first word: `icon "triangle-alert"`.
    text_part(WidgetKind::Icon, "icon", "Icon", (1, 1), None, BindTarget::Nothing, &[]),
    // A short text or number in a pill: written, or what it binds.
    Entry {
        kind: WidgetKind::Badge,
        keyword: "badge",
        name: "Badge",
        category: Category::Text,
        size: (1, 1),
        caption: Some("New"),
        bind: BindTarget::Field,
        field_types: &["text", "integer", "decimal", "money", "enum"],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        properties: &[],
    },
    // Text that opens an address, the address it binds, or runs a handler.
    Entry {
        kind: WidgetKind::Link,
        keyword: "link",
        name: "Link",
        category: Category::Text,
        size: (2, 1),
        caption: Some("Link"),
        bind: BindTarget::Field,
        field_types: &["url", "email", "text"],
        input: false,
        container: None,
        clickable: true,
        events: &[Event::Click],
        properties: &[WidgetProperty::To, WidgetProperty::TabOrder],
    },
    text_part(WidgetKind::Text, "text", "Text", (4, 1), None, BindTarget::Field, &[]),
    // Text written in Markdown: a file of the resources folder
    // (`source "rules.md"`, one per language) or a text it binds.
    Entry {
        field_types: &["text"],
        ..text_part(WidgetKind::Markdown, "markdown", "Markdown", (6, 4), None, BindTarget::Field, &[WidgetProperty::Source])
    },
    input(WidgetKind::Textbox, "textbox", "Text box", (4, 1), &["text", "email", "url"], INPUT_EVENTS, TYPED),
    input(WidgetKind::Memo, "memo", "Memo", (6, 3), &["text"], INPUT_EVENTS, TYPED),
    input(WidgetKind::Password, "password", "Password", (4, 1), &["text", "secret"], INPUT_EVENTS, TYPED),
    input(WidgetKind::Number, "number", "Number", (3, 1), &["integer", "decimal", "money", "id", "duration"], INPUT_EVENTS, TYPED),
    Entry {
        caption: Some("Check box"),
        ..input(WidgetKind::Checkbox, "checkbox", "Check box", (3, 1), &["bool"], &[Event::Change], PICKED)
    },
    Entry {
        caption: Some("Toggle"),
        ..input(WidgetKind::Toggle, "toggle", "Toggle button", (2, 1), &["bool"], &[Event::Change], &[WidgetProperty::Readonly, WidgetProperty::TabOrder, WidgetProperty::Icon])
    },
    Entry {
        caption: Some("Switch"),
        ..input(WidgetKind::Switch, "switch", "Switch", (3, 1), &["bool"], &[Event::Change], PICKED)
    },
    // Also the language and the theme of the settings: the project's languages, the offered themes.
    // `typed`, a text with suggestions; `search show name, city`, a reference among many records.
    input(
        WidgetKind::Dropdown,
        "dropdown",
        "Dropdown",
        (4, 1),
        &["enum", "ref", "language", "theme", "text"],
        &[Event::Change],
        &[WidgetProperty::Typed, WidgetProperty::Search, WidgetProperty::Show, WidgetProperty::Readonly, WidgetProperty::TabOrder],
    ),
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
    // A file of a record (U1c): its name, with buttons to choose, open, save a copy of and remove it.
    input(WidgetKind::File, "file", "File", (6, 1), &["file"], &[Event::Change], PICKED),
    input(WidgetKind::Folder, "folder", "Folder", (6, 1), &["folder"], INPUT_EVENTS, TYPED),
    // A list field of values (U1c): its items as chips, a typed item or an enum's value added, one removed.
    input(WidgetKind::Tags, "tags", "Tags", (6, 1), &["values"], &[Event::Change], PICKED),
    // A color (U1c): typed as #rrggbb, or picked from the palette every renderer shares.
    input(WidgetKind::Color, "color", "Color", (3, 1), &["color"], &[Event::Change], PICKED),
    // A number between its lowest and highest value (U1c): the range is written, or the field's own.
    input(
        WidgetKind::Slider,
        "slider",
        "Slider",
        (6, 1),
        &["integer", "decimal", "money"],
        &[Event::Change],
        &[WidgetProperty::Min, WidgetProperty::Max, WidgetProperty::Step, WidgetProperty::Readonly, WidgetProperty::TabOrder],
    ),
    input(
        WidgetKind::Spinbox,
        "spinbox",
        "Spin box",
        (3, 1),
        &["integer", "decimal", "money"],
        INPUT_EVENTS,
        &[
            WidgetProperty::Min,
            WidgetProperty::Max,
            WidgetProperty::Step,
            WidgetProperty::Placeholder,
            WidgetProperty::Readonly,
            WidgetProperty::TabOrder,
        ],
    ),
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
            WidgetProperty::Icon,
            WidgetProperty::Split,
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
        properties: &[WidgetProperty::Foldable, WidgetProperty::Labels],
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
        properties: &[WidgetProperty::Foldable],
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
    // Sections that fold, one open at a time.
    Entry {
        kind: WidgetKind::Accordion,
        keyword: "accordion",
        name: "Accordion",
        category: Category::Containers,
        size: (6, 8),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: Some(ContainerKind::Accordion),
        clickable: false,
        events: &[Event::Change],
        properties: &[],
    },
    // Parts that scroll into view below its rows.
    Entry {
        kind: WidgetKind::Scroll,
        keyword: "scroll",
        name: "Scroll box",
        category: Category::Containers,
        size: (6, 6),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: Some(ContainerKind::Plain),
        clickable: false,
        events: &[],
        properties: &[],
    },
    // Buttons and toggles joined in one row.
    Entry {
        kind: WidgetKind::Buttons,
        keyword: "buttons",
        name: "Button group",
        category: Category::Containers,
        size: (4, 1),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: Some(ContainerKind::Plain),
        clickable: false,
        events: &[],
        properties: &[],
    },
    // A line between parts.
    Entry {
        kind: WidgetKind::Separator,
        keyword: "separator",
        name: "Separator",
        category: Category::Containers,
        size: (12, 1),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        properties: &[WidgetProperty::Orientation],
    },
    Entry {
        kind: WidgetKind::Steps,
        keyword: "steps",
        name: "Steps",
        category: Category::Containers,
        size: (12, 8),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: Some(ContainerKind::Steps),
        clickable: false,
        events: &[Event::Next, Event::Finish],
        properties: &[],
    },
    Entry {
        kind: WidgetKind::Popover,
        keyword: "popover",
        name: "Popover",
        category: Category::Containers,
        size: (4, 3),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: Some(ContainerKind::Plain),
        clickable: false,
        events: &[],
        properties: &[WidgetProperty::For],
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
        events: &[Event::Select, Event::Open, Event::Change],
        properties: &[WidgetProperty::Columns, WidgetProperty::Edit, WidgetProperty::Resizable, WidgetProperty::Sortable, WidgetProperty::OrderBy, WidgetProperty::Page],
    },
    // A list the window holds as a tree, arranged by `children` (U1c).
    Entry {
        kind: WidgetKind::Tree,
        keyword: "tree",
        name: "Tree",
        category: Category::Data,
        size: (4, 8),
        caption: None,
        bind: BindTarget::Collection,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::Select, Event::Open],
        properties: &[WidgetProperty::Children, WidgetProperty::Show],
    },
    Entry {
        kind: WidgetKind::TreeTable,
        keyword: "treetable",
        name: "Tree table",
        category: Category::Data,
        size: (12, 8),
        caption: None,
        bind: BindTarget::Collection,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::Select, Event::Open],
        properties: &[WidgetProperty::Children, WidgetProperty::Columns, WidgetProperty::Resizable],
    },
    // A list the window holds as a chart: `chart bar ... by status count` (U1c).
    Entry {
        kind: WidgetKind::Chart,
        keyword: "chart",
        name: "Chart",
        category: Category::Data,
        size: (6, 6),
        caption: None,
        bind: BindTarget::Collection,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        properties: &[WidgetProperty::By, WidgetProperty::Measure, WidgetProperty::Points, WidgetProperty::Max],
    },
    // One number's last values as a line, taken every so many seconds (DESIGN 5.26).
    Entry {
        kind: WidgetKind::Trend,
        keyword: "trend",
        name: "Trend",
        category: Category::Data,
        size: (6, 4),
        caption: None,
        bind: BindTarget::Field,
        field_types: &["integer", "decimal", "money", "duration"],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        properties: &[WidgetProperty::Points, WidgetProperty::Max, WidgetProperty::Every],
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
        kind: WidgetKind::Progress,
        keyword: "progress",
        name: "Progress",
        category: Category::Data,
        size: (6, 1),
        caption: None,
        bind: BindTarget::Field,
        field_types: &["integer", "decimal"],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        properties: &[WidgetProperty::Max],
    },
    // Busy: shown while code says so, with what it is busy with.
    Entry {
        kind: WidgetKind::Busy,
        keyword: "busy",
        name: "Busy",
        category: Category::Data,
        size: (3, 1),
        caption: Some("Busy..."),
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        properties: &[],
    },
    Entry {
        kind: WidgetKind::Display,
        keyword: "display",
        name: "Display",
        category: Category::Graphics,
        size: (4, 2),
        caption: None,
        // A number in seven-segment digits, as on a calculator.
        bind: BindTarget::Field,
        field_types: &["integer", "decimal", "money"],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        properties: &[WidgetProperty::Digits],
    },
    Entry {
        kind: WidgetKind::Image,
        keyword: "image",
        name: "Image",
        category: Category::Graphics,
        size: (3, 3),
        caption: None,
        // A picture of a record (`bind album.cover`), or a fixed one of the
        // resources folder (`source "logo.png"`).
        bind: BindTarget::Field,
        field_types: &["image"],
        // Bound, it takes a picture from a file: a label or help is for it.
        input: true,
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
        // Drawn by its draw handler (`on draw`);
        // a click runs a handler, which reads where as `canvas.x` and `canvas.y`;
        // a drag and a drop also read where it was pressed, `fromX` and `fromY`.
        events: &[Event::Draw, Event::Click, Event::Drag, Event::Drop, Event::Pointer, Event::Key],
        properties: &[WidgetProperty::Units, WidgetProperty::Zoom, WidgetProperty::Grid, WidgetProperty::Snap, WidgetProperty::Runs, WidgetProperty::Surface],
    },
    // A custom part is written by its declared name; its keyword is two
    // words, so it is never typed and reads well in messages. The Parts
    // palette lists the declared parts in its place.
    Entry {
        kind: WidgetKind::Sound,
        keyword: "sound",
        name: "Sound",
        category: Category::Media,
        // Never in the grid: the designer keeps it in a tray under the window.
        size: (1, 1),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[],
        properties: &[WidgetProperty::Source, WidgetProperty::Volume, WidgetProperty::Repeat],
    },
    Entry {
        kind: WidgetKind::Timer,
        keyword: "timer",
        name: "Timer",
        category: Category::Time,
        // Never in the grid: the designer keeps it in the tray under the window, with the sounds.
        size: (1, 1),
        caption: None,
        bind: BindTarget::Nothing,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::Tick],
        properties: &[WidgetProperty::Every],
    },
    Entry {
        kind: WidgetKind::Custom,
        keyword: "custom part",
        name: "Custom part",
        category: Category::Graphics,
        size: (4, 3),
        caption: None,
        bind: BindTarget::Field,
        field_types: &[],
        input: false,
        container: None,
        clickable: false,
        events: &[Event::Change],
        properties: &[WidgetProperty::TabOrder],
    },
];

/// The words every placed custom part may have, which its own properties cannot be named.
pub const CUSTOM_PART_WORDS: [&str; 11] = ["at", "span", "rows", "bind", "name", "on", "tooltip", "taborder", "visible", "enabled", "value"];

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

/// The word a part's line starts with: a custom part's declared name
/// (`Arena`), else its kind's keyword.
pub fn written_word(props: &WidgetProps) -> String {
    match (&props.part, props.kind) {
        (Some(part), WidgetKind::Custom) => part.clone(),
        _ => keyword(props.kind).to_string(),
    }
}

/// What a part's name starts from when it has nothing better: `orbField` for
/// a custom part `OrbField`, else its kind's keyword, as in `button3`.
pub fn name_base(props: &WidgetProps) -> String {
    match (&props.part, props.kind) {
        (Some(part), WidgetKind::Custom) => crate::names::camel_case(part),
        _ => keyword(props.kind).to_string(),
    }
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

/// The range of a slider or a spin box: its lowest and highest value and
/// its step, as numbers written in Clarity. `None` is no end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct NumberRange {
    pub min: Option<String>,
    pub max: Option<String>,
    pub step: String,
}

/// The range a slider or a spin box moves in: `min`, `max` and `step` as
/// written, else the bound field's own `min` and `max` (`field`), and a
/// step of 1. A slider without either end runs from 0 to 100.
pub fn number_range(props: &WidgetProps, field: Option<&crate::model::Modifiers>) -> NumberRange {
    use crate::model::Literal;
    let own = |literal: Option<&Literal>| match literal {
        Some(Literal::Number(n)) => Some(n.clone()),
        _ => None,
    };
    let mut min = props.min.clone().or_else(|| own(field.and_then(|m| m.min.as_ref())));
    let mut max = props.max.clone().or_else(|| own(field.and_then(|m| m.max.as_ref())));
    if props.kind == WidgetKind::Slider {
        min.get_or_insert_with(|| "0".to_string());
        max.get_or_insert_with(|| "100".to_string());
    }
    NumberRange { min, max, step: props.step.clone().unwrap_or_else(|| "1".to_string()) }
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
        FieldType::Duration => "duration",
        FieldType::Image => "image",
        FieldType::File => "file",
        FieldType::Email => "email",
        FieldType::Url => "url",
        FieldType::Color => "color",
        FieldType::Secret => "secret",
        FieldType::Folder => "folder",
        FieldType::Language => "language",
        FieldType::Theme => "theme",
        FieldType::Id => "id",
        FieldType::Enum { .. } => "enum",
        FieldType::Ref { .. } => "ref",
        FieldType::Record { .. } => "record",
        FieldType::List { .. } => "list",
        FieldType::Values { .. } => "values",
    }
}

/// Whether a table cell can edit a field of this type: a checkbox for
/// bool, a dropdown for an enum or a reference, a picker for dates and
/// moments, an input for text and numbers.
pub fn cell_edits(ty: &FieldType) -> bool {
    matches!(
        ty,
        FieldType::Text { .. }
            | FieldType::Email
            | FieldType::Url
            | FieldType::Color
            | FieldType::Integer
            | FieldType::Decimal { .. }
            | FieldType::Money
            | FieldType::Bool
            | FieldType::Date
            | FieldType::DateTime
            | FieldType::Enum { .. }
            | FieldType::Ref { .. }
    )
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
        edit: Vec::new(),
        switches: Vec::new(),
        widths: Default::default(),
        resizable: false,
        sortable: false,
        order: None,
        show: Vec::new(),
        // A splitter is written with its direction; a separator is level unless written vertical.
        orientation: (has_property(kind, WidgetProperty::Orientation) && kind != WidgetKind::Separator)
            .then_some(Orientation::Horizontal),
        surface: has_property(kind, WidgetProperty::Surface).then_some(Surface::Svg),
        source: None,
        for_part: None,
        placeholder: None,
        digits: None,
        units: None,
        zoom: false,
        grid: None,
        snap: false,
        runs: None,
        tooltip: None,
        icon: (kind == WidgetKind::Icon).then(|| "star".to_string()),
        to: None,
        max: None,
        min: None,
        step: None,
        split: false,
        typed: false,
        page: None,
        children: None,
        chart: (kind == WidgetKind::Chart).then_some(crate::model::ChartKind::Bar),
        by: None,
        measure: (kind == WidgetKind::Chart).then_some(crate::model::ChartMeasure::Count),
        search: false,
        labels: None,
        volume: None,
        repeat: false,
        points: None,
        every: None,
        made: false,
        style: None,
        readonly: false,
        foldable: false,
        folded: false,
        tab_order: None,
        shortcut: None,
        default_button: false,
        cancel: false,
        part: None,
        values: Default::default(),
        tells: Vec::new(),
    }
}

/// Writes widget properties as one canonical spec line, in a fixed order.
/// A wired handler as written: `digit` or with its values, `digit(7)`.
pub fn handler_call(h: &crate::model::Handler) -> String {
    if h.args.is_empty() {
        return h.handler.clone();
    }
    let values: Vec<String> = h.args.iter().map(literal_text).collect();
    format!("{}({})", h.handler, values.join(", "))
}

/// A value as it is written in Clarity.
pub fn literal_text(value: &crate::model::Literal) -> String {
    use crate::model::Literal;
    match value {
        Literal::Number(n) | Literal::Ident(n) => n.clone(),
        Literal::Str(s) => crate::lexer::text_token(s),
        Literal::Bool(b) => b.to_string(),
    }
}

pub fn widget_line(props: &WidgetProps) -> String {
    // A timer has no place: its name, how often it ticks, its handler (DESIGN 5.26).
    if props.kind == WidgetKind::Timer {
        let mut parts = vec![keyword(props.kind).to_string()];
        if let Some(name) = &props.name {
            parts.push(format!("name {name}"));
        }
        if let Some(every) = &props.every {
            parts.push(format!("every {every}"));
        }
        for h in &props.handlers {
            parts.push(format!("on {}: {}", h.event.keyword(), handler_call(h)));
        }
        return parts.join(" ");
    }
    // A sound has no place: its name, its source, its volume, whether it repeats (DESIGN 5.22).
    if props.kind == WidgetKind::Sound {
        let mut parts = vec![keyword(props.kind).to_string()];
        if let Some(name) = &props.name {
            parts.push(format!("name {name}"));
        }
        if let Some(source) = &props.source {
            parts.push(format!("source {}", quote(source)));
        }
        if let Some(volume) = props.volume {
            parts.push(format!("volume {volume}"));
        }
        if props.repeat {
            parts.push("repeat".to_string());
        }
        return parts.join(" ");
    }
    let first = match (&props.part, props.kind) {
        (Some(part), WidgetKind::Custom) => part.clone(),
        _ => keyword(props.kind).to_string(),
    };
    let mut parts = vec![first];
    // A chart's first word is its kind.
    if let Some(chart) = props.chart.filter(|_| props.kind == WidgetKind::Chart) {
        parts.push(chart.word().to_string());
    }
    // The icon part's first word is the icon it shows; a button's icon follows its name.
    if let Some(icon) = props.icon.as_ref().filter(|_| props.kind == WidgetKind::Icon) {
        parts.push(quote(icon));
    }
    if let Some(caption) = &props.caption {
        parts.push(crate::lexer::text_token(caption));
    }
    if let Some(name) = &props.name {
        parts.push(format!("name {name}"));
    }
    if let Some(icon) = props.icon.as_ref().filter(|_| props.kind != WidgetKind::Icon) {
        parts.push(format!("icon {}", quote(icon)));
    }
    if props.split {
        parts.push("split".to_string());
    }
    // A popover has no place in the window's grid: it opens next to its part.
    if props.kind == WidgetKind::Popover {
        if let Some(target) = &props.for_part {
            parts.push(format!("for {target}"));
        }
    } else {
        parts.push(format!("at {},{}", props.col, props.row));
    }
    parts.push(format!("span {}", props.width));
    // `rows` is left out only where the catalog height is also 1, so a
    // one-row panel does not read back at its default height.
    if props.height != 1 || default_size(props.kind).1 != 1 {
        parts.push(format!("rows {}", props.height));
    }
    if let Some(orientation) = props.orientation {
        parts.push(orientation_keyword(orientation).to_string());
    }
    if let Some(to) = &props.to {
        parts.push(format!("to {}", quote(to)));
    }
    if let Some(bind) = &props.bind {
        parts.push(format!("bind {bind}"));
    }
    for (name, value) in &props.values {
        parts.push(format!("{name} {}", literal_text(value)));
    }
    if let Some(children) = &props.children {
        parts.push(format!("children {children}"));
    }
    if let Some(by) = &props.by {
        parts.push(format!("by {by}"));
    }
    if let Some(measure) = &props.measure {
        parts.push(measure.text());
    }
    if let Some(min) = &props.min {
        parts.push(format!("min {min}"));
    }
    if let Some(points) = props.points {
        parts.push(format!("points {points}"));
    }
    if let Some(max) = &props.max {
        parts.push(format!("max {max}"));
    }
    if let Some(every) = &props.every {
        parts.push(format!("every {every}"));
    }
    if let Some(step) = &props.step {
        parts.push(format!("step {step}"));
    }
    if let Some(labels) = props.labels {
        parts.push(format!("labels {labels}"));
    }
    if let Some(digits) = props.digits {
        parts.push(format!("digits {digits}"));
    }
    if let Some(units) = props.units {
        parts.push(format!("units {} x {}", units.width, units.height));
    }
    if props.zoom {
        parts.push("zoom".to_string());
    }
    if let Some(grid) = props.grid {
        parts.push(format!("grid {grid}"));
    }
    if props.snap {
        parts.push("snap".to_string());
    }
    if let Some(engine) = &props.runs {
        parts.push(format!("runs {engine}"));
    }
    if !props.columns.is_empty() {
        let columns: Vec<String> = props
            .columns
            .iter()
            .map(|f| match props.widths.get(f) {
                Some(width) => format!("{f} {width}"),
                None => f.clone(),
            })
            .collect();
        parts.push(format!("columns {}", columns.join(", ")));
    }
    if !props.edit.is_empty() {
        let edited: Vec<String> = props
            .edit
            .iter()
            .map(|f| if props.switches.contains(f) { format!("{f} as switch") } else { f.clone() })
            .collect();
        parts.push(format!("edit {}", edited.join(", ")));
    }
    if props.resizable {
        parts.push("resizable".to_string());
    }
    if props.sortable {
        parts.push("sortable".to_string());
    }
    if let Some(order) = &props.order {
        parts.push(if order.desc { format!("order by {} desc", order.field) } else { format!("order by {}", order.field) });
    }
    if let Some(page) = props.page {
        parts.push(format!("page {page}"));
    }
    if props.typed {
        parts.push("typed".to_string());
    }
    if props.search {
        parts.push("search".to_string());
    }
    if !props.show.is_empty() {
        parts.push(format!("show {}", props.show.join(", ")));
    }
    if let Some(surface) = props.surface {
        parts.push(format!("surface {}", surface_keyword(surface)));
    }
    if let Some(source) = &props.source {
        parts.push(format!("source {}", quote(source)));
    }
    if let Some(target) = props.for_part.as_ref().filter(|_| props.kind != WidgetKind::Popover) {
        parts.push(format!("for {target}"));
    }
    if let Some(placeholder) = &props.placeholder {
        parts.push(format!("placeholder {}", crate::lexer::text_token(placeholder)));
    }
    if let Some(tooltip) = &props.tooltip {
        parts.push(format!("tooltip {}", crate::lexer::text_token(tooltip)));
    }
    if let Some(style) = &props.style {
        parts.push(format!("style {style}"));
    }
    if props.readonly {
        parts.push("readonly".to_string());
    }
    if props.foldable {
        parts.push(if props.folded { "foldable folded" } else { "foldable" }.to_string());
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
        parts.push(format!("on {}: {}", h.event.keyword(), handler_call(h)));
    }
    for t in &props.tells {
        parts.push(format!("on {}: {}", t.tell, t.handler));
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
/// A dropdown is `typed` or `search`, and only a search shows fields.
pub fn dropdown_options(props: &WidgetProps) -> Result<(), String> {
    if props.kind != WidgetKind::Dropdown {
        return Ok(());
    }
    if props.typed && props.search {
        return Err("a dropdown is 'typed' or 'search', not both".to_string());
    }
    if !props.show.is_empty() && !props.search {
        return Err("'show' needs 'search': write 'search show name, city'".to_string());
    }
    Ok(())
}

/// A canvas's grid is 1 to 10000 units, and it snaps only to a grid it has.
pub fn canvas_options(props: &WidgetProps) -> Result<(), String> {
    if props.grid.is_some_and(|g| !(1..=10000).contains(&g)) {
        return Err(CANVAS_GRID_PROBLEM.to_string());
    }
    if props.snap && props.grid.is_none() {
        return Err("'snap' snaps the pointer to the grid; give the canvas one, as in grid 20".to_string());
    }
    Ok(())
}

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
        (WidgetProperty::Edit, !props.edit.is_empty(), "edit"),
        (WidgetProperty::Show, !props.show.is_empty(), "show"),
        (
            WidgetProperty::Orientation,
            props.orientation.is_some(),
            "orientation",
        ),
        (WidgetProperty::Surface, props.surface.is_some(), "surface"),
        (WidgetProperty::Source, props.source.is_some(), "source"),
        (WidgetProperty::For, props.for_part.is_some(), "for"),
        (
            WidgetProperty::Placeholder,
            props.placeholder.is_some(),
            "placeholder",
        ),
        (WidgetProperty::Readonly, props.readonly, "readonly"),
        (WidgetProperty::Foldable, props.foldable, "foldable"),
        (WidgetProperty::TabOrder, props.tab_order.is_some(), "taborder"),
        (WidgetProperty::Shortcut, props.shortcut.is_some(), "key"),
        (WidgetProperty::Default, props.default_button, "default"),
        (WidgetProperty::Cancel, props.cancel, "cancel"),
        (WidgetProperty::Digits, props.digits.is_some(), "digits"),
        (WidgetProperty::Units, props.units.is_some(), "units"),
        (WidgetProperty::Zoom, props.zoom, "zoom"),
        (WidgetProperty::Grid, props.grid.is_some(), "grid"),
        (WidgetProperty::Snap, props.snap, "snap"),
        (WidgetProperty::Runs, props.runs.is_some(), "runs"),
        (WidgetProperty::Resizable, props.resizable, "resizable"),
        (WidgetProperty::Sortable, props.sortable, "sortable"),
        (WidgetProperty::OrderBy, props.order.is_some(), "order by"),
        (WidgetProperty::Icon, props.icon.is_some() && props.kind != WidgetKind::Icon, "icon"),
        (WidgetProperty::To, props.to.is_some(), "to"),
        (WidgetProperty::Max, props.max.is_some(), "max"),
        (WidgetProperty::Points, props.points.is_some(), "points"),
        (WidgetProperty::Every, props.every.is_some(), "every"),
        (WidgetProperty::Min, props.min.is_some(), "min"),
        (WidgetProperty::Step, props.step.is_some(), "step"),
        (WidgetProperty::Split, props.split, "split"),
        (WidgetProperty::Labels, props.labels.is_some(), "labels"),
        (WidgetProperty::Typed, props.typed, "typed"),
        (WidgetProperty::Search, props.search, "search"),
        (WidgetProperty::Page, props.page.is_some(), "page"),
        (WidgetProperty::Children, props.children.is_some(), "children"),
        (WidgetProperty::By, props.by.is_some(), "by"),
        (WidgetProperty::Measure, props.measure.is_some(), "count"),
    ];
    for (property, present, what) in checks {
        if present && !has_property(props.kind, property) {
            return Err(format!("'{what}' does not apply to a {name}"));
        }
    }
    dropdown_options(props)?;
    if props.page.is_some_and(|p| !PAGE_SIZES.contains(&p)) {
        return Err(PAGE_PROBLEM.to_string());
    }
    if props.folded && !props.foldable {
        return Err("'folded' needs 'foldable': write 'foldable folded'".to_string());
    }
    if props.digits.is_some_and(|d| !(1..=MAX_DIGITS).contains(&d)) {
        return Err(format!("a display has 1 to {MAX_DIGITS} digits"));
    }
    if props.units.is_some_and(|u| !canvas_units_fit(u.width, u.height)) {
        return Err(CANVAS_UNITS_PROBLEM.to_string());
    }
    canvas_options(props)?;
    if let Some(field) = props.switches.iter().find(|f| !props.edit.contains(f)) {
        return Err(format!("'{field}' is a switch but is not edited"));
    }
    for field in props.columns.iter().chain(&props.edit).chain(&props.show) {
        identifier("field name", field)?;
    }
    if props.widths.values().any(|w| !(MIN_COLUMN_WIDTH..=MAX_COLUMN_WIDTH).contains(w)) {
        return Err(COLUMN_WIDTH_PROBLEM.to_string());
    }
    if let Some(order) = &props.order {
        identifier("field name", &order.field)?;
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

/// The units a canvas draws in without `units W x H`.
pub const DEFAULT_UNITS: (u32, u32) = (100, 100);
pub const CANVAS_UNITS_PROBLEM: &str = "a canvas is 1 to 10000 units wide and high";

/// A canvas's grid is so many of its units apart.
pub const CANVAS_GRID_PROBLEM: &str = "a canvas's grid is 1 to 10000 units";

pub fn canvas_units_fit(width: u32, height: u32) -> bool {
    (1..=10000).contains(&width) && (1..=10000).contains(&height)
}

/// The digits a display has without `digits n`, and the most it can have.
pub const DEFAULT_DIGITS: u32 = 12;
pub const MAX_DIGITS: u32 = 24;

/// Keys a shortcut can hold besides its modifiers, by name.
const NAMED_KEYS: [&str; 15] = [
    "Enter", "Escape", "Tab", "Space", "Backspace", "Delete", "Insert", "Home", "End", "PageUp",
    "PageDown", "Up", "Down", "Left", "Right",
];

/// Characters that are keys of their own: a calculator's operators, the
/// decimal point and equals.
pub const SYMBOL_KEYS: [&str; 7] = ["+", "-", "*", "/", ".", ",", "="];

/// The modifiers of a shortcut and its key: `Ctrl++` is Ctrl and `+`.
pub fn shortcut_keys(shortcut: &str) -> (Vec<&str>, &str) {
    let (modifiers, key) = match shortcut.strip_suffix("++") {
        // `Ctrl++`: the last `+` is the key.
        Some(rest) if !rest.is_empty() => (rest, "+"),
        _ if shortcut == "+" => ("", "+"),
        _ => shortcut.rsplit_once('+').unwrap_or(("", shortcut)),
    };
    let modifiers = if modifiers.is_empty() { Vec::new() } else { modifiers.split('+').collect() };
    (modifiers, key)
}

/// `Ctrl+S`, `Ctrl+Shift+F2`, `+`: modifiers, then one key.
pub fn check_shortcut(shortcut: &str) -> Result<(), String> {
    let not = || Err(format!("'{shortcut}' is not a shortcut, as in 'Ctrl+S'"));
    let (modifiers, key) = shortcut_keys(shortcut);
    for (i, modifier) in modifiers.iter().enumerate() {
        if !["Ctrl", "Alt", "Shift", "Meta"].contains(modifier) || modifiers[..i].contains(modifier) {
            return not();
        }
    }
    let single = key.len() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric());
    let function = key
        .strip_prefix('F')
        .and_then(|n| n.parse::<u32>().ok())
        .is_some_and(|n| (1..=12).contains(&n));
    if single || function || NAMED_KEYS.contains(&key) || SYMBOL_KEYS.contains(&key) {
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
            | WidgetKind::Steps
            | WidgetKind::Accordion
            | WidgetKind::Scroll
            | WidgetKind::Splitter
            | WidgetKind::Table
            | WidgetKind::List
            | WidgetKind::Tree
            | WidgetKind::TreeTable
            | WidgetKind::Chart
            | WidgetKind::Image
            | WidgetKind::Canvas
            | WidgetKind::Markdown
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
        // A secret shows only as dots (DESIGN 5.27).
        assert_eq!(widget_for(&FieldType::Secret), WidgetKind::Password);
    }
}
