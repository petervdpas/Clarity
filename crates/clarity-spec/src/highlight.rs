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
    "large", "group", "tab", "step", "section", "pane", "width", "resizable", "collapsible", "collapsed", "maximizable", "foldable",
    "folded", "heading", "status", "sortable", "ask", "yes", "no", "then", "gives", "to", "max", "split", "labels",
];

/// Clarity's own types.
const TYPES: &[&str] =
    &["text", "integer", "decimal", "money", "bool", "date", "datetime", "duration", "file", "bytes", "list", "email", "url", "id", "ref", "image"];

const LITERALS: &[&str] = &["true", "false", "none"];

/// Whether `word` is a word of the language: a keyword, a type, a literal
/// or a part. A name the project gives should not be one.
pub fn is_word_of_the_language(word: &str) -> bool {
    KEYWORDS.contains(&word) || TYPES.contains(&word) || LITERALS.contains(&word) || crate::widgets::catalog().iter().any(|w| w.keyword == word)
}

/// Words after which the next word is a name the project gives.
const NAMING: &[&str] =
    &["name", "bind", "handler", "function", "action", "let", "for", "data", "keep", "module", "use", "source", "store", "datasource", "filestore"];

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
                // `gameRules resource "rules-nl.md"`: a key that points at a file of the resources.
                TokenKind::Ident if translations && i == 1 && token.text == "resource" => PieceKind::Keyword,
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

/// Every word the highlighter colors as a keyword somewhere: the language's
/// own, the parts of the catalog and their events, sorted, each once.
pub fn keywords() -> Vec<String> {
    let mut words: Vec<String> = KEYWORDS.iter().map(|w| w.to_string()).collect();
    words.extend(crate::widgets::catalog().into_iter().map(|w| w.keyword));
    words.extend(crate::widgets::ALL_EVENTS.iter().map(|e| e.keyword().to_string()));
    words.sort();
    words.dedup();
    words
}

/// The scope name of `.clarity` files in editors that read TextMate grammars.
pub const SCOPE: &str = "source.clarity";

/// A TextMate grammar for `.clarity` files (VS Code, JetBrains, GitHub), with
/// the words this highlighter knows, so other editors color Clarity alike. A
/// TextMate grammar sees one line at a time and no tokens, so it keeps the
/// main rules: a word set, called or reached into is a name, a capital
/// starts a type, and the first word of a line in a language file is a key.
pub fn textmate_grammar() -> serde_json::Value {
    let alternatives = |words: &[String]| words.iter().map(|w| regex_escape(w)).collect::<Vec<_>>().join("|");
    let types: Vec<String> = TYPES.iter().map(|w| w.to_string()).collect();
    let literals: Vec<String> = LITERALS.iter().map(|w| w.to_string()).collect();
    let naming: Vec<String> = NAMING.iter().map(|w| w.to_string()).collect();
    serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
        "name": "Clarity",
        "scopeName": SCOPE,
        "fileTypes": ["clarity"],
        "patterns": [
            { "include": "#translation" },
            { "include": "#code" }
        ],
        "repository": {
            "translation": {
                "comment": "A language file: each line a key and its text.",
                "begin": "^(translation)\\s+([A-Za-z_][A-Za-z0-9_-]*)",
                "beginCaptures": { "1": { "name": "keyword.other.clarity" }, "2": { "name": "constant.language.clarity" } },
                "end": "(?!)",
                "patterns": [
                    { "include": "#comment" },
                    { "match": "^\\s+([A-Za-z_][A-Za-z0-9_]*)\\s+(resource)\\b", "captures": { "1": { "name": "variable.other.constant.key.clarity" }, "2": { "name": "keyword.other.clarity" } } },
                    { "match": "^\\s+([A-Za-z_][A-Za-z0-9_]*)", "captures": { "1": { "name": "variable.other.constant.key.clarity" } } },
                    { "include": "#text" }
                ]
            },
            "code": {
                "patterns": [
                    { "include": "#comment" },
                    { "include": "#text" },
                    { "include": "#key" },
                    { "include": "#number" },
                    { "include": "#name" },
                    { "include": "#literal" },
                    { "include": "#type" },
                    { "include": "#keyword" }
                ]
            },
            "comment": { "match": "//.*$", "name": "comment.line.double-slash.clarity" },
            "text": {
                "begin": "\"",
                "end": "\"",
                "name": "string.quoted.double.clarity",
                "patterns": [
                    { "match": "\\\\.", "name": "constant.character.escape.clarity" },
                    { "match": "\\{\\{|\\}\\}", "name": "constant.character.escape.clarity" },
                    { "match": "\\{[^{}\"]*\\}", "name": "variable.other.slot.clarity" }
                ]
            },
            "key": { "match": "@[A-Za-z_][A-Za-z0-9_]*", "name": "variable.other.constant.key.clarity" },
            "number": { "match": "(?<![A-Za-z0-9_])-?[0-9]+(\\.[0-9]+)?\\b", "name": "constant.numeric.clarity" },
            "name": {
                "comment": "A word set, called or reached into is a name, whatever it is elsewhere.",
                "patterns": [
                    {
                        "match": format!("\\b({})\\s+([a-z_][A-Za-z0-9_]*)\\b", alternatives(&naming)),
                        "captures": { "1": { "name": "keyword.other.clarity" }, "2": { "name": "variable.other.clarity" } }
                    },
                    { "match": "(?<=\\.)[A-Za-z_][A-Za-z0-9_]*", "name": "variable.other.property.clarity" },
                    { "match": "\\b([a-z_][A-Za-z0-9_]*)(?=\\s*\\()", "name": "entity.name.function.clarity" },
                    { "match": "(?<!\\bon\\s)\\b([a-z_][A-Za-z0-9_]*)(?=\\s*(=(?!=)|\\[|:))", "name": "variable.other.clarity" }
                ]
            },
            "literal": { "match": format!("\\b({})\\b", alternatives(&literals)), "name": "constant.language.clarity" },
            "type": {
                "patterns": [
                    { "match": format!("\\b({})\\b", alternatives(&types)), "name": "storage.type.clarity" },
                    { "match": "\\b[A-Z][A-Za-z0-9_]*\\b", "name": "entity.name.type.clarity" }
                ]
            },
            "keyword": { "match": format!("\\b({})\\b", alternatives(&keywords())), "name": "keyword.other.clarity" }
        }
    })
}

/// A word as it stands in a regular expression.
fn regex_escape(word: &str) -> String {
    word.chars().flat_map(|c| if c.is_ascii_alphanumeric() || c == '_' { vec![c] } else { vec!['\\', c] }).collect()
}
