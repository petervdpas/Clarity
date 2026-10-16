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
    pub enums: Vec<EnumDecl>,
    pub entities: Vec<Entity>,
    pub pages: Vec<Page>,
    pub apps: Vec<App>,
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
    pub span: Span,
}

impl Entity {
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name.name == name)
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
}

impl Project {
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
