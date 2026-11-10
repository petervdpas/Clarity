//! The neutral model: what a spec file declares, independent of any target
//! language. Every node keeps its span so tools (the designer, the LSP) can
//! map model elements back to source text.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::diagnostic::Diagnostic;
use crate::span::Span;
use crate::widgets::{Event, WidgetKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SpecFile {
    pub path: String,
    /// Hash of the source text. Editors send it back with an edit, so an
    /// edit based on an outdated file is rejected.
    pub version: String,
    pub module: Option<Ident>,
    pub uses: Vec<Ident>,
    /// `source erp`: the data source the module's entities live in; the
    /// project's first data source when there is none.
    pub source: Option<Ident>,
    pub enums: Vec<EnumDecl>,
    pub entities: Vec<Entity>,
    /// Its records: values with fields that belong to no table (DESIGN 5.12).
    pub records: Vec<Entity>,
    pub pages: Vec<Page>,
    pub apps: Vec<App>,
    /// Themes: named colors, a font and named styles for parts.
    pub themes: Vec<ThemeDecl>,
    /// Code behind: one block per window or app, with its handlers.
    pub code: Vec<crate::code::CodeBlock>,
    /// Functions and actions declared in this module.
    pub functions: Vec<crate::code::FunctionDecl>,
    /// Its constants (DESIGN 5.12): `const lives = 3`.
    pub consts: Vec<crate::code::ConstDecl>,
    /// Its engines (DESIGN 5.11).
    pub engines: Vec<crate::code::EngineDecl>,
    /// The `cli` block of a command-line program (DESIGN 5.20).
    pub clis: Vec<crate::code::CliDecl>,
    /// Its commands, for the command-line program (DESIGN 5.20).
    pub commands: Vec<crate::code::CommandDecl>,
    /// Its custom parts, drawn by outside libraries (DESIGN 5.19).
    pub parts: Vec<crate::code::PartDecl>,
    /// Its outside libraries, called from code (DESIGN 5.21).
    pub libraries: Vec<crate::code::LibraryDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EnumDecl {
    pub name: Ident,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EnumVariant {
    pub name: Ident,
    /// The explicit label, or one derived from the name.
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Entity {
    pub name: Ident,
    pub fields: Vec<Field>,
    /// `table "tblInvoice"`: the existing table of an entity in an external
    /// data source.
    pub table: Option<String>,
    /// `key "InvoiceID"`: the integer column that identifies a record, in
    /// an existing table; `id` otherwise.
    pub key: Option<String>,
    /// The project's settings (`settings` block, DESIGN 5.17): kept per
    /// person on their machine, never in the database; named `Settings`.
    #[serde(default)]
    pub settings: bool,
    /// `record Shape` (DESIGN 5.12): values with fields that belong to no
    /// table, never stored; kept in the module's `records`, not its entities.
    #[serde(default)]
    pub record: bool,
    pub span: Span,
}

impl Entity {
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name.name == name)
    }

    /// The table the records are stored in.
    pub fn table_name(&self) -> String {
        self.table.clone().unwrap_or_else(|| snake_case(&self.name.name))
    }

    /// The column that identifies a record.
    pub fn key_column(&self) -> &str {
        self.key.as_deref().unwrap_or("id")
    }

    /// The column a field is stored in: its `column`, else its name in
    /// snake case (`creditLimit` in `credit_limit`).
    pub fn column_of(&self, field: &str) -> String {
        self.field(field).and_then(|f| f.modifiers.column.clone()).unwrap_or_else(|| snake_case(field))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Field {
    pub name: Ident,
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub ty: FieldType,
    /// The type as written in spec files: `text(80)`, `ref Customer`.
    #[serde(rename = "typeText")]
    #[ts(rename = "typeText")]
    pub type_text: String,
    pub modifiers: Modifiers,
    /// `modifiers.label`, or a label derived from the name.
    pub label: String,
    pub span: Span,
}

/// How a graph places its nodes (docs/GRAPH-PLAN.md): pushed apart along
/// their edges, in layers along the edges' direction, in rings around the
/// chosen node, or on one circle. Every layout places the same graph the
/// same way each time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GraphLayout {
    #[default]
    Force,
    Layered,
    Radial,
    Circle,
}

/// The shape a graph draws a node as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NodeShape {
    Circle,
    Square,
    Diamond,
    /// A rounded box with the label inside it.
    Box,
}

/// The line a graph draws an edge with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EdgeLine {
    Solid,
    Dashed,
    Dotted,
}

/// How a graph draws the nodes of one kind; what is left out takes the
/// graph's own: a circle in the kind's color of the palette.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NodeLook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<NodeShape>,
    /// A color of the theme (`accent`) or written out (`#d97706`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Across, in pixels at 100%.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
}

/// How a graph draws the edges of one kind; what is left out takes the
/// graph's own: a solid line with an arrow at its target.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EdgeLook {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<EdgeLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrow: Option<bool>,
}

/// What a graph of the data model knows of the model (docs/GRAPH-PLAN.md,
/// step 2), worked out when the project loads: each list it shows, the
/// entity of its records, the field a record shows as its label, and its
/// references to the other entities shown, which become its edges.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GraphModel {
    pub lists: Vec<GraphList>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GraphList {
    pub list: String,
    pub entity: String,
    /// The record's first text field, as a reference shows a record; none: it shows its entity and id.
    pub label: Option<String>,
    pub refs: Vec<GraphRef>,
}

/// A reference field of a list's records and the entity it refers to.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GraphRef {
    pub field: String,
    pub to: String,
}

/// What a chart draws: bars, a line through points, or slices of a pie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ChartKind {
    Bar,
    Line,
    Pie,
}

impl ChartKind {
    pub fn word(self) -> &'static str {
        match self {
            ChartKind::Bar => "bar",
            ChartKind::Line => "line",
            ChartKind::Pie => "pie",
        }
    }

    pub fn of_word(word: &str) -> Option<ChartKind> {
        [ChartKind::Bar, ChartKind::Line, ChartKind::Pie].into_iter().find(|k| k.word() == word)
    }
}

/// The number of each group of a chart: its rows counted, or a number field summed or averaged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum ChartMeasure {
    Count,
    Sum { field: String },
    Average { field: String },
}

