//! The colors of code in a memo (`memo ... code csharp`): a memo of code
//! writes in the mono font and colors each language's comments, texts,
//! numbers, keywords and type names. The colors are the theme's: the roles
//! `code_keyword`, `code_type`, `code_text`, `code_number` and
//! `code_comment`, and without them Clarity's accent, success, warning and
//! muted colors, so every theme colors code its own way.

/// The languages a memo can color, by the word `code` takes.
pub const LANGUAGES: [&str; 18] = [
    "clarity", "csharp", "rust", "go", "c", "cpp", "java", "javascript", "typescript", "python", "sql", "json", "xml", "html", "css", "yaml", "markdown", "shell",
];

/// What a stretch of code is, which says its color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    Keyword,
    Type,
    Text,
    Number,
    Comment,
}

impl Ink {
    /// The theme's color role for it.
    pub fn role(self) -> &'static str {
        match self {
            Ink::Keyword => "code_keyword",
            Ink::Type => "code_type",
            Ink::Text => "code_text",
            Ink::Number => "code_number",
            Ink::Comment => "code_comment",
        }
    }

    /// Clarity's own color it takes when the theme gives none for its role.
    pub fn fallback(self) -> &'static str {
        match self {
            Ink::Keyword => "accent",
            Ink::Type => "success",
            Ink::Text | Ink::Number => "warning",
            Ink::Comment => "muted",
        }
    }
}

/// How a language is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// Words, texts, numbers and comments, as most programming languages are.
    Words,
    /// Tags with attributes: XML and HTML.
    Markup,
    /// Headings and inline code.
    Markdown,
    /// Keys before a colon.
    Yaml,
    /// Keys and values.
    Json,
}

/// A language and what it colors.
#[derive(Debug)]
pub struct Language {
    pub name: &'static str,
    shape: Shape,
    line_comments: &'static [&'static str],
    block_comments: &'static [(&'static str, &'static str)],
    quotes: &'static [char],
    keywords: &'static [&'static str],
    types: &'static [&'static str],
    /// A name that starts with a capital is a type: `Recipe`.
    capital_types: bool,
    /// Its keywords in any case: `SELECT`, `select`.
    any_case: bool,
    /// `#` starts a comment only at the start of a line or after a space (`$#` is no comment).
    hash_after_space: bool,
    /// `'` quotes one character, as `'a'` or `'\n'`; else it is a lifetime (`'a`).
    char_quotes: bool,
}

