//! The CSS a stylesheet is written in, read into blocks of declarations: a
//! selector or an at-rule (`@page`, its margin boxes inside it), and its
//! `name: value` lines, each with the line it starts on. Comments are
//! dropped; a block that is never closed is an error.

use crate::{CssError, CssErrorKind};

/// A block: what comes before its `{` and what is in it.
#[derive(Debug, Clone)]
pub(crate) struct Block {
    pub prelude: String,
    pub line: usize,
    pub decls: Vec<Decl>,
    /// At-rules inside it: the margin boxes of `@page`.
    pub nested: Vec<Block>,
}

/// A declaration: `name: value`.
#[derive(Debug, Clone)]
pub(crate) struct Decl {
    pub name: String,
    pub value: String,
    pub line: usize,
}

struct Reader {
    chars: Vec<char>,
    at: usize,
    line: usize,
}

impl Reader {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.chars.get(self.at).copied()?;
        self.at += 1;
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.next();
        }
    }

    /// Reads up to (not over) one of `ends`, outside quotes and brackets.
    fn until(&mut self, ends: &[char]) -> String {
        let mut out = String::new();
        let mut quote: Option<char> = None;
        let mut depth = 0usize;
        while let Some(c) = self.peek() {
            match quote {
                Some(q) => {
                    if c == q {
                        quote = None;
                    }
                }
                None => {
                    if depth == 0 && ends.contains(&c) {
                        break;
                    }
                    match c {
                        '"' | '\'' => quote = Some(c),
                        '(' => depth += 1,
                        ')' => depth = depth.saturating_sub(1),
                        _ => {}
                    }
                }
            }
            out.push(c);
            self.next();
        }
        out
    }
}

/// The text with its comments made spaces, keeping its lines.
fn without_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let (comment, next) = match after.find("*/") {
            Some(end) => (&after[..end], &after[end + 2..]),
            None => (after, ""),
        };
        out.extend(comment.chars().filter(|c| *c == '\n'));
        rest = next;
    }
    out.push_str(rest);
    out
}

/// Reads a stylesheet's text into its blocks.
pub(crate) fn parse(css: &str) -> (Vec<Block>, Vec<CssError>) {
    let mut reader = Reader { chars: without_comments(css).chars().collect(), at: 0, line: 1 };
    let mut blocks = Vec::new();
    let mut errors = Vec::new();
    loop {
        reader.skip_space();
        if reader.peek().is_none() {
            break;
        }
        match block(&mut reader) {
            Ok(b) => blocks.push(b),
            Err(e) => {
                errors.push(e);
                break;
            }
        }
    }
    (blocks, errors)
}

fn block(reader: &mut Reader) -> Result<Block, CssError> {
    let line = reader.line;
    let prelude = reader.until(&['{', '}', ';']).trim().to_string();
    let not_closed = |prelude: &str| CssError { line, kind: CssErrorKind::NotClosed, subject: prelude.to_string() };
    if reader.next() != Some('{') {
        return Err(not_closed(&prelude));
    }
    let mut decls = Vec::new();
    let mut nested = Vec::new();
    loop {
        reader.skip_space();
        match reader.peek() {
            None => return Err(not_closed(&prelude)),
            Some('}') => {
                reader.next();
                return Ok(Block { prelude, line, decls, nested });
            }
            Some(';') => {
                reader.next();
            }
            Some('@') => nested.push(block(reader)?),
            Some(_) => {
                let at = reader.line;
                let text = reader.until(&[';', '}', '{']);
                if reader.peek() == Some('{') {
                    // A selector inside a block: CSS nesting, which a stylesheet does not have.
                    return Err(CssError { line: at, kind: CssErrorKind::UnknownSelector, subject: text.trim().to_string() });
                }
                let (name, value) = text.split_once(':').unwrap_or((text.as_str(), ""));
                decls.push(Decl { name: name.trim().to_string(), value: value.trim().to_string(), line: at });
            }
        }
    }
}