impl ChartMeasure {
    /// As Clarity writes it: `count`, `sum total`.
    pub fn text(&self) -> String {
        match self {
            ChartMeasure::Count => "count".to_string(),
            ChartMeasure::Sum { field } => format!("sum {field}"),
            ChartMeasure::Average { field } => format!("average {field}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum FieldType {
    Text {
        #[serde(rename = "maxLength")]
        #[ts(rename = "maxLength")]
        max_length: Option<u32>,
    },
    Integer,
    Decimal {
        precision: u32,
        scale: u32,
    },
    Money,
    Bool,
    Date,
    DateTime,
    /// A length of time in whole seconds, shown as `3:45`.
    Duration,
    /// A picture: kept in the database, or as a file in a file store
    /// (`store photos`).
    Image,
    /// Any file, with its name: kept in the database (its name in a column
    /// beside it) or, with `store`, as a file of a file store (U1c).
    File,
    Email,
    Url,
    /// A color, kept as text `#rrggbb` (U1c).
    Color,
    /// A setting kept in the system's protected store, per person and per
    /// application, never in settings.json (DESIGN 5.27): a web API's token.
    Secret,
    /// A folder on the person's machine, a setting kept in settings.json:
    /// where a file store keeps its files (`filestore photos
    /// "setting:photosFolder"`), chosen while the application runs.
    Folder,
    /// The language shown, a setting of Clarity's own (`language` in `settings`).
    Language,
    /// The theme shown, a setting of Clarity's own (`theme` in `settings`).
    Theme,
    Id,
    /// A named type that must resolve to an enum.
    Enum {
        name: Ident,
    },
    Ref {
        entity: Ident,
    },
    /// A record of another entity held inside this one (DESIGN 5.25), as a
    /// web API's JSON nests it: `owner: User`. Only an entity of a web API
    /// has one; a stored entity refers to another with `ref`.
    Record {
        entity: Ident,
    },
    /// Records of another entity, kept in their own table; in an entity of
    /// a web API, held inside the record (DESIGN 5.25).
    List {
        item: Ident,
    },
    /// A list of plain values, kept in one column as JSON (U1c):
    /// `list<text>`, `list<Genre>`.
    Values {
        item: ListItem,
    },
}

impl FieldType {
    /// Whether the field holds records rather than a value: `list<Line>`,
    /// kept in its own table or (in a web API) inside the record, and a
    /// record held inside (DESIGN 5.25). Neither is a column of its table.
    pub fn is_held(&self) -> bool {
        matches!(self, FieldType::List { .. } | FieldType::Record { .. })
    }

    /// The places a decimal shows: its scale, or none for a record's decimal
    /// written without precision and scale (`decimal(28, 0)` as the parser
    /// reads it), which shows the places it has.
    pub fn shown_scale(&self) -> Option<u32> {
        match self {
            FieldType::Decimal { precision: 28, scale: 0 } => None,
            FieldType::Decimal { scale, .. } => Some(*scale),
            _ => None,
        }
    }
}

/// What a list field of plain values holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum ListItem {
    Text,
    Integer,
    Decimal,
    Date,
    Enum { name: Ident },
}

impl ListItem {
    /// The item type as Clarity writes it: `date`, `Genre`.
    pub fn word(&self) -> &str {
        match self {
            ListItem::Text => "text",
            ListItem::Integer => "integer",
            ListItem::Decimal => "decimal",
            ListItem::Date => "date",
            ListItem::Enum { name } => &name.name,
        }
    }

    /// One item as a field of its own type: what a query compares it as.
    pub fn field_type(&self) -> FieldType {
        match self {
            ListItem::Text => FieldType::Text { max_length: None },
            ListItem::Integer => FieldType::Integer,
            ListItem::Decimal => FieldType::Decimal { precision: 0, scale: 0 },
            ListItem::Date => FieldType::Date,
            ListItem::Enum { name } => FieldType::Enum { name: name.clone() },
        }
    }

    /// The item type of `list<word>` for a plain value a list field holds.
    pub fn of_word(word: &str) -> Option<ListItem> {
        match word {
            "text" => Some(ListItem::Text),
            "integer" => Some(ListItem::Integer),
            "decimal" => Some(ListItem::Decimal),
            "date" => Some(ListItem::Date),
            _ => None,
        }
    }
}

impl FieldType {
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Integer | Self::Decimal { .. } | Self::Money | Self::Id | Self::Duration
        )
    }

    pub fn is_textual(&self) -> bool {
        matches!(self, Self::Text { .. } | Self::Email | Self::Url | Self::Color | Self::Folder | Self::Language | Self::Theme)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Modifiers {
    pub required: bool,
    pub unique: bool,
    pub owned: bool,
    pub computed: bool,
    /// `search`: the field is in its record's full-text index, which
    /// `where a matches x` looks in (text and lists of text).
    #[serde(default)]
    pub search: bool,
    /// `cascade`, on a reference: deleting the record it refers to deletes
    /// this one first, with its data events, in the same transaction.
    #[serde(default)]
    pub cascade: bool,
    pub min: Option<Literal>,
    pub max: Option<Literal>,
    pub pattern: Option<String>,
    pub default: Option<Literal>,
    pub label: Option<String>,
    pub help: Option<String>,
    pub format: Option<String>,
    /// `column "InvNo"`: the existing column a field of an external entity
    /// is stored in.
    pub column: Option<String>,
    /// `store photos`: an image field keeps its pictures as files in this
    /// file store of the project settings, not in the database.
    pub store: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
#[ts(export)]
pub enum Literal {
    /// Kept as text so decimal and money values never lose precision.
    Number(String),
    Str(String),
    Bool(bool),
    /// An enum variant name.
    Ident(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PageKind {
    Window,
    Dialog,
    View,
    /// The splash screen the app shows itself while it starts (`splash Welcome` in the app).
    Splash,
    /// A toolbox (docs/IMAGE-PLAN.md): designed as a view, its parts the tools a canvas that names it shows (`toolbox Painting`).
    Toolbox,
}

impl PageKind {
    /// The word a page line writes it with: `page Welcome: splash`.
    pub fn word(self) -> &'static str {
        match self {
            PageKind::Window => "window",
            PageKind::Dialog => "dialog",
            PageKind::View => "view",
            PageKind::Splash => "splash",
            PageKind::Toolbox => "toolbox",
        }
    }
}

/// A widget-first page: widgets placed on a snap grid (see the widget
/// catalog in `widgets`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Page {
    pub name: Ident,
    pub kind: PageKind,
    pub title: Option<String>,
    /// The size its desktop window opens at (`size 320 x 460`); none for
    /// the standard size (layout::WINDOW_WIDTH and WINDOW_HEIGHT).
    pub size: Option<WindowSize>,
    /// A style of the project's theme for the window itself (`style
    /// splashBack`): its background and the text of its parts.
    #[serde(default)]
    pub style: Option<String>,
    pub data: Vec<DataDecl>,
    /// Number of grid columns.
    pub columns: u32,
    /// The height of a row of the page's own exact grid (`grid 56 columns
    /// rows 10`), in pixels: cells without gaps or padding, rows that keep
    /// their height, for a designed page such as a splash. None for the
    /// project's rows.
    #[serde(default)]
    pub cell: Option<u32>,
    pub widgets: Vec<Widget>,
    /// Regions of a window that override the app's.
    pub frame: Frame,
    /// Handlers of the window's own events: open, close, return.
    pub handlers: Vec<Handler>,
    /// What a dialog gives when it closes with a value (`page X: dialog
    /// gives Friend`), for the handler that asked for it (DESIGN 5.14).
    pub gives: Option<crate::code::TypeRef>,
    /// What a view tells the window it shows in (`tells saved(name: text)`,
    /// DESIGN 5.33): its code says `tell saved(name)`, and the window hears
    /// it on the slide-out or section that shows it (`on saved: handler`).
    #[serde(default)]
    pub tells: Vec<crate::code::PartMessage>,
    pub span: Span,
}

/// A window's size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

pub const DEFAULT_PAGE_COLUMNS: u32 = 12;
pub const MAX_PAGE_COLUMNS: u32 = 96;

/// The space a canvas draws in, in its own units: `units 200 x 100`. The
/// renderer scales it to fit the part, keeping its proportion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CanvasUnits {
    pub width: u32,
    pub height: u32,
}

/// What a window holds: `data customer: Customer`, `data orders:
/// list<Order>`, `data shown: decimal`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DataDecl {
    pub name: Ident,
    pub of: DataKind,
    /// Made by Clarity, not written: every window's `settings`.
    #[serde(default)]
    pub made: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum DataKind {
    /// One record, new when the window opens: `data customer: Customer`.
    Record { entity: Ident },
    /// Records, every one when the window opens: `data orders: list<Order>`.
    List { entity: Ident },
    /// A plain value, at its default or empty when the window opens: `data
    /// shown: decimal`, `data pending: Operation`, `data fresh: bool,
    /// default true`. Its type is written as in code.
    Value { ty: crate::code::TypeRef, default: Option<Literal> },
}

impl DataDecl {
    /// The entity of a record or of a list; None for a plain value.
    pub fn entity(&self) -> Option<&Ident> {
        match &self.of {
            DataKind::Record { entity } | DataKind::List { entity } => Some(entity),
            DataKind::Value { .. } => None,
        }
    }

    pub fn is_list(&self) -> bool {
        matches!(self.of, DataKind::List { .. })
    }

    /// The type of a plain value; None for a record or a list.
    pub fn value(&self) -> Option<&crate::code::TypeRef> {
        match &self.of {
            DataKind::Value { ty, .. } => Some(ty),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Widget {
    pub props: WidgetProps,
    /// Children of a plain container, on the container's own grid.
    pub children: Vec<Widget>,
    /// Tabs of a `tabs` widget or panes of a `splitter`.
    pub pages: Vec<WidgetPage>,
    /// Its context menu: a `menu` block under the part (DESIGN 5.16).
    pub menu: Option<Menu>,
    /// A tree of several kinds of records: its levels, each the records of an
    /// entity under the row of the level around it, or a group heading.
    #[serde(default)]
    pub levels: Vec<TreeLevel>,
    /// What a tree of levels runs when a row of a kind is picked: `on select Project: projectPicked`.
    #[serde(default)]
    pub selects: Vec<TreeSelect>,
    pub span: Span,
}

/// A level of a tree of several kinds of records (DESIGN 5.33): `level
/// Domain under project show name order by name`, the records of Domain
/// whose `project` is the row of the level around it; or `group
/// @dataObjectsNode`, a heading over the levels inside it, shown when they
/// have rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct TreeLevel {
    /// The entity whose records are its rows; none for a group.
    pub entity: Option<String>,
    /// The field of those records that refers to the row of the level around it; none at the top.
    pub under: Option<String>,
    /// What a row shows: a field of the record, or a text key (`@classNode`) whose slots are fields.
    pub show: Option<String>,
    /// A group's heading, a text key.
    pub group: Option<String>,
    /// The field its rows are ordered by, and whether from high to low.
    pub order: Option<String>,
    pub descending: bool,
    pub levels: Vec<TreeLevel>,
    pub span: Span,
}

/// `on select ClassDefinition: classPicked`: the handler a tree of levels
/// runs with the record when a row of that entity is picked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct TreeSelect {
    pub entity: String,
    pub handler: String,
    pub span: Span,
}

/// A `tab "Title"` or `pane` block inside a container widget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct WidgetPage {
    pub title: Option<String>,
    /// `pane span 4`: the columns a splitter's first pane starts with, so
    /// its grid has them; the second pane has the rest. Without it each pane
    /// has the splitter's columns and they start alike.
    pub columns: Option<u32>,
    /// `section "Class" view ClassEditor`: a tab, step or section showing a
    /// view (DESIGN 5.33), whose parts are then its own.
    pub view: Option<String>,
    /// The tells of the view it shows wired to the window's handlers: `on saved: recipeSaved`.
    #[serde(default)]
    pub tells: Vec<TellHandler>,
    /// `section "Chapters" color "#2e86de"`: an accordion section's header in
    /// a color of its own (written out, or a color of the theme by name); none
    /// keeps the theme's look.
    #[serde(default)]
    pub color: Option<String>,
    pub children: Vec<Widget>,
    pub span: Span,
}

/// What the person does on a canvas (DESIGN 5.31): drags the records of a
/// list, paints a picture, or draws and edits shapes. A canvas without one is
/// drawn by its draw handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CanvasMode {
    Objects,
    Paint,
    Vector,
}

impl CanvasMode {
    pub fn keyword(self) -> &'static str {
        match self {
            CanvasMode::Objects => "objects",
            CanvasMode::Paint => "paint",
            CanvasMode::Vector => "vector",
        }
    }

