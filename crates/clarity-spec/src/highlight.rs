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
pub const KEYWORDS: &[&str] = &[
    // Declarations and settings.
    "module", "use", "source", "enum", "entity", "settings", "page", "app", "theme", "code", "function", "action", "engine", "handler", "project",
    "stack", "output", "datasource", "filestore", "language", "currency", "auth", "external", "readonly", "translation", "migration", "port", "keys", "editable", "ui",
    "baseline", "title", "main", "splash", "version", "data", "size", "grid", "columns", "label", "help", "column", "table", "key", "default", "format",
    "pattern", "min", "max", "required", "unique", "owned", "computed", "search", "cascade", "store", "from", "uses", "says", "state", "per", "window",
    "keep", "limit", "seconds", "MB", "open", "mode", "color", "font", "radius", "padding", "border", "shadow", "part", "style",
    "background", "weight", "align", "dark", "light", "system", "x", "units", "add", "remove", "rename", "change", "map", "field",
    // Code.
    "before", "after", "save", "delete", "let", "if", "else", "for", "in", "return", "message", "info", "warning", "error", "with",
    "close", "refuse", "reload", "write", "to", "find", "where", "order", "by", "desc", "files", "read", "of", "new", "and", "or",
    "not", "contains", "matches", "div", "mod", "require", "ensure", "effects", "quit", "cli", "command", "option", "print", "exit", "copy",
    // HTTP calls (DESIGN 5.23).
    "http", "get", "post", "put", "patch", "header", "query", "when", "every",
    // A web API data source: its paging and its OpenAPI description.
    "paging", "offset", "cursor", "next", "items", "link", "openapi", "pick", "schema",
    // Custom parts (DESIGN 5.19).
    "native", "crate", "go", "nuget", "dll", "binds", "tells", "hears", "preview", "grows", "library", "cmake",
    // Engines beside the screen and written in Clarity (DESIGN 5.12).
    "record", "tell", "beside", "engines", "compiled", "interpreted", "const",
    // Parts and the frame.
    "name", "at", "span", "rows", "bottom", "bind", "placeholder", "tooltip", "taborder", "cancel", "horizontal", "vertical", "edit", "as",
    "switch", "show", "surface", "on", "menu", "toolbar", "ribbon", "sidebar", "footer", "item", "separator", "icon",
    "large", "group", "tab", "step", "section", "pane", "width", "resizable", "collapsible", "collapsed", "maximizable", "foldable",
    "folded", "heading", "status", "sortable", "ask", "yes", "no", "then", "gives", "to", "max", "split", "labels",
    // Undo (DESIGN 5.29): `no undo`, and the targets `undo` and `redo`.
    "undo", "redo",
    // Reordering (DESIGN 5.30): `reorder by position`.
    "reorder",
    // Canvas modes (DESIGN 5.31): `mode objects ... | paint | vector`, `tools`, `brush`, `ink`.
    "mode", "objects", "paint", "vector", "tools", "brush", "ink",
    // A tree's levels (DESIGN 5.33): `level Domain under project show name`.
    "level", "under",
    // Mappings (docs/FORMATS-PLAN.md, step 4): `mapping OrderFeed of Order from xml`, `order is ".."`, `check "orders.xsd"`, `using OrderFeed`.
    "mapping", "is", "check", "using",
    // Sign-in (docs/APP-CAPABILITIES-PLAN.md step 4): `hashed`, `me`, `allow when`, `readable when`, `writable when`, the kind and the targets `signin` and `signout`.
    "hashed", "me", "allow", "readable", "writable", "signin", "signout",
    // Versions (docs/APP-CAPABILITIES-PLAN.md step 5): `save customer anyway`, `savedSince @key` on an entity; `version` is above.
    "anyway", "savedSince",
    // Apps as MCP servers (docs/ledger/app-mcp.md): `agent` offers a command to AI agents;
    // the project file's `agents while closed read change` is what they may do while the app is closed.
    "agent", "agents", "while", "closed", "most", "agentReadsWhileClosed", "agentChangesWhileClosed",
    // Toasts: `toast saved` with `text`, `lifespan`, `level`, `icon` and `corner`; `toast saved(name = x)` shows one.
    "toast", "lifespan", "corner", "success", "topRight", "topLeft", "bottomRight", "bottomLeft",
    // A tree's Expand all and Collapse all: `fold buttons` on its line, `tree.foldButtons` in code.
    "fold", "buttons", "foldButtons",
];

