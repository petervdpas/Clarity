//! Structural edits on spec source text, used by the designer.
//!
//! The source is read into a line tree: every content line becomes a node,
//! nested by indentation, and comment and blank lines travel with the node
//! that follows them. Untouched lines are written back byte for byte, so an
//! edit changes only the lines it is about. Every edit is re-parsed before
//! it is accepted; an edit that would add a parse error is rejected.
//!
//! Elements are addressed by the byte offset where their line content
//! starts, which is `span.start` of the corresponding model element.

use serde::Deserialize;
use ts_rs::TS;

use crate::diagnostic::Severity;
use crate::dictionary::{FieldSpec, validate_field};
use crate::frame::{self, RegionKind};
use crate::lexer::{Token, TokenKind, lex, text_token};
use crate::model::{
    Project,
    CommandKind, CommandProps, PageKind, Side, WidgetProps,
};
use crate::parser::{
    is_app_setting, is_page_setting, is_region_line, parse, starts_with_keyword,
};
use crate::templates::PageTemplate;
use crate::widgets::{self, ContainerKind, Event, WidgetKind, is_identifier};

/// Indentation added for a new nesting level when no sibling shows the
/// indentation already in use.
const INDENT_STEP: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(tag = "op", rename_all = "camelCase")]
#[ts(export)]
pub enum Edit {
    /// Remove a window, the app, a widget, tab, region, command or data
    /// declaration.
    Remove { item: u32 },
    /// Set the title of a page, app, tab, ribbon tab or group, or the footer
    /// status. An empty title removes a page's or app's title.
    SetTitle { target: u32, title: String },
    /// Set the columns a splitter's first pane starts with (`pane span 4`),
    /// or none, so the panes start alike.
    SetPaneSpan { target: u32, columns: Option<u32> },
    /// Set the view a tab, step or section shows (`section "Class" view
    /// ClassEditor`), or none, so it holds parts of its own.
    SetPageView { target: u32, view: Option<String> },
    /// Declare what a view tells its window (`<Tells message="saved(name:
    /// text)" />`), in place of a tell named `name`; with no message, remove
    /// the tell `name`.
    SetPageTell { page: u32, name: String, message: Option<String> },
    /// Wire a tell of the view a view line (or tab, step or section) shows
    /// to a handler of the window (`onsaved="recipeSaved"`), or unwire it.
    SetViewTell { target: u32, tell: String, handler: Option<String> },
    /// Set the color of an accordion section's header (`section "Chapters"
    /// color "#2e86de"`), or none, so it takes the theme's look.
    SetPageColor { target: u32, color: Option<String> },
    /// Place a new widget on a page, panel, tab or pane.
    PlaceWidget { container: u32, props: WidgetProps },
    /// Place a new widget with the catalog defaults for its kind, optionally
    /// bound to data (used when a field is dropped onto a page).
    CreateWidget {
        container: u32,
        kind: WidgetKind,
        col: u32,
        row: u32,
        /// Overrides the catalog width, for containers narrower than it.
        width: Option<u32>,
        /// Overrides the catalog caption.
        caption: Option<String>,
        bind: Option<String>,
        /// A custom part (DESIGN 5.19): the name it is declared by, with `kind` custom.
        #[serde(default)]
        part: Option<String>,
        /// Overrides the catalog height, as a custom part's declared size does.
        #[serde(default)]
        height: Option<u32>,
        /// A sign-in field's input (`input password`), as the palette of a sign-in view offers it.
        #[serde(default)]
        input: Option<String>,
        /// A sign-in button's strategy (`strategy password`).
        #[serde(default)]
        strategy: Option<String>,
        /// The sign-in view an "other way" shows (`shows CodeView`).
        #[serde(default)]
        shows: Option<String>,
    },
    /// Several edits applied together: one write, one check, all or nothing.
    /// Offsets in later edits refer to the original text.
    Batch { edits: Vec<Edit> },
    /// Rewrite a widget's line from its properties; with `container`, also
    /// move it (with its children) into another container.
    UpdateWidget {
        item: u32,
        container: Option<u32>,
        props: WidgetProps,
    },
    /// Add a tab to a tabs widget.
    AddTab { item: u32, title: String },
    /// Give a part a `menu` block with one item: its context menu, or a split button's dropdown.
    AddMenu { item: u32 },
    /// Declare data on a page.
    AddData {
        page: u32,
        name: String,
        entity: String,
        list: bool,
    },
    /// Set the number of grid columns of a page.
    /// A page's grid: its columns, and the row height of its own exact grid (`grid 56 columns rows 10`); none for the project's rows.
    SetGrid {
        page: u32,
        columns: u32,
        #[serde(default)]
        rows: Option<u32>,
    },
    /// Set the size a window opens at, in pixels; none for the standard size.
    SetSize { page: u32, width: Option<u32>, height: Option<u32> },
    /// The window's own style of the theme (`style splashBack`); empty for none.
    SetPageStyle { page: u32, style: String },
    /// Who may open a window or view (`allow="me.role == Admin"`); empty for everyone.
    SetPageAllow { page: u32, allow: String },
    /// Add an empty page at the end of the file.
    AddPage { name: String, kind: PageKind },
    /// Add a window at the end of the file, blank or from a template.
    /// Needs the project (see `apply_in`).
    AddWindow { name: String, template: PageTemplate },
    /// Place a field of the window's data at a cell: its input, with a
    /// label for it and its help text when there is room. `room` is the
    /// number of columns to the edge of the container. Needs the project.
    PlaceField {
        container: u32,
        col: u32,
        row: u32,
        room: u32,
        /// `data.field`
        bind: String,
    },
    /// Add an entity with a first field, after the other data declarations.
    AddEntity { name: String },
    /// Add an enum with a first variant, after the other data declarations.
    AddEnum { name: String },
    /// Add a field at the end of an entity.
    AddField { entity: u32, spec: FieldSpec },
    /// Rewrite a field's line.
    UpdateField { item: u32, spec: FieldSpec },
    /// Add a variant at the end of an enum.
    AddVariant {
        owner: u32,
        name: String,
        label: Option<String>,
    },
    /// Rewrite a variant's line.
    UpdateVariant {
        item: u32,
        name: String,
        label: Option<String>,
    },
    /// Set the modules this file uses, replacing its `use` lines.
    SetUses { uses: Vec<String> },
    /// Add one of Clarity's own settings (`OWN_SETTINGS`) to the file's
    /// settings block as its bare word, starting the block when there is none.
    AddOwnSetting { word: String },
    /// Add a setting (`pageSize: integer, default 50`) at the end of the
    /// file's settings block, starting the block when there is none.
    AddSetting { spec: FieldSpec },
    /// Give every part of the file's windows and app that has no name the
    /// first free one of its kind, as a new part gets: every part has a name.
    NameParts,
    /// Rename every part and handler of the file that is not camelCase to
    /// camelCase, wherever the file uses it.
    CamelNames,
    /// Add an app at the end of the file.
    AddApp { name: String },
    /// Set the main window of an app.
    SetMain { app: u32, main: String },
    /// Set the app's splash page and how long it shows at least, in
    /// milliseconds (`splash Welcome for 2 seconds`); an empty name takes the splash away.
    SetSplash { app: u32, splash: String, ms: Option<u32> },
    /// Wire one of a window's or the app's own events (open, close, return,
    /// start) to a handler, or unwire it with None.
    SetOwnerEvent {
        owner: u32,
        event: Event,
        handler: Option<String>,
    },
    /// Dock a region (with starter content) to a page or app.
    AddRegion {
        owner: u32,
        region: RegionKind,
        /// Sidebars only.
        side: Option<Side>,
    },
    /// Set the side and width of a sidebar.
    /// Rewrite a sidebar's line: its side, name, width and what the person may do with it.
    UpdateSidebar { item: u32, props: crate::model::SidebarProps },
    /// Rewrite a ribbon's line: its name and whether the person folds it to its tabs.
    UpdateRibbon { item: u32, name: Option<String>, collapsible: bool, collapsed: bool },
    /// Give the menu, toolbar, or ribbon tab or group at `item` the name `name`.
    SetName { item: u32, name: String },
    /// Write every sidebar of the file in the current way: old ones, given in
    /// columns (`span 3`), get their width in pixels, and each gets a name.
    UpdateSidebars,
    /// Add an item at the end of a menu, submenu, toolbar, ribbon, ribbon
    /// tab or ribbon group.
    AddFrameItem {
        parent: u32,
        item: FrameItem,
        caption: Option<String>,
    },
    /// Rewrite a menu item's or button's line from its properties.
    UpdateCommand { item: u32, props: CommandProps },
    /// Move a command, ribbon tab, ribbon group or region one place up
    /// (`-1`) or down (`1`) among its siblings of the same kind.
    MoveSibling { item: u32, delta: i32 },
    /// Replace the code behind of a window or the app (section 5.9) with
    /// `text`, written one level less indented than in the file. Empty
    /// text removes the code block.
    SetCode { owner: String, text: String },
    /// Add an empty handler to the code behind of a window or the app,
    /// unless it is there already. Sent with the edit that wires an event.
    AddHandler { owner: String, name: String },
    /// As `AddHandler`, with the values it takes: the handler of a custom
    /// part's tell takes the tell's values (`points: integer`).
    AddHandlerWith { owner: String, name: String, params: String },
    /// Replace the top-level declaration that starts at `item` (a window,
    /// the app, an entity, an enum, a function, an action or a code block)
    /// with `text`, which must be one declaration.
    ReplaceDecl { item: u32, text: String },
    /// Add `text`, one declaration, at the end of the file.
    AppendDecl { text: String },
    /// Whether the custom part declared at `item` grows with its window:
    /// its `grows` line is added or taken away, the rest as written.
    SetPartGrows { item: u32, grows: bool },
    /// Whether the command declared at `item` is offered to AI agents: its
    /// `agent` line is added after its help, options and allow, or taken away.
    SetCommandAgent { item: u32, agent: bool },
    /// Set the line with `key` of the theme, custom part or library
    /// declared at `item` to `line`, add it when there is none, or take it
    /// away (`None`); the keys are `decl_lines`'s (`color accent`,
    /// `part button hover`, `property speed`, `function spell`).
    SetDeclLine { item: u32, key: String, line: Option<String> },
}