    /// The tools of the mode, in the order its toolbar shows them.
    pub fn tools(self) -> &'static [&'static str] {
        match self {
            // A diagram's: drag its objects, or draw a new line from one to another (`on connect`).
            CanvasMode::Objects => &["select", "connect"],
            CanvasMode::Paint => &["pen", "pencil", "spray", "eraser", "fill", "picker", "select", "ellipseSelect", "lasso", "line", "rect", "ellipse"],
            CanvasMode::Vector => &["select", "line", "rect", "ellipse", "path", "text"],
        }
    }
}

/// What a toolbox's tool may do built in (`does pencil`): the canvas tools of
/// paint and vector, which it takes in hand, and actions done at once.
pub const TOOL_ACTIONS: [&str; 24] = [
    "pen", "pencil", "spray", "eraser", "fill", "picker", "select", "ellipseSelect", "lasso", "line", "rect", "ellipse", "path", "text", "connect",
    "rotate", "flipAcross", "flipDown", "crop", "import", "export", "zoomIn", "zoomOut", "adjust",
];

/// How a painting canvas's toolbox shows (docs/IMAGE-PLAN.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ToolboxKind {
    /// Floats over the app, dragged anywhere and folded, as GIMP's.
    Popup,
    /// A slide-out against the canvas's edge, with its handle and pin.
    Slideout,
    /// A memo's: called up at the caret (Ctrl+Space, or a right-click
    /// there), its action done on the text where the person is, then gone.
    Caret,
}

impl ToolboxKind {
    pub fn keyword(self) -> &'static str {
        match self {
            ToolboxKind::Popup => "popup",
            ToolboxKind::Slideout => "slideout",
            ToolboxKind::Caret => "caret",
        }
    }
}

/// The objects of a canvas in `mode objects`: the records of `list`, placed
/// by their fields `x` and `y` (`mode objects types at x, y`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CanvasObjects {
    pub list: String,
    pub x: String,
    pub y: String,
}

/// The order a table's rows show in first: `order by title`, `order by year desc`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TableOrder {
    pub field: String,
    pub desc: bool,
}