const C_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "const", "continue", "default", "do", "else", "enum", "extern", "for", "goto", "if", "inline", "register", "restrict", "return", "sizeof", "static", "struct", "switch", "typedef", "union", "volatile", "while", "NULL", "true", "false",
];
const C_TYPES: &[&str] = &["char", "double", "float", "int", "long", "short", "signed", "unsigned", "void", "bool", "size_t", "int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t"];
const CPP_KEYWORDS: &[&str] = &[
    "auto", "break", "case", "catch", "class", "const", "constexpr", "continue", "default", "delete", "do", "else", "enum", "explicit", "export", "extern", "false", "for", "friend", "goto", "if", "inline", "mutable", "namespace", "new", "noexcept", "nullptr", "operator",
    "override", "private", "protected", "public", "return", "sizeof", "static", "struct", "switch", "template", "this", "throw", "true", "try", "typedef", "typename", "union", "using", "virtual", "volatile", "while", "co_await", "co_return", "final",
];
const CPP_TYPES: &[&str] = &["bool", "char", "double", "float", "int", "long", "short", "signed", "unsigned", "void", "size_t", "string", "vector", "map", "optional", "unique_ptr", "shared_ptr"];
const CSHARP_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "base", "break", "case", "catch", "checked", "class", "const", "continue", "default", "delegate", "do", "else", "enum", "event", "explicit", "extern", "false", "finally", "fixed", "for", "foreach", "get", "goto", "if", "implicit",
    "in", "init", "interface", "internal", "is", "lock", "namespace", "new", "null", "operator", "out", "override", "params", "partial", "private", "protected", "public", "readonly", "record", "ref", "required", "return", "sealed", "set", "sizeof", "static", "struct",
    "switch", "this", "throw", "true", "try", "typeof", "unchecked", "unsafe", "using", "value", "var", "virtual", "void", "volatile", "when", "where", "while", "yield",
];
const CSHARP_TYPES: &[&str] = &["bool", "byte", "char", "decimal", "double", "dynamic", "float", "int", "long", "object", "sbyte", "short", "string", "uint", "ulong", "ushort", "nint", "nuint"];
const JAVA_KEYWORDS: &[&str] = &[
    "abstract", "assert", "break", "case", "catch", "class", "const", "continue", "default", "do", "else", "enum", "extends", "final", "finally", "for", "goto", "if", "implements", "import", "instanceof", "interface", "native", "new", "null", "package", "private",
    "protected", "public", "record", "return", "static", "strictfp", "super", "switch", "synchronized", "this", "throw", "throws", "transient", "true", "false", "try", "var", "void", "volatile", "while", "yield",
];
const JAVA_TYPES: &[&str] = &["boolean", "byte", "char", "double", "float", "int", "long", "short"];
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true",
    "type", "unsafe", "use", "where", "while",
];
const RUST_TYPES: &[&str] = &["bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "str", "u8", "u16", "u32", "u64", "u128", "usize", "Self"];
const GO_KEYWORDS: &[&str] = &[
    "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough", "for", "func", "go", "goto", "if", "import", "interface", "map", "package", "range", "return", "select", "struct", "switch", "type", "var", "true", "false", "nil",
];
const GO_TYPES: &[&str] = &[
    "any", "bool", "byte", "complex64", "complex128", "error", "float32", "float64", "int", "int8", "int16", "int32", "int64", "rune", "string", "uint", "uint8", "uint16", "uint32", "uint64", "uintptr",
];
const JS_KEYWORDS: &[&str] = &[
    "async", "await", "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete", "do", "else", "export", "extends", "false", "finally", "for", "from", "function", "if", "import", "in", "instanceof", "let", "new", "null", "of", "return",
    "static", "super", "switch", "this", "throw", "true", "try", "typeof", "undefined", "var", "void", "while", "with", "yield",
];
const TS_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "break", "case", "catch", "class", "const", "continue", "declare", "default", "delete", "do", "else", "enum", "export", "extends", "false", "finally", "for", "from", "function", "if", "implements", "import", "in", "instanceof",
    "interface", "keyof", "let", "namespace", "new", "null", "of", "private", "protected", "public", "readonly", "return", "satisfies", "static", "super", "switch", "this", "throw", "true", "try", "type", "typeof", "undefined", "var", "void", "while", "yield",
];
const TS_TYPES: &[&str] = &["any", "bigint", "boolean", "never", "number", "object", "string", "symbol", "unknown"];
const PYTHON_KEYWORDS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif", "else", "except", "False", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return", "True",
    "try", "while", "with", "yield", "match", "case", "self",
];
const PYTHON_TYPES: &[&str] = &["bool", "bytes", "dict", "float", "int", "list", "object", "set", "str", "tuple"];
const SQL_KEYWORDS: &[&str] = &[
    "add", "all", "alter", "and", "as", "asc", "between", "by", "case", "check", "column", "constraint", "create", "default", "delete", "desc", "distinct", "drop", "else", "end", "exists", "foreign", "from", "group", "having", "if", "in", "index", "inner", "insert",
    "into", "is", "join", "key", "left", "like", "limit", "not", "null", "offset", "on", "or", "order", "outer", "primary", "references", "right", "select", "set", "table", "then", "union", "unique", "update", "values", "view", "when", "where", "with",
];
const SQL_TYPES: &[&str] = &["bigint", "blob", "boolean", "char", "date", "datetime", "decimal", "float", "int", "integer", "numeric", "real", "smallint", "text", "time", "timestamp", "varchar", "uuid"];
const CSS_KEYWORDS: &[&str] = &["important", "media", "import", "keyframes", "from", "to", "supports", "font-face", "root", "hover", "focus", "active", "before", "after"];
const SHELL_KEYWORDS: &[&str] = &[
    "case", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if", "in", "local", "readonly", "return", "select", "then", "until", "while", "echo", "exit", "set", "unset", "source",
];
const CLARITY_KEYWORDS: &[&str] = &[
    "module", "use", "project", "app", "page", "window", "dialog", "view", "data", "entity", "enum", "record", "function", "action", "handler", "code", "engine", "part", "settings", "theme", "translation", "migration", "if", "else", "for", "in", "to", "return", "let",
    "save", "delete", "reload", "find", "where", "order", "by", "desc", "and", "or", "not", "true", "false", "new", "none", "tell", "tells", "hears", "message", "ask", "open", "close", "refuse", "print", "exit", "copy", "write", "on", "effects", "cli", "command",
];
const CLARITY_TYPES: &[&str] = &["text", "integer", "decimal", "money", "bool", "date", "datetime", "duration", "list", "ref", "file", "folder", "color", "picture", "secret", "bytes", "email", "url", "values", "id"];