/// What `AddFrameItem` adds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FrameItem {
    Item,
    Button,
    Separator,
    Tab,
    Group,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditOutcome {
    pub text: String,
    /// Offset of the element the edit produced or moved, in the new text.
    pub focus: Option<u32>,
    /// The keys the edit made of texts it wrote, with their values in the
    /// default language: the language files get them (crate::keyed).
    pub keys: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditError(pub String);

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EditError {}

pub type EditResult<T> = Result<T, EditError>;

/// What an edit of a window, view or app says when it is made on a module's text.
pub const IN_DESIGN_FILE: &str = "this change belongs in the window's, view's or app's own .cxaml file, not in a module's file of Clarity";

fn fail<T>(message: impl Into<String>) -> EditResult<T> {
    Err(EditError(message.into()))
}

/// Applies one edit to a spec source text.
pub fn apply(source: &str, edit: &Edit) -> EditResult<EditOutcome> {
    apply_with(source, edit, None)
}

/// Applies one edit to a spec file of a project. Edits that build on
/// declarations in other files (window templates) need this.
pub fn apply_in(project: &Project, source: &str, edit: &Edit) -> EditResult<EditOutcome> {
    let mut outcome = apply_with(source, edit, Some(project))?;
    if outcome.text != source {
        // A text people read that the edit wrote (a template's labels, a
        // caption typed for a new part) becomes a key, beside the project's.
        let default = crate::translations::default_language(project);
        let existing = project.translations.get(&default).cloned().unwrap_or_default();
        let before_focus = outcome.focus.map(|f| outcome.text[..f as usize].to_string());
        let (text, keys) = crate::keyed::key_module("", &outcome.text, &default, &existing);
        if !keys.is_empty() {
            // The focus moves with the text before it.
            if let Some(before) = before_focus {
                let (keyed_before, _) = crate::keyed::key_module("", &before, &default, &existing);
                outcome.focus = text.starts_with(&keyed_before).then_some(keyed_before.len() as u32).or(outcome.focus);
            }
            outcome.text = text;
            outcome.keys = keys;
        }
    }
    Ok(outcome)
}

/// Edits that work on the text instead of the line tree, so the comments
/// and blank lines a developer writes stay exactly as written.
fn is_text_edit(edit: &Edit) -> bool {
    matches!(
        edit,
        Edit::SetCode { .. } | Edit::AddHandler { .. } | Edit::AddHandlerWith { .. } | Edit::ReplaceDecl { .. } | Edit::AppendDecl { .. } | Edit::SetPartGrows { .. } | Edit::SetCommandAgent { .. } | Edit::SetDeclLine { .. } | Edit::CamelNames
    )
}

fn apply_with(source: &str, edit: &Edit, project: Option<&Project>) -> EditResult<EditOutcome> {
    // Text edits work on the text after the layout edits of the same batch.
    let (layout, code): (Vec<&Edit>, Vec<&Edit>) = match edit {
        Edit::Batch { edits } => edits.iter().partition(|e| !is_text_edit(e)),
        edit if is_text_edit(edit) => (Vec::new(), vec![edit]),
        edit => (vec![edit], Vec::new()),
    };
    let (mut text, mut focus) = match layout.as_slice() {
        [] => (source.to_string(), None),
        [single] if !matches!(edit, Edit::Batch { .. }) => apply_layout(source, single, project)?,
        many => {
            let batch = Edit::Batch {
                edits: many.iter().map(|e| (*e).clone()).collect(),
            };
            apply_layout(source, &batch, project)?
        }
    };
    for edit in code {
        text = match edit {
            Edit::SetCode { owner, text: code } => crate::code::edit::set_code(&text, owner, code)?,
            Edit::AddHandler { owner, name } => crate::code::edit::add_handler(&text, owner, name)?,
            Edit::AddHandlerWith { owner, name, params } => crate::code::edit::add_handler_with(&text, owner, name, params)?,
            Edit::ReplaceDecl { item, text: decl } => {
                focus = Some(*item);
                crate::source::replace_decl(&text, *item, decl)?
            }
            Edit::SetPartGrows { item, grows } => {
                focus = Some(*item);
                crate::source::set_part_grows(&text, *item, *grows)?
            }
            Edit::SetDeclLine { item, key, line } => {
                focus = Some(*item);
                crate::decl_lines::set_decl_line(&text, *item, key, line.as_deref())?
            }
            Edit::SetCommandAgent { item, agent } => {
                focus = Some(*item);
                crate::source::set_command_agent(&text, *item, *agent)?
            }
            Edit::CamelNames => crate::names::camel_names(&text),
            Edit::AppendDecl { text: decl } => {
                focus = Some(crate::source::appended_at(&text));
                crate::source::append_decl(&text, decl)?
            }
            _ => unreachable!("only text edits are left"),
        };
    }
    if text != source {
        reject_new_parse_errors(source, &text)?;
    }
    Ok(EditOutcome { text, focus, keys: Vec::new() })
}

fn apply_layout(source: &str, edit: &Edit, _project: Option<&Project>) -> EditResult<(String, Option<u32>)> {
    let mut doc = Doc::read(source);
    let focus = doc.apply(edit)?;
    Ok(doc.write(focus))
}

fn reject_new_parse_errors(before: &str, after: &str) -> EditResult<()> {
    let errors = |text: &str| {
        parse(text, "")
            .1
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .collect::<Vec<_>>()
    };
    let old = errors(before);
    let new = errors(after);
    if new.len() > old.len() {
        let first = new
            .iter()
            .find(|d| !old.iter().any(|o| o.message == d.message))
            .unwrap_or(&new[0]);
        return fail(format!(
            "the edit would make the spec invalid: {}",
            first.message
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Line tree

#[derive(Debug, Clone)]
struct Trivia {
    indent: usize,
    /// The indentation as read, until the line is shifted.
    raw_indent: Option<String>,
    /// Text after the indentation; empty for blank lines.
    text: String,
}

#[derive(Debug, Clone)]
struct LNode {
    /// Offset of the line content in the original text, or a fresh id for
    /// nodes created by the edit.
    id: u32,
    indent: usize,
    /// The indentation as read, until the node is shifted.
    raw_indent: Option<String>,
    /// Text after the indentation, including any trailing comment.
    text: String,
    /// Tokens with spans relative to the start of `text`.
    tokens: Vec<Token>,
    leading: Vec<Trivia>,
    children: Vec<LNode>,
}

impl LNode {
    fn set_text(&mut self, text: String) {
        self.tokens = lex_tokens(&text);
        self.text = text;
    }



    fn child_indent(&self) -> usize {
        self.children
            .first()
            .map(|c| c.indent)
            .unwrap_or(self.indent + INDENT_STEP)
    }
}

fn lex_tokens(text: &str) -> Vec<Token> {
    lex(text)
        .0
        .into_iter()
        .next()
        .map(|line| line.tokens)
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Page,
    PageSetting,
    Widget(Option<ContainerKind>),
    WidgetTab,
    Pane,
    App,
    AppSetting,
    Region(RegionKind),
    Entity,
    Field,
    Enum,
    Variant,
    /// An engine's declaration, which its editor writes as a whole.
    Engine,
    /// A theme's declaration, which its view writes as a whole.
    Theme,
    /// A mapping's declaration, which its page writes as a whole.
    Mapping,
    /// A toast's declaration, which its form sets line by line and Delete takes away.
    Toast,
    Use,
    /// A menu item, button or separator.
    Command,
    /// A part's context menu: a `menu` block under it (DESIGN 5.16).
    ContextMenu,
    RibbonTab,
    RibbonGroup,
    FooterStatus,
    Other,
}

impl Kind {

}

fn widget_kind(node: &LNode) -> Kind {
    match node.tokens.first().and_then(|t| widgets::by_keyword(&t.text).or_else(|| custom_word(t))) {
        Some(kind) => Kind::Widget(widgets::container(kind)),
        None => Kind::Other,
    }
}

/// A word in capitals starting a part's line: a custom part's declared name (DESIGN 5.19).
fn custom_word(token: &Token) -> Option<WidgetKind> {
    (token.kind == TokenKind::Ident && token.text.starts_with(|c: char| c.is_ascii_uppercase())).then_some(WidgetKind::Custom)
}

fn child_kind(parent: Kind, node: &LNode) -> Kind {
    match parent {
        Kind::Page if is_page_setting(&node.tokens) => Kind::PageSetting,
        Kind::App if is_app_setting(&node.tokens) => Kind::AppSetting,
        Kind::Entity => Kind::Field,
        Kind::Enum => Kind::Variant,
        Kind::Page | Kind::App if is_region_line(&node.tokens) => region_kind(node),
        Kind::Widget(_) if starts_with_keyword(&node.tokens, "menu", None) => Kind::ContextMenu,
        Kind::Region(RegionKind::Menu) | Kind::Command | Kind::ContextMenu
            if command_keyword_is(node, &[CommandKind::Item, CommandKind::Separator]) =>
        {
            Kind::Command
        }
        Kind::Region(RegionKind::Toolbar) | Kind::RibbonGroup
            if command_keyword_is(node, &[CommandKind::Button, CommandKind::Separator]) =>
        {
            Kind::Command
        }
        Kind::Region(RegionKind::Ribbon)
            if starts_with_keyword(&node.tokens, "tab", Some(TokenKind::Str)) =>
        {
            Kind::RibbonTab
        }
        Kind::RibbonTab if starts_with_keyword(&node.tokens, "group", Some(TokenKind::Str)) => {
            Kind::RibbonGroup
        }
        Kind::Region(RegionKind::Footer)
            if starts_with_keyword(&node.tokens, "status", Some(TokenKind::Str)) =>
        {
            Kind::FooterStatus
        }
        Kind::Page
        | Kind::Widget(Some(ContainerKind::Plain))
        | Kind::WidgetTab
        | Kind::Pane
        | Kind::Region(RegionKind::Sidebar)
        | Kind::Region(RegionKind::Footer) => widget_kind(node),
        Kind::Widget(Some(ContainerKind::Tabs))
            if starts_with_keyword(&node.tokens, "tab", Some(TokenKind::Str)) =>
        {
            Kind::WidgetTab
        }
        // A wizard's step is edited as a tab is: its title, its parts.
        Kind::Widget(Some(ContainerKind::Accordion))
            if starts_with_keyword(&node.tokens, "section", Some(TokenKind::Str)) =>
        {
            Kind::WidgetTab
        }
        Kind::Widget(Some(ContainerKind::Steps))
            if starts_with_keyword(&node.tokens, "step", Some(TokenKind::Str)) =>
        {
            Kind::WidgetTab
        }
        // A view a window's content shows is edited as a tab showing a view is.
        Kind::Widget(Some(ContainerKind::Content))
            if starts_with_keyword(&node.tokens, "view", Some(TokenKind::Ident)) =>
        {
            Kind::WidgetTab
        }
        Kind::Widget(Some(ContainerKind::Splitter))
            if starts_with_keyword(&node.tokens, "pane", None) || starts_with_keyword(&node.tokens, "pane", Some(TokenKind::Ident)) =>
        {
            Kind::Pane
        }
        _ => Kind::Other,
    }
}

fn region_kind(node: &LNode) -> Kind {
    node.tokens
        .first()
        .and_then(|t| frame::by_keyword(&t.text))
        .map(Kind::Region)
        .unwrap_or(Kind::Other)
}

fn command_keyword_is(node: &LNode, kinds: &[CommandKind]) -> bool {
    node.tokens
        .first()
        .and_then(|t| frame::command_by_keyword(&t.text))
        .is_some_and(|k| kinds.contains(&k))
}

fn top_kind(node: &LNode) -> Kind {
    if starts_with_keyword(&node.tokens, "page", Some(TokenKind::Ident)) {
        Kind::Page
    } else if starts_with_keyword(&node.tokens, "app", Some(TokenKind::Ident)) {
        Kind::App
    } else if starts_with_keyword(&node.tokens, "entity", Some(TokenKind::Ident)) {
        Kind::Entity
    } else if node.tokens.len() == 1 && node.tokens[0].text == "settings" {
        // The settings block's lines are fields, edited as an entity's.
        Kind::Entity
    } else if starts_with_keyword(&node.tokens, "enum", Some(TokenKind::Ident)) {
        Kind::Enum
    } else if starts_with_keyword(&node.tokens, "engine", Some(TokenKind::Ident)) {
        Kind::Engine
    } else if starts_with_keyword(&node.tokens, "theme", Some(TokenKind::Ident)) {
        Kind::Theme
    } else if starts_with_keyword(&node.tokens, "mapping", Some(TokenKind::Ident)) {
        Kind::Mapping
    } else if starts_with_keyword(&node.tokens, "toast", Some(TokenKind::Ident)) {
        Kind::Toast
    } else if starts_with_keyword(&node.tokens, "use", Some(TokenKind::Ident)) {
        Kind::Use
    } else {
        Kind::Other
    }
}

struct Doc {
    nodes: Vec<LNode>,
    trailing: Vec<Trivia>,
    next_id: u32,
}

impl Doc {
    fn read(source: &str) -> Self {
        let (lines, _) = lex(source);
        let mut lexed = lines.into_iter().peekable();

        // Flat list of content nodes, each carrying the trivia before it.
        let mut flat = Vec::new();
        let mut pending = Vec::new();
        for (index, raw) in source.split('\n').enumerate() {
            let line_no = index as u32 + 1;
            let indent = raw.len() - raw.trim_start_matches([' ', '\t']).len();
            match lexed.next_if(|l| l.span.line == line_no) {
                Some(line) => {
                    let id = line.span.start;
                    let tokens = line
                        .tokens
                        .into_iter()
                        .map(|mut t| {
                            t.span.start -= id;
                            t.span.end -= id;
                            t
                        })
                        .collect();
                    flat.push(LNode {
                        id,
                        indent: line.indent,
                        raw_indent: Some(raw[..line.indent].to_string()),
                        text: raw[line.indent..].to_string(),
                        tokens,
                        leading: std::mem::take(&mut pending),
                        children: Vec::new(),
                    });
                }
                None => pending.push(Trivia {
                    indent,
                    raw_indent: Some(raw[..indent].to_string()),
                    text: raw[indent..].to_string(),
                }),
            }
        }

        let mut iter = flat.into_iter().peekable();
        let mut nodes = Vec::new();
        while iter.peek().is_some() {
            let indent = iter.peek().map(|n| n.indent).unwrap_or(0);
            nodes.extend(nest(&mut iter, indent));
        }

        Doc {
            nodes,
            trailing: pending,
            next_id: u32::MAX,
        }
    }

    fn write(&self, focus: Option<u32>) -> (String, Option<u32>) {
        struct Writer {
            out: String,
            first: bool,
            focus: Option<u32>,
            found: Option<u32>,
        }
        impl Writer {
            fn line(&mut self, indent: usize, raw_indent: Option<&str>, text: &str) -> u32 {
                if !self.first {
                    self.out.push('\n');
                }
                self.first = false;
                match raw_indent {
                    Some(raw) => self.out.push_str(raw),
                    None => self.out.extend(std::iter::repeat_n(' ', indent)),
                }
                let start = self.out.len();
                self.out.push_str(text);
                start as u32
            }
            fn node(&mut self, node: &LNode) {
                for trivia in &node.leading {
                    self.line(trivia.indent, trivia.raw_indent.as_deref(), &trivia.text);
                }
                let start = self.line(node.indent, node.raw_indent.as_deref(), &node.text);
                if self.focus == Some(node.id) {
                    self.found = Some(start);
                }
                for child in &node.children {
                    self.node(child);
                }
            }
        }

        let mut writer = Writer {
            out: String::new(),
            first: true,
            focus,
            found: None,
        };
        for node in &self.nodes {
            writer.node(node);
        }
        for trivia in &self.trailing {
            writer.line(trivia.indent, trivia.raw_indent.as_deref(), &trivia.text);
        }
        (writer.out, writer.found)
    }

    fn fresh_node(&mut self, indent: usize, text: String) -> LNode {
        let id = self.next_id;
        self.next_id -= 1;
        LNode {
            id,
            indent,
            raw_indent: None,
            tokens: lex_tokens(&text),
            text,
            leading: Vec::new(),
            children: Vec::new(),
        }
    }

    // -- navigation -------------------------------------------------------

    fn path_of(&self, id: u32) -> Option<Vec<usize>> {
        fn search(nodes: &[LNode], id: u32, path: &mut Vec<usize>) -> bool {
            for (i, node) in nodes.iter().enumerate() {
                path.push(i);
                if node.id == id || search(&node.children, id, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
        let mut path = Vec::new();
        search(&self.nodes, id, &mut path).then_some(path)
    }

    fn node(&self, path: &[usize]) -> &LNode {
        let mut node = &self.nodes[path[0]];
        for &i in &path[1..] {
            node = &node.children[i];
        }
        node
    }

    fn node_mut(&mut self, path: &[usize]) -> &mut LNode {
        let mut node = &mut self.nodes[path[0]];
        for &i in &path[1..] {
            node = &mut node.children[i];
        }
        node
    }

    fn siblings_mut(&mut self, path: &[usize]) -> &mut Vec<LNode> {
        match path.split_last() {
            Some((_, [])) | None => &mut self.nodes,
            Some((_, parent)) => &mut self.node_mut(parent).children,
        }
    }

    fn kind(&self, path: &[usize]) -> Kind {
        let mut kind = top_kind(&self.nodes[path[0]]);
        let mut node = &self.nodes[path[0]];
        for &i in &path[1..] {
            node = &node.children[i];
            kind = child_kind(kind, node);
        }
        kind
    }

    /// Finds an element by id and checks its kind.
    fn find(
        &self,
        id: u32,
        what: &str,
        accept: impl Fn(Kind) -> bool,
    ) -> EditResult<(Vec<usize>, Kind)> {
        let Some(path) = self.path_of(id) else {
            return fail(format!(
                "no element at offset {id}; the file may have changed"
            ));
        };
        let kind = self.kind(&path);
        if !accept(kind) {
            return fail(format!("the element at offset {id} is not {what}"));
        }
        Ok((path, kind))
    }



    // -- primitive operations ---------------------------------------------

    fn detach(&mut self, path: &[usize]) -> LNode {
        let index = *path.last().expect("detach needs a non-empty path");
        self.siblings_mut(path).remove(index)
    }

    /// Detaches a node that moves away or is removed. Blank lines before it
    /// are spacing, not content: they stay where they were, carried by the
    /// node that takes its place. Comments travel with the node.
    fn take(&mut self, path: &[usize]) -> LNode {
        let index = *path.last().expect("take needs a non-empty path");
        let mut node = self.detach(path);
        let blanks = node
            .leading
            .iter()
            .take_while(|t| t.text.is_empty())
            .count();
        let spacing: Vec<Trivia> = node.leading.drain(..blanks).collect();
        if let Some(next) = self.siblings_mut(path).get_mut(index) {
            let mut leading = spacing;
            leading.append(&mut next.leading);
            next.leading = leading;
        }
        node
    }





    // -- edits ------------------------------------------------------------

    /// An edit of a module's text: its entities, enums and the modules it
    /// uses. What a window, view or app holds is edited in its own `.cxaml`
    /// (`cxaml::apply`); a new one is files of its own (`cxaml::added`).
    fn apply(&mut self, edit: &Edit) -> EditResult<Option<u32>> {
        match edit {
            Edit::Remove { item } => self.remove(*item),
            Edit::Batch { edits } => {
                let mut focus = None;
                for edit in edits {
                    focus = self.apply(edit)?.or(focus);
                }
                Ok(focus)
            }
            Edit::AddEntity { name } => self.add_declaration("entity", name, "name: text(80)"),
            Edit::AddEnum { name } => self.add_declaration("enum", name, "First"),
            Edit::AddField { entity, spec } => self.add_field(*entity, spec),
            Edit::UpdateField { item, spec } => self.update_field(*item, spec),
            Edit::AddVariant { owner, name, label } => self.add_variant(*owner, name, label.as_deref()),
            Edit::UpdateVariant { item, name, label } => self.update_variant(*item, name, label.as_deref()),
            Edit::SetUses { uses } => self.set_uses(uses),
            Edit::AddOwnSetting { word } => self.add_own_setting(word),
            Edit::AddSetting { spec } => self.add_setting(spec),
            Edit::MoveSibling { item, delta } => self.move_sibling(*item, *delta),
            Edit::SetCode { .. } | Edit::AddHandler { .. } | Edit::AddHandlerWith { .. } | Edit::ReplaceDecl { .. } | Edit::AppendDecl { .. } | Edit::SetPartGrows { .. } | Edit::SetCommandAgent { .. } | Edit::SetDeclLine { .. } | Edit::CamelNames => {
                fail("text edits cannot be nested in a batch inside a batch")
            }
            Edit::AddWindow { .. } | Edit::AddPage { .. } | Edit::AddApp { .. } => fail("a new window, view or app is a file of its own in design/; add it with cxaml::added"),
            _ => fail(IN_DESIGN_FILE),
        }
    }




    /// Removes an entity, field, enum, variant, engine, theme or mapping of the module.
    fn remove(&mut self, item: u32) -> EditResult<Option<u32>> {
        let (path, _) = self.find(item, "an entity, field, enum, variant, engine, theme, mapping or toast", |k| {
            matches!(k, Kind::Entity | Kind::Field | Kind::Enum | Kind::Variant | Kind::Engine | Kind::Theme | Kind::Mapping | Kind::Toast)
        })?;
        self.take(&path);
        Ok(None)
    }




}

impl Doc {
    // -- pages ------------------------------------------------------------

























}

impl Doc {
    // -- apps and frame regions -------------------------------------------



















    fn move_sibling(&mut self, item: u32, delta: i32) -> EditResult<Option<u32>> {
        let (path, kind) =
            self.find(item, "a field or variant", |k| matches!(k, Kind::Field | Kind::Variant))?;
        if delta != -1 && delta != 1 {
            return fail("move by one place at a time");
        }
        let parent = &path[..path.len() - 1];
        let parent_kind = self.kind(parent);
        let index = *path.last().expect("item path is not empty");
        let same = |c: &LNode| match (kind, child_kind(parent_kind, c)) {
            (Kind::Region(_), Kind::Region(_)) => true,
            (a, b) => a == b,
        };
        let siblings = &self.node(parent).children;
        let other = if delta < 0 {
            siblings[..index].iter().rposition(same)
        } else {
            siblings[index + 1..]
                .iter()
                .position(same)
                .map(|i| i + index + 1)
        };
        let Some(other) = other else {
            return Ok(Some(item));
        };
        let siblings = &mut self.node_mut(parent).children;
        // Leading trivia (blank lines, comments) stays with each line.
        siblings.swap(index, other);
        Ok(Some(item))
    }
}





/// Nests a flat run of nodes by indentation, like the parser does.
fn nest(iter: &mut std::iter::Peekable<impl Iterator<Item = LNode>>, indent: usize) -> Vec<LNode> {
    let mut nodes = Vec::new();
    while let Some(mut node) = iter.next_if(|n| n.indent >= indent) {
        if let Some(next) = iter.peek()
            && next.indent > node.indent
        {
            let child_indent = next.indent;
            node.children = nest(iter, child_indent);
        }
        nodes.push(node);
    }
    nodes
}


impl Doc {
    // -- data dictionary ----------------------------------------------------

    /// Adds `entity Name` or `enum Name` with one first line, after the
    /// last data declaration (or after `module` and `use`).
    /// Whether `name` is free for a new declaration of the module's file.
    fn check_new_name(&self, name: &str) -> EditResult<()> {
        if !is_identifier(name) {
            return fail(format!("'{name}' is not a valid name"));
        }
        let taken = self.nodes.iter().any(|n| n.tokens.get(1).is_some_and(|t| t.text == name) && n.tokens.len() > 1);
        if taken {
            return fail(format!("'{name}' is already declared in this file"));
        }
        Ok(())
    }

    fn add_declaration(&mut self, keyword: &str, name: &str, first: &str) -> EditResult<Option<u32>> {
        self.check_new_name(name)?;
        let mut node = self.fresh_node(0, format!("{keyword} {name}"));
        node.leading.push(Trivia {
            indent: 0,
            raw_indent: None,
            text: String::new(),
        });
        let line = self.fresh_node(INDENT_STEP, first.to_string());
        node.children.push(line);
        let id = node.id;
        let after = |kinds: &[Kind]| self.nodes.iter().rposition(|n| kinds.contains(&top_kind(n)));
        let index = after(&[Kind::Entity, Kind::Enum])
            .or_else(|| after(&[Kind::Use]))
            .map(|i| i + 1)
            .unwrap_or(1.min(self.nodes.len()));
        self.nodes.insert(index, node);
        Ok(Some(id))
    }

    fn add_field(&mut self, entity: u32, spec: &FieldSpec) -> EditResult<Option<u32>> {
        let (path, _) = self.find(entity, "an entity", |k| k == Kind::Entity)?;
        let line = validate_field(spec).map_err(EditError)?;
        if self.node(&path).children.iter().any(|c| c.tokens.first().is_some_and(|t| t.text == spec.name)) {
            return fail(format!("the entity already has a field '{}'", spec.name));
        }
        let indent = self.node(&path).child_indent();
        let node = self.fresh_node(indent, line);
        let id = node.id;
        self.node_mut(&path).children.push(node);
        Ok(Some(id))
    }

    fn update_field(&mut self, item: u32, spec: &FieldSpec) -> EditResult<Option<u32>> {
        let (path, _) = self.find(item, "a field", |k| k == Kind::Field)?;
        let line = validate_field(spec).map_err(EditError)?;
        let parent = &path[..path.len() - 1];
        let clash = self
            .node(parent)
            .children
            .iter()
            .any(|c| c.id != item && c.tokens.first().is_some_and(|t| t.text == spec.name));
        if clash {
            return fail(format!("the entity already has a field '{}'", spec.name));
        }
        set_line_keeping_comment(self.node_mut(&path), line);
        Ok(Some(item))
    }

    fn variant_line(name: &str, label: Option<&str>) -> EditResult<String> {
        if !is_identifier(name) {
            return fail(format!("'{name}' is not a valid variant name"));
        }
        Ok(match label.map(str::trim).filter(|l| !l.is_empty()) {
            Some(label) => format!("{name} label {}", text_token(label)),
            None => name.to_string(),
        })
    }

    fn add_variant(&mut self, owner: u32, name: &str, label: Option<&str>) -> EditResult<Option<u32>> {
        let (path, _) = self.find(owner, "an enum", |k| k == Kind::Enum)?;
        let line = Self::variant_line(name, label)?;
        if self.node(&path).children.iter().any(|c| c.tokens.first().is_some_and(|t| t.text == name)) {
            return fail(format!("the enum already has a variant '{name}'"));
        }
        let indent = self.node(&path).child_indent();
        let node = self.fresh_node(indent, line);
        let id = node.id;
        self.node_mut(&path).children.push(node);
        Ok(Some(id))
    }

    fn update_variant(&mut self, item: u32, name: &str, label: Option<&str>) -> EditResult<Option<u32>> {
        let (path, _) = self.find(item, "a variant", |k| k == Kind::Variant)?;
        let line = Self::variant_line(name, label)?;
        set_line_keeping_comment(self.node_mut(&path), line);
        Ok(Some(item))
    }

    fn add_own_setting(&mut self, word: &str) -> EditResult<Option<u32>> {
        if !crate::parser::OWN_SETTINGS.contains(&word) {
            return fail(format!("'{word}' is none of Clarity's own settings: {}", crate::parser::OWN_SETTINGS.join(", ")));
        }
        let at = self.settings_block();
        if let Some(there) = self.nodes[at].children.iter().find(|c| c.tokens.len() == 1 && c.tokens[0].text == word) {
            return Ok(Some(there.id));
        }
        let indent = self.nodes[at].child_indent();
        let node = self.fresh_node(indent, word.to_string());
        let id = node.id;
        self.nodes[at].children.push(node);
        Ok(Some(id))
    }

    /// The index of the file's settings block, started at the end when there is none.
    fn settings_block(&mut self) -> usize {
        match self.nodes.iter().position(|n| n.tokens.len() == 1 && n.tokens[0].text == "settings") {
            Some(at) => at,
            None => {
                let mut node = self.fresh_node(0, "settings".to_string());
                node.leading.push(Trivia { indent: 0, raw_indent: None, text: String::new() });
                self.nodes.push(node);
                self.nodes.len() - 1
            }
        }
    }

    fn add_setting(&mut self, spec: &FieldSpec) -> EditResult<Option<u32>> {
        let line = validate_field(spec).map_err(EditError)?;
        let at = self.settings_block();
        if self.nodes[at].children.iter().any(|c| c.tokens.first().is_some_and(|t| t.text == spec.name)) {
            return fail(format!("the settings already have '{}'", spec.name));
        }
        let indent = self.nodes[at].child_indent();
        let node = self.fresh_node(indent, line);
        let id = node.id;
        self.nodes[at].children.push(node);
        Ok(Some(id))
    }

    fn set_uses(&mut self, uses: &[String]) -> EditResult<Option<u32>> {
        for name in uses {
            if !is_identifier(name) {
                return fail(format!("'{name}' is not a valid module name"));
            }
        }
        // Drop the current `use` lines with the blank line before them; the
        // new block brings its own.
        while let Some(index) = self.nodes.iter().position(|n| top_kind(n) == Kind::Use) {
            self.detach(&[index]);
        }
        let module = self
            .nodes
            .iter()
            .position(|n| starts_with_keyword(&n.tokens, "module", Some(TokenKind::Ident)))
            .map(|i| i + 1)
            .unwrap_or(0);
        for (offset, name) in uses.iter().enumerate() {
            let mut node = self.fresh_node(0, format!("use {name}"));
            if offset == 0 {
                node.leading.push(Trivia {
                    indent: 0,
                    raw_indent: None,
                    text: String::new(),
                });
            }
            self.nodes.insert(module + offset, node);
        }
        Ok(None)
    }
}

/// Replaces a line's content, keeping a trailing comment.
fn set_line_keeping_comment(node: &mut LNode, line: String) {
    let rest_start = node.tokens.last().map(|t| t.span.end as usize).unwrap_or(0);
    let rest = node.text[rest_start..].to_string();
    node.set_text(line + &rest);
}