/// Everything on a widget's own line. The designer sends these back in
/// edits, and the edit engine writes them as one canonical line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WidgetProps {
    pub kind: WidgetKind,
    pub caption: Option<String>,
    /// The part's name, unique in its window; the code behind refers to the
    /// part by it.
    pub name: Option<String>,
    /// Grid column and row of the top-left cell, 1-based.
    pub col: u32,
    pub row: u32,
    /// Columns and rows covered.
    pub width: u32,
    pub height: u32,
    /// `data.field` or `data`.
    pub bind: Option<String>,
    /// Window opened on click.
    pub on_click: Option<String>,
    /// The window opens in a desktop window of its own (`in new window`);
    /// otherwise it opens in place, on top of this one.
    #[serde(default)]
    pub new_window: bool,
    /// The data the opened window is filled with, each from the data of the
    /// same name of the window the click is in: `on click -> ChapterForm with book`.
    #[serde(default)]
    pub open_with: Vec<String>,
    /// Handlers in the code behind, per event.
    pub handlers: Vec<Handler>,
    pub columns: Vec<String>,
    /// The columns of a table whose cells are inputs: `edit done`.
    #[serde(default)]
    pub edit: Vec<String>,
    /// The columns of a table whose cells are links: `link chapter: toChapter`.
    #[serde(default)]
    pub links: Vec<ColumnLink>,
    /// Rows of other parts it takes when dragged onto it: `on drop from fragmentList: linkFragment`.
    #[serde(default)]
    pub drops: Vec<DropFrom>,
    /// The edited bool columns shown as a switch instead of a checkbox:
    /// `edit done as switch`.
    #[serde(default)]
    pub switches: Vec<String>,
    /// The width in pixels of a table's columns that have one:
    /// `columns title 240, artist`. A column without one shares what is left.
    #[serde(default)]
    pub widths: std::collections::BTreeMap<String, u32>,
    /// `resizable`: the person drags a table's header edges, and the widths
    /// are remembered; or a slide-out's inner edge, between `min` and `max`.
    #[serde(default)]
    pub resizable: bool,
    /// `side left`, `side top`, `side bottom`: the edge a slide-out comes in from; the right without it.
    #[serde(default)]
    pub side: Option<SlideSide>,
    /// `beside chapterTable`: a slide-out lies against that part, at its `side`
    /// of it, instead of at the window's edge; the part stands in the same container.
    #[serde(default)]
    pub beside: Option<String>,
    /// `width 420`: how wide a slide-out at the left or right is, in pixels; `SLIDEOUT_WIDTH` without it.
    #[serde(default)]
    pub slide_width: Option<u32>,
    /// `height 300`: how high a slide-out at the top or bottom is, in pixels; `SLIDEOUT_HEIGHT` without it.
    #[serde(default)]
    pub slide_height: Option<u32>,
    /// `pinnable`: a slide-out has a pin in its header; pinned, it stays
    /// open beside the window's parts, which make room for it.
    #[serde(default)]
    pub pinnable: bool,
    /// `code csharp`: a memo of code, in the mono font, its words colored
    /// by the language (one of `widgets::CODE_LANGUAGES`).
    #[serde(default)]
    pub code: Option<String>,
    /// `copy`: a memo or text box has a button in its corner that puts its
    /// text on the clipboard, read only too.
    #[serde(default)]
    pub copy: bool,
    /// `markdown`: a memo of Markdown, an editor with a preview: Write and
    /// Preview on its top, the text colored as Markdown while it is written,
    /// Ctrl+B and Ctrl+I making the selection bold and italic.
    #[serde(default)]
    pub markdown: bool,
    /// `view ClassEditor`: a slide-out shows that view, its parts, data and
    /// handlers joining the window's (`crate::views`); it holds none of its own.
    #[serde(default)]
    pub view: Option<String>,
    /// `sortable`: a click on a table's header sorts its rows by that column.
    #[serde(default)]
    pub sortable: bool,
    /// `add`: a table has an Add button above its header, at the right,
    /// which runs `on add`.
    #[serde(default)]
    pub add: bool,
    /// `remove`: every row of a table has a trash button in a last column,
    /// which picks the row and runs `on remove`.
    #[serde(default)]
    pub remove: bool,
    /// `remove confirm`: the trash button asks first, in Clarity's dialog, and runs `on remove` on yes.
    #[serde(default)]
    pub confirm: bool,
    /// `striped`: a table, list, tree or tree table draws every other row
    /// with a faint stripe.
    #[serde(default)]
    pub striped: bool,
    /// `order by field [desc]`: the order a table's rows show in first.
    #[serde(default)]
    pub order: Option<TableOrder>,
    /// `reorder by position`: the person puts the rows of a list, table,
    /// tree or tree table in their own order, numbered in this integer field
    /// (DESIGN 5.30). Without it the rows stay where the data has them.
    #[serde(default)]
    pub reorder: Option<String>,
    /// `reorder by chapter from 0`: the number the first row at each level gets; 1 without it.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub reorder_from: Option<i64>,
    /// `mode objects types at x, y`, `mode paint`, `mode vector`: what the
    /// person does on a canvas (DESIGN 5.31); none: its draw handler draws it.
    #[serde(default)]
    pub mode: Option<CanvasMode>,
    /// The list and fields of a canvas in `mode objects`.
    #[serde(default)]
    pub objects: Option<CanvasObjects>,
    /// `tools pen, eraser`: the tools of the canvas's mode the person may use; all without it.
    #[serde(default)]
    pub tools: Vec<String>,
    /// `toolbox popup` or `toolbox slideout`: a painting canvas's tools in a
    /// toolbox (docs/IMAGE-PLAN.md) instead of the strip on the canvas.
    #[serde(default)]
    pub toolbox: Option<ToolboxKind>,
    /// `toolbox Painting`: the toolbox page whose parts the toolbox shows; none: the built-in one of the canvas's mode.
    #[serde(default)]
    pub toolbox_page: Option<String>,
    /// The columns of the toolbox page's grid, which its parts are placed on (set where it is placed).
    #[serde(default)]
    pub toolbox_columns: Option<u32>,
    /// `does pencil`: the built-in action of a toolbox's tool (TOOL_ACTIONS).
    #[serde(default)]
    pub does: Option<String>,
    /// `brush 4`: how wide a canvas paints or draws, in pixels.
    #[serde(default)]
    pub brush: Option<u32>,
    /// `ink accent`: the color a canvas paints or draws with.
    #[serde(default)]
    pub ink: Option<String>,
    /// `page 50`: a table shows its rows that many at a time (U1c).
    #[serde(default)]
    pub page: Option<u32>,
    /// `chart bar`: the kind of a chart (U1c).
    #[serde(default)]
    pub chart: Option<ChartKind>,
    /// `by status`: the field a chart groups its rows by.
    #[serde(default)]
    pub by: Option<String>,
    /// `count`, `sum total`, `average total`: each group's number.
    #[serde(default)]
    pub measure: Option<ChartMeasure>,
    /// A graph's edges (docs/GRAPH-PLAN.md): the list of records that join
    /// its nodes, the records of the list it binds.
    #[serde(default)]
    pub edges: Option<String>,
    /// The field of a graph's node its edges name it by; `id` without it.
    #[serde(default)]
    pub node_key: Option<String>,
    /// The field a graph's node shows as its label; its key without it.
    #[serde(default)]
    pub node_label: Option<String>,
    /// The field that gives a graph's node its kind, which its look is chosen by.
    #[serde(default)]
    pub node_kind: Option<String>,
    /// The field of an edge that holds the key of the node it starts at; `source` without it.
    #[serde(default)]
    pub edge_from: Option<String>,
    /// The field of an edge that holds the key of the node it ends at; `target` without it.
    #[serde(default)]
    pub edge_to: Option<String>,
    /// The field an edge shows as its label.
    #[serde(default)]
    pub edge_label: Option<String>,
    /// The field that gives an edge its kind, which its look is chosen by.
    #[serde(default)]
    pub edge_kind: Option<String>,
    /// How a graph places its nodes; force without it.
    #[serde(default)]
    pub layout: Option<GraphLayout>,
    /// How a graph draws the nodes of each kind, by the kind's value.
    #[serde(default)]
    pub node_looks: std::collections::BTreeMap<String, NodeLook>,
    /// How a graph draws the edges of each kind, by the kind's value.
    #[serde(default)]
    pub edge_looks: std::collections::BTreeMap<String, EdgeLook>,
    /// The most nodes a graph shows; `widgets::GRAPH_LIMIT` without it.
    #[serde(default)]
    pub limit: Option<u32>,
    /// A graph of the data model (step 2): the lists of records it shows,
    /// of any entities, joined by their references to each other.
    #[serde(default)]
    pub lists: Vec<String>,
    /// The text data a graph of the data model writes the picked node's entity into, and reads it from.
    #[serde(default)]
    pub picked_kind: Option<String>,
    /// The whole-number data a graph of the data model writes the picked node's id into, and reads it from.
    #[serde(default)]
    pub picked_id: Option<String>,
    /// A graph shows only the nodes this many edges from the picked one, with a control to change it.
    #[serde(default)]
    pub hops: Option<u32>,
    /// A graph shows its kinds as chips that hide and show them.
    #[serde(default)]
    pub legend: bool,
    /// What a graph of the data model knows of the model, worked out when
    /// the project loads (`make_graph_models`); it is never written back.
    #[serde(default)]
    pub graph_model: Option<GraphModel>,
    /// `children children`: the list field of a tree's rows that lists
    /// records of their own kind, arranged by its reference (U1c).
    #[serde(default)]
    pub children: Option<String>,
    /// `key id`: the field that names a tree's row, which the parent key
    /// of the rows under it holds; with `parent`, instead of `children`.
    #[serde(default)]
    pub row_key: Option<String>,
    /// `parent parentId`: the field of a tree's row that holds the key of
    /// the row it shows under.
    #[serde(default)]
    pub parent_key: Option<String>,
    pub show: Vec<String>,
    /// `cards`: a list's rows as small cards, the first field of `show` the
    /// card's title and the others lines below it.
    #[serde(default)]
    pub cards: bool,
    /// `frame`: a list, table or tree shows its own border inside a
    /// container (a tab page, a pane, a panel), which otherwise frames it alone.
    #[serde(default)]
    pub frame: bool,
    /// `typed`: a dropdown of a text field any text may be typed into, the
    /// values the field already has offered as it is typed (U1c).
    #[serde(default)]
    pub typed: bool,
    /// `search`: a dropdown of a reference whose records filter as it is
    /// typed, each showing the fields of `show` (U1c).
    #[serde(default)]
    pub search: bool,
    pub orientation: Option<Orientation>,
    pub surface: Option<Surface>,
    pub source: Option<String>,
    /// The input a label or help text belongs to, by part name.
    #[serde(rename = "for")]
    #[ts(rename = "for")]
    pub for_part: Option<String>,
    /// Text shown in an empty input.
    pub placeholder: Option<String>,
    /// The digits a display shows: `digits 12`.
    pub digits: Option<u32>,
    /// The units a canvas draws in: `units 200 x 100`.
    pub units: Option<CanvasUnits>,
    /// `zoom`: the person zooms and scrolls the canvas's drawing.
    #[serde(default)]
    pub zoom: bool,
    /// `grid 20`: a canvas shows a grid, every so many of its units.
    #[serde(default)]
    pub grid: Option<u32>,
    /// `snap`: the pointer on a canvas snaps to its grid.
    #[serde(default)]
    pub snap: bool,
    /// `background panel`: what a canvas's `clear()` fills it with, a theme
    /// color by name or written out; the theme's panel without it.
    #[serde(default)]
    pub background: Option<String>,
    /// `dots 3 muted`: how many pixels across a canvas's grid dots are, and
    /// their color; 2 and the theme's border without it.
    #[serde(default)]
    pub dot_size: Option<u32>,
    #[serde(default)]
    pub dot_color: Option<String>,
    /// `runs Screen`: the engine beside the screen that draws the canvas
    /// (DESIGN 5.12), in place of a draw handler.
    pub runs: Option<String>,
    /// Text shown when the pointer rests on the part.
    pub tooltip: Option<String>,
    /// The icon a button shows beside its caption, or the icon an `icon` part is: `icon "save"`.
    pub icon: Option<String>,
    /// The address a link opens: `to "https://lucide.dev"`.
    pub to: Option<String>,
    /// The number a progress bar is full at, as written: `max 250`; 100 without it.
    /// For a slider or a spin box, the highest value (see `widgets::number_range`).
    pub max: Option<String>,
    /// The lowest value of a slider or a spin box, as written: `min 0`.
    pub min: Option<String>,
    /// How far a slider or a spin box moves at a time, as written: `step 0.5`; 1 without it.
    pub step: Option<String>,
    /// A split button: its `menu` block opens from the arrow beside it.
    pub split: bool,
    /// A form's label column: `labels 3` gives each input in it a label in its first 3 columns.
    pub labels: Option<u32>,
    /// A sound's volume, 0 to 100: `volume 80`; 100 without it (DESIGN 5.22).
    #[serde(default)]
    pub volume: Option<u32>,
    /// `repeat`: a sound plays its source over and over until it is stopped.
    #[serde(default)]
    pub repeat: bool,
    /// How many values a trend keeps, or a chart of numbers shows: `points 60`
    /// (DESIGN 5.26); a trend keeps 60 without it, a chart shows all.
    #[serde(default)]
    pub points: Option<u32>,
    /// Seconds between a timer's ticks, or a trend's values, as written:
    /// `every 1`, `every 0.5`; a trend takes one a second without it.
    #[serde(default)]
    pub every: Option<String>,
    /// Made by Clarity, not written: a label of a form's label column. The
    /// designer leaves it alone and it is never written back.
    pub made: bool,
    /// A named style of the project's theme: `style primary`.
    pub style: Option<String>,
    /// An input that shows its value but cannot be changed.
    pub readonly: bool,
    /// A part the person edits without an undo of its own (DESIGN 5.29):
    /// `no undo`. Every other one takes back its own edits with Ctrl+Z.
    #[serde(default)]
    pub no_undo: bool,
    /// Where its edits are undone, worked out when the project loads
    /// (`widgets::undo_scope`): its form or panel, or itself; none without
    /// undo. Every screen uses it as it is; it is never written back.
    #[serde(default)]
    pub undo_scope: Option<String>,
    /// A panel or form the person using the application folds and unfolds
    /// by its header: `foldable`.
    #[serde(default)]
    pub foldable: bool,
    /// A foldable part that starts folded: `foldable folded`.
    #[serde(default)]
    pub folded: bool,
    /// Position in the keyboard order; parts without one follow in reading
    /// order.
    pub tab_order: Option<u32>,
    /// The key that clicks a button, as in `Ctrl+S`.
    pub shortcut: Option<String>,
    /// The button Enter clicks.
    #[serde(rename = "default")]
    #[ts(rename = "default")]
    pub default_button: bool,
    /// The button Escape clicks.
    pub cancel: bool,
    /// Sticks to the bottom of its grid: when no row grows, its rows and
    /// those below move down by the height to spare (`bottom`).
    #[serde(default)]
    pub bottom: bool,
    /// A custom part (DESIGN 5.19): the name it is declared by, as in `Arena`.
    #[serde(default)]
    pub part: Option<String>,
    /// A custom part's properties set in the window: `speed 5`.
    #[serde(default)]
    pub values: std::collections::BTreeMap<String, Literal>,
    /// A custom part's tells wired to handlers: `on scored: arenaScored`.
    #[serde(default)]
    pub tells: Vec<TellHandler>,
    /// A custom part whose declaration says `grows`: it takes the height
    /// its window has left. Worked out when the project loads, for every
    /// screen and the designer; it is never written back.
    #[serde(default)]
    pub grows: bool,
    /// A custom part's description for screen readers, from its
    /// declaration's `description`. Worked out when the project loads, as
    /// `grows` is; it is never written back.
    #[serde(default)]
    pub description: Option<String>,
}