const LANGUAGE_LIST: [Language; 18] = [
    Language { name: "clarity", shape: Shape::Words, line_comments: &["//"], block_comments: &[], quotes: &['"'], keywords: CLARITY_KEYWORDS, types: CLARITY_TYPES, capital_types: true, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "csharp", shape: Shape::Words, line_comments: &["//"], block_comments: &[("/*", "*/")], quotes: &['"', '\''], keywords: CSHARP_KEYWORDS, types: CSHARP_TYPES, capital_types: true, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "rust", shape: Shape::Words, line_comments: &["//"], block_comments: &[("/*", "*/")], quotes: &['"', '\''], keywords: RUST_KEYWORDS, types: RUST_TYPES, capital_types: true, any_case: false, hash_after_space: false, char_quotes: true },
    Language { name: "go", shape: Shape::Words, line_comments: &["//"], block_comments: &[("/*", "*/")], quotes: &['"', '\'', '`'], keywords: GO_KEYWORDS, types: GO_TYPES, capital_types: false, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "c", shape: Shape::Words, line_comments: &["//"], block_comments: &[("/*", "*/")], quotes: &['"', '\''], keywords: C_KEYWORDS, types: C_TYPES, capital_types: false, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "cpp", shape: Shape::Words, line_comments: &["//"], block_comments: &[("/*", "*/")], quotes: &['"', '\''], keywords: CPP_KEYWORDS, types: CPP_TYPES, capital_types: true, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "java", shape: Shape::Words, line_comments: &["//"], block_comments: &[("/*", "*/")], quotes: &['"', '\''], keywords: JAVA_KEYWORDS, types: JAVA_TYPES, capital_types: true, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "javascript", shape: Shape::Words, line_comments: &["//"], block_comments: &[("/*", "*/")], quotes: &['"', '\'', '`'], keywords: JS_KEYWORDS, types: &[], capital_types: true, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "typescript", shape: Shape::Words, line_comments: &["//"], block_comments: &[("/*", "*/")], quotes: &['"', '\'', '`'], keywords: TS_KEYWORDS, types: TS_TYPES, capital_types: true, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "python", shape: Shape::Words, line_comments: &["#"], block_comments: &[], quotes: &['"', '\''], keywords: PYTHON_KEYWORDS, types: PYTHON_TYPES, capital_types: true, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "sql", shape: Shape::Words, line_comments: &["--"], block_comments: &[("/*", "*/")], quotes: &['\'', '"'], keywords: SQL_KEYWORDS, types: SQL_TYPES, capital_types: false, any_case: true, hash_after_space: false, char_quotes: false },
    Language { name: "json", shape: Shape::Json, line_comments: &[], block_comments: &[], quotes: &['"'], keywords: &["true", "false", "null"], types: &[], capital_types: false, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "xml", shape: Shape::Markup, line_comments: &[], block_comments: &[("<!--", "-->")], quotes: &['"', '\''], keywords: &[], types: &[], capital_types: false, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "html", shape: Shape::Markup, line_comments: &[], block_comments: &[("<!--", "-->")], quotes: &['"', '\''], keywords: &[], types: &[], capital_types: false, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "css", shape: Shape::Words, line_comments: &[], block_comments: &[("/*", "*/")], quotes: &['"', '\''], keywords: CSS_KEYWORDS, types: &[], capital_types: false, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "yaml", shape: Shape::Yaml, line_comments: &["#"], block_comments: &[], quotes: &['"', '\''], keywords: &["true", "false", "null", "yes", "no"], types: &[], capital_types: false, any_case: false, hash_after_space: true, char_quotes: false },
    Language { name: "markdown", shape: Shape::Markdown, line_comments: &[], block_comments: &[], quotes: &[], keywords: &[], types: &[], capital_types: false, any_case: false, hash_after_space: false, char_quotes: false },
    Language { name: "shell", shape: Shape::Words, line_comments: &["#"], block_comments: &[], quotes: &['"', '\''], keywords: SHELL_KEYWORDS, types: &[], capital_types: false, any_case: false, hash_after_space: true, char_quotes: false },
];

