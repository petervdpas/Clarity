//! Parses one spec file into the neutral model.
//!
//! The lexer delivers lines; this module first nests them into a block tree
//! by indentation, then interprets each block. Errors are collected per line,
//! so one bad line does not hide problems further down the file.
//!
//! Settings (`title`, `data`, `grid`, ...) are recognized by their shape:
//! the keyword followed by the kind of token they take.

use crate::diagnostic::Diagnostic;
use crate::frame::{self, RegionKind};
use crate::lexer::{Line, Token, TokenKind, lex};
use crate::model::{
    App, Command, CommandKind, Handler, DEFAULT_PAGE_COLUMNS, DataDecl, Entity,
    EnumDecl, EnumVariant, FOOTER_COLUMNS, Field, FieldType, Footer, Frame, Ident,
    Literal, Menu, Modifiers, Orientation, Page, PageKind, Ribbon, RibbonGroup,
    RibbonTab, Side, Sidebar, SpecFile, Surface,
    Toolbar, TreeLevel, TreeSelect, Widget, WidgetPage, WidgetProps, default_label,
    WindowSize,
};
use crate::span::Span;
use crate::widgets::{self, APP_EVENTS, BindTarget, ContainerKind, Event, WINDOW_EVENTS, WidgetKind, WidgetProperty};

mod app;
mod data;
mod parts;
mod windows;

pub use self::{data::*, parts::*};
pub(crate) use self::{app::*, windows::*};

pub(crate) type ParseResult<T> = Result<T, Diagnostic>;

pub fn parse(source: &str, path: &str) -> (SpecFile, Vec<Diagnostic>) {
    let (lines, mut diagnostics) = lex(source);
    let tree = build_tree(&lines, &mut diagnostics);

    let mut file = SpecFile {
        path: path.to_string(),
        version: crate::content_version(source),
        module: None,
        uses: Vec::new(),
        source: None,
        enums: Vec::new(),
        entities: Vec::new(),
        records: Vec::new(),
        pages: Vec::new(),
        apps: Vec::new(),
        themes: Vec::new(),
        code: Vec::new(),
        functions: Vec::new(),
        consts: Vec::new(),
        engines: Vec::new(),
        clis: Vec::new(),
        commands: Vec::new(),
        parts: Vec::new(),
        libraries: Vec::new(),
        mappings: Vec::new(),
        toasts: Vec::new(),
    };

    for (index, node) in tree.iter().enumerate() {
        if let Err(diag) = parse_top_level(node, index, &mut file, &mut diagnostics) {
            diagnostics.push(diag);
        }
    }

    if file.module.is_none() {
        diagnostics.push(Diagnostic::error(
            "a spec file must start with 'module <name>'",
            tree.first().map(|n| n.line.span).unwrap_or_default(),
        ));
    }

    let diagnostics = diagnostics.into_iter().map(|d| d.in_file(path)).collect();
    (file, diagnostics)
}

// ---------------------------------------------------------------------------
// Block tree

#[derive(Clone)]
pub(crate) struct Node<'a> {
    pub(crate) line: &'a Line,
    pub(crate) children: Vec<Node<'a>>,
}

impl Node<'_> {
    /// Span from the start of this line to the end of its last descendant.
    pub(crate) fn span(&self) -> Span {
        let mut span = self.line.span;
        if let Some(last) = self.children.last() {
            span.end = last.span().end;
        }
        span
    }
}

pub(crate) fn build_tree<'a>(lines: &'a [Line], diagnostics: &mut Vec<Diagnostic>) -> Vec<Node<'a>> {
    if let Some(first) = lines.first()
        && first.indent != 0
    {
        diagnostics.push(Diagnostic::error(
            "the first line must not be indented",
            first.span,
        ));
    }
    let mut index = 0;
    let mut nodes = build_block(lines, &mut index, 0, diagnostics);
    // Lines left over can only be indented less than the first line.
    while index < lines.len() {
        let indent = lines[index].indent;
        nodes.extend(build_block(lines, &mut index, indent, diagnostics));
    }
    nodes
}

fn build_block<'a>(
    lines: &'a [Line],
    index: &mut usize,
    indent: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Node<'a>> {
    let mut nodes = Vec::new();
    while *index < lines.len() {
        let line = &lines[*index];
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            // Only reachable after a dedent to a level that was never opened.
            diagnostics.push(Diagnostic::error(
                "indentation does not match any enclosing block",
                line.span,
            ));
        }
        *index += 1;
        let children = match lines.get(*index) {
            Some(next) if next.indent > line.indent => {
                build_block(lines, index, next.indent, diagnostics)
            }
            _ => Vec::new(),
        };
        nodes.push(Node { line, children });
    }
    nodes
}

// ---------------------------------------------------------------------------
// Token cursor