/// `link chapter: toChapter`: a table column whose cells are links; a click
/// on one chooses the row and runs the handler, which finds it as the table's `selected`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ColumnLink {
    pub column: String,
    pub handler: String,
}

/// `on drop from fragmentList: linkFragment`: a row of that list, table or tree
/// dragged onto this one runs the handler, with the dragged row chosen in its
/// part and the row it landed on chosen in this one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DropFrom {
    pub from: String,
    pub handler: String,
}

/// `on scored: arenaScored`: the handler a custom part's tell runs, given its values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TellHandler {
    pub tell: String,
    pub handler: String,
}

/// `on change: name_changed`: the handler in the code behind that runs
/// when the event happens, with the values it is given: `on click: digit(7)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Handler {
    pub event: Event,
    pub handler: String,
    /// The values the handler is run with, in the order of its parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Literal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Surface {
    Svg,
    Bitmap,
    Gpu,
}

// ---------------------------------------------------------------------------
// Application frame

/// The application: its main window and the frame regions every window
/// shares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct App {
    pub name: Ident,
    pub title: Option<String>,
    /// The window shown at start.
    pub main: Option<Ident>,
    /// The splash page shown while the application starts, before its main
    /// window (`splash Welcome for 2 seconds`); none for no splash.
    #[serde(default)]
    pub splash: Option<Ident>,
    /// How long the splash shows at least, in milliseconds; none to close it as soon as the main window shows.
    #[serde(default, rename = "splashFor")]
    #[ts(rename = "splashFor")]
    pub splash_for: Option<u32>,
    /// Data for widgets in the sidebars and the footer.
    pub data: Vec<DataDecl>,
    pub frame: Frame,
    /// Handlers of the app's events: start.
    pub handlers: Vec<Handler>,
    pub span: Span,
}

