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
use crate::frame::{self, RegionKind, command_line, new_command, validate_command};
use crate::lexer::{Token, TokenKind, lex, quote, text_token};
use crate::model::{
    Project,
    CommandKind, CommandProps, MAX_PAGE_COLUMNS, PageKind, Side, WidgetProps,
};
use crate::parser::{
    is_app_setting, is_page_setting, is_region_line, parse, starts_with_keyword,
};
use crate::templates::{self, PageTemplate};
use crate::widgets::{
    self, APP_EVENTS, ContainerKind, Event, WINDOW_EVENTS, WidgetKind, is_identifier, new_props, validate_props,
    widget_line,
};

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
    /// Declare data on a page.
    AddData {
        page: u32,
        name: String,
        entity: String,
        list: bool,
    },
    /// Set the number of grid columns of a page.
    SetGrid { page: u32, columns: u32 },
    /// Set the size a window opens at, in pixels; none for the standard size.
    SetSize { page: u32, width: Option<u32>, height: Option<u32> },
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
    /// Replace the top-level declaration that starts at `item` (a window,
    /// the app, an entity, an enum, a function, an action or a code block)
    /// with `text`, which must be one declaration.
    ReplaceDecl { item: u32, text: String },
    /// Add `text`, one declaration, at the end of the file.
    AppendDecl { text: String },
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
        Edit::SetCode { .. } | Edit::AddHandler { .. } | Edit::ReplaceDecl { .. } | Edit::AppendDecl { .. } | Edit::CamelNames
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
            Edit::ReplaceDecl { item, text: decl } => {
                focus = Some(*item);
                crate::source::replace_decl(&text, *item, decl)?
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

fn apply_layout(source: &str, edit: &Edit, project: Option<&Project>) -> EditResult<(String, Option<u32>)> {
    let mut doc = Doc::read(source);
    let focus = match (edit, project) {
        (Edit::AddWindow { name, template }, Some(project)) => doc.add_window(project, name, template)?,
        (Edit::PlaceField { container, col, row, room, bind }, Some(project)) => {
            doc.place_field(project, *container, *col, *row, *room, bind)?
        }
        (Edit::AddWindow { .. } | Edit::PlaceField { .. }, None) => {
            return fail("this edit needs the whole project");
        }
        _ => doc.apply(edit)?,
    };
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

    fn reindent(&mut self, indent: usize) {
        let delta = indent as isize - self.indent as isize;
        self.shift(delta);
    }

    fn shift(&mut self, delta: isize) {
        if delta == 0 {
            return;
        }
        self.indent = self.indent.saturating_add_signed(delta);
        self.raw_indent = None;
        for trivia in &mut self.leading {
            if !trivia.text.is_empty() {
                trivia.indent = trivia.indent.saturating_add_signed(delta);
                trivia.raw_indent = None;
            }
        }
        for child in &mut self.children {
            child.shift(delta);
        }
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
    Use,
    /// A menu item, button or separator.
    Command,
    RibbonTab,
    RibbonGroup,
    FooterStatus,
    Other,
}

impl Kind {
    /// Containers whose children are widgets on a grid.
    fn is_widget_container(self) -> bool {
        matches!(
            self,
            Kind::Page
                | Kind::Widget(Some(ContainerKind::Plain))
                | Kind::WidgetTab
                | Kind::Pane
                | Kind::Region(RegionKind::Sidebar)
                | Kind::Region(RegionKind::Footer)
        )
    }

    /// Elements that hold commands.
    fn is_command_parent(self) -> bool {
        matches!(
            self,
            Kind::Region(RegionKind::Menu)
                | Kind::Region(RegionKind::Toolbar)
                | Kind::Command
                | Kind::RibbonGroup
        )
    }
}

fn widget_kind(node: &LNode) -> Kind {
    match node
        .tokens
        .first()
        .and_then(|t| widgets::by_keyword(&t.text))
    {
        Some(kind) => Kind::Widget(widgets::container(kind)),
        None => Kind::Other,
    }
}

fn child_kind(parent: Kind, node: &LNode) -> Kind {
    match parent {
        Kind::Page if is_page_setting(&node.tokens) => Kind::PageSetting,
        Kind::App if is_app_setting(&node.tokens) => Kind::AppSetting,
        Kind::Entity => Kind::Field,
        Kind::Enum => Kind::Variant,
        Kind::Page | Kind::App if is_region_line(&node.tokens) => region_kind(node),
        Kind::Region(RegionKind::Menu) | Kind::Command
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
        Kind::Widget(Some(ContainerKind::Splitter))
            if starts_with_keyword(&node.tokens, "pane", None) =>
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
    } else if starts_with_keyword(&node.tokens, "enum", Some(TokenKind::Ident)) {
        Kind::Enum
    } else if starts_with_keyword(&node.tokens, "engine", Some(TokenKind::Ident)) {
        Kind::Engine
    } else if starts_with_keyword(&node.tokens, "theme", Some(TokenKind::Ident)) {
        Kind::Theme
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

    fn apply(&mut self, edit: &Edit) -> EditResult<Option<u32>> {
        match edit {
            Edit::Remove { item } => self.remove(*item),
            Edit::SetTitle { target, title } => self.set_title(*target, title),
            Edit::PlaceWidget { container, props } => self.place_widget(*container, props),
            Edit::CreateWidget {
                container,
                kind,
                col,
                row,
                width,
                caption,
                bind,
            } => {
                let mut props = new_props(*kind, *col, *row);
                if let Some(width) = width {
                    props.width = *width;
                }
                if caption.is_some() {
                    props.caption = caption.clone();
                }
                props.bind = bind.clone();
                self.place_widget(*container, &props)
            }
            Edit::Batch { edits } => {
                let mut focus = None;
                for edit in edits {
                    focus = self.apply(edit)?.or(focus);
                }
                Ok(focus)
            }
            Edit::UpdateWidget {
                item,
                container,
                props,
            } => self.update_widget(*item, *container, props),
            Edit::AddTab { item, title } => self.add_tab(*item, title),
            Edit::AddData {
                page,
                name,
                entity,
                list,
            } => self.add_data(*page, name, entity, *list),
            Edit::SetGrid { page, columns } => self.set_grid(*page, *columns),
            Edit::SetSize { page, width, height } => self.set_size(*page, *width, *height),
            Edit::AddPage { name, kind } => self.add_page(name, *kind),
            Edit::AddWindow { .. } | Edit::PlaceField { .. } => {
                fail("this edit needs the whole project and cannot be part of a batch")
            }
            Edit::AddEntity { name } => self.add_declaration("entity", name, "name: text(80)"),
            Edit::AddEnum { name } => self.add_declaration("enum", name, "First"),
            Edit::AddField { entity, spec } => self.add_field(*entity, spec),
            Edit::UpdateField { item, spec } => self.update_field(*item, spec),
            Edit::AddVariant { owner, name, label } => self.add_variant(*owner, name, label.as_deref()),
            Edit::UpdateVariant { item, name, label } => self.update_variant(*item, name, label.as_deref()),
            Edit::SetUses { uses } => self.set_uses(uses),
            Edit::NameParts => self.name_parts(),
            Edit::AddApp { name } => self.add_app(name),
            Edit::SetMain { app, main } => self.set_main(*app, main),
            Edit::SetOwnerEvent { owner, event, handler } => self.set_owner_event(*owner, *event, handler.as_deref()),
            Edit::AddRegion {
                owner,
                region,
                side,
            } => self.add_region(*owner, *region, *side),
            Edit::UpdateSidebar { item, props } => self.update_sidebar(*item, props),
            Edit::UpdateSidebars => self.update_sidebars(),
            Edit::UpdateRibbon { item, name, collapsible, collapsed } => self.update_ribbon(*item, name.as_deref(), *collapsible, *collapsed),
            Edit::SetName { item, name } => self.set_name(*item, name),
            Edit::AddFrameItem {
                parent,
                item,
                caption,
            } => self.add_frame_item(*parent, *item, caption.clone()),
            Edit::UpdateCommand { item, props } => self.update_command(*item, props),
            Edit::MoveSibling { item, delta } => self.move_sibling(*item, *delta),
            Edit::SetCode { .. } | Edit::AddHandler { .. } | Edit::ReplaceDecl { .. } | Edit::AppendDecl { .. } | Edit::CamelNames => {
                fail("text edits cannot be nested in a batch inside a batch")
            }
        }
    }




    fn remove(&mut self, item: u32) -> EditResult<Option<u32>> {
        let (path, _) = self
            .find(
                item,
                "a window, app, entity, field, enum, variant, engine, theme, widget, tab, region, command or data declaration",
                |k| {
                    matches!(
                        k,
                        Kind::Page
                            | Kind::App
                            | Kind::Entity
                            | Kind::Field
                            | Kind::Enum
                            | Kind::Variant
                            | Kind::Engine
                            | Kind::Theme
                            | Kind::Widget(_)
                            | Kind::WidgetTab
                            | Kind::Region(_)
                            | Kind::Command
                            | Kind::RibbonTab
                            | Kind::RibbonGroup
                    )
                },
            )
            .or_else(|err| {
                // `data` lines are page settings; other settings stay.
                match self.path_of(item) {
                    Some(path)
                        if matches!(self.kind(&path), Kind::PageSetting | Kind::AppSetting)
                            && starts_with_keyword(
                                &self.node(&path).tokens,
                                "data",
                                Some(TokenKind::Ident),
                            ) =>
                    {
                        Ok((path, Kind::PageSetting))
                    }
                    _ => Err(err),
                }
            })?;
        self.take(&path);
        Ok(None)
    }



    fn set_title(&mut self, target: u32, title: &str) -> EditResult<Option<u32>> {
        let (path, kind) = self.find(target, "a page, app, tab or status", |k| {
            matches!(
                k,
                Kind::Page
                    | Kind::App
                    | Kind::WidgetTab
                    | Kind::RibbonTab
                    | Kind::RibbonGroup
                    | Kind::FooterStatus
            )
        })?;
        let title = title.trim();

        if matches!(kind, Kind::Page | Kind::App) {
            let existing = self
                .node(&path)
                .children
                .iter()
                .position(|c| starts_with_keyword(&c.tokens, "title", Some(TokenKind::Str)));
            match (existing, title.is_empty()) {
                (Some(index), true) => {
                    self.node_mut(&path).children.remove(index);
                }
                (Some(index), false) => {
                    replace_token(&mut self.node_mut(&path).children[index], 1, &text_token(title));
                }
                (None, true) => {}
                (None, false) => {
                    let indent = self.node(&path).child_indent();
                    let line = self.fresh_node(indent, format!("title {}", text_token(title)));
                    self.node_mut(&path).children.insert(0, line);
                }
            }
            return Ok(Some(target));
        }

        if title.is_empty() {
            return fail("a tab or status needs a title");
        }
        replace_token(self.node_mut(&path), 1, &text_token(title));
        Ok(Some(target))
    }

}

impl Doc {
    // -- pages ------------------------------------------------------------

    fn widget_container(&self, id: u32) -> EditResult<Vec<usize>> {
        let (path, _) = self.find(id, "a page, panel, tab or pane", Kind::is_widget_container)?;
        Ok(path)
    }

    /// Places a field of the window's data with the parts that go with it
    /// (see `templates::field_parts`); the input is named after the field.
    fn place_field(
        &mut self,
        project: &Project,
        container: u32,
        col: u32,
        row: u32,
        room: u32,
        bind: &str,
    ) -> EditResult<Option<u32>> {
        let path = self.widget_container(container)?;
        let Some((data, field_name)) = bind.split_once('.') else {
            return fail(format!("'{bind}' is not a field, use data.field"));
        };
        // The data declaration of the window: `data customer: Customer`.
        let entity = self.nodes[path[0]]
            .children
            .iter()
            .map(|n| &n.tokens)
            .find(|t| t.len() >= 4 && t[0].text == "data" && t[1].text == data)
            .map(|t| t[3].text.clone());
        let Some(entity) = entity else {
            return fail(format!("the window has no data '{data}'"));
        };
        let field = project
            .entity(&entity)
            .and_then(|e| e.field(field_name))
            .ok_or_else(|| EditError(format!("'{entity}' has no field '{field_name}'")))?;
        // The input is named after its field, in camelCase like every part name.
        let base = crate::names::camel_case(field_name);
        let input_name = if self.used_names(path[0]).iter().any(|n| *n == base) {
            self.free_name(path[0], &base)
        } else {
            base
        };
        let parts = templates::field_parts(field, data, &input_name, col, row, room.max(1));
        let mut focus = None;
        for props in &parts {
            let id = self.place_widget(container, props)?;
            focus = focus.or(id);
        }
        Ok(focus)
    }

    fn name_parts(&mut self) -> EditResult<Option<u32>> {
        fn name_in(node: &mut LNode, kind: Kind, used: &mut Vec<String>) {
            for child in &mut node.children {
                let child_kind = child_kind(kind, child);
                let named = child.tokens.windows(2).any(|w| w[0].kind == TokenKind::Ident && w[0].text == "name" && w[1].kind == TokenKind::Ident);
                if matches!(child_kind, Kind::Widget(_)) && !named {
                    let at = child
                        .tokens
                        .windows(2)
                        .find(|w| w[0].kind == TokenKind::Ident && w[0].text == "at" && w[1].kind == TokenKind::Number)
                        .map(|w| w[0].span.start as usize);
                    if let (Some(at), Some(keyword)) = (at, child.tokens.first().map(|t| t.text.clone())) {
                        // After its purpose when the line says it, else numbered.
                        let name = match props_of_line(&child.tokens).as_ref().and_then(crate::names::suggested_name) {
                            Some(base) if !used.contains(&base) => base,
                            Some(base) => (2..).map(|n| format!("{base}{n}")).find(|n| !used.contains(n)).expect("an unused name exists"),
                            None => (1..).map(|n| format!("{keyword}{n}")).find(|n| !used.contains(n)).expect("an unused name exists"),
                        };
                        let text = format!("{}name {name} {}", &child.text[..at], &child.text[at..]);
                        used.push(name);
                        child.set_text(text);
                    }
                }
                // A ribbon is `mainRibbon`, a tab and a group are named after their titles: `homeTab`, `clipboardGroup`.
                let ribbon_base = match child_kind {
                    Kind::Region(RegionKind::Ribbon) => Some("mainRibbon".to_string()),
                    Kind::Region(RegionKind::Menu) => Some("mainMenu".to_string()),
                    Kind::Region(RegionKind::Toolbar) => Some("mainToolbar".to_string()),
                    Kind::RibbonTab | Kind::RibbonGroup => {
                        let title = child.tokens.get(1).filter(|t| t.kind == TokenKind::Str).map(|t| t.text.clone()).unwrap_or_default();
                        Some(ribbon_part_base(&title, if child_kind == Kind::RibbonTab { "Tab" } else { "Group" }))
                    }
                    _ => None,
                };
                if let Some(base) = ribbon_base.filter(|_| !named) {
                    let name = if used.contains(&base) { (2..).map(|n| format!("{base}{n}")).find(|n| !used.contains(n)).expect("an unused name exists") } else { base };
                    let at = child.tokens.last().map(|t| t.span.end as usize).unwrap_or(child.text.len());
                    let text = format!("{} name {name}{}", &child.text[..at], &child.text[at..]);
                    used.push(name);
                    child.set_text(text);
                }
                // A sidebar is named after its side: `leftSidebar`.
                if child_kind == Kind::Region(RegionKind::Sidebar) && !named && child.tokens.len() > 1 {
                    let base = format!("{}Sidebar", child.tokens[1].text);
                    let name = if used.contains(&base) { (2..).map(|n| format!("{base}{n}")).find(|n| !used.contains(n)).expect("an unused name exists") } else { base };
                    let at = child.tokens[1].span.end as usize;
                    let text = format!("{} name {name}{}", &child.text[..at], &child.text[at..]);
                    used.push(name);
                    child.set_text(text);
                }
                // A menu item or button is named after its caption and kind: "New game" is `newGameItem`.
                if child_kind == Kind::Command && !named && child.tokens.first().is_some_and(|t| t.text != "separator") {
                    let caption = child.tokens.get(1).filter(|t| t.kind == TokenKind::Str);
                    let base = crate::names::command_name(caption.map(|t| t.text.as_str()), &child.tokens[0].text);
                    let name = if used.contains(&base) { (2..).map(|n| format!("{base}{n}")).find(|n| !used.contains(n)).expect("an unused name exists") } else { base };
                    let at = caption.unwrap_or(&child.tokens[0]).span.end as usize;
                    let text = format!("{} name {name}{}", &child.text[..at], &child.text[at..]);
                    used.push(name);
                    child.set_text(text);
                }
                name_in(child, child_kind, used);
            }
        }
        for top in 0..self.nodes.len() {
            let kind = top_kind(&self.nodes[top]);
            if !matches!(kind, Kind::Page | Kind::App) {
                continue;
            }
            let mut used = self.used_names(top);
            name_in(&mut self.nodes[top], kind, &mut used);
        }
        Ok(None)
    }

    /// Part names used in the top-level block (window or app) at index `top`.
    fn used_names(&self, top: usize) -> Vec<String> {
        fn collect(node: &LNode, out: &mut Vec<String>) {
            let tokens = &node.tokens;
            for (i, t) in tokens.iter().enumerate() {
                if t.kind == TokenKind::Ident && t.text == "name"
                    && let Some(next) = tokens.get(i + 1)
                    && next.kind == TokenKind::Ident
                {
                    out.push(next.text.clone());
                }
            }
            for child in &node.children {
                collect(child, out);
            }
        }
        let mut used = Vec::new();
        collect(&self.nodes[top], &mut used);
        used
    }

    /// A name after its purpose for a part that still has its numbered name
    /// (`label1`), when it knows its purpose and nothing else in its window
    /// or its code goes by the number.
    fn better_name(&self, top: usize, props: &WidgetProps) -> Option<String> {
        let name = props.name.as_deref()?;
        let kind = widgets::keyword(props.kind);
        let numbered = name.strip_prefix(kind).is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        if !numbered || self.mentions(top, name) {
            return None;
        }
        crate::names::suggested_name(props).map(|base| self.free(top, &base))
    }

    /// Whether anything in the block `top`, or in the code block of the same
    /// name, mentions `name` other than where a part is named it.
    fn mentions(&self, top: usize, name: &str) -> bool {
        fn walk(node: &LNode, name: &str) -> bool {
            let said = node.tokens.iter().enumerate().any(|(i, t)| {
                t.kind == TokenKind::Ident && t.text == name && !(i > 0 && node.tokens[i - 1].text == "name")
            });
            said || node.children.iter().any(|c| walk(c, name))
        }
        let owner = self.nodes[top].tokens.get(1).map(|t| t.text.clone()).unwrap_or_default();
        self.nodes.iter().enumerate().any(|(i, node)| {
            let code = node.tokens.first().is_some_and(|t| t.text == "code") && node.tokens.get(1).is_some_and(|t| t.text == owner);
            (i == top || code) && walk(node, name)
        })
    }

    /// The first `{base}{n}` not yet used as a part name in the top-level
    /// block `top`.
    fn free_name(&self, top: usize, base: &str) -> String {
        let used = self.used_names(top);
        (1..)
            .map(|n| format!("{base}{n}"))
            .find(|name| !used.contains(name))
            .expect("an unused name exists")
    }

    /// Inserts a widget node into a container, keeping widget lines in
    /// reading order (by row, then column).
    fn insert_widget(&mut self, container: &[usize], mut node: LNode, at: (u32, u32)) {
        let target = self.node_mut(container);
        node.reindent(target.child_indent());
        let position = target
            .children
            .iter()
            .position(|c| widget_position(c).is_some_and(|p| p > at))
            .unwrap_or(target.children.len());
        target.children.insert(position, node);
    }

    /// A new widget node, with the child blocks its container kind needs.
    fn widget_node(&mut self, props: &WidgetProps) -> LNode {
        let mut node = self.fresh_node(0, widget_line(props));
        match widgets::container(props.kind) {
            Some(ContainerKind::Tabs) => {
                let tab = self.fresh_node(INDENT_STEP, format!("tab {}", quote("Tab 1")));
                node.children.push(tab);
            }
            Some(ContainerKind::Splitter) => {
                for _ in 0..2 {
                    let pane = self.fresh_node(INDENT_STEP, "pane".to_string());
                    node.children.push(pane);
                }
            }
            _ => {}
        }
        node
    }

    fn place_widget(&mut self, container: u32, props: &WidgetProps) -> EditResult<Option<u32>> {
        validate_props(props).map_err(EditError)?;
        let path = self.widget_container(container)?;
        // A new part gets a name, after what it shows when it says, else like `button3`, so events can be wired.
        let mut props = props.clone();
        if props.name.is_none() {
            props.name = Some(match crate::names::suggested_name(&props) {
                Some(base) => self.free(path[0], &base),
                None => self.free_name(path[0], widgets::keyword(props.kind)),
            });
        }
        let node = self.widget_node(&props);
        let id = node.id;
        self.insert_widget(&path, node, (props.row, props.col));
        Ok(Some(id))
    }

    fn update_widget(
        &mut self,
        item: u32,
        container: Option<u32>,
        props: &WidgetProps,
    ) -> EditResult<Option<u32>> {
        validate_props(props).map_err(EditError)?;
        let (path, _) = self.find(item, "a widget", |k| matches!(k, Kind::Widget(_)))?;
        // Every part has a name: one written without gets one now, like a new part.
        let named;
        let props = if props.name.is_none() {
            let mut with = props.clone();
            with.name = Some(self.free_name(path[0], widgets::keyword(props.kind)));
            named = with;
            &named
        } else if let Some(better) = self.better_name(path[0], props) {
            // A numbered part takes a name after its purpose once it knows it.
            let mut with = props.clone();
            with.name = Some(better);
            named = with;
            &named
        } else {
            props
        };
        let current = self.node(&path).tokens[0].text.clone();
        if current != widgets::keyword(props.kind) {
            return fail(format!(
                "a {current} cannot be turned into a {}",
                widgets::keyword(props.kind)
            ));
        }
        let parent = self.node(&path[..path.len() - 1]).id;
        let target = container.unwrap_or(parent);
        let target_path = self.widget_container(target)?;
        if target_path.starts_with(&path) {
            return fail("a widget cannot be moved into itself");
        }

        let mut node = self.take(&path);
        let rest_start = node.tokens.last().map(|t| t.span.end as usize).unwrap_or(0);
        let rest = node.text[rest_start..].to_string();
        node.set_text(widget_line(props) + &rest);
        // Detaching may shift sibling indexes, so find the container again.
        let target_path = self.widget_container(target)?;
        self.insert_widget(&target_path, node, (props.row, props.col));
        Ok(Some(item))
    }

    fn add_tab(&mut self, item: u32, title: &str) -> EditResult<Option<u32>> {
        let (path, _) = self.find(item, "a tabs widget", |k| {
            k == Kind::Widget(Some(ContainerKind::Tabs))
        })?;
        let title = title.trim();
        if title.is_empty() {
            return fail("a tab needs a title");
        }
        let indent = self.node(&path).child_indent();
        let tab = self.fresh_node(indent, format!("tab {}", text_token(title)));
        let id = tab.id;
        self.node_mut(&path).children.push(tab);
        Ok(Some(id))
    }

    /// Index after the last page setting, where new settings go.
    fn settings_end(&self, page: &[usize]) -> usize {
        let node = self.node(page);
        node.children
            .iter()
            .rposition(|c| child_kind(Kind::Page, c) == Kind::PageSetting)
            .map(|i| i + 1)
            .unwrap_or(0)
    }

    fn add_data(
        &mut self,
        page: u32,
        name: &str,
        entity: &str,
        list: bool,
    ) -> EditResult<Option<u32>> {
        let (path, owner) = self.find(page, "a page or app", |k| {
            matches!(k, Kind::Page | Kind::App)
        })?;
        if !is_identifier(name) {
            return fail(format!("'{name}' is not a valid data name"));
        }
        if !is_identifier(entity) {
            return fail(format!("'{entity}' is not a valid entity name"));
        }
        let text = if list {
            format!("data {name}: list<{entity}>")
        } else {
            format!("data {name}: {entity}")
        };
        let indent = self.node(&path).child_indent();
        let line = self.fresh_node(indent, text);
        let id = line.id;
        let index = self.owner_settings_end(&path, owner);
        self.node_mut(&path).children.insert(index, line);
        Ok(Some(id))
    }

    fn set_grid(&mut self, page: u32, columns: u32) -> EditResult<Option<u32>> {
        let (path, _) = self.find(page, "a page", |k| k == Kind::Page)?;
        if columns == 0 || columns > MAX_PAGE_COLUMNS {
            return fail(format!("a page grid has 1 to {MAX_PAGE_COLUMNS} columns"));
        }
        let text = format!("grid {columns} columns");
        let existing = self
            .node(&path)
            .children
            .iter()
            .position(|c| starts_with_keyword(&c.tokens, "grid", Some(TokenKind::Number)));
        match existing {
            Some(index) => {
                let node = &mut self.node_mut(&path).children[index];
                let rest_start = node.tokens.last().map(|t| t.span.end as usize).unwrap_or(0);
                let rest = node.text[rest_start..].to_string();
                node.set_text(text + &rest);
            }
            None => {
                let indent = self.node(&path).child_indent();
                let line = self.fresh_node(indent, text);
                let index = self.settings_end(&path);
                self.node_mut(&path).children.insert(index, line);
            }
        }
        Ok(Some(page))
    }

    fn set_size(&mut self, page: u32, width: Option<u32>, height: Option<u32>) -> EditResult<Option<u32>> {
        let (path, _) = self.find(page, "a page", |k| k == Kind::Page)?;
        let existing = self.node(&path).children.iter().position(|c| starts_with_keyword(&c.tokens, "size", Some(TokenKind::Number)));
        let Some((width, height)) = width.zip(height) else {
            // The standard size: the line goes.
            if let Some(index) = existing {
                self.node_mut(&path).children.remove(index);
            }
            return Ok(Some(page));
        };
        if !crate::layout::window_size_fits(width, height) {
            return fail(crate::layout::window_size_problem());
        }
        let text = format!("size {width} x {height}");
        match existing {
            Some(index) => self.node_mut(&path).children[index].set_text(text),
            None => {
                let indent = self.node(&path).child_indent();
                let line = self.fresh_node(indent, text);
                let index = self.settings_end(&path);
                self.node_mut(&path).children.insert(index, line);
            }
        }
        Ok(Some(page))
    }

    fn check_new_name(&self, name: &str) -> EditResult<()> {
        if !is_identifier(name) {
            return fail(format!("'{name}' is not a valid window name"));
        }
        let taken = self
            .nodes
            .iter()
            .any(|n| n.tokens.get(1).is_some_and(|t| t.text == name) && n.tokens.len() > 1);
        if taken {
            return fail(format!("'{name}' is already declared in this file"));
        }
        Ok(())
    }

    fn add_window(
        &mut self,
        project: &Project,
        name: &str,
        template: &PageTemplate,
    ) -> EditResult<Option<u32>> {
        if *template == PageTemplate::Blank {
            return self.add_page(name, PageKind::Window);
        }
        self.check_new_name(name)?;
        let text = templates::page_from_template(project, name, template).map_err(EditError)?;
        let code = templates::code_from_template(project, name, template).map_err(EditError)?;
        let mut id = None;
        for text in std::iter::once(text).chain(code) {
            let mut node = self.adopt(&text)?;
            node.leading.push(Trivia {
                indent: 0,
                raw_indent: None,
                text: String::new(),
            });
            // The window is what the designer selects; its code follows it.
            id = id.or(Some(node.id));
            self.nodes.push(node);
        }
        Ok(id)
    }


    /// Reads generated text as one top-level node with fresh ids, so it
    /// cannot be confused with nodes of the file.
    fn adopt(&mut self, text: &str) -> EditResult<LNode> {
        fn renumber(node: &mut LNode, next: &mut u32) {
            node.id = *next;
            *next -= 1;
            for child in &mut node.children {
                renumber(child, next);
            }
        }
        let mut doc = Doc::read(text);
        if doc.nodes.len() != 1 {
            return fail("a template must produce one block");
        }
        let mut node = doc.nodes.remove(0);
        renumber(&mut node, &mut self.next_id);
        Ok(node)
    }

    fn add_page(&mut self, name: &str, kind: PageKind) -> EditResult<Option<u32>> {
        self.check_new_name(name)?;
        let kind = match kind {
            PageKind::Window => "window",
            PageKind::Dialog => "dialog",
            PageKind::View => "view",
        };
        let mut page = self.fresh_node(0, format!("page {name}: {kind}"));
        page.leading.push(Trivia {
            indent: 0,
            raw_indent: None,
            text: String::new(),
        });
        let title = self.fresh_node(
            INDENT_STEP,
            format!("title {}", quote(&crate::model::default_label(name))),
        );
        page.children.push(title);
        let id = page.id;
        self.nodes.push(page);
        Ok(Some(id))
    }

}

impl Doc {
    // -- apps and frame regions -------------------------------------------

    fn add_app(&mut self, name: &str) -> EditResult<Option<u32>> {
        if !is_identifier(name) {
            return fail(format!("'{name}' is not a valid app name"));
        }
        if self.nodes.iter().any(|n| top_kind(n) == Kind::App) {
            return fail("this file already has an app");
        }
        let mut app = self.fresh_node(0, format!("app {name}"));
        app.leading.push(Trivia {
            indent: 0,
            raw_indent: None,
            text: String::new(),
        });
        let title = self.fresh_node(
            INDENT_STEP,
            format!("title {}", quote(&crate::model::default_label(name))),
        );
        app.children.push(title);
        let id = app.id;
        self.nodes.push(app);
        Ok(Some(id))
    }

    fn set_main(&mut self, app: u32, main: &str) -> EditResult<Option<u32>> {
        let (path, _) = self.find(app, "an app", |k| k == Kind::App)?;
        let main = main.trim();
        if !main.is_empty() && !is_identifier(main) {
            return fail(format!("'{main}' is not a valid window name"));
        }
        let existing = self
            .node(&path)
            .children
            .iter()
            .position(|c| starts_with_keyword(&c.tokens, "main", Some(TokenKind::Ident)));
        match (existing, main.is_empty()) {
            (Some(index), true) => {
                self.node_mut(&path).children.remove(index);
            }
            (Some(index), false) => {
                replace_token(&mut self.node_mut(&path).children[index], 1, main)
            }
            (None, true) => {}
            (None, false) => {
                let indent = self.node(&path).child_indent();
                let line = self.fresh_node(indent, format!("main {main}"));
                let index = self.owner_settings_end(&path, Kind::App);
                self.node_mut(&path).children.insert(index, line);
            }
        }
        Ok(Some(app))
    }

    fn set_owner_event(&mut self, owner: u32, event: Event, handler: Option<&str>) -> EditResult<Option<u32>> {
        let (path, kind) = self.find(owner, "a window or the app", |k| matches!(k, Kind::Page | Kind::App))?;
        let allowed: &[Event] = if kind == Kind::App { &APP_EVENTS } else { &WINDOW_EVENTS };
        if !allowed.contains(&event) {
            return fail(if kind == Kind::App {
                format!("the app has the event start, not {}", event.keyword())
            } else {
                format!("a window has the events open, close and return, not {}", event.keyword())
            });
        }
        let handler = handler.map(str::trim).filter(|h| !h.is_empty());
        if let Some(h) = handler
            && !is_identifier(h)
        {
            return fail(format!("'{h}' is not a valid handler name"));
        }
        let existing = self.node(&path).children.iter().position(|c| {
            starts_with_keyword(&c.tokens, "on", Some(TokenKind::Ident))
                && c.tokens.get(1).is_some_and(|t| t.text == event.keyword())
        });
        let text = handler.map(|h| format!("on {}: {h}", event.keyword()));
        match (existing, text) {
            (Some(index), None) => {
                self.node_mut(&path).children.remove(index);
            }
            (Some(index), Some(_)) => {
                if let Some(h) = handler {
                    replace_token(&mut self.node_mut(&path).children[index], 3, h);
                }
            }
            (None, None) => {}
            (None, Some(text)) => {
                let indent = self.node(&path).child_indent();
                let line = self.fresh_node(indent, text);
                let index = self.owner_settings_end(&path, kind);
                self.node_mut(&path).children.insert(index, line);
            }
        }
        Ok(Some(owner))
    }

    /// Index after the settings of a page or app.
    fn owner_settings_end(&self, path: &[usize], owner: Kind) -> usize {
        self.node(path)
            .children
            .iter()
            .rposition(|c| matches!(child_kind(owner, c), Kind::PageSetting | Kind::AppSetting))
            .map(|i| i + 1)
            .unwrap_or(0)
    }

    fn region_line(&self, top: usize, region: RegionKind, side: Option<Side>) -> String {
        match region {
            RegionKind::Sidebar => {
                let side = side.unwrap_or(Side::Left);
                frame::sidebar_line(&frame::new_sidebar(side, Some(self.sidebar_name(top, side))))
            }
            RegionKind::Ribbon => format!("ribbon name {}", self.free(top, "mainRibbon")),
            RegionKind::Menu => format!("menu name {}", self.free(top, "mainMenu")),
            RegionKind::Toolbar => format!("toolbar name {}", self.free(top, "mainToolbar")),
            other => frame::keyword(other).to_string(),
        }
    }

    /// `base`, or `base2`, `base3`... whichever no part of the block `top` has.
    fn free(&self, top: usize, base: &str) -> String {
        let used = self.used_names(top);
        if used.iter().any(|u| u == base) { (2..).map(|n| format!("{base}{n}")).find(|n| !used.contains(n)).expect("an unused name exists") } else { base.to_string() }
    }

    /// The first free name for a sidebar on `side`: `leftSidebar`.
    fn sidebar_name(&self, top: usize, side: Side) -> String {
        let base = format!("{}Sidebar", frame::side_keyword(side));
        let used = self.used_names(top);
        if used.contains(&base) { (2..).map(|n| format!("{base}{n}")).find(|n| !used.contains(n)).expect("an unused name exists") } else { base }
    }

    fn add_region(
        &mut self,
        owner: u32,
        region: RegionKind,
        side: Option<Side>,
    ) -> EditResult<Option<u32>> {
        let (path, owner_kind) = self.find(owner, "a page or app", |k| {
            matches!(k, Kind::Page | Kind::App)
        })?;
        if region != RegionKind::Sidebar && side.is_some() {
            return fail("only a sidebar has a side");
        }
        let side = (region == RegionKind::Sidebar).then(|| side.unwrap_or(Side::Left));
        let taken = self.node(&path).children.iter().any(|c| {
            child_kind(owner_kind, c) == Kind::Region(region)
                && (side.is_none() || sidebar_side(c) == side)
        });
        if taken {
            let what = match side {
                Some(s) => format!("a {} sidebar", frame::side_keyword(s)),
                None => format!("a {}", frame::keyword(region)),
            };
            return fail(format!("this window already has {what}"));
        }

        let indent = self.node(&path).child_indent();
        let line = self.region_line(path[0], region, side);
        let mut node = self.fresh_node(indent, line);
        let inner = indent + INDENT_STEP;
        // Starter content, so the new region shows up and is ready to edit.
        match region {
            RegionKind::Menu => {
                let mut file = self.fresh_node(
                    inner,
                    command_line(&new_command(CommandKind::Item, Some("File".into()))),
                );
                let mut quit = new_command(CommandKind::Item, Some("Quit".into()));
                quit.on_click = Some("quit".into());
                let quit = self.fresh_node(inner + INDENT_STEP, command_line(&quit));
                file.children.push(quit);
                node.children.push(file);
            }
            RegionKind::Toolbar => {
                let button = self.fresh_node(
                    inner,
                    command_line(&new_command(CommandKind::Button, Some("Button".into()))),
                );
                node.children.push(button);
            }
            RegionKind::Ribbon => {
                let tab_name = self.free(path[0], "homeTab");
                let group_name = self.free(path[0], "actionsGroup");
                let mut tab = self.fresh_node(inner, format!("tab {} name {tab_name}", quote("Home")));
                let mut group = self.fresh_node(inner + INDENT_STEP, format!("group {} name {group_name}", quote("Actions")));
                let mut props = new_command(CommandKind::Button, Some("Button".into()));
                props.name = Some(self.command_name(path[0], &props));
                let button = self.fresh_node(inner + 2 * INDENT_STEP, command_line(&props));
                group.children.push(button);
                tab.children.push(group);
                node.children.push(tab);
            }
            RegionKind::Footer => {
                let status = self.fresh_node(inner, format!("status {}", quote("Ready")));
                node.children.push(status);
            }
            RegionKind::Sidebar => {}
        }
        let id = node.id;

        // Regions follow the settings, in catalog order, before the widgets.
        let order = frame::order(region);
        let children = &self.node(&path).children;
        let index = children
            .iter()
            .rposition(|c| match child_kind(owner_kind, c) {
                Kind::PageSetting | Kind::AppSetting => true,
                Kind::Region(r) => frame::order(r) <= order,
                _ => false,
            })
            .map(|i| i + 1)
            .unwrap_or(0);
        self.node_mut(&path).children.insert(index, node);
        Ok(Some(id))
    }

    fn update_sidebar(&mut self, item: u32, props: &crate::model::SidebarProps) -> EditResult<Option<u32>> {
        let (path, _) = self.find(item, "a sidebar", |k| k == Kind::Region(RegionKind::Sidebar))?;
        let parent = &path[..path.len() - 1];
        let clash = self.node(parent).children.iter().any(|c| {
            c.id != item && starts_with_keyword(&c.tokens, "sidebar", Some(TokenKind::Ident)) && sidebar_side(c) == Some(props.side)
        });
        if clash {
            return fail(format!("there is already a {} sidebar", frame::side_keyword(props.side)));
        }
        let mut props = props.clone();
        if props.name.is_none() {
            props.name = Some(self.sidebar_name(path[0], props.side));
        }
        let node = self.node_mut(&path);
        let rest_start = node.tokens.last().map(|t| t.span.end as usize).unwrap_or(0);
        let rest = node.text[rest_start..].to_string();
        node.set_text(frame::sidebar_line(&props) + &rest);
        Ok(Some(item))
    }

    fn update_ribbon(&mut self, item: u32, name: Option<&str>, collapsible: bool, collapsed: bool) -> EditResult<Option<u32>> {
        let (path, _) = self.find(item, "a ribbon", |k| k == Kind::Region(RegionKind::Ribbon))?;
        let name = match name {
            Some(n) => n.to_string(),
            None => self.free(path[0], "mainRibbon"),
        };
        let mut line = format!("ribbon name {name}");
        if collapsible {
            line.push_str(if collapsed { " collapsible collapsed" } else { " collapsible" });
        }
        let node = self.node_mut(&path);
        let rest_start = node.tokens.last().map(|t| t.span.end as usize).unwrap_or(0);
        let rest = node.text[rest_start..].to_string();
        node.set_text(line + &rest);
        Ok(Some(item))
    }

    fn set_name(&mut self, item: u32, name: &str) -> EditResult<Option<u32>> {
        if !crate::names::is_camel_case(name) {
            return fail(crate::names::camel_case_problem("part", name));
        }
        let (path, _) = self.find(item, "a menu, a toolbar, or a ribbon tab or group", |k| {
            matches!(k, Kind::RibbonTab | Kind::RibbonGroup | Kind::Region(RegionKind::Menu) | Kind::Region(RegionKind::Toolbar))
        })?;
        let node = self.node_mut(&path);
        let at = node.tokens.windows(2).position(|w| w[0].text == "name" && w[1].kind == TokenKind::Ident);
        let text = match at {
            Some(i) => {
                let t = &node.tokens[i + 1];
                format!("{}{name}{}", &node.text[..t.span.start as usize], &node.text[t.span.end as usize..])
            }
            None => {
                let end = node.tokens.last().map(|t| t.span.end as usize).unwrap_or(node.text.len());
                format!("{} name {name}{}", &node.text[..end], &node.text[end..])
            }
        };
        node.set_text(text);
        Ok(Some(item))
    }

    fn update_sidebars(&mut self) -> EditResult<Option<u32>> {
        for top in 0..self.nodes.len() {
            if !matches!(top_kind(&self.nodes[top]), Kind::Page | Kind::App) {
                continue;
            }
            let mut used = self.used_names(top);
            for child in &mut self.nodes[top].children {
                if !starts_with_keyword(&child.tokens, "sidebar", Some(TokenKind::Ident)) {
                    continue;
                }
                let Some(mut props) = sidebar_props_of(&child.tokens) else { continue };
                if props.name.is_none() {
                    let base = format!("{}Sidebar", frame::side_keyword(props.side));
                    let name = if used.contains(&base) { (2..).map(|n| format!("{base}{n}")).find(|n| !used.contains(n)).expect("an unused name exists") } else { base };
                    used.push(name.clone());
                    props.name = Some(name);
                }
                let rest_start = child.tokens.last().map(|t| t.span.end as usize).unwrap_or(0);
                let rest = child.text[rest_start..].to_string();
                child.set_text(frame::sidebar_line(&props) + &rest);
            }
        }
        Ok(None)
    }


    fn add_frame_item(
        &mut self,
        parent: u32,
        item: FrameItem,
        caption: Option<String>,
    ) -> EditResult<Option<u32>> {
        let (path, kind) = self.find(parent, "a menu, toolbar, ribbon or part of one", |k| {
            k.is_command_parent() || matches!(k, Kind::Region(RegionKind::Ribbon) | Kind::RibbonTab)
        })?;
        let caption = caption
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty());
        let text = match (kind, item) {
            (
                Kind::Region(RegionKind::Menu) | Kind::Command,
                FrameItem::Item | FrameItem::Separator,
            )
            | (
                Kind::Region(RegionKind::Toolbar) | Kind::RibbonGroup,
                FrameItem::Button | FrameItem::Separator,
            ) => {
                let command = match item {
                    FrameItem::Item => CommandKind::Item,
                    FrameItem::Button => CommandKind::Button,
                    _ => CommandKind::Separator,
                };
                let caption = match command {
                    CommandKind::Separator => None,
                    CommandKind::Item => caption.or(Some("Item".into())),
                    CommandKind::Button => caption.or(Some("Button".into())),
                };
                let mut props = new_command(command, caption);
                // A new item or button is named after its caption, like a new part.
                if command != CommandKind::Separator {
                    props.name = Some(self.command_name(path[0], &props));
                }
                validate_command(&props).map_err(EditError)?;
                command_line(&props)
            }
            (Kind::Region(RegionKind::Ribbon), FrameItem::Tab) => {
                let title = caption.as_deref().unwrap_or("Tab");
                format!("tab {} name {}", text_token(title), self.free(path[0], &ribbon_part_base(title, "Tab")))
            }
            (Kind::RibbonTab, FrameItem::Group) => {
                let title = caption.as_deref().unwrap_or("Group");
                format!("group {} name {}", text_token(title), self.free(path[0], &ribbon_part_base(title, "Group")))
            }
            _ => return fail("that cannot go there"),
        };
        if kind == Kind::Command && !command_keyword_is(self.node(&path), &[CommandKind::Item]) {
            return fail("only a menu item can hold a submenu");
        }
        let indent = self.node(&path).child_indent();
        let node = self.fresh_node(indent, text);
        let id = node.id;
        self.node_mut(&path).children.push(node);
        Ok(Some(id))
    }

    /// The first free name for a command after its caption and kind: "New game" is `newGameItem`.
    fn command_name(&self, top: usize, props: &CommandProps) -> String {
        let base = crate::names::command_name(props.caption.as_deref(), frame::command_keyword(props.kind));
        let used = self.used_names(top);
        if used.contains(&base) { (2..).map(|n| format!("{base}{n}")).find(|n| !used.contains(n)).expect("an unused name exists") } else { base }
    }

    fn update_command(&mut self, item: u32, props: &CommandProps) -> EditResult<Option<u32>> {
        validate_command(props).map_err(EditError)?;
        let (path, _) = self.find(item, "a menu item, button or separator", |k| {
            k == Kind::Command
        })?;
        // Every item and button has a name: one written without gets one now.
        let named;
        let props = if props.name.is_none() && props.kind != CommandKind::Separator {
            let mut with = props.clone();
            with.name = Some(self.command_name(path[0], props));
            named = with;
            &named
        } else {
            props
        };
        let node = self.node_mut(&path);
        let current = node.tokens[0].text.clone();
        if current != frame::command_keyword(props.kind) {
            return fail(format!(
                "a {current} cannot be turned into a {}",
                frame::command_keyword(props.kind)
            ));
        }
        let rest_start = node.tokens.last().map(|t| t.span.end as usize).unwrap_or(0);
        let rest = node.text[rest_start..].to_string();
        node.set_text(command_line(props) + &rest);
        Ok(Some(item))
    }

    fn move_sibling(&mut self, item: u32, delta: i32) -> EditResult<Option<u32>> {
        let (path, kind) =
            self.find(item, "a field, variant, command, ribbon tab, ribbon group or region", |k| {
                matches!(
                    k,
                    Kind::Field
                        | Kind::Variant
                        | Kind::Command
                        | Kind::RibbonTab
                        | Kind::RibbonGroup
                        | Kind::Region(_)
                )
            })?;
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

/// A ribbon tab's or group's name after its title and kind: `homeTab`, `clipboardGroup`.
fn ribbon_part_base(title: &str, kind: &str) -> String {
    let title = crate::names::camel_case(title);
    if title.is_empty() { kind.to_lowercase() } else { format!("{title}{kind}") }
}

/// What a sidebar's line says, read from its tokens, the old `span` form
/// (columns, now pixels) included; none when it is no sidebar line.
fn sidebar_props_of(tokens: &[crate::lexer::Token]) -> Option<crate::model::SidebarProps> {
    let side = match tokens.get(1).map(|t| t.text.as_str()) {
        Some("left") => Side::Left,
        Some("right") => Side::Right,
        _ => return None,
    };
    let mut props = frame::new_sidebar(side, None);
    let number = |i: usize| tokens.get(i).and_then(|t| t.text.parse::<u32>().ok());
    let mut i = 2;
    while i < tokens.len() {
        match tokens[i].text.as_str() {
            "name" => props.name = tokens.get(i + 1).map(|t| t.text.clone()),
            "width" => props.width = number(i + 1).unwrap_or(props.width),
            "min" => props.min = number(i + 1),
            "max" => props.max = number(i + 1),
            "grid" => props.columns = number(i + 1).unwrap_or(props.columns),
            "span" => {
                if let Some(columns) = number(i + 1) {
                    props.columns = columns;
                    props.width = columns * crate::layout::SIDEBAR_COLUMN;
                }
            }
            "resizable" => props.resizable = true,
            "collapsible" => props.collapsible = true,
            "collapsed" => props.collapsed = true,
            "maximizable" => props.maximizable = true,
            _ => {}
        }
        i += 1;
    }
    Some(props)
}

fn sidebar_side(node: &LNode) -> Option<Side> {
    match node.tokens.get(1).map(|t| t.text.as_str()) {
        Some("left") => Some(Side::Left),
        Some("right") => Some(Side::Right),
        _ => None,
    }
}

/// The `at column,row` of a widget line, as (row, column) for ordering.
fn widget_position(node: &LNode) -> Option<(u32, u32)> {
    let tokens = &node.tokens;
    let at = tokens
        .iter()
        .position(|t| t.kind == TokenKind::Ident && t.text == "at")?;
    let col = tokens.get(at + 1)?.text.parse().ok()?;
    let row = tokens.get(at + 3)?.text.parse().ok()?;
    Some((row, col))
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

fn replace_token(node: &mut LNode, index: usize, replacement: &str) {
    let Some(token) = node.tokens.get(index) else {
        return;
    };
    let (start, end) = (token.span.start as usize, token.span.end as usize);
    let text = format!("{}{replacement}{}", &node.text[..start], &node.text[end..]);
    node.set_text(text);
}

impl Doc {
    // -- data dictionary ----------------------------------------------------

    /// Adds `entity Name` or `enum Name` with one first line, after the
    /// last data declaration (or after `module` and `use`).
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

/// What a part's line says of its purpose, enough to name it: its kind,
/// caption, `for`, `bind` and `source`.
fn props_of_line(tokens: &[Token]) -> Option<WidgetProps> {
    let kind = widgets::by_keyword(&tokens.first()?.text)?;
    let mut props = widgets::new_props(kind, 1, 1);
    props.caption = match tokens.get(1) {
        Some(t) if t.kind == TokenKind::Str => Some(t.text.clone()),
        Some(t) if t.kind == TokenKind::Key => Some(format!("@{}", t.text.trim_start_matches('@'))),
        _ => None,
    };
    let after = |word: &str| tokens.iter().position(|t| t.kind == TokenKind::Ident && t.text == word).map(|i| i + 1);
    props.for_part = after("for").and_then(|i| tokens.get(i)).filter(|t| t.kind == TokenKind::Ident).map(|t| t.text.clone());
    props.source = after("source").and_then(|i| tokens.get(i)).filter(|t| t.kind == TokenKind::Str).map(|t| t.text.clone());
    props.bind = after("bind").map(|i| {
        tokens[i..].iter().take_while(|t| t.kind == TokenKind::Ident || t.kind == TokenKind::Dot).map(|t| t.text.as_str()).collect::<String>()
    }).filter(|b| !b.is_empty());
    Some(props)
}