pub(crate) struct Cursor<'a> {
    tokens: &'a [Token],
    pos: usize,
    line_span: Span,
    /// Whether `to` makes a range here: not in what `write ... to` writes.
    pub(crate) ranges: bool,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(line: &'a Line) -> Self {
        Self {
            tokens: &line.tokens,
            pos: 0,
            line_span: line.span,
            ranges: true,
        }
    }

    pub(crate) fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    /// The token `ahead` places after the next one.
    pub(crate) fn peek_at(&self, ahead: usize) -> Option<&'a Token> {
        self.tokens.get(self.pos + ahead)
    }

    pub(crate) fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|t| t.kind)
    }

    pub(crate) fn end_span(&self) -> Span {
        Span {
            start: self.line_span.end,
            end: self.line_span.end,
            line: self.line_span.line,
            col: self.line_span.col + (self.line_span.end - self.line_span.start),
        }
    }

    pub(crate) fn error_here(&self, message: impl Into<String>) -> Diagnostic {
        let span = self
            .peek()
            .map(|t| t.span)
            .unwrap_or_else(|| self.end_span());
        Diagnostic::error(message, span)
    }

    pub(crate) fn next(&mut self) -> Option<&'a Token> {
        let token = self.tokens.get(self.pos);
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    pub(crate) fn eat(&mut self, kind: TokenKind) -> bool {
        if self.peek_kind() == Some(kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    pub(crate) fn eat_keyword(&mut self, keyword: &str) -> bool {
        match self.peek() {
            Some(t) if t.kind == TokenKind::Ident && t.text == keyword => {
                self.pos += 1;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn expect(&mut self, kind: TokenKind, what: &str) -> ParseResult<&'a Token> {
        match self.peek() {
            Some(t) if t.kind == kind => {
                self.pos += 1;
                Ok(t)
            }
            _ => Err(self.error_here(format!("expected {what}"))),
        }
    }

    pub(crate) fn expect_ident(&mut self, what: &str) -> ParseResult<Ident> {
        let token = self.expect(TokenKind::Ident, what)?;
        Ok(Ident {
            name: token.text.clone(),
            span: token.span,
        })
    }

    pub(crate) fn expect_string(&mut self, what: &str) -> ParseResult<String> {
        Ok(self.expect(TokenKind::Str, what)?.text.clone())
    }

    /// A text people read: a key, `@saveCaption` (kept as written, with its
    /// `@`), or a string, which only a text without letters may be.
    pub(crate) fn expect_text(&mut self, what: &str) -> ParseResult<String> {
        if self.peek_kind() == Some(TokenKind::Key) {
            let token = self.expect(TokenKind::Key, what)?;
            return Ok(format!("@{}", token.text));
        }
        self.expect_string(what)
    }

    /// Whether a text, a key or a string, comes next.
    pub(crate) fn peek_is_text(&self) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Str | TokenKind::Key))
    }

    pub(crate) fn expect_number(&mut self, what: &str) -> ParseResult<u32> {
        let token = self.expect(TokenKind::Number, what)?;
        token
            .text
            .parse()
            .map_err(|_| Diagnostic::error(format!("expected {what}"), token.span))
    }

    pub(crate) fn expect_end(&self) -> ParseResult<()> {
        match self.peek() {
            None => Ok(()),
            Some(t) => Err(Diagnostic::error(
                format!("unexpected '{}'", t.text),
                t.span,
            )),
        }
    }

    /// `ident (, ident)*`
    pub(crate) fn ident_list(&mut self, what: &str) -> ParseResult<Vec<Ident>> {
        let mut items = vec![self.expect_ident(what)?];
        while self.eat(TokenKind::Comma) {
            items.push(self.expect_ident(what)?);
        }
        Ok(items)
    }
}

/// The words a line of a module starts with, one per kind of declaration,
/// as the module's parser takes them: what the manual has a page for each of.
pub const DECLARATION_WORDS: [&str; 21] = [
    "module", "use", "source", "enum", "entity", "record", "settings", "page", "app", "function", "action", "engine", "const", "cli", "command", "part", "library", "mapping", "toast", "theme", "code",
];

pub(crate) fn first_word<'a>(node: &Node<'a>) -> Option<&'a str> {
    node.line
        .tokens
        .first()
        .filter(|t| t.kind == TokenKind::Ident)
        .map(|t| t.text.as_str())
}

pub(crate) fn no_children(node: &Node) -> ParseResult<()> {
    match node.children.first() {
        None => Ok(()),
        Some(child) => Err(Diagnostic::error(
            "unexpected indented block",
            child.line.span,
        )),
    }
}

// ---------------------------------------------------------------------------
// Top level

