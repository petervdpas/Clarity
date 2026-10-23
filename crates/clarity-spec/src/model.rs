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
    pub pages: Vec<Page>,
    pub apps: Vec<App>,
    /// Themes: named colors, a font and named styles for parts.
    pub themes: Vec<ThemeDecl>,
    /// Code behind: one block per window or app, with its handlers.
    pub code: Vec<crate::code::CodeBlock>,
    /// Functions and actions declared in this module.
    pub functions: Vec<crate::code::FunctionDecl>,
    /// Its engines (DESIGN 5.11).
    pub engines: Vec<crate::code::EngineDecl>,
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

    /// The column a field is stored in.
    pub fn column_of<'a>(&'a self, field: &'a str) -> &'a str {
        self.field(field).and_then(|f| f.modifiers.column.as_deref()).unwrap_or(field)
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
    Email,
    Url,
    Id,
    /// A named type that must resolve to an enum.
    Enum {
        name: Ident,
    },
    Ref {
        entity: Ident,
    },
    List {
        item: Ident,
    },
}

impl FieldType {
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Integer | Self::Decimal { .. } | Self::Money | Self::Id | Self::Duration
        )
    }

    pub fn is_textual(&self) -> bool {
        matches!(self, Self::Text { .. } | Self::Email | Self::Url)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Modifiers {
    pub required: bool,
    pub unique: bool,
    pub owned: bool,
    pub computed: bool,
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
    pub data: Vec<DataDecl>,
    /// Number of grid columns.
    pub columns: u32,
    pub widgets: Vec<Widget>,
    /// Regions of a window that override the app's.
    pub frame: Frame,
    /// Handlers of the window's own events: open, close, return.
    pub handlers: Vec<Handler>,
    /// What a dialog gives when it closes with a value (`page X: dialog
    /// gives Friend`), for the handler that asked for it (DESIGN 5.14).
    pub gives: Option<crate::code::TypeRef>,
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
pub const MAX_PAGE_COLUMNS: u32 = 24;

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
    pub span: Span,
}

/// A `tab "Title"` or `pane` block inside a container widget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct WidgetPage {
    pub title: Option<String>,
    pub children: Vec<Widget>,
    pub span: Span,
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
    /// Handlers in the code behind, per event.
    pub handlers: Vec<Handler>,
    pub columns: Vec<String>,
    /// The columns of a table whose cells are inputs: `edit done`.
    #[serde(default)]
    pub edit: Vec<String>,
    /// The edited bool columns shown as a switch instead of a checkbox:
    /// `edit done as switch`.
    #[serde(default)]
    pub switches: Vec<String>,
    /// The width in pixels of a table's columns that have one:
    /// `columns title 240, artist`. A column without one shares what is left.
    #[serde(default)]
    pub widths: std::collections::BTreeMap<String, u32>,
    /// `resizable`: the person drags a table's header edges, and the widths are remembered.
    #[serde(default)]
    pub resizable: bool,
    /// `sortable`: a click on a table's header sorts its rows by that column.
    #[serde(default)]
    pub sortable: bool,
    /// `order by field [desc]`: the order a table's rows show in first.
    #[serde(default)]
    pub order: Option<TableOrder>,
    pub show: Vec<String>,
    pub orientation: Option<Orientation>,
    pub surface: Option<Surface>,
    pub renderer: Option<String>,
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
    /// Made by Clarity, not written: a label of a form's label column. The
    /// designer leaves it alone and it is never written back.
    pub made: bool,
    /// A named style of the project's theme: `style primary`.
    pub style: Option<String>,
    /// An input that shows its value but cannot be changed.
    pub readonly: bool,
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
    /// The project's translations per language: each text a person sees,
    /// as written, with its translation (design section 1.5, I1c).
    pub translations: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    /// The keys whose value in a language is a file of the resources folder
    /// (`gameRules resource "rules-nl.md"`), per language: the file's name.
    pub resource_keys: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    /// The themes the application offers, the first being the one it starts
    /// with; empty when the project file names none (design section 1.5, T1).
    pub themes: Vec<String>,
    /// The files of the project's `resources` folder. None when the project
    /// was loaded without its folder.
    pub resources: Option<crate::resources::Resources>,
    /// The written diagrams of the project's `diagrams` folder (DESIGN
    /// 5.13), by name; empty when there are none or the folder was not read.
    pub diagrams: Vec<crate::diagrams::DiagramFile>,
    /// The files of the project's `engines` folder, by name. None when the
    /// project was loaded without its folder.
    #[serde(skip)]
    #[ts(skip)]
    pub engines: Option<Vec<crate::engines::EngineFile>>,
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
        self.files.iter().filter(move |f| f.source.as_ref().map_or(self.default_source(), |s| s.name.as_str()) == source).flat_map(|f| &f.entities)
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == crate::diagnostic::Severity::Error)
    }

    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.files.iter().flat_map(|f| f.entities.iter())
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

    pub fn entity(&self, name: &str) -> Option<&Entity> {
        self.entities().find(|e| e.name.name == name)
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