/// Clarity's own types.
pub const TYPES: &[&str] =
    &["text", "integer", "decimal", "money", "bool", "date", "datetime", "duration", "file", "bytes", "list", "email", "url", "color", "id", "ref", "image", "picture"];

pub const LITERALS: &[&str] = &["true", "false", "none"];

/// Whether `word` is a word of the language: a keyword, a type, a literal
/// or a part. A name the project gives should not be one.
pub fn is_word_of_the_language(word: &str) -> bool {
    KEYWORDS.contains(&word) || TYPES.contains(&word) || LITERALS.contains(&word) || crate::widgets::catalog().iter().any(|w| w.keyword == word) || is_format(word)
}

/// The options of the project file's signin block, each at the start of its line.
const SIGNIN_LINE_WORDS: &[&str] = &["minimum", "session", "tries", "remember", "client", "secret", "tenant", "issuer", "register"];

/// Whether `word` starts a line of the signin block: an option, or a strategy of Clarity.Auth's catalog.
fn is_signin_line_word(word: &str) -> bool {
    SIGNIN_LINE_WORDS.contains(&word) || crate::signin::strategy_info(word).is_some()
}

/// Whether `word` names a format of Clarity.Formats' catalog (`json`, `csv`, `xml`).
fn is_format(word: &str) -> bool {
    clarity_formats::format(word).is_some()
}

