//! Clarity text in colors: each piece of each line with what it is, read by
//! the lexer, so the editor colors what the language sees. A word is a
//! keyword where the language uses it as one; the same word set, called or
//! named by the code (`title = ...`, `size(3)`, `name title`) is a name.

use serde::Serialize;
use ts_rs::TS;

use crate::lexer::{self, Token, TokenKind};

/// What a piece of text is, which the editor gives its color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum PieceKind {
    /// A word of the language: `engine`, `if`, `button`, `span`.
    Keyword,
    /// A type, or a name that starts with a capital: `integer`, `Order`.
    Type,
    /// A name the project gives: a field, a part, a local, a function.
    Name,
    /// `true`, `false` and `none`.
    Literal,
    Number,
    /// A text as written, with its quotes.
    Text,
    /// `{customer.name}` inside a text.
    Slot,
    /// `@saveCaption`, a text by its key.
    Key,
    /// `// ...` to the end of the line.
    Comment,
}

/// One piece of a line: its line (from 1) and its columns, `from` up to
/// `to`, counted in UTF-16 units as the editor counts them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Piece {
    pub line: u32,
    pub from: u32,
    pub to: u32,
    pub kind: PieceKind,
}

/// The words of the language: declarations, settings, statements, part
/// properties and the frame. The parts themselves and their events come
/// from the widget catalog.
const KEYWORDS: &[&str] = &[
    // Declarations and settings.
    "module", "use", "source", "enum", "entity", "page", "app", "theme", "code", "function", "action", "engine", "handler", "project",
    "stack", "output", "datasource", "filestore", "language", "currency", "auth", "external", "readonly", "translation", "migration",
    "baseline", "title", "main", "data", "size", "grid", "columns", "label", "help", "column", "table", "key", "default", "format",
    "pattern", "min", "max", "required", "unique", "owned", "computed", "store", "from", "uses", "says", "state", "per", "window",
    "keep", "limit", "seconds", "MB", "open", "mode", "color", "font", "radius", "padding", "border", "shadow", "part", "style",
    "background", "weight", "align", "dark", "light", "system", "x", "units", "add", "remove", "rename", "change", "map", "field",
    // Code.
    "before", "after", "save", "delete", "let", "if", "else", "for", "in", "return", "message", "info", "warning", "error", "with",
    "close", "refuse", "reload", "write", "to", "find", "where", "order", "by", "desc", "files", "read", "of", "new", "and", "or",
    "not", "contains", "require", "ensure", "effects", "quit",
    // Parts and the frame.
    "name", "at", "span", "rows", "bind", "placeholder", "tooltip", "taborder", "cancel", "horizontal", "vertical", "edit", "as",
    "switch", "show", "surface", "renderer", "on", "menu", "toolbar", "ribbon", "sidebar", "footer", "item", "separator", "icon",
    "large", "group", "tab", "pane", "left", "right", "width", "resizable", "collapsible", "collapsed", "maximizable", "foldable",
    "folded", "heading", "status",
];

/// Clarity's own types.
const TYPES: &[&str] =
    &["text", "integer", "decimal", "money", "bool", "date", "datetime", "duration", "file", "bytes", "list", "email", "url", "id", "ref", "image"];

const LITERALS: &[&str] = &["true", "false", "none"];

/// Words after which the next word is a name the project gives.
const NAMING: &[&str] =
    &["name", "handler", "function", "action", "let", "for", "data", "keep", "module", "use", "source", "store", "datasource", "filestore"];