/// The language named `name`, as `code` writes it; none for a language Clarity does not color.
pub fn language(name: &str) -> Option<&'static Language> {
    LANGUAGE_LIST.iter().find(|l| l.name == name)
}

/// The colored stretches of `text` in `language`, in order: from, to (character places) and what each is. What lies between keeps the text's color.
pub fn spans(language: &Language, text: &[char]) -> Vec<(usize, usize, Ink)> {
    match language.shape {
        Shape::Words => words(language, text),
        Shape::Markup => markup(language, text),
        Shape::Markdown => markdown(text),
        Shape::Yaml => yaml(language, text),
        Shape::Json => json(text),
    }
}

fn starts(text: &[char], at: usize, word: &str) -> bool {
    let mut i = at;
    for c in word.chars() {
        if text.get(i) != Some(&c) {
            return false;
        }
        i += 1;
    }
    true
}

fn is_word_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Where the text quoted with `quote` at `at` ends, after its closing quote (or at the line's end for an unclosed one, a backtick's at the text's end).
fn text_end(text: &[char], at: usize, quote: char) -> usize {
    let mut i = at + 1;
    while i < text.len() {
        let c = text[i];
        if c == '\\' && quote != '`' {
            i += 2;
            continue;
        }
        if c == quote {
            return i + 1;
        }
        if c == '\n' && quote != '`' {
            return i;
        }
        i += 1;
    }
    text.len()
}

fn line_end(text: &[char], at: usize) -> usize {
    text[at..].iter().position(|c| *c == '\n').map_or(text.len(), |n| at + n)
}

/// Where a comment starts at `at`, if one does: its end.
fn comment_at(language: &Language, text: &[char], at: usize) -> Option<usize> {
    for line in language.line_comments {
        if starts(text, at, line) {
            if *line == "#" && language.hash_after_space && at > 0 && !text[at - 1].is_whitespace() {
                continue;
            }
            return Some(line_end(text, at));
        }
    }
    for (open, close) in language.block_comments {
        if starts(text, at, open) {
            let from = at + open.chars().count();
            let close_len = close.chars().count();
            let end = (from..text.len()).find(|&i| starts(text, i, close)).map_or(text.len(), |i| i + close_len);
            return Some(end);
        }
    }
    None
}

fn number_end(text: &[char], at: usize) -> usize {
    let mut i = at;
    while i < text.len() && (text[i].is_alphanumeric() || text[i] == '.' || text[i] == '_') {
        i += 1;
    }
    i
}

fn words(language: &Language, text: &[char]) -> Vec<(usize, usize, Ink)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let c = text[i];
        if let Some(end) = comment_at(language, text, i) {
            out.push((i, end, Ink::Comment));
            i = end;
            continue;
        }
        if language.quotes.contains(&c) {
            // A lifetime (`'a`) quotes nothing; a character is `'a'` or `'\n'`.
            if c == '\'' && language.char_quotes && !(text.get(i + 1) == Some(&'\\') || text.get(i + 2) == Some(&'\'')) {
                i += 1;
                continue;
            }
            let end = text_end(text, i, c);
            out.push((i, end, Ink::Text));
            i = end;
            continue;
        }
        if c.is_ascii_digit() && (i == 0 || !is_word(text[i - 1])) {
            let end = number_end(text, i);
            out.push((i, end, Ink::Number));
            i = end;
            continue;
        }
        if is_word_start(c) {
            let mut end = i;
            while end < text.len() && (is_word(text[end]) || (language.name == "css" && text[end] == '-')) {
                end += 1;
            }
            let word: String = text[i..end].iter().collect();
            let keyword = if language.any_case { language.keywords.iter().any(|k| k.eq_ignore_ascii_case(&word)) } else { language.keywords.contains(&word.as_str()) };
            let typed = if language.any_case { language.types.iter().any(|k| k.eq_ignore_ascii_case(&word)) } else { language.types.contains(&word.as_str()) };
            let capital = language.capital_types && word.starts_with(|c: char| c.is_uppercase()) && word.chars().any(|c| c.is_lowercase());
            if keyword {
                out.push((i, end, Ink::Keyword));
            } else if typed || (capital && !declared(text, i, &out)) {
                out.push((i, end, Ink::Type));
            }
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// Whether the word at `at` is a name being declared, which a type comes before (`int Serves`, `List<Recipe> Items`): a name, not a type, capital or not.
fn declared(text: &[char], at: usize, found: &[(usize, usize, Ink)]) -> bool {
    let mut before = at;
    while before > 0 && text[before - 1] == ' ' {
        before -= 1;
    }
    if before == at || before == 0 {
        return false;
    }
    matches!(text[before - 1], '>' | ']' | '?') || found.last().is_some_and(|(_, end, ink)| *end == before && *ink == Ink::Type)
}

fn markup(language: &Language, text: &[char]) -> Vec<(usize, usize, Ink)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if let Some(end) = comment_at(language, text, i) {
            out.push((i, end, Ink::Comment));
            i = end;
            continue;
        }
        if text[i] != '<' {
            i += 1;
            continue;
        }
        // A tag: its name, then its attributes and their values, to its `>`.
        i += 1;
        while i < text.len() && matches!(text[i], '/' | '?' | '!') {
            i += 1;
        }
        let name = i;
        while i < text.len() && (is_word(text[i]) || matches!(text[i], '-' | ':' | '.')) {
            i += 1;
        }
        if i > name {
            out.push((name, i, Ink::Keyword));
        }
        while i < text.len() && text[i] != '>' {
            let c = text[i];
            if c == '"' || c == '\'' {
                let end = text_end(text, i, c);
                out.push((i, end, Ink::Text));
                i = end;
            } else if is_word_start(c) {
                let from = i;
                while i < text.len() && (is_word(text[i]) || matches!(text[i], '-' | ':' | '.')) {
                    i += 1;
                }
                out.push((from, i, Ink::Type));
            } else {
                i += 1;
            }
        }
    }
    out
}

