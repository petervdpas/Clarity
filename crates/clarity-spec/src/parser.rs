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
                "expected 'module', 'use', 'source', 'enum', 'entity', 'page', 'app', 'theme', 'code', 'function', 'action', 'engine', 'settings', 'cli', 'command', 'part', 'library' or 'mapping'",
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

/// `record Dot` and its fields (DESIGN 5.12): as an entity's, but values
/// only, so a decimal needs no precision and scale.
fn parse_record(node: &Node, name: Ident, diagnostics: &mut Vec<Diagnostic>) -> Entity {
    let mut fields = Vec::new();
    for child in &node.children {
        if let Some(word @ ("readable" | "writable")) = first_word(child)
            && child.line.tokens.get(1).is_some_and(|t| t.text == "when")
        {
            diagnostics.push(Diagnostic::error(format!("a record is never stored, so it has no '{word} when'"), child.line.span));
            continue;
        }
        if let Some(word @ ("version" | "savedSince")) = first_word(child)
            && child.line.tokens.get(1).is_some_and(|t| t.kind != TokenKind::Colon)
        {
            diagnostics.push(Diagnostic::error(format!("a record is never stored, so it has no '{word}'"), child.line.span));
            continue;
        }
        let field = no_children(child).and_then(|_| {
            let mut cursor = Cursor::new(child.line);
            parse_field_typed(&mut cursor, child.line.span, true)
        });
        match field {
            Ok(field) => fields.push(field),
            Err(diag) => diagnostics.push(diag),
        }
    }
    if node.children.is_empty() {
        diagnostics.push(Diagnostic::error(format!("record '{}' has no fields", name.name), name.span));
    }
    Entity { name, fields, table: None, key: None, settings: false, record: true, people: false, readable: None, writable: None, version: None, version_none: false, saved_since: None, kept_version: None, span: node.span() }
}

fn parse_entity(node: &Node, name: Ident, diagnostics: &mut Vec<Diagnostic>) -> Entity {
    let mut fields = Vec::new();
    let (mut table, mut key) = (None, None);
    let (mut readable, mut writable) = (None, None);
    let (mut version, mut version_none, mut saved_since) = (None, false, None);
    for child in &node.children {
        // `readable when <condition>` and `writable when <condition>`: who may read or change a record.
        if let Some(word @ ("readable" | "writable")) = first_word(child)
            && child.line.tokens.get(1).is_some_and(|t| t.text == "when")
        {
            let slot = if word == "readable" { &mut readable } else { &mut writable };
            if let Err(diag) = parse_access(child, word, slot) {
                diagnostics.push(diag);
            }
            continue;
        }
        // `version "RowVersion"`, `version none` and `savedSince @key`:
        // how a save is checked; field lines have a colon after the name.
        if let Some(word @ ("version" | "savedSince")) = first_word(child)
            && child.line.tokens.get(1).is_some_and(|t| t.kind != TokenKind::Colon)
        {
            let result = if word == "version" { parse_version(child, &mut version, &mut version_none) } else { parse_saved_since(child, &name.name, &mut saved_since) };
            if let Err(diag) = result {
                diagnostics.push(diag);
            }
            continue;
        }
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
        settings: false,
        record: false,
        people: false,
        readable,
        writable,
        version,
        version_none,
        saved_since,
        kept_version: None,
        span: node.span(),
    }
}

/// `version "RowVersion"` or `version none` of an entity.
fn parse_version(node: &Node, version: &mut Option<String>, none: &mut bool) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    cursor.next();
    let named = if cursor.eat_keyword("none") { None } else { Some(cursor.expect_string("the version column in quotes, as in version \"RowVersion\", or none")?) };
    cursor.expect_end()?;
    if version.is_some() || *none {
        return Err(Diagnostic::error("duplicate 'version'", node.line.span));
    }
    match named {
        Some(column) => *version = Some(column),
        None => *none = true,
    }
    Ok(())
}

/// `savedSince @customerChangedSince` of an entity: a key, as every text people read.
fn parse_saved_since(node: &Node, entity: &str, slot: &mut Option<String>) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    cursor.next();
    if cursor.peek_kind() != Some(TokenKind::Key) {
        let key = crate::names::camel_case(&format!("{entity} changed since"));
        return Err(cursor.error_here(format!("the text of savedSince is a key: write savedSince @{key}, with its text in the language files")));
    }
    let key = cursor.expect(TokenKind::Key, "a key")?.text.clone();
    cursor.expect_end()?;
    if slot.is_some() {
        return Err(Diagnostic::error("duplicate 'savedSince'", node.line.span));
    }
    *slot = Some(key);
    Ok(())
}

/// `readable when <condition>` or `writable when <condition>` of an entity.
fn parse_access(node: &Node, word: &str, slot: &mut Option<crate::code::Expr>) -> ParseResult<()> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    cursor.next();
    cursor.next();
    let condition = crate::code::parse::parse_expr(&mut cursor)?;
    cursor.expect_end()?;
    if slot.is_some() {
        return Err(Diagnostic::error(format!("'{word} when' is written twice; join the conditions with 'or'"), node.line.span));
    }
    *slot = Some(condition);
    Ok(())
}