/// The pieces of every line of `source`, in order.
pub fn highlight(source: &str) -> Vec<Piece> {
    let (lines, _) = lexer::lex(source);
    let raw: Vec<&str> = source.split('\n').collect();
    // Byte offset where each line starts, to read a token's line text.
    let mut starts = Vec::with_capacity(raw.len());
    let mut offset = 0;
    for line in &raw {
        starts.push(offset);
        offset += line.len() + 1;
    }
    // In a language file, each line is a key and its text.
    let translations = source.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with("//")).is_some_and(|l| l.starts_with("translation "));
    let mut pieces = Vec::new();
    let mut lexed = lines.iter().peekable();
    for (index, text) in raw.iter().enumerate() {
        let number = index as u32 + 1;
        let start = starts[index];
        let tokens: &[Token] = match lexed.peek() {
            Some(line) if line.span.line == number => &lexed.next().expect("peeked").tokens,
            _ => &[],
        };
        let column = |byte: usize| text[..byte.min(text.len())].encode_utf16().count() as u32;
        let mut last = 0;
        for (i, token) in tokens.iter().enumerate() {
            let (from, to) = (token.span.start as usize - start, token.span.end as usize - start);
            last = to;
            let kind = match token.kind {
                TokenKind::Ident if translations && i == 0 && token.text != "translation" => PieceKind::Key,
                TokenKind::Ident => word_kind(tokens, i),
                TokenKind::Number => PieceKind::Number,
                TokenKind::Key => PieceKind::Key,
                TokenKind::Str => {
                    text_pieces(&text[from..to], from, &column, number, &mut pieces);
                    continue;
                }
                _ => continue,
            };
            pieces.push(Piece { line: number, from: column(from), to: column(to), kind });
        }
        // A comment runs from `//` after the last token to the end of the line.
        if let Some(at) = text[last..].find("//") {
            let from = last + at;
            pieces.push(Piece { line: number, from: column(from), to: column(text.trim_end_matches('\r').len()), kind: PieceKind::Comment });
        }
    }
    pieces
}

/// What the word at `i` is, by itself and by the tokens around it.
fn word_kind(tokens: &[Token], i: usize) -> PieceKind {
    let word = tokens[i].text.as_str();
    let before = i.checked_sub(1).map(|b| &tokens[b]);
    let after = tokens.get(i + 1).map(|t| t.kind);
    let named = |kind| if word.starts_with(|c: char| c.is_ascii_uppercase()) { PieceKind::Type } else { kind };
    if LITERALS.contains(&word) {
        return PieceKind::Literal;
    }
    // A member, or a word set, called, indexed or reached into.
    if before.is_some_and(|t| t.kind == TokenKind::Dot)
        || matches!(after, Some(TokenKind::Assign | TokenKind::LParen | TokenKind::LBracket | TokenKind::Dot))
    {
        return named(PieceKind::Name);
    }
    // `on click:` is an event; `squares: list<integer>` a name with its type.
    if after == Some(TokenKind::Colon) && !before.is_some_and(|t| t.kind == TokenKind::Ident && t.text == "on") {
        return named(PieceKind::Name);
    }
    // The name a declaration or setting gives.
    if before.is_some_and(|t| t.kind == TokenKind::Ident && NAMING.contains(&t.text.as_str())) && i >= 1 {
        return named(PieceKind::Name);
    }
    // A part starts its line; elsewhere `text` is a type.
    if i == 0 && crate::widgets::by_keyword(word).is_some() {
        return PieceKind::Keyword;
    }
    if TYPES.contains(&word) {
        return PieceKind::Type;
    }
    if word.starts_with(|c: char| c.is_ascii_uppercase()) && word != "MB" {
        return PieceKind::Type;
    }
    if KEYWORDS.contains(&word) || crate::widgets::by_keyword(word).is_some() || crate::widgets::Event::by_keyword(word).is_some() {
        return PieceKind::Keyword;
    }
    PieceKind::Name
}

/// A text with its quotes, its slots (`{customer.name}`) apart; `{{` and
/// `}}` are braces.
fn text_pieces(written: &str, from: usize, column: &dyn Fn(usize) -> u32, line: u32, pieces: &mut Vec<Piece>) {
    let bytes = written.as_bytes();
    let mut start = 0;
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'{' && bytes.get(at + 1) == Some(&b'{') || bytes[at] == b'}' && bytes.get(at + 1) == Some(&b'}') {
            at += 2;
            continue;
        }
        if bytes[at] == b'{'
            && let Some(close) = written[at..].find('}')
        {
            if at > start {
                pieces.push(Piece { line, from: column(from + start), to: column(from + at), kind: PieceKind::Text });
            }
            let end = at + close + 1;
            pieces.push(Piece { line, from: column(from + at), to: column(from + end), kind: PieceKind::Slot });
            start = end;
            at = end;
            continue;
        }
        at += 1;
    }
    if start < bytes.len() {
        pieces.push(Piece { line, from: column(from + start), to: column(from + bytes.len()), kind: PieceKind::Text });
    }
}