/// Regions docked around a window's grid. A window's own region replaces
/// the app's region of the same kind (and, for sidebars, the same side).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Frame {
    pub menu: Option<Menu>,
    pub toolbar: Option<Toolbar>,
    pub ribbon: Option<Ribbon>,
    pub sidebars: Vec<Sidebar>,
    pub footer: Option<Footer>,
}

impl Frame {
    pub fn is_empty(&self) -> bool {
        self.menu.is_none()
            && self.toolbar.is_none()
            && self.ribbon.is_none()
            && self.sidebars.is_empty()
            && self.footer.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Menu {
    /// Its part name, as code hides or disables it: `mainMenu`.
    pub name: Option<String>,
    pub items: Vec<Command>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Toolbar {
    /// Its part name, as code hides or disables it: `mainToolbar`.
    pub name: Option<String>,
    pub items: Vec<Command>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Ribbon {
    /// The name code goes by; every part has one.
    pub name: Option<String>,
    /// The person folds it to its tabs, and a tab opens it again.
    pub collapsible: bool,
    /// A collapsible ribbon that starts folded.
    pub collapsed: bool,
    pub tabs: Vec<RibbonTab>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RibbonTab {
    pub title: String,
    pub name: Option<String>,
    pub groups: Vec<RibbonGroup>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RibbonGroup {
    pub title: String,
    pub name: Option<String>,
    pub items: Vec<Command>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Side {
    Left,
    Right,
}

/// The edge of the window's content a slide-out comes in from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SlideSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl SlideSide {
    /// The word its line says: `side top`.
    pub fn keyword(self) -> &'static str {
        match self {
            SlideSide::Left => "left",
            SlideSide::Right => "right",
            SlideSide::Top => "top",
            SlideSide::Bottom => "bottom",
        }
    }

    /// Whether it lies across the window, at the top or the bottom: it has a height, not a width.
    pub fn across(self) -> bool {
        matches!(self, SlideSide::Top | SlideSide::Bottom)
    }
}

/// A sidebar docked to the left or right edge of a window, with its own
/// grid of parts: `sidebar left name nav width 280 min 200 max 480
/// resizable collapsible collapsed maximizable grid 4 columns`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Sidebar {
    pub props: SidebarProps,
    pub widgets: Vec<Widget>,
    pub span: Span,
}

/// What a sidebar's line says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SidebarProps {
    pub side: Side,
    /// The name code and events go by; every part has one.
    pub name: Option<String>,
    /// Its width in pixels.
    pub width: u32,
    /// How narrow and how wide the person may drag a resizable sidebar.
    pub min: Option<u32>,
    pub max: Option<u32>,
    /// The person drags its inner edge to size it.
    pub resizable: bool,
    /// It folds to a button at its edge, and back.
    pub collapsible: bool,
    /// A collapsible sidebar that starts folded.
    pub collapsed: bool,
    /// It has a button that lets it fill the window, and give it back.
    pub maximizable: bool,
    /// Columns of its own grid.
    pub columns: u32,
}

/// A slide-out's width when its line says none, in pixels; it takes the widths a sidebar takes.
pub const SLIDEOUT_WIDTH: u32 = 420;

/// A slide-out's height at the top or bottom when its line says none, in pixels; it takes the same sizes.
pub const SLIDEOUT_HEIGHT: u32 = 300;

/// A sidebar's width and grid when its line says none.
pub const DEFAULT_SIDEBAR_WIDTH: u32 = 240;
pub const DEFAULT_SIDEBAR_COLUMNS: u32 = 3;
/// The widths a sidebar may have, in pixels.
pub const SIDEBAR_WIDTHS: (u32, u32) = (100, 1600);

/// A one-row grid at the bottom of the window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Footer {
    /// The status text, shown at the left.
    pub status: Option<String>,
    pub status_span: Option<Span>,
    /// Columns of the footer grid, always `FOOTER_COLUMNS`.
    pub columns: u32,
    pub widgets: Vec<Widget>,
    pub span: Span,
}

/// Columns of the footer grid.
pub const FOOTER_COLUMNS: u32 = 12;

/// A menu item, a toolbar or ribbon button, or a separator. An item with
/// items of its own is a submenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Command {
    pub props: CommandProps,
    pub items: Vec<Command>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CommandKind {
    Item,
    Button,
    Separator,
}

/// Everything on a command's own line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CommandProps {
    pub kind: CommandKind,
    pub caption: Option<String>,
    /// The part's name, unique in its window.
    pub name: Option<String>,
    pub icon: Option<String>,
    /// Keyboard shortcut, menu items only: `key "Ctrl+N"`.
    pub shortcut: Option<String>,
    /// A large ribbon button.
    pub large: bool,
    /// Window opened on click.
    pub on_click: Option<String>,
    /// The window opens in a desktop window of its own (`in new window`);
    /// otherwise it opens in place, on top of this one.
    #[serde(default)]
    pub new_window: bool,
    /// The data the opened window is filled with, each from the data of the
    /// same name of the window the click is in: `on click -> ChapterForm with book`.
    #[serde(default)]
    pub open_with: Vec<String>,
    /// Handler in the code behind that runs on click.
    pub handler: Option<String>,
}

/// All spec files of a project plus every diagnostic from lexing, parsing
/// and checking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Project {
    pub files: Vec<SpecFile>,
    pub diagnostics: Vec<Diagnostic>,
    /// The steps in the project's `migrations` folder, in order, per data
    /// source. None when the project was loaded without its folder
    /// (sources given in memory).
    #[serde(skip)]
    #[ts(skip)]
    pub migrations: Option<std::collections::BTreeMap<String, Vec<crate::migrate::Migration>>>,
    /// The project's data sources, the first being the default. Empty when
    /// the project was loaded without them.
    #[serde(skip)]
    #[ts(skip)]
    pub sources: Vec<crate::SourceRef>,
    /// The project's language and currency, for showing values (section
    /// 1.5 of the design). Default when the project was loaded without them.
    pub locale: Locale,
    /// The application's icon, a picture among its resources, as the project
    /// file names it (`icon "app.png"`); none for Clarity's own
    /// (docs/APP-ICON-SPLASH-PLAN.md).
    #[serde(default)]
    pub icon: Option<String>,
    /// The application's version as the project file says it (`version
    /// "1.2.0"`): `app.version` in code, `{app.version}` in texts; none
    /// without one (docs/VERSION-PLAN.md).
    #[serde(default)]
    pub version: Option<String>,
    /// The project's translations per language: each text a person sees,
    /// as written, with its translation (design section 1.5, I1c).
    pub translations: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    /// The keys whose value in a language is a file of the resources folder
    /// (`gameRules resource "rules-nl.md"`), per language: the file's name.
    pub resource_keys: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    /// Whether the people using the application set its keys themselves,
    /// as the project file says (`keys editable`, DESIGN 5.28).
    #[serde(rename = "keysEditable")]
    #[ts(rename = "keysEditable")]
    pub keys_editable: bool,
    /// How high a row of the windows' grid is, in pixels, as the project
    /// file says (`row height 30`); `layout::ROW_HEIGHT` without the line.
    #[serde(rename = "rowHeight")]
    #[ts(rename = "rowHeight")]
    pub row_height: u32,
    /// The themes the application offers, the first being the one it starts
    /// with; empty when the project file names none (design section 1.5, T1).
    pub themes: Vec<String>,
    /// The files of the project's `resources` folder. None when the project
    /// was loaded without its folder.
    pub resources: Option<crate::resources::Resources>,
    /// The written diagrams of the project's `diagrams` folder (DESIGN
    /// 5.13), by name; empty when there are none or the folder was not read.
    pub diagrams: Vec<crate::diagrams::DiagramFile>,
    /// The project's README.md (its design goal and guide for whoever works
    /// on it next), from its folder; None when it has none or the folder was
    /// not read.
    pub readme: Option<String>,
    /// The files of the project's `engines` folder, by name. None when the
    /// project was loaded without its folder.
    #[serde(skip)]
    #[ts(skip)]
    pub engines: Option<Vec<crate::engines::EngineFile>>,
    /// The files of the folders of the project's native custom parts
    /// (DESIGN 5.19). None when the project was loaded without its folder.
    #[serde(skip)]
    #[ts(skip)]
    pub part_files: Option<Vec<crate::native::PartFile>>,
    /// The file stores of the project settings, by name. None when the
    /// project was loaded without its settings.
    #[serde(skip)]
    #[ts(skip)]
    pub stores: Option<Vec<String>>,
}

