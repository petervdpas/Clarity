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
    Toolbar, Widget, WidgetPage, WidgetProps, default_label,
    WindowSize,
};
use crate::span::Span;
use crate::widgets::{self, APP_EVENTS, BindTarget, ContainerKind, Event, WINDOW_EVENTS, WidgetKind, WidgetProperty};

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
        pages: Vec::new(),
        apps: Vec::new(),
        themes: Vec::new(),
        code: Vec::new(),
        functions: Vec::new(),
        engines: Vec::new(),
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
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(line: &'a Line) -> Self {
        Self {
            tokens: &line.tokens,
            pos: 0,
            line_span: line.span,
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
            return Err(cursor.error_here(
                "expected 'module', 'use', 'source', 'enum', 'entity', 'page', 'app', 'theme', 'code', 'function', 'action' or 'engine'",
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Enums and entities

fn parse_enum(node: &Node, name: Ident, diagnostics: &mut Vec<Diagnostic>) -> EnumDecl {
    let mut variants = Vec::new();
    for child in &node.children {
        match parse_variant(child) {
            Ok(variant) => variants.push(variant),
            Err(diag) => diagnostics.push(diag),
        }
    }
    if variants.is_empty() && node.children.is_empty() {
        diagnostics.push(Diagnostic::error(
            format!("enum '{}' has no variants", name.name),
            name.span,
        ));
    }
    EnumDecl {
        name,
        variants,
        span: node.span(),
    }
}

fn parse_variant(node: &Node) -> ParseResult<EnumVariant> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    let name = cursor.expect_ident("a variant name")?;
    let label = if cursor.eat_keyword("label") {
        cursor.expect_text("a label string")?
    } else {
        default_label(&name.name)
    };
    cursor.expect_end()?;
    Ok(EnumVariant { name, label })
}

fn parse_entity(node: &Node, name: Ident, diagnostics: &mut Vec<Diagnostic>) -> Entity {
    let mut fields = Vec::new();
    let (mut table, mut key) = (None, None);
    for child in &node.children {
        // `table "tblInvoice"` and `key "InvoiceID"` map the entity onto an
        // existing table; field lines have a colon after the name.
        let setting = match first_word(child) {
            Some(word @ ("table" | "key")) if child.line.tokens.get(1).is_some_and(|t| t.kind == TokenKind::Str) => Some(word),
            _ => None,
        };
        if let Some(word) = setting {
            let mut cursor = Cursor::new(child.line);
            cursor.next();
            let result = cursor.expect_string("a name").and_then(|value| {
                cursor.expect_end()?;
                no_children(child)?;
                let slot = if word == "table" { &mut table } else { &mut key };
                if slot.is_some() {
                    return Err(Diagnostic::error(format!("duplicate '{word}'"), child.line.span));
                }
                *slot = Some(value);
                Ok(())
            });
            if let Err(diag) = result {
                diagnostics.push(diag);
            }
            continue;
        }
        match parse_field(child) {
            Ok(field) => fields.push(field),
            Err(diag) => diagnostics.push(diag),
        }
    }
    if node.children.is_empty() {
        diagnostics.push(Diagnostic::error(
            format!("entity '{}' has no fields", name.name),
            name.span,
        ));
    }
    Entity {
        name,
        fields,
        table,
        key,
        span: node.span(),
    }
}

fn parse_field(node: &Node) -> ParseResult<Field> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    parse_field_at(&mut cursor, node.line.span)
}

/// `name: type, modifiers` from the cursor to the end of the line.
pub(crate) fn parse_field_at(cursor: &mut Cursor, span: Span) -> ParseResult<Field> {
    let name = cursor.expect_ident("a field name")?;
    cursor.expect(TokenKind::Colon, "':' after the field name")?;
    let ty = parse_type(cursor)?;
    let mut modifiers = Modifiers::default();
    while cursor.eat(TokenKind::Comma) {
        parse_modifier(cursor, &mut modifiers)?;
    }
    cursor.expect_end()?;
    let label = modifiers
        .label
        .clone()
        .unwrap_or_else(|| default_label(&name.name));
    Ok(Field {
        name,
        type_text: crate::dictionary::type_text(&ty),
        ty,
        modifiers,
        label,
        span,
    })
}

pub(crate) fn parse_type(cursor: &mut Cursor) -> ParseResult<FieldType> {
    let ident = cursor.expect_ident("a type")?;
    let ty = match ident.name.as_str() {
        "text" => {
            let max_length = if cursor.eat(TokenKind::LParen) {
                let n = cursor.expect_number("a maximum length")?;
                cursor.expect(TokenKind::RParen, "')'")?;
                Some(n)
            } else {
                None
            };
            FieldType::Text { max_length }
        }
        "decimal" => {
            cursor.expect(
                TokenKind::LParen,
                "'(' with precision and scale, as in decimal(10, 2)",
            )?;
            let precision = cursor.expect_number("a precision")?;
            cursor.expect(TokenKind::Comma, "',' and a scale")?;
            let scale = cursor.expect_number("a scale")?;
            cursor.expect(TokenKind::RParen, "')'")?;
            FieldType::Decimal { precision, scale }
        }
        "integer" => FieldType::Integer,
        "money" => FieldType::Money,
        "bool" => FieldType::Bool,
        "date" => FieldType::Date,
        "datetime" => FieldType::DateTime,
        "duration" => FieldType::Duration,
        "image" => FieldType::Image,
        "email" => FieldType::Email,
        "url" => FieldType::Url,
        "id" => FieldType::Id,
        "ref" => FieldType::Ref {
            entity: cursor.expect_ident("an entity name after 'ref'")?,
        },
        "list" => {
            cursor.expect(TokenKind::Lt, "'<' after 'list'")?;
            let item = cursor.expect_ident("an entity name")?;
            cursor.expect(TokenKind::Gt, "'>'")?;
            FieldType::List { item }
        }
        _ => FieldType::Enum { name: ident },
    };
    Ok(ty)
}

fn parse_modifier(cursor: &mut Cursor, modifiers: &mut Modifiers) -> ParseResult<()> {
    let keyword = cursor.expect_ident("a modifier")?;
    let duplicate = || {
        Diagnostic::error(
            format!("duplicate modifier '{}'", keyword.name),
            keyword.span,
        )
    };

    fn set<T>(
        slot: &mut Option<T>,
        value: T,
        duplicate: impl Fn() -> Diagnostic,
    ) -> ParseResult<()> {
        if slot.is_some() {
            return Err(duplicate());
        }
        *slot = Some(value);
        Ok(())
    }

    fn flag(slot: &mut bool, duplicate: impl Fn() -> Diagnostic) -> ParseResult<()> {
        if *slot {
            return Err(duplicate());
        }
        *slot = true;
        Ok(())
    }

    match keyword.name.as_str() {
        "required" => flag(&mut modifiers.required, duplicate),
        "unique" => flag(&mut modifiers.unique, duplicate),
        "owned" => flag(&mut modifiers.owned, duplicate),
        "computed" => flag(&mut modifiers.computed, duplicate),
        "min" => set(&mut modifiers.min, parse_literal(cursor)?, duplicate),
        "max" => set(&mut modifiers.max, parse_literal(cursor)?, duplicate),
        "default" => set(&mut modifiers.default, parse_literal(cursor)?, duplicate),
        "pattern" => set(
            &mut modifiers.pattern,
            cursor.expect_string("a pattern string")?,
            duplicate,
        ),
        "label" => set(
            &mut modifiers.label,
            cursor.expect_text("a label string")?,
            duplicate,
        ),
        "help" => set(
            &mut modifiers.help,
            cursor.expect_text("a help string")?,
            duplicate,
        ),
        "format" => set(
            &mut modifiers.format,
            cursor.expect_ident("a format name, such as short or plain")?.name,
            duplicate,
        ),
        "column" => set(
            &mut modifiers.column,
            cursor.expect_string("a column name")?,
            duplicate,
        ),
        "store" => set(
            &mut modifiers.store,
            cursor.expect_ident("a file store, as in 'store photos'")?.name,
            duplicate,
        ),
        other => Err(Diagnostic::error(
            format!("unknown modifier '{other}'"),
            keyword.span,
        )),
    }
}

fn parse_literal(cursor: &mut Cursor) -> ParseResult<Literal> {
    let Some(token) = cursor.peek() else {
        return Err(cursor.error_here("expected a value"));
    };
    let literal = match token.kind {
        TokenKind::Number => Literal::Number(token.text.clone()),
        TokenKind::Str => Literal::Str(token.text.clone()),
        TokenKind::Key => Literal::Str(format!("@{}", token.text)),
        TokenKind::Ident if token.text == "true" => Literal::Bool(true),
        TokenKind::Ident if token.text == "false" => Literal::Bool(false),
        TokenKind::Ident => Literal::Ident(token.text.clone()),
        _ => return Err(cursor.error_here("expected a value")),
    };
    cursor.next();
    Ok(literal)
}

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

// ---------------------------------------------------------------------------
// Pages

/// Page settings (`title`, `size`, `data`, `grid`) as opposed to widget lines.
pub(crate) fn is_page_setting(tokens: &[Token]) -> bool {
    starts_with_keyword(tokens, "title", Some(TokenKind::Str))
        || starts_with_keyword(tokens, "size", Some(TokenKind::Number))
        || starts_with_keyword(tokens, "data", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "grid", Some(TokenKind::Number))
        || starts_with_keyword(tokens, "on", Some(TokenKind::Ident))
}

fn parse_page(
    node: &Node,
    mut cursor: Cursor,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<Page> {
    let name = cursor.expect_ident("a page name")?;
    cursor.expect(
        TokenKind::Colon,
        "':' and a page kind, as in 'page X: window'",
    )?;
    let kind_ident = cursor.expect_ident("a page kind ('window', 'dialog' or 'view')")?;
    let kind = match kind_ident.name.as_str() {
        "window" => PageKind::Window,
        "dialog" => PageKind::Dialog,
        "view" => PageKind::View,
        other => {
            return Err(Diagnostic::error(
                format!("unknown page kind '{other}', expected 'window', 'dialog' or 'view'"),
                kind_ident.span,
            ));
        }
    };
    cursor.expect_end()?;

    let mut page = Page {
        name,
        kind,
        title: None,
        size: None,
        data: Vec::new(),
        columns: DEFAULT_PAGE_COLUMNS,
        widgets: Vec::new(),
        frame: Frame::default(),
        handlers: Vec::new(),
        span: node.span(),
    };
    let mut grid_seen = false;

    for child in &node.children {
        let result = if is_page_setting(&child.line.tokens) {
            parse_page_setting(child, &mut page, &mut grid_seen)
        } else if is_region_line(&child.line.tokens) {
            parse_region(child, &mut page.frame, diagnostics)
        } else {
            parse_widget(child, diagnostics).map(|w| page.widgets.push(w))
        };
        if let Err(diag) = result {
            diagnostics.push(diag);
        }
    }
    Ok(page)
}

/// `on open: window_opened`: one of the owner's own events and its handler.
fn parse_owner_event(
    cursor: &mut Cursor,
    handlers: &mut Vec<Handler>,
    allowed: &[Event],
    refused: impl Fn(&str) -> String,
    owner: &str,
) -> ParseResult<()> {
    let word = cursor.expect_ident("an event")?;
    let event = Event::by_keyword(&word.name)
        .filter(|e| allowed.contains(e))
        .ok_or_else(|| Diagnostic::error(refused(&word.name), word.span))?;
    cursor.expect(TokenKind::Colon, "':' and a handler, as in 'on open: window_opened'")?;
    let handler = cursor.expect_ident("a handler name")?;
    if handlers.iter().any(|h| h.event == event) {
        return Err(Diagnostic::error(
            format!("{owner} already has 'on {}'", event.keyword()),
            word.span,
        ));
    }
    handlers.push(Handler {
        event,
        handler: handler.name,
        args: Vec::new(),
    });
    Ok(())
}

fn parse_page_setting(node: &Node, page: &mut Page, grid_seen: &mut bool) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    let keyword = cursor.expect_ident("a page setting")?;
    match keyword.name.as_str() {
        "title" => {
            if page.title.is_some() {
                return Err(Diagnostic::error("duplicate 'title'", node.line.span));
            }
            page.title = Some(cursor.expect_text("a title string")?);
        }
        "size" => {
            if page.size.is_some() {
                return Err(Diagnostic::error("duplicate 'size'", node.line.span));
            }
            let width = cursor.expect_number("a width in pixels")?;
            if !cursor.eat_keyword("x") {
                return Err(cursor.error_here("expected 'x' and a height, as in 'size 320 x 460'"));
            }
            let height = cursor.expect_number("a height in pixels, as in 'size 320 x 460'")?;
            page.size = Some(WindowSize { width, height });
        }
        "data" => {
            page.data.push(parse_data(&mut cursor, node.line.span)?);
        }
        "on" => {
            let owner = page.name.name.clone();
            parse_owner_event(
                &mut cursor,
                &mut page.handlers,
                &WINDOW_EVENTS,
                |e| format!("a window has the events open, close and return, not {e}"),
                &owner,
            )?;
        }
        _ => {
            // grid N columns
            if *grid_seen {
                return Err(Diagnostic::error("duplicate 'grid'", node.line.span));
            }
            *grid_seen = true;
            page.columns = cursor.expect_number("a number of columns")?;
            if !cursor.eat_keyword("columns") {
                return Err(cursor.error_here("expected 'columns', as in 'grid 12 columns'"));
            }
        }
    }
    cursor.expect_end()
}

/// `name: Entity`, `name: list<Entity>` or `name: type, default value`,
/// after the `data` keyword. A name is a record here; once every file is
/// read, one that names an enum becomes a plain value (`resolve_data`).
fn parse_data(cursor: &mut Cursor, span: Span) -> ParseResult<DataDecl> {
    use crate::code::TypeRefKind;
    use crate::model::DataKind;
    let name = cursor.expect_ident("a data name")?;
    cursor.expect(
        TokenKind::Colon,
        "':' and an entity or a type, as in 'data customer: Customer'",
    )?;
    let ty = crate::code::parse::parse_type_ref(cursor)?;
    let default = if cursor.eat(TokenKind::Comma) {
        if !cursor.eat_keyword("default") {
            return Err(cursor.error_here("expected 'default' and a value, as in 'data fresh: bool, default true'"));
        }
        Some(parse_literal(cursor)?)
    } else {
        None
    };
    let of = match ty.kind {
        TypeRefKind::List { item } if crate::code::parse::is_type_word(&item) => {
            return Err(Diagnostic::error("a list holds records, as in 'list<Order>'", ty.span));
        }
        TypeRefKind::List { .. } if default.is_some() => {
            return Err(Diagnostic::error(
                format!("'{}' is a list, which starts with every record; only a plain value has a default", name.name),
                ty.span,
            ));
        }
        TypeRefKind::List { item } => {
            // The entity's name sits between the angle brackets.
            let start = ty.span.end - 1 - item.len() as u32;
            let col = ty.span.col + (start - ty.span.start);
            DataKind::List { entity: Ident { name: item, span: Span { start, end: ty.span.end - 1, line: ty.span.line, col } } }
        }
        TypeRefKind::File => {
            return Err(Diagnostic::error(
                "a window holds records, lists and plain values; a file belongs to a record",
                ty.span,
            ));
        }
        TypeRefKind::Bytes => {
            return Err(Diagnostic::error("bytes are no window data: the screen has nothing to show of them, so code keeps them while it works", ty.span));
        }
        TypeRefKind::Named { name: entity } if default.is_none() => DataKind::Record { entity: Ident { name: entity, span: ty.span } },
        _ => DataKind::Value { ty, default },
    };
    Ok(DataDecl { name, of, span })
}

// ---------------------------------------------------------------------------
// Apps and frame regions

/// App settings (`title`, `main`, `data`) as opposed to regions.
pub(crate) fn is_app_setting(tokens: &[Token]) -> bool {
    starts_with_keyword(tokens, "title", Some(TokenKind::Str))
        || starts_with_keyword(tokens, "main", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "on", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "data", Some(TokenKind::Ident))
}

/// A region block: `menu`, `toolbar`, `ribbon`, `footer` or
/// `sidebar left span 3`.
pub(crate) fn is_region_line(tokens: &[Token]) -> bool {
    tokens
        .first()
        .is_some_and(|t| t.kind == TokenKind::Ident && frame::by_keyword(&t.text).is_some())
}

fn parse_app(
    node: &Node,
    mut cursor: Cursor,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<App> {
    let name = cursor.expect_ident("an app name")?;
    cursor.expect_end()?;
    let mut app = App {
        name,
        title: None,
        main: None,
        data: Vec::new(),
        frame: Frame::default(),
        handlers: Vec::new(),
        span: node.span(),
    };
    for child in &node.children {
        let result = if is_app_setting(&child.line.tokens) {
            parse_app_setting(child, &mut app)
        } else if is_region_line(&child.line.tokens) {
            parse_region(child, &mut app.frame, diagnostics)
        } else {
            let c = Cursor::new(child.line);
            Err(c.error_here(
                "expected 'title', 'main', 'data', 'menu', 'toolbar', 'ribbon', 'sidebar' or 'footer'",
            ))
        };
        if let Err(diag) = result {
            diagnostics.push(diag);
        }
    }
    Ok(app)
}

fn parse_app_setting(node: &Node, app: &mut App) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    let keyword = cursor.expect_ident("an app setting")?;
    match keyword.name.as_str() {
        "title" => {
            if app.title.is_some() {
                return Err(Diagnostic::error("duplicate 'title'", node.line.span));
            }
            app.title = Some(cursor.expect_text("a title string")?);
        }
        "main" => {
            if app.main.is_some() {
                return Err(Diagnostic::error("duplicate 'main'", node.line.span));
            }
            app.main = Some(cursor.expect_ident("the main window")?);
        }
        "on" => {
            let owner = app.name.name.clone();
            parse_owner_event(
                &mut cursor,
                &mut app.handlers,
                &APP_EVENTS,
                |e| format!("the app has the event start, not {e}"),
                &owner,
            )?;
        }
        _ => app.data.push(parse_data(&mut cursor, node.line.span)?),
    }
    cursor.expect_end()
}

fn parse_region(
    node: &Node,
    frame: &mut Frame,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<()> {
    let mut cursor = Cursor::new(node.line);
    let keyword = cursor.expect_ident("a region")?;
    let region = frame::by_keyword(&keyword.name).expect("checked by is_region_line");
    let duplicate = || Diagnostic::error(format!("duplicate '{}'", keyword.name), keyword.span);
    match region {
        RegionKind::Menu => {
            let name = region_name(&mut cursor, "menu")?;
            if frame.menu.is_some() {
                return Err(duplicate());
            }
            let items = parse_commands(
                &node.children,
                &[CommandKind::Item, CommandKind::Separator],
                diagnostics,
            );
            frame.menu = Some(Menu {
                name,
                items,
                span: node.span(),
            });
        }
        RegionKind::Toolbar => {
            let name = region_name(&mut cursor, "toolbar")?;
            if frame.toolbar.is_some() {
                return Err(duplicate());
            }
            let items = parse_commands(
                &node.children,
                &[CommandKind::Button, CommandKind::Separator],
                diagnostics,
            );
            frame.toolbar = Some(Toolbar {
                name,
                items,
                span: node.span(),
            });
        }
        RegionKind::Ribbon => {
            let (mut name, mut collapsible, mut collapsed) = (None, false, false);
            while let Some(word) = cursor.peek().map(|t| t.text.clone()) {
                let clause = cursor.expect_ident("a ribbon property")?;
                match word.as_str() {
                    "name" => name = Some(cursor.expect_ident("a part name")?.name),
                    "collapsible" => collapsible = true,
                    "collapsed" => collapsed = true,
                    other => {
                        return Err(Diagnostic::error(
                            format!("unknown ribbon property '{other}'; a ribbon has name, collapsible and collapsed"),
                            clause.span,
                        ));
                    }
                }
            }
            if frame.ribbon.is_some() {
                return Err(duplicate());
            }
            let mut tabs = Vec::new();
            for child in &node.children {
                match parse_ribbon_tab(child, diagnostics) {
                    Ok(tab) => tabs.push(tab),
                    Err(diag) => diagnostics.push(diag),
                }
            }
            frame.ribbon = Some(Ribbon {
                name,
                collapsible,
                collapsed,
                tabs,
                span: node.span(),
            });
        }
        RegionKind::Sidebar => {
            let side = cursor.expect_ident("'left' or 'right', as in 'sidebar left name nav'")?;
            let side = match side.name.as_str() {
                "left" => Side::Left,
                "right" => Side::Right,
                other => {
                    return Err(Diagnostic::error(
                        format!("unknown side '{other}', expected 'left' or 'right'"),
                        side.span,
                    ));
                }
            };
            let mut props = frame::new_sidebar(side, None);
            let mut old_columns = None;
            while let Some(word) = cursor.peek().map(|t| t.text.clone()) {
                let clause = cursor.expect_ident("a sidebar property")?;
                match word.as_str() {
                    "name" => props.name = Some(cursor.expect_ident("a part name")?.name),
                    "width" => props.width = cursor.expect_number("a width in pixels")?,
                    "min" => props.min = Some(cursor.expect_number("a width in pixels")?),
                    "max" => props.max = Some(cursor.expect_number("a width in pixels")?),
                    "resizable" => props.resizable = true,
                    "collapsible" => props.collapsible = true,
                    "collapsed" => props.collapsed = true,
                    "maximizable" => props.maximizable = true,
                    "grid" => {
                        props.columns = cursor.expect_number("a number of columns")?;
                        if !cursor.eat_keyword("columns") {
                            return Err(Diagnostic::error("expected 'columns', as in 'grid 4 columns'", clause.span));
                        }
                    }
                    // The old sidebar, `span 3` columns wide: read, for its fix.
                    "span" => old_columns = Some(cursor.expect_number("a number of columns")?),
                    other => {
                        return Err(Diagnostic::error(
                            format!("unknown sidebar property '{other}'; a sidebar has name, width, min, max, resizable, collapsible, collapsed, maximizable and grid"),
                            clause.span,
                        ));
                    }
                }
            }
            if let Some(columns) = old_columns {
                diagnostics.push(Diagnostic::error(
                    format!("a sidebar's width is in pixels now; Update the sidebars, in Problems, writes it so ('width {} grid {columns} columns')", columns * crate::layout::SIDEBAR_COLUMN),
                    keyword.span,
                ));
                props.columns = columns;
                props.width = columns * crate::layout::SIDEBAR_COLUMN;
            }
            if frame.sidebars.iter().any(|s| s.props.side == side) {
                return Err(Diagnostic::error(
                    format!("duplicate '{} {}'", keyword.name, frame::side_keyword(side)),
                    keyword.span,
                ));
            }
            frame.sidebars.push(Sidebar {
                props,
                widgets: parse_widgets(&node.children, diagnostics),
                span: node.span(),
            });
        }
        RegionKind::Footer => {
            cursor.expect_end()?;
            if frame.footer.is_some() {
                return Err(duplicate());
            }
            let mut footer = Footer {
                status: None,
                status_span: None,
                columns: FOOTER_COLUMNS,
                widgets: Vec::new(),
                span: node.span(),
            };
            for child in &node.children {
                if is_keyword_line(child, "status", Some(TokenKind::Str)) {
                    let result = (|| {
                        no_children(child)?;
                        let mut c = Cursor::new(child.line);
                        c.next();
                        if footer.status.is_some() {
                            return Err(Diagnostic::error("duplicate 'status'", child.line.span));
                        }
                        footer.status = Some(c.expect_text("a status text")?);
                        footer.status_span = Some(child.line.span);
                        c.expect_end()
                    })();
                    if let Err(diag) = result {
                        diagnostics.push(diag);
                    }
                } else {
                    match parse_widget(child, diagnostics) {
                        Ok(widget) => footer.widgets.push(widget),
                        Err(diag) => diagnostics.push(diag),
                    }
                }
            }
            frame.footer = Some(footer);
        }
    }
    Ok(())
}

fn parse_ribbon_tab(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<RibbonTab> {
    let mut cursor = Cursor::new(node.line);
    if !is_keyword_line(node, "tab", Some(TokenKind::Str)) {
        return Err(cursor.error_here("expected 'tab \"Title\"'"));
    }
    cursor.next();
    let title = cursor.expect_text("a tab title")?;
    let name = part_name(&mut cursor)?;
    let mut groups = Vec::new();
    for child in &node.children {
        let mut c = Cursor::new(child.line);
        if !is_keyword_line(child, "group", Some(TokenKind::Str)) {
            diagnostics.push(c.error_here("expected 'group \"Title\"'"));
            continue;
        }
        c.next();
        let result = c.expect_text("a group title").and_then(|title| {
            let name = part_name(&mut c)?;
            Ok(RibbonGroup {
                title,
                name,
                items: parse_commands(
                    &child.children,
                    &[CommandKind::Button, CommandKind::Separator],
                    diagnostics,
                ),
                span: child.span(),
            })
        });
        match result {
            Ok(group) => groups.push(group),
            Err(diag) => diagnostics.push(diag),
        }
    }
    Ok(RibbonTab {
        title,
        name,
        groups,
        span: node.span(),
    })
}

/// An optional `name x` at the end of a line, as tabs and groups of a ribbon have.
/// The name on a menu's or toolbar's line: `menu name mainMenu`.
fn region_name(cursor: &mut Cursor, what: &str) -> ParseResult<Option<String>> {
    let mut name = None;
    while let Some(word) = cursor.peek().map(|t| t.text.clone()) {
        let clause = cursor.expect_ident(&format!("a {what} property"))?;
        match word.as_str() {
            "name" => name = Some(cursor.expect_ident("a part name")?.name),
            other => {
                return Err(Diagnostic::error(format!("unknown {what} property '{other}'; a {what} has a name"), clause.span));
            }
        }
    }
    Ok(name)
}

fn part_name(cursor: &mut Cursor) -> ParseResult<Option<String>> {
    let name = if cursor.eat_keyword("name") { Some(cursor.expect_ident("a part name")?.name) } else { None };
    cursor.expect_end()?;
    Ok(name)
}

fn parse_commands(
    nodes: &[Node],
    allowed: &[CommandKind],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Command> {
    let mut commands = Vec::new();
    for node in nodes {
        match parse_command(node, allowed, diagnostics) {
            Ok(command) => commands.push(command),
            Err(diag) => diagnostics.push(diag),
        }
    }
    commands
}

fn parse_command(
    node: &Node,
    allowed: &[CommandKind],
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<Command> {
    let mut cursor = Cursor::new(node.line);
    let expected = allowed
        .iter()
        .map(|k| format!("'{}'", frame::command_keyword(*k)))
        .collect::<Vec<_>>()
        .join(" or ");
    let keyword = cursor.expect_ident(&expected)?;
    let kind = match frame::command_by_keyword(&keyword.name) {
        Some(kind) if allowed.contains(&kind) => kind,
        _ => {
            return Err(Diagnostic::error(
                format!("expected {expected}, found '{}'", keyword.name),
                keyword.span,
            ));
        }
    };
    let mut props = frame::new_command(kind, None);
    if cursor.peek_is_text() {
        props.caption = Some(cursor.expect_text("a caption")?);
    }
    let mut seen: Vec<String> = Vec::new();
    while cursor.peek().is_some() {
        let clause = cursor.expect_ident("a property")?;
        if seen.contains(&clause.name) {
            return Err(Diagnostic::error(
                format!("duplicate '{}'", clause.name),
                clause.span,
            ));
        }
        seen.push(clause.name.clone());
        match clause.name.as_str() {
            "icon" => props.icon = Some(cursor.expect_string("an icon name")?),
            "key" => {
                let text = cursor.peek().map(|t| t.span).unwrap_or(clause.span);
                let shortcut = cursor.expect_string("a shortcut, as in key \"Ctrl+N\"")?;
                widgets::check_shortcut(&shortcut).map_err(|m| Diagnostic::error(m, text))?;
                props.shortcut = Some(shortcut);
            }
            "large" => props.large = true,
            "name" => props.name = Some(cursor.expect_ident("a part name")?.name),
            "on" => {
                let event = cursor.expect_ident("an event")?;
                if event.name != "click" {
                    return Err(Diagnostic::error(
                        format!("unknown event '{}', commands have 'click'", event.name),
                        event.span,
                    ));
                }
                if cursor.eat(TokenKind::Colon) {
                    props.handler = Some(cursor.expect_ident("a handler name")?.name);
                    continue;
                }
                cursor.expect(TokenKind::Arrow, "'->' and a target")?;
                props.on_click = Some(cursor.expect_ident("a page, screen or command")?.name);
                if cursor.eat_keyword("in") {
                    if !(cursor.eat_keyword("new") && cursor.eat_keyword("window")) {
                        return Err(cursor.error_here("expected 'in new window'".to_string()));
                    }
                    props.new_window = true;
                }
            }
            other => {
                return Err(Diagnostic::error(
                    format!("unknown property '{other}'"),
                    clause.span,
                ));
            }
        }
    }
    frame::validate_command(&props).map_err(|m| Diagnostic::error(m, node.line.span))?;
    let items = if kind == CommandKind::Item {
        parse_commands(
            &node.children,
            &[CommandKind::Item, CommandKind::Separator],
            diagnostics,
        )
    } else {
        no_children(node)?;
        Vec::new()
    };
    Ok(Command {
        props,
        items,
        span: node.span(),
    })
}

fn parse_widgets(nodes: &[Node], diagnostics: &mut Vec<Diagnostic>) -> Vec<Widget> {
    let mut widgets = Vec::new();
    for node in nodes {
        match parse_widget(node, diagnostics) {
            Ok(widget) => widgets.push(widget),
            Err(diag) => diagnostics.push(diag),
        }
    }
    widgets
}

fn parse_widget(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<Widget> {
    let mut cursor = Cursor::new(node.line);
    let keyword = cursor.expect_ident("a widget")?;
    let Some(kind) = widgets::by_keyword(&keyword.name) else {
        return Err(Diagnostic::error(
            format!("unknown widget '{}'", keyword.name),
            keyword.span,
        ));
    };
    let props = parse_widget_props(&mut cursor, kind, node.line.span)?;

    let mut widget = Widget {
        props,
        children: Vec::new(),
        pages: Vec::new(),
        span: node.span(),
    };
    match widgets::container(kind) {
        None => no_children(node)?,
        Some(ContainerKind::Plain) => widget.children = parse_widgets(&node.children, diagnostics),
        Some(ContainerKind::Tabs) => {
            for child in &node.children {
                match parse_widget_page(child, "tab", diagnostics) {
                    Ok(page) => widget.pages.push(page),
                    Err(diag) => diagnostics.push(diag),
                }
            }
        }
        Some(ContainerKind::Splitter) => {
            for child in &node.children {
                match parse_widget_page(child, "pane", diagnostics) {
                    Ok(page) => widget.pages.push(page),
                    Err(diag) => diagnostics.push(diag),
                }
            }
        }
    }
    Ok(widget)
}

/// `tab "Title"` or `pane`, each with widgets inside.
fn parse_widget_page(
    node: &Node,
    keyword: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<WidgetPage> {
    let mut cursor = Cursor::new(node.line);
    if !cursor.eat_keyword(keyword) {
        let expected = if keyword == "tab" {
            "'tab \"Title\"'"
        } else {
            "'pane'"
        };
        return Err(cursor.error_here(format!("expected {expected}")));
    }
    let title = if keyword == "tab" {
        Some(cursor.expect_text("a tab title")?)
    } else {
        None
    };
    cursor.expect_end()?;
    Ok(WidgetPage {
        title,
        children: parse_widgets(&node.children, diagnostics),
        span: node.span(),
    })
}

fn parse_widget_props(
    cursor: &mut Cursor,
    kind: WidgetKind,
    line: Span,
) -> ParseResult<WidgetProps> {
    let name = widgets::keyword(kind);
    let (width, height) = widgets::default_size(kind);
    let mut props = WidgetProps {
        kind,
        caption: None,
        name: None,
        col: 0,
        row: 0,
        width,
        height,
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
        orientation: None,
        surface: None,
        renderer: None,
        source: None,
        for_part: None,
        placeholder: None,
        digits: None,
        units: None,
        tooltip: None,
        style: None,
        readonly: false,
        foldable: false,
        folded: false,
        tab_order: None,
        shortcut: None,
        default_button: false,
        cancel: false,
    };
    if cursor.peek_is_text() {
        props.caption = Some(cursor.expect_text("a caption")?);
    }

    let mut seen: Vec<String> = Vec::new();
    let mut placed = false;
    while let Some(token) = cursor.peek() {
        let clause = cursor.expect_ident("a widget property")?;
        let word = clause.name.as_str();
        // `horizontal` and `vertical` share one property; every event is one
        // property (`on click`, `on change`).
        let event_word = cursor.peek().map(|t| t.text.as_str()).unwrap_or("");
        let on_event = format!("on {event_word}");
        let property_name = match word {
            "horizontal" | "vertical" => "orientation",
            "on" => on_event.as_str(),
            other => other,
        };
        if seen.iter().any(|s| s == property_name) {
            return Err(Diagnostic::error(
                format!("duplicate '{property_name}'"),
                clause.span,
            ));
        }
        seen.push(property_name.to_string());

        let applies = |property: WidgetProperty| {
            if widgets::has_property(kind, property) {
                Ok(())
            } else {
                Err(Diagnostic::error(
                    format!("'{property_name}' does not apply to a {name}"),
                    clause.span,
                ))
            }
        };

        match word {
            "at" => {
                props.col = cursor.expect_number("a column")?;
                cursor.expect(TokenKind::Comma, "',' and a row, as in 'at 1,2'")?;
                props.row = cursor.expect_number("a row")?;
                placed = true;
            }
            "span" => props.width = cursor.expect_number("a number of columns")?,
            "rows" => props.height = cursor.expect_number("a number of rows")?,
            "bind" => {
                if widgets::info(kind).bind == BindTarget::Nothing {
                    return Err(Diagnostic::error(
                        format!("a {name} cannot be bound to data"),
                        clause.span,
                    ));
                }
                let mut path = cursor.expect_ident("a data name")?.name;
                if cursor.eat(TokenKind::Dot) {
                    path.push('.');
                    path.push_str(&cursor.expect_ident("a field name")?.name);
                }
                props.bind = Some(path);
            }
            "name" => props.name = Some(cursor.expect_ident("a part name")?.name),
            "on" => {
                let event_ident = cursor.expect_ident("an event")?;
                let Some(event) = Event::by_keyword(&event_ident.name) else {
                    return Err(Diagnostic::error(
                        format!("unknown event '{}'", event_ident.name),
                        event_ident.span,
                    ));
                };
                // `on change: name_changed` wires a handler in the code behind.
                if cursor.eat(TokenKind::Colon) {
                    if !widgets::has_event(kind, event) {
                        return Err(Diagnostic::error(
                            format!("a {name} has no {} event", event.keyword()),
                            event_ident.span,
                        ));
                    }
                    let handler = cursor.expect_ident("a handler name")?.name;
                    // `digit(7)`: the values the handler runs with.
                    let mut args = Vec::new();
                    if cursor.eat(TokenKind::LParen) && !cursor.eat(TokenKind::RParen) {
                        loop {
                            args.push(parse_literal(cursor)?);
                            if cursor.eat(TokenKind::RParen) {
                                break;
                            }
                            cursor.expect(TokenKind::Comma, "',' or ')' after a value")?;
                        }
                    }
                    props.handlers.push(Handler { event, handler, args });
                    continue;
                }
                // `on click -> Window` opens a window.
                if event != Event::Click {
                    return Err(Diagnostic::error(
                        format!("'on {} ->' is not supported; wire a handler with 'on {}: handler_name'", event.keyword(), event.keyword()),
                        event_ident.span,
                    ));
                }
                if !widgets::info(kind).clickable {
                    return Err(Diagnostic::error(
                        format!("a {name} has no click event"),
                        event_ident.span,
                    ));
                }
                cursor.expect(TokenKind::Arrow, "'->' and a target")?;
                props.on_click = Some(cursor.expect_ident("a window")?.name);
                if cursor.eat_keyword("in") {
                    if !(cursor.eat_keyword("new") && cursor.eat_keyword("window")) {
                        return Err(cursor.error_here("expected 'in new window'".to_string()));
                    }
                    props.new_window = true;
                }
            }
            "columns" => {
                applies(WidgetProperty::Columns)?;
                // `columns title 240, artist`: a width in pixels may follow a field.
                loop {
                    let field = cursor.expect_ident("a field name")?.name;
                    if cursor.peek().is_some_and(|t| t.kind == TokenKind::Number) {
                        let at = cursor.peek().map(|t| t.span).unwrap_or(clause.span);
                        let width = cursor.expect_number("a width")?;
                        if !(widgets::MIN_COLUMN_WIDTH..=widgets::MAX_COLUMN_WIDTH).contains(&width) {
                            return Err(Diagnostic::error(widgets::COLUMN_WIDTH_PROBLEM.to_string(), at));
                        }
                        props.widths.insert(field.clone(), width);
                    }
                    props.columns.push(field);
                    if !cursor.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }
            "resizable" => {
                applies(WidgetProperty::Resizable)?;
                props.resizable = true;
            }
            "sortable" => {
                applies(WidgetProperty::Sortable)?;
                props.sortable = true;
            }
            "order" => {
                applies(WidgetProperty::OrderBy)?;
                if !cursor.eat_keyword("by") {
                    return Err(cursor.error_here("expected 'by', as in 'order by title'".to_string()));
                }
                let field = cursor.expect_ident("a column to order by")?.name;
                let desc = cursor.eat_keyword("desc");
                props.order = Some(crate::model::TableOrder { field, desc });
            }
            "edit" => {
                applies(WidgetProperty::Edit)?;
                // `edit title, done as switch`: a bool column can be a switch.
                loop {
                    let field = cursor.expect_ident("a field name")?.name;
                    if cursor.eat_keyword("as") {
                        if !cursor.eat_keyword("switch") {
                            return Err(cursor.error_here("expected 'switch', as in 'edit done as switch'".to_string()));
                        }
                        props.switches.push(field.clone());
                    }
                    props.edit.push(field);
                    if !cursor.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }
            "show" => {
                applies(WidgetProperty::Show)?;
                props.show = cursor
                    .ident_list("a field name")?
                    .into_iter()
                    .map(|i| i.name)
                    .collect();
            }
            "horizontal" | "vertical" => {
                applies(WidgetProperty::Orientation)?;
                props.orientation = Some(if word == "horizontal" {
                    Orientation::Horizontal
                } else {
                    Orientation::Vertical
                });
            }
            "surface" => {
                applies(WidgetProperty::Surface)?;
                let surface = cursor.expect_ident("'svg', 'bitmap' or 'gpu'")?;
                props.surface = Some(match surface.name.as_str() {
                    "svg" => Surface::Svg,
                    "bitmap" => Surface::Bitmap,
                    "gpu" => Surface::Gpu,
                    other => {
                        return Err(Diagnostic::error(
                            format!("unknown surface '{other}', expected 'svg', 'bitmap' or 'gpu'"),
                            surface.span,
                        ));
                    }
                });
            }
            "renderer" => {
                applies(WidgetProperty::Renderer)?;
                props.renderer = Some(cursor.expect_ident("a renderer name")?.name);
            }
            "source" => {
                applies(WidgetProperty::Source)?;
                props.source = Some(cursor.expect_string("a file name")?);
            }
            "for" => {
                applies(WidgetProperty::For)?;
                props.for_part = Some(cursor.expect_ident("a part name")?.name);
            }
            "units" => {
                applies(WidgetProperty::Units)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or_default();
                let width = cursor.expect_number("a width in units")?;
                if !cursor.eat_keyword("x") {
                    return Err(cursor.error_here("expected 'x' and a height, as in 'units 200 x 100'"));
                }
                let height = cursor.expect_number("a height in units")?;
                if !widgets::canvas_units_fit(width, height) {
                    return Err(Diagnostic::error(widgets::CANVAS_UNITS_PROBLEM, at));
                }
                props.units = Some(crate::model::CanvasUnits { width, height });
            }
            "digits" => {
                applies(WidgetProperty::Digits)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or_default();
                let digits = cursor.expect_number("a number of digits")?;
                if !(1..=widgets::MAX_DIGITS).contains(&digits) {
                    return Err(Diagnostic::error(format!("a display has 1 to {} digits", widgets::MAX_DIGITS), at));
                }
                props.digits = Some(digits);
            }
            "placeholder" => {
                applies(WidgetProperty::Placeholder)?;
                props.placeholder = Some(cursor.expect_text("a placeholder text")?);
            }
            "tooltip" => props.tooltip = Some(cursor.expect_text("a tooltip text")?),
            "style" => props.style = Some(cursor.expect_ident("a style of the project's theme")?.name),
            "readonly" => {
                applies(WidgetProperty::Readonly)?;
                props.readonly = true;
            }
            "foldable" => {
                applies(WidgetProperty::Foldable)?;
                props.foldable = true;
            }
            "folded" => {
                if !props.foldable {
                    return Err(Diagnostic::error("'folded' needs 'foldable': write 'foldable folded'", clause.span));
                }
                props.folded = true;
            }
            "taborder" => {
                applies(WidgetProperty::TabOrder)?;
                let order = cursor.expect_number("a position in the tab order")?;
                if order == 0 {
                    return Err(Diagnostic::error("tab order starts at 1", clause.span));
                }
                props.tab_order = Some(order);
            }
            "key" => {
                applies(WidgetProperty::Shortcut)?;
                let text = cursor.peek().map(|t| t.span).unwrap_or(clause.span);
                let shortcut = cursor.expect_string("a shortcut, as in key \"Ctrl+S\"")?;
                widgets::check_shortcut(&shortcut).map_err(|m| Diagnostic::error(m, text))?;
                props.shortcut = Some(shortcut);
            }
            "default" => {
                applies(WidgetProperty::Default)?;
                props.default_button = true;
            }
            "cancel" => {
                applies(WidgetProperty::Cancel)?;
                props.cancel = true;
            }
            _ => {
                return Err(Diagnostic::error(
                    format!("unknown widget property '{}'", token.text),
                    token.span,
                ));
            }
        }
    }

    if !placed {
        return Err(Diagnostic::error(
            format!("a {name} needs a position, as in 'at 1,1'"),
            line,
        ));
    }
    Ok(props)
}