fn parse_top_level(
    node: &Node,
    index: usize,
    file: &mut SpecFile,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<()> {
    let mut cursor = Cursor::new(node.line);
    match first_word(node) {
        Some("module") => {
            cursor.next();
            let name = cursor.expect_ident("a module name")?;
            cursor.expect_end()?;
            no_children(node)?;
            if index != 0 {
                return Err(Diagnostic::error(
                    "'module' must be the first declaration in a file",
                    node.line.span,
                ));
            }
            file.module = Some(name);
        }
        Some("use") => {
            cursor.next();
            let name = cursor.expect_ident("a module name")?;
            cursor.expect_end()?;
            no_children(node)?;
            file.uses.push(name);
        }
        Some("source") => {
            cursor.next();
            let name = cursor.expect_ident("a data source of the project")?;
            cursor.expect_end()?;
            no_children(node)?;
            if file.source.is_some() {
                return Err(Diagnostic::error("a module has one 'source'", node.line.span));
            }
            file.source = Some(name);
        }
        Some("enum") => {
            cursor.next();
            let name = cursor.expect_ident("an enum name")?;
            cursor.expect_end()?;
            file.enums.push(parse_enum(node, name, diagnostics));
        }
        Some("entity") => {
            cursor.next();
            let name = cursor.expect_ident("an entity name")?;
            cursor.expect_end()?;
            file.entities.push(parse_entity(node, name, diagnostics));
        }
        Some("record") => {
            cursor.next();
            let name = cursor.expect_ident("a record name")?;
            cursor.expect_end()?;
            file.records.push(parse_record(node, name, diagnostics));
        }
        Some("settings") => {
            cursor.next();
            cursor.expect_end()?;
            file.entities.push(parse_settings(node, diagnostics));
        }
        Some("page") => {
            cursor.next();
            file.pages.push(parse_page(node, cursor, diagnostics)?);
        }
        Some("app") => {
            cursor.next();
            file.apps.push(parse_app(node, cursor, diagnostics)?);
        }
        Some(word @ ("function" | "action")) => {
            let kind = if word == "function" {
                crate::code::FunctionKind::Function
            } else {
                crate::code::FunctionKind::Action
            };
            file.functions.push(crate::code::parse::parse_function(node, kind, diagnostics)?);
        }
        Some("engine") => {
            file.engines.push(crate::code::parse::parse_engine(node, diagnostics)?);
        }
        Some("const") => {
            file.consts.push(crate::code::parse::parse_const(node)?);
        }
        Some("cli") => {
            file.clis.push(crate::code::parse::parse_cli(node)?);
        }
        Some("command") => {
            file.commands.push(crate::code::parse::parse_command(node, diagnostics)?);
        }
        Some("part") => {
            file.parts.push(crate::code::parse::parse_part(node, diagnostics)?);
        }
        Some("library") => {
            file.libraries.push(crate::code::parse::parse_library(node, diagnostics)?);
        }
        Some("mapping") => {
            file.mappings.push(crate::code::parse::parse_mapping(node, diagnostics)?);
        }
        Some("toast") => {
            file.toasts.push(crate::code::parse::parse_toast(node, diagnostics)?);
        }
        Some("theme") => {
            cursor.next();
            let name = cursor.expect_ident("a theme name")?;
            cursor.expect_end()?;
            file.themes.push(crate::themes::parse_theme(node, name, diagnostics));
        }
        Some("code") => {
            cursor.next();
            let name = cursor.expect_ident("the name of a window or the app")?;
            cursor.expect_end()?;
            file.code.push(crate::code::parse::parse_code(node, name, diagnostics));
        }
        _ => {
            let words: Vec<String> = DECLARATION_WORDS.iter().map(|w| format!("'{w}'")).collect();
            let (last, rest) = words.split_last().expect("there are declaration words");
            return Err(cursor.error_here(format!("expected {} or {last}", rest.join(", "))));
        }
    }
    Ok(())
}

/// The plain types a list field cannot hold.
const VALUE_WORDS: [&str; 12] = ["bool", "money", "datetime", "duration", "image", "blob", "email", "url", "color", "id", "ref", "list"];

// ---------------------------------------------------------------------------
// Keywords

/// True when a line starts with `keyword` followed by a token of kind `next`.
fn is_keyword_line(node: &Node, keyword: &str, next: Option<TokenKind>) -> bool {
    starts_with_keyword(&node.line.tokens, keyword, next)
}

/// True when `tokens` start with `keyword` followed by a token of kind
/// `next` (or by nothing, when `next` is `None`).
pub(crate) fn starts_with_keyword(
    tokens: &[Token],
    keyword: &str,
    next: Option<TokenKind>,
) -> bool {
    let starts_with = tokens
        .first()
        .is_some_and(|t| t.kind == TokenKind::Ident && t.text == keyword);
    // A key stands where a text does: `title @cardTitle`.
    let next_kind = tokens.get(1).map(|t| if t.kind == TokenKind::Key { TokenKind::Str } else { t.kind });
    starts_with && next_kind == next
}