impl Project {
    /// The data source modules use when they name none.
    pub fn default_source(&self) -> &str {
        self.sources.first().map_or(crate::DEFAULT_SOURCE, |s| s.name.as_str())
    }

    /// The data source an entity lives in: its module's, or the default.
    pub fn source_of(&self, entity: &str) -> Option<&str> {
        let file = self.files.iter().find(|f| f.entities.iter().any(|e| e.name.name == entity))?;
        Some(file.source.as_ref().map_or(self.default_source(), |s| s.name.as_str()))
    }

    /// The entities that live in data source `source`.
    pub fn entities_in<'a>(&'a self, source: &'a str) -> impl Iterator<Item = &'a Entity> + 'a {
        // The settings are kept per person, never in a data source.
        self.files
            .iter()
            .filter(move |f| f.source.as_ref().map_or(self.default_source(), |s| s.name.as_str()) == source)
            .flat_map(|f| &f.entities)
            .filter(|e| !e.settings)
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == crate::diagnostic::Severity::Error)
    }

    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.files.iter().flat_map(|f| f.entities.iter())
    }

    /// The entities whose records a database keeps: all but the settings.
    pub fn stored_entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities().filter(|e| !e.settings && !self.in_web_api(&e.name.name))
    }

    /// Whether `entity` lives in a web API data source (DESIGN 5.23): a
    /// service keeps its records, so no database stores them.
    /// The settings kept in the system's protected store (DESIGN 5.27), by name.
    pub fn secret_settings(&self) -> Vec<&str> {
        self.entities().filter(|e| e.settings).flat_map(|e| &e.fields).filter(|f| f.ty == FieldType::Secret).map(|f| f.name.name.as_str()).collect()
    }

    /// Whether any window has a part of `kind`, inside containers too.
    pub fn uses_part(&self, kind: crate::widgets::WidgetKind) -> bool {
        self.files.iter().flat_map(|f| &f.pages).any(|p| crate::translations::any_widget(&p.widgets, &|w| w.props.kind == kind))
    }

    /// The fields that cascade from `entity`: deleting one of its records
    /// deletes the records whose field refers to it first (`cascade`).
    pub fn cascades(&self, entity: &str) -> Vec<(&Entity, &Field)> {
        self.entities()
            .flat_map(|e| e.fields.iter().map(move |f| (e, f)))
            .filter(|(_, f)| f.modifiers.cascade && matches!(&f.ty, FieldType::Ref { entity: target } if target.name == entity))
            .collect()
    }

    /// The settings that name a folder, where a file store may keep its files, by name.
    pub fn folder_settings(&self) -> Vec<&str> {
        self.entities().filter(|e| e.settings).flat_map(|e| &e.fields).filter(|f| f.ty == FieldType::Folder).map(|f| f.name.name.as_str()).collect()
    }

    /// The secret settings a database connects with (DESIGN 5.27): its
    /// connection is `setting:name` in the project file.
    pub fn secret_connections(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.sources.iter().filter(|s| s.sql).filter_map(|s| s.address.strip_prefix("setting:")).collect();
        names.dedup();
        names
    }

    /// Whether an app of the project has an `on unconnected` handler (DESIGN 5.27).
    pub fn handles_unconnected(&self) -> bool {
        self.files.iter().flat_map(|f| &f.apps).any(|a| a.handlers.iter().any(|h| h.event == crate::widgets::Event::Unconnected))
    }

    /// The window where the person keeps a database's secret connection: the
    /// first that shows one. An application whose database cannot connect
    /// opens it first, and connects once it closes (DESIGN 5.27).
    pub fn connection_window(&self) -> Option<&Page> {
        let names = self.secret_connections();
        self.files.iter().flat_map(|f| &f.pages).find(|page| names.iter().any(|name| shows_setting(&page.widgets, name)))
    }

    pub fn in_web_api(&self, entity: &str) -> bool {
        self.source_of(entity).is_some_and(|source| self.sources.iter().any(|s| s.name == source && !s.sql))
    }

    /// Where the project reads its version: the first `app.version` in its
    /// code, or the first text that shows `{app.version}`, as its file and
    /// span (docs/VERSION-PLAN.md); none when it reads it nowhere.
    pub fn app_version_use(&self) -> Option<(String, crate::span::Span)> {
        let mut found: Option<(String, crate::span::Span)> = None;
        for file in &self.files {
            let mut look = |body: &[crate::code::Stmt]| {
                crate::code::each_expr_in(body, &mut |e| {
                    if found.is_none() && crate::code::reads_app_version(e) {
                        found = Some((file.path.clone(), e.span));
                    }
                });
            };
            for block in &file.code {
                block.handlers.iter().for_each(|h| look(&h.body));
                block.events.iter().for_each(|e| look(&e.body));
            }
            file.functions.iter().for_each(|f| look(&f.body));
        }
        found.or_else(|| {
            let shows = self.translations.values().flat_map(|v| v.values()).any(|text| crate::translations::slots(text).iter().any(|s| s == "app.version"));
            shows.then(|| (String::new(), crate::span::Span::default()))
        })
    }

    /// Whether any code of the project calls a web API (DESIGN 5.23): a
    /// stack then brings its HTTP client along.
    pub fn calls_out(&self) -> bool {
        self.any_expr(&|e| matches!(e.kind, crate::code::ExprKind::Http { .. }))
    }

    /// Whether the code reads or writes a format (docs/FORMATS-PLAN.md), so its program carries Clarity.Formats.
    pub fn uses_formats(&self) -> bool {
        self.any_expr(&|e| matches!(e.kind, crate::code::ExprKind::ReadFormat { .. } | crate::code::ExprKind::FormatOf { .. }))
    }

    /// Whether the code calls a function `wanted` says yes to, such as a picture function.
    pub fn calls_any(&self, wanted: &dyn Fn(&str) -> bool) -> bool {
        self.any_expr(&|e| matches!(&e.kind, crate::code::ExprKind::Call { function, .. } if wanted(&function.name)))
    }

    /// Whether an expression of the handlers, events, functions or commands is one `wanted` says yes to.
    pub fn any_expr(&self, wanted: &dyn Fn(&crate::code::Expr) -> bool) -> bool {
        let mut found = false;
        let mut look = |body: &[crate::code::Stmt]| {
            crate::code::each_expr_in(body, &mut |e| found |= wanted(e));
        };
        for file in &self.files {
            for block in &file.code {
                block.handlers.iter().for_each(|h| look(&h.body));
                block.events.iter().for_each(|e| look(&e.body));
            }
            file.functions.iter().for_each(|f| look(&f.body));
            file.commands.iter().for_each(|c| look(&c.body));
        }
        found
    }

    pub fn enums(&self) -> impl Iterator<Item = &EnumDecl> {
        self.files.iter().flat_map(|f| f.enums.iter())
    }

    pub fn pages(&self) -> impl Iterator<Item = &Page> {
        self.files.iter().flat_map(|f| f.pages.iter())
    }

    pub fn apps(&self) -> impl Iterator<Item = &App> {
        self.files.iter().flat_map(|f| f.apps.iter())
    }

    /// An entity or a record (DESIGN 5.12) by its name: what has fields.
    pub fn entity(&self, name: &str) -> Option<&Entity> {
        self.entities().find(|e| e.name.name == name).or_else(|| self.records().find(|r| r.name.name == name))
    }

    /// The records of every module (DESIGN 5.12): values with fields, never stored.
    pub fn records(&self) -> impl Iterator<Item = &Entity> {
        self.files.iter().flat_map(|f| f.records.iter())
    }

    pub fn enum_decl(&self, name: &str) -> Option<&EnumDecl> {
        self.enums().find(|e| e.name.name == name)
    }
}