/// The `settings` block (DESIGN 5.17): fields as an entity's, and Clarity's
/// own `language`, `theme`, `agentReadsWhileClosed` and `agentChangesWhileClosed`
/// without a type; read as the record type `Settings`.
fn parse_settings(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> Entity {
    let mut fields = Vec::new();
    for child in &node.children {
        // Labeled with Clarity's own words, which head the pickers too.
        let own = match child.line.tokens.as_slice() {
            [word] if word.text == "language" => Some((FieldType::Language, "@languagePicker")),
            [word] if word.text == "theme" => Some((FieldType::Theme, "@themePicker")),
            // What AI agents may do while the app is closed (docs/ledger/app-mcp.md).
            [word] if word.text == "agentReadsWhileClosed" => Some((FieldType::Bool, "@agentReadsWhileClosedSetting")),
            [word] if word.text == "agentChangesWhileClosed" => Some((FieldType::Bool, "@agentChangesWhileClosedSetting")),
            _ => None,
        };
        let field = match own {
            Some((ty, label)) => no_children(child).map(|_| {
                let word = &child.line.tokens[0];
                Field {
                    name: Ident { name: word.text.clone(), span: word.span },
                    type_text: if ty == FieldType::Bool { "bool".into() } else { word.text.clone() },
                    ty,
                    modifiers: Modifiers { label: Some(label.to_string()), ..Modifiers::default() },
                    label: label.to_string(),
                    span: child.line.span,
                }
            }),
            None => parse_field(child),
        };
        match field {
            Ok(field) => fields.push(field),
            Err(diag) => diagnostics.push(diag),
        }
    }
    let span = node.span();
    Entity {
        name: Ident { name: SETTINGS_ENTITY.to_string(), span: node.line.tokens.first().map_or(span, |t| t.span) },
        fields,
        table: None,
        key: None,
        settings: true,
        record: false,
        people: false,
        readable: None,
        writable: None,
        version: None,
        version_none: false,
        saved_since: None,
        kept_version: None,
        span,
    }
}

/// Clarity's own settings, written in the settings block as a bare word.
pub const OWN_SETTINGS: [&str; 4] = ["language", "theme", "agentReadsWhileClosed", "agentChangesWhileClosed"];

/// The record type of the project's settings.
pub const SETTINGS_ENTITY: &str = "Settings";
/// Every window's own data with the settings.
pub const SETTINGS_DATA: &str = "settings";

fn parse_field(node: &Node) -> ParseResult<Field> {
    no_children(node)?;
    let mut cursor = Cursor::new(node.line);
    parse_field_at(&mut cursor, node.line.span)
}

/// `name: type, modifiers` from the cursor to the end of the line.
pub(crate) fn parse_field_at(cursor: &mut Cursor, span: Span) -> ParseResult<Field> {
    parse_field_typed(cursor, span, false)
}

/// A field of an entity, or of a record (`value`), whose decimal needs no precision and scale.
fn parse_field_typed(cursor: &mut Cursor, span: Span, value: bool) -> ParseResult<Field> {
    let name = cursor.expect_ident("a field name")?;
    cursor.expect(TokenKind::Colon, "':' after the field name")?;
    let ty = parse_type_of(cursor, value)?;
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

/// The plain types a list field cannot hold.
const VALUE_WORDS: [&str; 12] = ["bool", "money", "datetime", "duration", "image", "file", "email", "url", "color", "id", "ref", "list"];

pub(crate) fn parse_type(cursor: &mut Cursor) -> ParseResult<FieldType> {
    parse_type_of(cursor, false)
}

fn parse_type_of(cursor: &mut Cursor, value: bool) -> ParseResult<FieldType> {
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
        // A record's decimal is a value, with no column to size.
        "decimal" if value && cursor.peek_kind() != Some(TokenKind::LParen) => FieldType::Decimal { precision: 28, scale: 0 },
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
        "color" => FieldType::Color,
        "secret" => FieldType::Secret,
        "folder" => FieldType::Folder,
        "file" => FieldType::File,
        "id" => FieldType::Id,
        "ref" => FieldType::Ref {
            entity: cursor.expect_ident("an entity name after 'ref'")?,
        },
        "list" => {
            cursor.expect(TokenKind::Lt, "'<' after 'list'")?;
            let item = cursor.expect_ident("an entity name")?;
            cursor.expect(TokenKind::Gt, "'>'")?;
            match crate::model::ListItem::of_word(&item.name) {
                Some(item) => FieldType::Values { item },
                None if VALUE_WORDS.contains(&item.name.as_str()) => {
                    return Err(Diagnostic::error("a list field holds text, integer, decimal, date or an enum", item.span));
                }
                // An entity, or an enum once the project is loaded.
                None => FieldType::List { item },
            }
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
        "search" => flag(&mut modifiers.search, duplicate),
        "cascade" => flag(&mut modifiers.cascade, duplicate),
        "hashed" => flag(&mut modifiers.hashed, duplicate),
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

/// A click target after `->`: a window or a command word, and the strategy
/// after `signin` (`signin password`).
fn click_target(cursor: &mut Cursor, expected: &str) -> ParseResult<String> {
    let target = cursor.expect_ident(expected)?.name;
    if target == frame::SIGNIN_TARGET {
        let strategy = cursor.expect_ident("a strategy of the project file's signin, as in 'signin password'")?;
        return Ok(format!("{target} {}", strategy.name));
    }
    Ok(target)
}

/// An `allow when <condition>` to the end of the line: the condition as
/// written again and as read.
fn allow_clause(cursor: &mut Cursor) -> ParseResult<(String, crate::code::Expr)> {
    if !cursor.eat_keyword("when") {
        return Err(cursor.error_here("expected 'when' and a condition, as in 'allow when me.role == Admin'".to_string()));
    }
    let condition = crate::code::parse::parse_expr(cursor)?;
    Ok((crate::code::print::expr(&condition), condition))
}

/// A condition written in an attribute: `me.role == Admin`.
pub(crate) fn condition_value(text: &str, at: Span) -> ParseResult<crate::code::Expr> {
    let line = value_line(text, at)?;
    let mut cursor = Cursor::new(&line);
    let condition = crate::code::parse::parse_expr(&mut cursor)?;
    cursor.expect_end()?;
    Ok(condition)
}

/// Page settings (`title`, `size`, `style`, `data`, `grid`) as opposed to widget lines.
pub(crate) fn is_page_setting(tokens: &[Token]) -> bool {
    starts_with_keyword(tokens, "title", Some(TokenKind::Str))
        || starts_with_keyword(tokens, "size", Some(TokenKind::Number))
        || starts_with_keyword(tokens, "style", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "data", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "grid", Some(TokenKind::Number))
        || starts_with_keyword(tokens, "on", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "tells", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "allow", Some(TokenKind::Ident))
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
    let kind_ident = cursor.expect_ident("a page kind ('window', 'dialog', 'view', 'splash', 'toolbox' or 'signin')")?;
    let kind = match kind_ident.name.as_str() {
        "window" => PageKind::Window,
        "dialog" => PageKind::Dialog,
        "view" => PageKind::View,
        "splash" => PageKind::Splash,
        "toolbox" => PageKind::Toolbox,
        "signin" => PageKind::Signin,
        other => {
            return Err(Diagnostic::error(
                format!("unknown page kind '{other}', expected 'window', 'dialog', 'view', 'splash', 'toolbox' or 'signin'"),
                kind_ident.span,
            ));
        }
    };
    // `dialog gives Friend`: what it closes with.
    let gives = if cursor.eat_keyword("gives") { Some(crate::code::parse::parse_type_ref(&mut cursor)?) } else { None };
    cursor.expect_end()?;

    let mut page = Page {
        name,
        kind,
        title: None,
        size: None,
        style: None,
        data: Vec::new(),
        columns: DEFAULT_PAGE_COLUMNS,
        cell: None,
        widgets: Vec::new(),
        frame: Frame::default(),
        handlers: Vec::new(),
        gives,
        tells: Vec::new(),
        allow: None,
        allow_condition: None,
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
        // `style splashBack`: a style of the theme for the window itself.
        "style" => {
            if page.style.is_some() {
                return Err(Diagnostic::error("duplicate 'style'", node.line.span));
            }
            page.style = Some(cursor.expect_ident("a style of the project's theme")?.name);
            cursor.expect_end()?;
        }
        "data" => {
            page.data.push(parse_data(&mut cursor, node.line.span)?);
        }
        // `allow when me.role == Admin`: who may open it.
        "allow" => {
            if page.allow.is_some() {
                return Err(Diagnostic::error("duplicate 'allow'", node.line.span));
            }
            let (written, condition) = allow_clause(&mut cursor)?;
            page.allow = Some(written);
            page.allow_condition = Some(condition);
        }
        // What a view tells its window: `tells saved(name: text)`.
        "tells" => {
            let name = cursor.expect_ident("a tell's name, as in tells saved(name: text)")?;
            let params = if cursor.eat(TokenKind::LParen) { crate::code::parse::parse_params(&mut cursor)? } else { Vec::new() };
            page.tells.push(crate::code::PartMessage { name, params, span: node.span() });
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
            // `rows 10`: the page's own exact grid, rows of 10 pixels.
            if cursor.eat_keyword("rows") {
                page.cell = Some(cursor.expect_number("a row height in pixels, as in 'grid 56 columns rows 10'")?);
            }
        }
    }
    cursor.expect_end()
}

/// `name: Entity`, `name: list<Entity>` or `name: type, default value`,
/// after the `data` keyword. A name is a record here; once every file is
/// read, one that names an enum becomes a plain value (`resolve_data`).
fn parse_data(cursor: &mut Cursor, span: Span) -> ParseResult<DataDecl> {
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
    data_decl(name, ty, default, span)
}

/// A window's or app's data of `name`, of type `ty`, starting at `default`:
/// a record, a list of records or a plain value, by the type.
fn data_decl(name: Ident, ty: crate::code::TypeRef, default: Option<Literal>, span: Span) -> ParseResult<DataDecl> {
    use crate::code::TypeRefKind;
    use crate::model::DataKind;
    let of = match ty.kind {
        // A list of values is a plain value of the window, starting empty (DESIGN 5.26).
        TypeRefKind::List { ref item } if crate::code::parse::is_type_word(item) => {
            if crate::model::ListItem::of_word(item).is_none() {
                return Err(Diagnostic::error("a list of values holds text, integer, decimal or date, as in 'list<decimal>'; a list of records 'list<Order>'", ty.span));
            }
            if default.is_some() {
                return Err(Diagnostic::error(format!("'{}' is a list, which starts empty; it has no default", name.name), ty.span));
            }
            DataKind::Value { ty, default: None }
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
    Ok(DataDecl { name, of, made: false, span })
}

// ---------------------------------------------------------------------------
// Values written in a design file's attributes (DESIGN 5.33): read by the
// rules Clarity's own lines are read by, each where it is in the markup.

/// `text`, an attribute's value at `at`, as one line of Clarity there: each
/// token where it is in the markup.
fn value_line(text: &str, at: Span) -> ParseResult<Line> {
    let (lines, diagnostics) = crate::lexer::lex(text);
    let place = |span: Span| {
        let start = (at.start + span.start).min(at.end);
        Span { start, end: (at.start + span.end).min(at.end), line: at.line, col: at.col + span.start }
    };
    if let Some(d) = diagnostics.into_iter().next() {
        return Err(Diagnostic::error(d.message, place(d.span)));
    }
    let mut lines = lines.into_iter();
    let mut line = lines.next().unwrap_or(Line { indent: 0, tokens: Vec::new(), span: Span::default() });
    if lines.next().is_some() {
        return Err(Diagnostic::error("a value is written on one line", at));
    }
    for token in &mut line.tokens {
        token.span = place(token.span);
    }
    line.span = at;
    Ok(line)
}

/// A type written in an attribute: `Customer`, `list<Order>`, `decimal`.
pub(crate) fn type_value(text: &str, at: Span) -> ParseResult<crate::code::TypeRef> {
    let line = value_line(text, at)?;
    let mut cursor = Cursor::new(&line);
    let ty = crate::code::parse::parse_type_ref(&mut cursor)?;
    cursor.expect_end()?;
    Ok(ty)
}

/// A value written in an attribute, as Clarity writes one: `80`, `true`, `"text"`, `Business`.
pub(crate) fn literal_value(text: &str, at: Span) -> ParseResult<Literal> {
    let line = value_line(text, at)?;
    let mut cursor = Cursor::new(&line);
    let value = parse_literal(&mut cursor)?;
    cursor.expect_end()?;
    Ok(value)
}

/// The values a handler runs with, as in `digit(7, true)`: what is between the brackets.
pub(crate) fn literals_value(text: &str, at: Span) -> ParseResult<Vec<Literal>> {
    let line = value_line(text, at)?;
    let mut cursor = Cursor::new(&line);
    let mut out = Vec::new();
    while cursor.peek().is_some() {
        out.push(parse_literal(&mut cursor)?);
        if cursor.peek().is_some() {
            cursor.expect(TokenKind::Comma, "',' between values")?;
        }
    }
    Ok(out)
}

/// A window's or app's data: `<Data name="customer" type="Customer" default="...">`.
pub(crate) fn data_value(name: Ident, ty: (&str, Span), default: Option<(&str, Span)>, span: Span) -> ParseResult<DataDecl> {
    let ty = type_value(ty.0, ty.1)?;
    let default = default.map(|(text, at)| literal_value(text, at)).transpose()?;
    data_decl(name, ty, default, span)
}

/// What a view tells its window: `<Tells message="saved(name: text)" />`.
pub(crate) fn tells_value(text: &str, at: Span, span: Span) -> ParseResult<crate::code::PartMessage> {
    let line = value_line(text, at)?;
    let mut cursor = Cursor::new(&line);
    let name = cursor.expect_ident("a tell's name, as in saved(name: text)")?;
    let params = if cursor.eat(TokenKind::LParen) { crate::code::parse::parse_params(&mut cursor)? } else { Vec::new() };
    cursor.expect_end()?;
    Ok(crate::code::PartMessage { name, params, span })
}

// ---------------------------------------------------------------------------
// Apps and frame regions

/// App settings (`title`, `main`, `data`) as opposed to regions.
pub(crate) fn is_app_setting(tokens: &[Token]) -> bool {
    starts_with_keyword(tokens, "title", Some(TokenKind::Str))
        || starts_with_keyword(tokens, "main", Some(TokenKind::Ident))
        || starts_with_keyword(tokens, "splash", Some(TokenKind::Ident))
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
        splash: None,
        splash_for: None,
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
        // `splash Welcome for 2 seconds`: the splash page, and how long it shows at least.
        "splash" => {
            if app.splash.is_some() {
                return Err(Diagnostic::error("duplicate 'splash'", node.line.span));
            }
            app.splash = Some(cursor.expect_ident("the splash page")?);
            if cursor.eat_keyword("for") {
                let wrong = || Diagnostic::error("write 'for' and how many seconds the splash shows at least, from 0.1 to 30: splash Welcome for 2 seconds", node.line.span);
                let number = cursor.expect(TokenKind::Number, "a number of seconds").map_err(|_| wrong())?;
                let seconds: f64 = number.text.parse().map_err(|_| wrong())?;
                if !(cursor.eat_keyword("seconds") || cursor.eat_keyword("second")) || !(0.1..=30.0).contains(&seconds) {
                    return Err(wrong());
                }
                app.splash_for = Some((seconds * 1000.0).round() as u32);
            }
            cursor.expect_end()?;
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
            // `allow when me.role == Admin`, to the end of the line.
            "allow" => {
                let (written, condition) = allow_clause(&mut cursor)?;
                props.allow = Some(written);
                props.allow_condition = Some(condition);
            }
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
                props.on_click = Some(click_target(&mut cursor, "a page, screen or command")?);
                // `with book, chapter`: the opened window's data, from the data of the same name here.
                if cursor.eat_keyword("with") {
                    loop {
                        props.open_with.push(cursor.expect_ident("the name of data to open the window with")?.name);
                        if !cursor.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                }
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
    // A word in capitals is a custom part, by the name it is declared by (DESIGN 5.19).
    let custom = keyword.name.starts_with(|c: char| c.is_ascii_uppercase());
    let Some(kind) = widgets::by_keyword(&keyword.name).or(custom.then_some(WidgetKind::Custom)) else {
        return Err(Diagnostic::error(
            format!("unknown widget '{}'", keyword.name),
            keyword.span,
        ));
    };
    let mut props = parse_widget_props(&mut cursor, kind, node.line.span)?;
    if custom {
        props.part = Some(keyword.name.clone());
    }

    let mut widget = Widget {
        props,
        children: Vec::new(),
        pages: Vec::new(),
        menu: None,
        levels: Vec::new(),
        selects: Vec::new(),
        span: node.span(),
    };
    // A `menu` block under a part is its context menu (DESIGN 5.16); the rest is its content.
    let (menus, content): (Vec<&Node>, Vec<&Node>) = node.children.iter().partition(|c| Cursor::new(c.line).peek().is_some_and(|t| t.text == "menu"));
    for (i, menu) in menus.iter().enumerate() {
        if i > 0 {
            diagnostics.push(Diagnostic::error("a part has one context menu", menu.line.span));
            continue;
        }
        let mut cursor = Cursor::new(menu.line);
        cursor.eat_keyword("menu");
        match region_name(&mut cursor, "menu") {
            Ok(name) => {
                let items = parse_commands(&menu.children, &[CommandKind::Item, CommandKind::Separator], diagnostics);
                widget.menu = Some(Menu { name, items, span: menu.span() });
            }
            Err(diag) => diagnostics.push(diag),
        }
    }
    let content: Vec<Node> = content.into_iter().cloned().collect();
    let node = &Node { children: content, ..node.clone() };
    // A tree of several kinds of records: its levels and what it runs when a row is picked.
    if kind == WidgetKind::Tree && !node.children.is_empty() {
        for child in &node.children {
            let cursor = Cursor::new(child.line);
            if cursor.peek().is_some_and(|t| t.text == "on") {
                match parse_tree_select(child) {
                    Ok(select) => widget.selects.push(select),
                    Err(diag) => diagnostics.push(diag),
                }
                continue;
            }
            match parse_tree_level(child, diagnostics) {
                Ok(level) => widget.levels.push(level),
                Err(diag) => diagnostics.push(diag),
            }
        }
        return Ok(widget);
    }
    match widgets::container(kind) {
        None => no_children(node)?,
        Some(ContainerKind::Plain) => widget.children = parse_widgets(&node.children, diagnostics),
        Some(container @ (ContainerKind::Tabs | ContainerKind::Steps | ContainerKind::Accordion | ContainerKind::Content)) => {
            let keyword = match container {
                ContainerKind::Tabs => "tab",
                ContainerKind::Steps => "step",
                ContainerKind::Content => "view",
                _ => "section",
            };
            for child in &node.children {
                match parse_widget_page(child, keyword, diagnostics) {
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

/// `level Domain under project show name order by name`, or `group @dataObjectsNode`,
/// with the levels inside it.
fn parse_tree_level(node: &Node, diagnostics: &mut Vec<Diagnostic>) -> ParseResult<TreeLevel> {
    let mut cursor = Cursor::new(node.line);
    let mut level = TreeLevel { entity: None, under: None, show: None, group: None, order: None, descending: false, levels: Vec::new(), span: node.span() };
    if cursor.eat_keyword("group") {
        level.group = Some(cursor.expect_text("the group's heading, as in group @dataObjectsNode")?);
    } else if cursor.eat_keyword("level") {
        level.entity = Some(cursor.expect_ident("the entity whose records the level shows")?.name);
        loop {
            if cursor.eat_keyword("under") {
                level.under = Some(cursor.expect_ident("the field that refers to the row above, as in under project")?.name);
            } else if cursor.eat_keyword("show") {
                level.show = Some(match cursor.peek().map(|t| t.kind) {
                    Some(TokenKind::Ident) => cursor.expect_ident("what a row shows")?.name,
                    _ => cursor.expect_text("a field or a text key, as in show name or show @classNode")?,
                });
            } else if cursor.eat_keyword("order") {
                if !cursor.eat_keyword("by") {
                    return Err(cursor.error_here("expected 'by' and a field, as in order by name"));
                }
                level.order = Some(cursor.expect_ident("the field the rows are ordered by")?.name);
                level.descending = cursor.eat_keyword("desc");
            } else {
                break;
            }
        }
    } else {
        return Err(cursor.error_here("a tree holds 'level' lines, as in level Project show name, 'group' lines, and 'on select Entity: handler'"));
    }
    cursor.expect_end()?;
    for child in &node.children {
        match parse_tree_level(child, diagnostics) {
            Ok(inner) => level.levels.push(inner),
            Err(diag) => diagnostics.push(diag),
        }
    }
    Ok(level)
}

/// `on select ClassDefinition: classPicked`.
fn parse_tree_select(node: &Node) -> ParseResult<TreeSelect> {
    let mut cursor = Cursor::new(node.line);
    cursor.eat_keyword("on");
    if !cursor.eat_keyword("select") {
        return Err(cursor.error_here("a tree of levels runs a handler when a row is picked: on select Project: projectPicked"));
    }
    let entity = cursor.expect_ident("the entity of the rows, as in on select Project: projectPicked")?.name;
    cursor.expect(TokenKind::Colon, "':' and a handler, as in on select Project: projectPicked")?;
    let handler = cursor.expect_ident("a handler name")?.name;
    cursor.expect_end()?;
    Ok(TreeSelect { entity, handler, span: node.span() })
}

/// `tab "Title"` or `pane`, each with widgets inside.
fn parse_widget_page(
    node: &Node,
    keyword: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> ParseResult<WidgetPage> {
    let mut cursor = Cursor::new(node.line);
    if !cursor.eat_keyword(keyword) {
        let expected = match keyword {
            "tab" => "'tab \"Title\"'",
            "step" => "'step \"Title\"'",
            "section" => "'section \"Title\"'",
            "view" => "'view' and the name of a view, as in view OrderEditor",
            _ => "'pane'",
        };
        return Err(cursor.error_here(format!("expected {expected}")));
    }
    let title = if matches!(keyword, "tab" | "step" | "section") {
        Some(cursor.expect_text(match keyword {
            "tab" => "a tab title",
            "step" => "a step title",
            _ => "a section title",
        })?)
    } else {
        None
    };
    // `section "Chapters" color "#2e86de"`: a section's header in a color of its own.
    let color = if keyword == "section" && cursor.eat_keyword("color") {
        let color = match cursor.peek().map(|t| t.kind) {
            Some(TokenKind::Str) => {
                let span = cursor.peek().map(|t| t.span).unwrap_or(node.span());
                let written = cursor.expect_string("a color, as in color \"#2e86de\"")?;
                if !crate::themes::is_hex(&written) {
                    return Err(Diagnostic::error(format!("'{written}' is not a color; write it as \"#rrggbb\" or \"#rgb\", or name a color of the theme"), span));
                }
                written
            }
            _ => cursor.expect_ident("a color, as in color \"#2e86de\" or color accent")?.name,
        };
        Some(color)
    } else {
        None
    };
    // `section "Class" view ClassEditor`: what it holds is the view's, and
    // `on saved: recipeSaved` hears what the view tells.
    // `view OrderEditor` in a window's content: the keyword is the view's own line.
    let view = if keyword == "view" {
        Some(cursor.expect_ident("the name of the view it shows")?.name)
    } else if keyword != "pane" && cursor.eat_keyword("view") {
        Some(cursor.expect_ident("the name of the view it shows")?.name)
    } else {
        None
    };
    let mut tells = Vec::new();
    while keyword != "pane" && cursor.eat_keyword("on") {
        let tell = cursor.expect_ident("a tell of the view it shows")?.name;
        cursor.expect(TokenKind::Colon, "':' and a handler, as in 'on saved: recipeSaved'")?;
        let handler = cursor.expect_ident("a handler name")?.name;
        tells.push(crate::model::TellHandler { tell, handler });
    }
    let columns = if keyword == "pane" && cursor.eat_keyword("span") {
        let token = cursor.expect(TokenKind::Number, "the columns the pane starts with")?;
        Some(token.text.parse().map_err(|_| Diagnostic::error("a pane's span is a whole number", token.span))?)
    } else {
        None
    };
    cursor.expect_end()?;
    Ok(WidgetPage {
        title,
        columns,
        view,
        tells,
        color,
        children: parse_widgets(&node.children, diagnostics),
        span: node.span(),
    })
}

/// The words of a part's line, which every editor colors as Clarity's own (the VS Code grammar too).
pub const LINE_WORDS: [&str; 85] = [
    "name", "on", "at", "span", "rows", "bind", "columns", "resizable", "sortable", "striped", "order", "reorder", "mode", "tools", "brush", "ink", "edit", "show", "surface", "renderer", "source", "for",
    "runs", "units", "digits", "placeholder", "tooltip", "labels", "volume", "repeat", "points", "every", "split", "by", "count", "sum", "average",
    "children", "page", "typed", "cards", "frame", "search", "zoom", "grid", "snap", "background", "dots", "to", "max", "min", "step", "icon", "style", "readonly", "no",
    "foldable", "folded", "taborder", "key", "parent", "default", "cancel", "tells", "hears", "value", "values", "in", "new", "window", "side", "width", "top", "pinnable", "view", "add", "remove", "confirm", "code", "copy", "markdown", "color", "input", "strategy", "shows",
];

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
        open_with: Vec::new(),
        handlers: Vec::new(),
        columns: Vec::new(),
        edit: Vec::new(),
        links: Vec::new(),
        drops: Vec::new(),
        switches: Vec::new(),
        widths: Default::default(),
        resizable: false,
        side: None,
        beside: None,
        slide_width: None,
        slide_height: None,
        pinnable: false,
        code: None,
        copy: false,
        markdown: false,
        view: None,
        sortable: false,
        add: false,
        remove: false,
        confirm: false,
        striped: false,
        reorder: None,
        reorder_from: None,
        mode: None,
        objects: None,
        tools: Vec::new(),
        toolbox: None,
        toolbox_page: None,
        toolbox_columns: None,
        does: None,
        input: None,
        strategy: None,
        shows: None,
        brush: None,
        ink: None,
        order: None,
        show: Vec::new(),
        orientation: None,
        surface: None,
        source: None,
        for_part: None,
        placeholder: None,
        digits: None,
        units: None,
        zoom: false,
        grid: None,
        snap: false,
        background: None,
        dot_size: None,
        dot_color: None,
        runs: None,
        tooltip: None,
        icon: None,
        to: None,
        max: None,
        min: None,
        step: None,
        split: false,
        cards: false,
        frame: false,
        typed: false,
        page: None,
        children: None,
        row_key: None,
        parent_key: None,
        chart: None,
        by: None,
        measure: None,
        edges: None,
        node_key: None,
        node_label: None,
        node_kind: None,
        edge_from: None,
        edge_to: None,
        edge_label: None,
        edge_kind: None,
        layout: None,
        node_looks: Default::default(),
        edge_looks: Default::default(),
        limit: None,
        lists: Vec::new(),
        picked_kind: None,
        picked_id: None,
        hops: None,
        legend: false,
        graph_model: None,
        search: false,
        labels: None,
        volume: None,
        repeat: false,
        points: None,
        every: None,
        made: false,
        style: None,
        readonly: false,
        no_undo: false,
        undo_scope: None,
        foldable: false,
        folded: false,
        tab_order: None,
        shortcut: None,
        default_button: false,
        cancel: false,
        bottom: false,
        part: None,
        values: Default::default(),
        tells: Vec::new(),
        grows: false,
        description: None,
    };
    // A chart's first word is its kind; an icon part's the icon it shows, no text people read.
    if kind == WidgetKind::Chart {
        let word = cursor.expect_ident("the kind of chart, as in chart bar")?;
        props.chart = Some(crate::model::ChartKind::of_word(&word.name).ok_or_else(|| Diagnostic::error("a chart is bar, line or pie, as in 'chart bar'", word.span))?);
    } else if kind == WidgetKind::Icon {
        props.icon = Some(cursor.expect_string("the icon it shows, as in icon \"triangle-alert\"")?);
    } else if cursor.peek_is_text() {
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
                let written = if property == WidgetProperty::NoUndo { "no undo" } else { property_name };
                Err(Diagnostic::error(
                    format!("'{written}' does not apply to a {name}"),
                    clause.span,
                ))
            }
        };

        // A custom part's own property, or a tell wired to a handler: its
        // declaration says which there are, and the checker holds it to them.
        if kind == WidgetKind::Custom && !widgets::CUSTOM_PART_WORDS.contains(&word) {
            // A text is shown, so it may be a key: `title @levelOne`.
            let value = if cursor.peek_is_text() { Literal::Str(cursor.expect_text("a text")?) } else { parse_literal(cursor)? };
            props.values.insert(word.to_string(), value);
            continue;
        }
        if kind == WidgetKind::Custom && word == "on" && Event::by_keyword(event_word) != Some(Event::Change) {
            let tell = cursor.expect_ident("a tell of the part")?.name;
            cursor.expect(TokenKind::Colon, "':' and a handler, as in 'on scored: arenaScored'")?;
            let handler = cursor.expect_ident("a handler name")?.name;
            props.tells.push(crate::model::TellHandler { tell, handler });
            continue;
        }
        // A tell of the view a slide-out shows (DESIGN 5.33): the view's `tells` say which there are.
        if kind == WidgetKind::Slideout && word == "on" && Event::by_keyword(event_word).is_none() {
            let tell = cursor.expect_ident("a tell of the view the slide-out shows")?.name;
            cursor.expect(TokenKind::Colon, "':' and a handler, as in 'on saved: recipeSaved'")?;
            let handler = cursor.expect_ident("a handler name")?.name;
            props.tells.push(crate::model::TellHandler { tell, handler });
            continue;
        }
        // A tell of the engine beside the screen a canvas runs (DESIGN 5.12):
        // the engine's declaration says which there are.
        if kind == WidgetKind::Canvas && word == "on" && Event::by_keyword(event_word).is_none() {
            let tell = cursor.expect_ident("a tell of the engine the canvas runs")?.name;
            cursor.expect(TokenKind::Colon, "':' and a handler, as in 'on finished: gameOver'")?;
            let handler = cursor.expect_ident("a handler name")?.name;
            props.tells.push(crate::model::TellHandler { tell, handler });
            continue;
        }
        match word {
            "at" | "span" | "rows" if kind == WidgetKind::Sound => {
                return Err(Diagnostic::error("a sound has no place in the window: it is heard, not seen, so it has no 'at', 'span' or 'rows'", clause.span));
            }
            "at" | "span" | "rows" if kind == WidgetKind::Timer => {
                return Err(Diagnostic::error("a timer has no place in the window: it keeps time, not seen, so it has no 'at', 'span' or 'rows'", clause.span));
            }
            "at" if kind == WidgetKind::Popover => {
                return Err(Diagnostic::error("a popover has no 'at': it opens next to the part named in 'for'", clause.span));
            }
            "at" if kind == WidgetKind::Slideout => {
                return Err(Diagnostic::error("a slide-out has no 'at': it comes in from the side of the window named in 'side'", clause.span));
            }
            "for" if kind == WidgetKind::Slideout => {
                return Err(Diagnostic::error("a slide-out has no 'for': it comes in from the side of the window; a part that opens next to another is a popover", clause.span));
            }
            "side" => {
                applies(WidgetProperty::Side)?;
                let word = cursor.expect_ident("left, right, top or bottom, as in side left")?;
                props.side = Some(match word.name.as_str() {
                    "left" => crate::model::SlideSide::Left,
                    "right" => crate::model::SlideSide::Right,
                    "top" => crate::model::SlideSide::Top,
                    "bottom" => crate::model::SlideSide::Bottom,
                    other => return Err(Diagnostic::error(format!("a slide-out comes in from the left, the right, the top or the bottom, not '{other}': write side left, side right, side top or side bottom"), word.span)),
                });
            }
            "does" => {
                applies(WidgetProperty::Does)?;
                props.does = Some(cursor.expect_ident("what the tool does, as in does pencil")?.name);
            }
            "input" => {
                applies(WidgetProperty::Input)?;
                props.input = Some(cursor.expect_ident("the input it takes, as in input password")?.name);
            }
            "strategy" => {
                applies(WidgetProperty::Strategy)?;
                props.strategy = Some(cursor.expect_ident("the strategy it signs in with, as in strategy password")?.name);
            }
            "shows" => {
                applies(WidgetProperty::Shows)?;
                props.shows = Some(cursor.expect_ident("the sign-in view it shows, as in shows CodeView")?.name);
            }
            "beside" => {
                applies(WidgetProperty::Beside)?;
                props.beside = Some(cursor.expect_ident("the part it lies against, as in beside chapterTable")?.name);
            }
            "width" => {
                applies(WidgetProperty::SlideWidth)?;
                props.slide_width = Some(cursor.expect_number("a width in pixels, as in width 420")?);
            }
            "height" if kind == WidgetKind::Slideout => {
                applies(WidgetProperty::SlideHeight)?;
                props.slide_height = Some(cursor.expect_number("a height in pixels, as in height 300")?);
            }
            "pinnable" => {
                applies(WidgetProperty::Pinnable)?;
                props.pinnable = true;
            }
            // `code csharp`: a memo colors its text as code of the language.
            "code" => {
                applies(WidgetProperty::Code)?;
                let language = cursor.expect_ident("a language, as in code csharp")?;
                if !widgets::CODE_LANGUAGES.contains(&language.name.as_str()) {
                    return Err(Diagnostic::error(format!("a memo colors code of {}, not '{}'", widgets::CODE_LANGUAGES.join(", "), language.name), language.span));
                }
                if props.markdown {
                    return Err(Diagnostic::error("a markdown memo colors its Markdown itself; leave out 'code'", language.span));
                }
                props.code = Some(language.name);
            }
            "copy" => {
                applies(WidgetProperty::Copy)?;
                props.copy = true;
            }
            // `markdown`: a memo of Markdown, with Write and Preview.
            "markdown" => {
                applies(WidgetProperty::Markdown)?;
                if props.code.is_some() {
                    return Err(Diagnostic::error("a markdown memo colors its Markdown itself; leave out 'code'", clause.span));
                }
                props.markdown = true;
            }
            "view" => {
                applies(WidgetProperty::View)?;
                props.view = Some(cursor.expect_ident("the view it shows, as in view ClassEditor")?.name);
            }
            "at" => {
                props.col = cursor.expect_number("a column")?;
                cursor.expect(TokenKind::Comma, "',' and a row, as in 'at 1,2'")?;
                props.row = cursor.expect_number("a row")?;
                placed = true;
            }
            "span" => props.width = cursor.expect_number("a number of columns")?,
            "rows" => props.height = cursor.expect_number("a number of rows")?,
            "bottom" if widgets::off_grid(kind) => {
                return Err(Diagnostic::error(format!("a {name} has no place in the window's grid, so no 'bottom'"), clause.span));
            }
            "bottom" => props.bottom = true,
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
                // `on drop from fragmentList: linkFragment`: a row of that part dragged onto this one.
                if event == Event::Drop && cursor.eat_keyword("from") {
                    // A memo takes them too: dropped at a place in its text, the caret goes there first.
                    if !matches!(kind, WidgetKind::List | WidgetKind::Table | WidgetKind::Tree | WidgetKind::TreeTable | WidgetKind::Memo) {
                        return Err(Diagnostic::error(format!("a {name} takes no rows dragged onto it; a list, table, tree or memo does"), event_ident.span));
                    }
                    let from = cursor.expect_ident("the part whose rows are dragged here, as in on drop from fragmentList: linkFragment")?.name;
                    cursor.expect(TokenKind::Colon, "':' and the handler the drop runs")?;
                    let handler = cursor.expect_ident("the handler the drop runs")?.name;
                    props.drops.push(crate::model::DropFrom { from, handler });
                    continue;
                }
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
                props.on_click = Some(click_target(cursor, "a window")?);
                // `with book, chapter`: the opened window's data, from the data of the same name here.
                if cursor.eat_keyword("with") {
                    loop {
                        props.open_with.push(cursor.expect_ident("the name of data to open the window with")?.name);
                        if !cursor.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                }
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
            "add" => {
                applies(WidgetProperty::Add)?;
                props.add = true;
            }
            "remove" => {
                applies(WidgetProperty::Remove)?;
                props.remove = true;
                // `remove confirm`: it asks first.
                if cursor.peek().is_some_and(|t| t.text == "confirm") {
                    cursor.next();
                    props.confirm = true;
                }
            }
            "striped" => {
                applies(WidgetProperty::Striped)?;
                props.striped = true;
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
            // `mode objects types at x, y`, `mode paint`, `mode vector` (DESIGN 5.31).
            "mode" => {
                applies(WidgetProperty::Mode)?;
                let word = cursor.expect_ident("objects, paint or vector, as in mode paint")?;
                props.mode = Some(match word.name.as_str() {
                    "objects" => {
                        let list = cursor.expect_ident("the list whose records are the objects, as in mode objects types at x, y")?.name;
                        if !cursor.eat_keyword("at") {
                            return Err(cursor.error_here("expected 'at', as in mode objects types at x, y".to_string()));
                        }
                        let x = cursor.expect_ident("the field of an object's x")?.name;
                        if !cursor.eat(TokenKind::Comma) {
                            return Err(cursor.error_here("expected ',' and the field of an object's y, as in at x, y".to_string()));
                        }
                        let y = cursor.expect_ident("the field of an object's y")?.name;
                        props.objects = Some(crate::model::CanvasObjects { list, x, y });
                        crate::model::CanvasMode::Objects
                    }
                    "paint" => crate::model::CanvasMode::Paint,
                    "vector" => crate::model::CanvasMode::Vector,
                    other => return Err(Diagnostic::error(format!("a canvas's mode is objects, paint or vector, not '{other}'"), word.span)),
                });
            }
            "toolbox" => {
                applies(WidgetProperty::Toolbox)?;
                // `toolbox` alone floats; `toolbox slideout` lies against the canvas's edge; `toolbox Painting` shows that toolbox page.
                if cursor.peek().is_some_and(|t| t.kind == TokenKind::Ident && t.text.chars().next().is_some_and(char::is_uppercase)) {
                    props.toolbox_page = Some(cursor.expect_ident("a toolbox")?.name);
                } else if kind == WidgetKind::Memo {
                    // A memo has no built-in tools: its toolbox is a page of its own, with coded tools.
                    return Err(Diagnostic::error("a memo's toolbox is a page of its own: toolbox WritingTools, written 'page WritingTools: toolbox'", clause.span));
                }
                // A memo's comes at the caret unless it is written to float.
                props.toolbox = Some(if cursor.eat_keyword("slideout") {
                    crate::model::ToolboxKind::Slideout
                } else if cursor.eat_keyword("popup") || kind != WidgetKind::Memo {
                    crate::model::ToolboxKind::Popup
                } else {
                    crate::model::ToolboxKind::Caret
                });
            }
            "tools" => {
                applies(WidgetProperty::Tools)?;
                loop {
                    props.tools.push(cursor.expect_ident("a tool, as in tools pen, eraser")?.name);
                    if !cursor.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }
            "brush" => {
                applies(WidgetProperty::Brush)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or(clause.span);
                let width = cursor.expect_number("how wide the brush is, in pixels, as in brush 4")?;
                if !(1..=100).contains(&width) {
                    return Err(Diagnostic::error(format!("a brush is 1 to 100 pixels wide, not {width}"), at));
                }
                props.brush = Some(width);
            }
            "ink" => {
                applies(WidgetProperty::Ink)?;
                props.ink = Some(match cursor.peek().map(|t| t.kind) {
                    Some(TokenKind::Str) => cursor.expect_string("a color")?,
                    _ => cursor.expect_ident("a color of the theme, as in ink accent")?.name,
                });
            }
            "reorder" => {
                applies(WidgetProperty::Reorder)?;
                if !cursor.eat_keyword("by") {
                    return Err(cursor.error_here("expected 'by', as in 'reorder by position'".to_string()));
                }
                props.reorder = Some(cursor.expect_ident("the integer field that keeps the order")?.name);
                // `from 0`: the first row's number, for lists that count from 0.
                if cursor.eat_keyword("from") {
                    props.reorder_from = Some(i64::from(cursor.expect_number("the number the first row gets, as in 'from 0'")?));
                }
            }
            "link" => {
                applies(WidgetProperty::Link)?;
                // `link chapter: toChapter, layer: toLayer`: columns whose cells are links.
                loop {
                    let column = cursor.expect_ident("a column, as in link chapter: toChapter")?.name;
                    cursor.expect(TokenKind::Colon, "':' and the handler a click on the link runs")?;
                    let handler = cursor.expect_ident("the handler a click on the link runs")?.name;
                    props.links.push(crate::model::ColumnLink { column, handler });
                    if !cursor.eat(TokenKind::Comma) {
                        break;
                    }
                }
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
            // Graphics an outside library draws are a custom part now (DESIGN 5.19).
            "renderer" => {
                return Err(Diagnostic::error("a canvas is drawn by its draw handler; graphics an outside library draws are a custom part (part Name with native \"folder\")", clause.span));
            }
            "source" => {
                applies(WidgetProperty::Source)?;
                props.source = Some(cursor.expect_string("a file name")?);
            }
            "for" => {
                applies(WidgetProperty::For)?;
                props.for_part = Some(cursor.expect_ident("a part name")?.name);
            }
            "runs" => {
                applies(WidgetProperty::Runs)?;
                props.runs = Some(cursor.expect_ident("the engine beside the screen that draws it, as in 'runs Screen'")?.name);
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
            "labels" => {
                applies(WidgetProperty::Labels)?;
                props.labels = Some(cursor.expect_number("the columns of its label column, as in labels 3")?);
            }
            "volume" => {
                applies(WidgetProperty::Volume)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or_default();
                let volume = cursor.expect_number("a volume from 0 to 100")?;
                if volume > 100 {
                    return Err(Diagnostic::error("a sound's volume is 0 to 100", at));
                }
                props.volume = Some(volume);
            }
            "repeat" => {
                applies(WidgetProperty::Repeat)?;
                props.repeat = true;
            }
            "points" => {
                applies(WidgetProperty::Points)?;
                props.points = Some(cursor.expect_number("how many values it shows, as in points 60")?);
            }
            "every" => {
                applies(WidgetProperty::Every)?;
                props.every = Some(cursor.expect(TokenKind::Number, "a number of seconds, as in every 1")?.text.clone());
            }
            "split" => {
                applies(WidgetProperty::Split)?;
                props.split = true;
            }
            "by" => {
                applies(WidgetProperty::By)?;
                props.by = Some(cursor.expect_ident("the field a chart groups its rows by, as in by status")?.name);
            }
            "count" => {
                applies(WidgetProperty::Measure)?;
                props.measure = Some(crate::model::ChartMeasure::Count);
            }
            "sum" => {
                applies(WidgetProperty::Measure)?;
                props.measure = Some(crate::model::ChartMeasure::Sum { field: cursor.expect_ident("the number field it sums, as in sum total")?.name });
            }
            "average" => {
                applies(WidgetProperty::Measure)?;
                props.measure = Some(crate::model::ChartMeasure::Average { field: cursor.expect_ident("the number field it averages, as in average total")?.name });
            }
            "children" => {
                applies(WidgetProperty::Children)?;
                props.children = Some(cursor.expect_ident("the list field of the rows' own kind, as in children children")?.name);
            }
            "page" => {
                applies(WidgetProperty::Page)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or(clause.span);
                let size = cursor.expect_number("the rows of a page, as in page 50")?;
                if !widgets::PAGE_SIZES.contains(&size) {
                    return Err(Diagnostic::error(widgets::PAGE_PROBLEM.to_string(), at));
                }
                props.page = Some(size);
            }
            "typed" => {
                applies(WidgetProperty::Typed)?;
                props.typed = true;
            }
            "cards" => {
                applies(WidgetProperty::Cards)?;
                props.cards = true;
            }
            "frame" => {
                applies(WidgetProperty::Frame)?;
                props.frame = true;
            }
            "search" => {
                applies(WidgetProperty::Search)?;
                props.search = true;
            }
            "zoom" => {
                applies(WidgetProperty::Zoom)?;
                props.zoom = true;
            }
            "grid" => {
                applies(WidgetProperty::Grid)?;
                let at = cursor.peek().map(|t| t.span).unwrap_or_default();
                let grid = cursor.expect_number("the grid's size in units, as in grid 20")?;
                if !(1..=10000).contains(&grid) {
                    return Err(Diagnostic::error(widgets::CANVAS_GRID_PROBLEM, at));
                }
                props.grid = Some(grid);
            }
            "snap" => {
                applies(WidgetProperty::Snap)?;
                props.snap = true;
            }
            "dots" => {
                applies(WidgetProperty::Dots)?;
                props.dot_size = Some(cursor.expect_number("the dots' size in pixels, as in dots 3")?);
                // Their color, when it follows: a string, or a name that is no word of the line.
                props.dot_color = match cursor.peek_kind() {
                    Some(TokenKind::Str) => Some(cursor.expect_string("a color")?),
                    Some(TokenKind::Ident) if cursor.peek().is_some_and(|t| !LINE_WORDS.contains(&t.text.as_str())) => Some(cursor.expect_ident("a color")?.name),
                    _ => None,
                };
            }
            "background" => {
                applies(WidgetProperty::Background)?;
                props.background = Some(if matches!(cursor.peek_kind(), Some(TokenKind::Str)) {
                    cursor.expect_string("a color, as in background panel or background \"#101810\"")?
                } else {
                    cursor.expect_ident("a color, as in background panel or background \"#101810\"")?.name
                });
            }
            "to" => {
                applies(WidgetProperty::To)?;
                props.to = Some(cursor.expect_string("an address, as in to \"https://lucide.dev\"")?);
            }
            "max" => {
                applies(WidgetProperty::Max)?;
                let what = match kind {
                    WidgetKind::Progress | WidgetKind::StatusItem => "the number it is full at, as in max 250",
                    WidgetKind::Slideout => "the widest it is dragged, in pixels, as in max 900",
                    _ => "the highest value, as in max 100",
                };
                props.max = Some(cursor.expect(TokenKind::Number, what)?.text.clone());
            }
            "min" => {
                applies(WidgetProperty::Min)?;
                let what = if kind == WidgetKind::Slideout { "the narrowest it is dragged, in pixels, as in min 300" } else { "the lowest value, as in min 0" };
                props.min = Some(cursor.expect(TokenKind::Number, what)?.text.clone());
            }
            "step" => {
                applies(WidgetProperty::Step)?;
                props.step = Some(cursor.expect(TokenKind::Number, "how far it moves at a time, as in step 5")?.text.clone());
            }
            "icon" => {
                applies(WidgetProperty::Icon)?;
                props.icon = Some(cursor.expect_string("an icon's name, as in icon \"save\"")?);
            }
            "style" => props.style = Some(cursor.expect_ident("a style of the project's theme")?.name),
            "readonly" => {
                applies(WidgetProperty::Readonly)?;
                props.readonly = true;
            }
            // `no undo`: the part has no undo of its own (DESIGN 5.29).
            "no" => {
                applies(WidgetProperty::NoUndo)?;
                let word = cursor.expect_ident("undo, as in no undo")?;
                if word.name != "undo" {
                    return Err(Diagnostic::error(format!("expected undo after no, as in no undo, not '{}'", word.name), word.span));
                }
                props.no_undo = true;
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
            // A tree's key field (`key id`), beside a shortcut (`key "Ctrl+S"`).
            "key" if widgets::has_property(kind, WidgetProperty::Children) && cursor.peek().is_some_and(|t| t.kind == TokenKind::Ident) => {
                props.row_key = Some(cursor.expect_ident("the field that names a row, as in key id")?.name);
            }
            "parent" => {
                applies(WidgetProperty::Children)?;
                props.parent_key = Some(cursor.expect_ident("the field that holds the key of a row's parent, as in parent parentId")?.name);
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

    // A popover stands outside the window's grid, on a grid of its own; a
    // sound has no place at all.
    if widgets::off_grid(kind) {
        props.col = 1;
        props.row = 1;
        placed = true;
    }
    // A tool in a tool group lies in its flyout, not on a grid: it needs no place.
    if kind == WidgetKind::Tool && !placed {
        props.col = 1;
        props.row = 1;
        placed = true;
    }
    if !placed {
        return Err(Diagnostic::error(
            format!("a {name} needs a position, as in 'at 1,1'"),
            line,
        ));
    }
    if kind == WidgetKind::Slideout {
        widgets::slide_widths_fit(&props).map_err(|m| Diagnostic::error(m, line))?;
    }
    Ok(props)
}
