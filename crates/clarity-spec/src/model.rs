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
            Self::Integer | Self::Decimal { .. } | Self::Money | Self::Id
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
    pub data: Vec<DataDecl>,
    /// Number of grid columns.
    pub columns: u32,
    pub widgets: Vec<Widget>,
    /// Regions of a window that override the app's.
    pub frame: Frame,
    /// Handlers of the window's own events: open, close, return.
    pub handlers: Vec<Handler>,
    pub span: Span,
}

pub const DEFAULT_PAGE_COLUMNS: u32 = 12;
pub const MAX_PAGE_COLUMNS: u32 = 24;

/// `data customer: Customer` or `data orders: list<Order>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DataDecl {
    pub name: Ident,
    pub entity: Ident,
    pub list: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Widget {
    pub props: WidgetProps,
    /// Children of a plain container, on the container's own grid.
    pub children: Vec<Widget>,
    /// Tabs of a `tabs` widget or panes of a `splitter`.
    pub pages: Vec<WidgetPage>,
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
    /// Handlers in the code behind, per event.
    pub handlers: Vec<Handler>,
    pub columns: Vec<String>,
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
    /// Text shown when the pointer rests on the part.
    pub tooltip: Option<String>,
    /// A named style of the project's theme: `style primary`.
    pub style: Option<String>,
    /// An input that shows its value but cannot be changed.
    pub readonly: bool,
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
/// when the event happens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Handler {
    pub event: Event,
    pub handler: String,
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
    pub items: Vec<Command>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Toolbar {
    pub items: Vec<Command>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Ribbon {
    pub tabs: Vec<RibbonTab>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RibbonTab {
    pub title: String,
    pub groups: Vec<RibbonGroup>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RibbonGroup {
    pub title: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Sidebar {
    pub side: Side,
    /// Columns of the sidebar's own grid.
    pub width: u32,
    pub widgets: Vec<Widget>,
    pub span: Span,
}

pub const DEFAULT_SIDEBAR_WIDTH: u32 = 3;

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
    /// The themes the application offers, the first being the one it starts
    /// with; empty when the project file names none (design section 1.5, T1).
    pub themes: Vec<String>,
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
    pub font: Option<String>,
    /// The text size, in pixels.
    pub size: Option<u32>,
    /// The corner radius of inputs, buttons and panels, in pixels.
    pub radius: Option<u32>,
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
    pub background: Option<String>,
    pub text: Option<String>,
    pub border: Option<String>,
    pub span: Span,
}