/// Turns a name into a default label: `credit_limit` -> `Credit limit`,
/// `CustomerSheet` -> `Customer sheet`.
pub fn default_label(name: &str) -> String {
    let mut spaced = String::new();
    for (i, c) in name.chars().enumerate() {
        if c == '_' {
            spaced.push(' ');
        } else if c.is_uppercase() && i > 0 {
            spaced.push(' ');
            spaced.extend(c.to_lowercase());
        } else {
            spaced.push(c);
        }
    }
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `CustomerKind` and `My shop` as `customer_kind` and `my_shop`: the
/// default name of a table, and of a Rust module.
pub fn snake_case(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_').to_string();
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        format!("app_{out}")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_label_humanizes_names() {
        assert_eq!(default_label("credit_limit"), "Credit limit");
        assert_eq!(default_label("name"), "Name");
        assert_eq!(default_label(""), "");
        assert_eq!(default_label("CustomerSheet"), "Customer sheet");
    }
}

/// The language and currency a project names in its project file. Values
/// are shown in the locale of the person using the application; the
/// language is the one to use where the system gives none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Locale {
    /// A language tag such as `en` or `nl-NL`.
    pub language: Option<String>,
    /// The ISO 4217 code of the currency of `money` fields, such as `EUR`.
    pub currency: Option<String>,
}

/// `theme Shop`: named values for how an application looks, which each
/// stack maps onto its own means (design section 1.5, T1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ThemeDecl {
    pub name: Ident,
    /// The class an application holds to use the theme: `theme-night-owl`.
    pub class: String,
    /// The name people see when they choose a theme: its `title`, or one
    /// derived from its name.
    pub label: String,
    pub title: Option<String>,
    pub mode: Option<ThemeMode>,
    pub colors: Vec<ThemeColor>,
    /// `font "Inter"`, `font heading "Georgia"`, `font mono "..."`: the
    /// family per role; a line without a role is `text`.
    pub fonts: Vec<ThemeValue>,
    /// `size 14`, `size small 12`, `size heading 18`: text sizes in pixels
    /// per role; a line without a role is `text`.
    pub sizes: Vec<ThemeValue>,
    /// `radius 6`, `radius input 4`: corner radii in pixels per kind of
    /// part; a line without a kind is `all`.
    pub radii: Vec<ThemeValue>,
    /// The space inside inputs and buttons, in pixels.
    pub padding: Option<u32>,
    /// The width of borders, in pixels.
    pub border: Option<u32>,
    /// The shadow under menus, popups and dialogs.
    pub shadow: Option<Shadow>,
    /// How a kind of part looks, in a state or by default:
    /// `part button hover background accent_soft`.
    pub parts: Vec<ThemePart>,
    pub styles: Vec<ThemeStyle>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ThemeMode {
    Light,
    Dark,
    /// Follows the system's light or dark mode.
    System,
}

/// `color accent "#2f6fdf" dark "#5b8cff"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ThemeColor {
    pub name: Ident,
    pub light: String,
    /// The value in dark mode, when it differs.
    pub dark: Option<String>,
    pub span: Span,
}

/// `style primary background accent, text "#ffffff"`: a color is a color
/// of the theme by name, or written as `"#rrggbb"`; a border can be `none`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ThemeStyle {
    pub name: Ident,
    /// The class a part holds to take the style: `style-primary`.
    pub class: String,
    pub look: Look,
    pub span: Span,
}

/// What a style or a part look sets. A color is a color of the theme by
/// name, or written as `"#rrggbb"`; a border can be `none`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Look {
    pub background: Option<String>,
    pub text: Option<String>,
    pub border: Option<String>,
    pub radius: Option<u32>,
    pub weight: Option<Weight>,
    /// The size of a style's text, in pixels.
    pub size: Option<u32>,
    /// A font of the theme for a style's text, by role: `mono`.
    pub font: Option<String>,
    /// Where a style's text sits across its part.
    pub align: Option<Align>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Weight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Shadow {
    None,
    Flat,
    Raised,
}

/// A value of a theme by role: `font heading "Georgia"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ThemeValue {
    pub role: String,
    pub value: String,
    pub span: Span,
}

/// `part button hover background accent_soft`: how a kind of part looks in
/// a state, or by default without one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ThemePart {
    pub kind: String,
    pub state: Option<String>,
    pub look: Look,
    pub span: Span,
}

/// Whether a part of `widgets`, or of a container among them, binds the setting `name`.
pub fn shows_setting(widgets: &[Widget], name: &str) -> bool {
    let path = format!("{}.{name}", crate::parser::SETTINGS_DATA);
    widgets.iter().any(|w| {
        w.props.bind.as_deref() == Some(path.as_str()) || shows_setting(&w.children, name) || w.pages.iter().any(|p| shows_setting(&p.children, name))
    })
}