fn markdown(text: &[char]) -> Vec<(usize, usize, Ink)> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut fenced = false;
    while i < text.len() {
        let end = line_end(text, i);
        let line: String = text[i..end].iter().collect();
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            fenced = !fenced;
            out.push((i, end, Ink::Text));
        } else if fenced {
            out.push((i, end, Ink::Text));
        } else if trimmed.starts_with('#') {
            out.push((i, end, Ink::Keyword));
        } else {
            let mut j = i;
            while j < end {
                if text[j] == '`' {
                    let close = (j + 1..end).find(|&k| text[k] == '`').map_or(end, |k| k + 1);
                    out.push((j, close, Ink::Text));
                    j = close;
                } else {
                    j += 1;
                }
            }
        }
        i = end + 1;
    }
    out
}

fn yaml(language: &Language, text: &[char]) -> Vec<(usize, usize, Ink)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let end = line_end(text, i);
        // A key: the line's first word, after its indent and a list's dash, before a colon.
        let mut j = i;
        while j < end && (text[j] == ' ' || text[j] == '-') {
            j += 1;
        }
        let key = j;
        while j < end && (is_word(text[j]) || text[j] == '-' || text[j] == '.') {
            j += 1;
        }
        if j > key && text.get(j) == Some(&':') {
            out.push((key, j, Ink::Type));
        }
        // The rest of the line as words: comments, texts, numbers and keywords.
        for (from, to, ink) in words(language, &text[j..end]) {
            out.push((j + from, j + to, ink));
        }
        i = end + 1;
    }
    out
}

fn json(text: &[char]) -> Vec<(usize, usize, Ink)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let c = text[i];
        if c == '"' {
            let end = text_end(text, i, '"');
            // A key is a text a colon follows.
            let after = (end..text.len()).find(|&k| !text[k].is_whitespace());
            let ink = if after.is_some_and(|k| text[k] == ':') { Ink::Type } else { Ink::Text };
            out.push((i, end, ink));
            i = end;
        } else if c.is_ascii_digit() || (c == '-' && text.get(i + 1).is_some_and(|n| n.is_ascii_digit())) {
            let end = number_end(text, i + 1);
            out.push((i, end, Ink::Number));
            i = end;
        } else if is_word_start(c) {
            let mut end = i;
            while end < text.len() && is_word(text[end]) {
                end += 1;
            }
            if matches!(text[i..end].iter().collect::<String>().as_str(), "true" | "false" | "null") {
                out.push((i, end, Ink::Keyword));
            }
            i = end;
        } else {
            i += 1;
        }
    }
    out
}