/// Words after which the next word is a name the project gives.
const NAMING: &[&str] =
    &["name", "bind", "handler", "function", "action", "let", "for", "data", "keep", "const", "module", "use", "source", "store", "datasource", "filestore", "mapping"];

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
    // `row height 30`, the project's setting, at the start of its line; `row` alone is a name code gives.
    if (i == 0 && word == "row" && tokens.get(1).is_some_and(|t| t.kind == TokenKind::Ident && t.text == "height"))
        || (i == 1 && word == "height" && tokens[0].kind == TokenKind::Ident && tokens[0].text == "row")
    {
        return PieceKind::Keyword;
    }
    // A line of the project file's signin block: a strategy of Clarity.Auth's catalog and the
    // fields it names (`account name`), or one of its options (`tries 5 per 15 minutes`). Only
    // there: a project may name a field `account` or `session`.
    if i == 0 && tokens.len() > 1 && tokens[1].kind != TokenKind::Assign && tokens[1].kind != TokenKind::Dot && tokens[1].kind != TokenKind::LParen && is_signin_line_word(word) {
        return PieceKind::Keyword;
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
    // A part's line has words of its own (`cards`, `typed`), which the parser names.
    if KEYWORDS.contains(&word) || crate::parser::LINE_WORDS.contains(&word) || crate::widgets::by_keyword(word).is_some() || crate::widgets::Event::by_keyword(word).is_some() || is_format(word) {
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
/// own, the words of a part's line, the parts of the catalog and their
/// events, sorted, each once.
pub fn keywords() -> Vec<String> {
    let mut words: Vec<String> = KEYWORDS.iter().chain(crate::parser::LINE_WORDS.iter()).map(|w| w.to_string()).collect();
    // A custom part is written by its own name, never by the catalog's word for its kind.
    words.extend(crate::widgets::catalog().into_iter().filter(|w| w.kind != crate::widgets::WidgetKind::Custom).map(|w| w.keyword));
    words.extend(crate::widgets::ALL_EVENTS.iter().map(|e| e.keyword().to_string()));
    words.extend(clarity_formats::catalog().into_iter().map(|f| f.name.to_string()));
    words.extend(SIGNIN_LINE_WORDS.iter().map(|w| w.to_string()));
    words.extend(crate::signin::strategy_names().into_iter().map(str::to_string));
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
                    { "include": "#setting" },
                    { "include": "#key" },
                    { "include": "#number" },
                    { "include": "#name" },
                    { "include": "#literal" },
                    { "include": "#type" },
                    { "include": "#keyword" }
                ]
            },
            "setting": {
                "comment": "Settings of two words at the start of a project file's line: `row height 30`. Alone, `row` is a name.",
                "match": "^\\s*(row)\\s+(height)\\b",
                "captures": { "1": { "name": "keyword.other.clarity" }, "2": { "name": "keyword.other.clarity" } }
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

/// A window's or view's `.cxaml` in colored pieces, with the same kinds as
/// Clarity's own: an element's name as a word of the language (a part's kind
/// as a type, as Clarity colors `Order`), an attribute's name as a name, its
/// value as a text, a key (`@saveCaption`), a number or `true` and `false` as
/// such, `{slots}` inside a text, and comments.
pub fn highlight_markup(source: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for (index, text) in source.split('\n').enumerate() {
        let line = index as u32 + 1;
        let column = |byte: usize| text[..byte.min(text.len())].encode_utf16().count() as u32;
        let mut push = |from: usize, to: usize, kind: PieceKind| {
            if to > from {
                pieces.push(Piece { line, from: column(from), to: column(to), kind });
            }
        };
        let bytes = text.as_bytes();
        let mut i = 0;
        let mut in_tag = false;
        while i < bytes.len() {
            let rest = &text[i..];
            if rest.starts_with("<!--") || (!in_tag && rest.trim_start().is_empty()) {
                if rest.starts_with("<!--") {
                    let end = rest.find("-->").map_or(text.len(), |e| i + e + 3);
                    push(i, end, PieceKind::Comment);
                    i = end;
                    continue;
                }
            }
            match bytes[i] {
                b'<' => {
                    in_tag = true;
                    let start = i + 1 + usize::from(rest[1..].starts_with('/') || rest[1..].starts_with('?'));
                    let len = text[start..].find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':' || c == '.')).unwrap_or(text.len() - start);
                    let name = &text[start..start + len];
                    let kind = if name.chars().next().is_some_and(char::is_uppercase) { PieceKind::Type } else { PieceKind::Keyword };
                    push(start, start + len, kind);
                    i = start + len;
                }
                b'>' => {
                    in_tag = false;
                    i += 1;
                }
                b'"' | b'\'' if in_tag => {
                    let quote = bytes[i] as char;
                    let end = text[i + 1..].find(quote).map_or(text.len(), |e| i + 1 + e + 1);
                    let inner = &text[i + 1..end.saturating_sub(1).max(i + 1)];
                    let kind = if inner.starts_with('@') && inner.len() > 1 && inner[1..].chars().all(|c| c.is_alphanumeric() || c == '_') {
                        PieceKind::Key
                    } else if inner == "true" || inner == "false" {
                        PieceKind::Literal
                    } else if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit() || c == ',' || c == '.') {
                        PieceKind::Number
                    } else {
                        PieceKind::Text
                    };
                    push(i, end, kind);
                    // A slot inside a text: `{customer.name}`.
                    if kind == PieceKind::Text {
                        let mut at = i + 1;
                        while let Some(open) = text[at..end].find('{') {
                            let Some(close) = text[at + open..end].find('}') else { break };
                            let (from, to) = (at + open, at + open + close + 1);
                            push(from, to, PieceKind::Slot);
                            at = to;
                        }
                    }
                    i = end;
                }
                c if in_tag && (c as char).is_alphabetic() => {
                    let len = rest.find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':' || c == '-')).unwrap_or(rest.len());
                    push(i, i + len, PieceKind::Name);
                    i += len;
                }
                _ => i += 1,
            }
        }
    }
    pieces
}
