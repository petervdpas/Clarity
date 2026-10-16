//! Line-based lexer. Spec files are indentation-structured, so the lexer
//! produces one `Line` per non-blank, non-comment line, each with its
//! indentation and tokens. Building the block tree is the parser's job.

use crate::diagnostic::Diagnostic;
use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    Str,
    Number,
    Colon,
    Comma,
    LParen,
    RParen,
    Lt,
    Gt,
    Dot,
    Arrow,
    /// `=`
    Assign,
    /// `==`
    EqEq,
    /// `!=`
    NotEq,
    /// `<=`
    LtEq,
    /// `>=`
    GtEq,
    Plus,
    Minus,
    Star,
    Slash,
    LBracket,
    RBracket,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    /// Identifier or number text, or the unescaped value of a string.
    pub text: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Number of leading spaces.
    pub indent: usize,
    pub tokens: Vec<Token>,
    /// Span of the line content, without indentation and trailing comment.
    pub span: Span,
}

pub fn lex(source: &str) -> (Vec<Line>, Vec<Diagnostic>) {
    let mut lines = Vec::new();
    let mut diagnostics = Vec::new();
    let mut offset = 0;

    for (index, raw) in source.split('\n').enumerate() {
        let line_no = index as u32 + 1;
        let text = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(line) = lex_line(text, offset, line_no, &mut diagnostics) {
            lines.push(line);
        }
        offset += raw.len() + 1;
    }

    (lines, diagnostics)
}

fn lex_line(
    text: &str,
    line_start: usize,
    line_no: u32,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Line> {
    let bytes = text.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        if bytes[pos] == b'\t' {
            diagnostics.push(Diagnostic::error(
                "tabs are not allowed for indentation, use spaces",
                span_at(line_start, line_no, pos, pos + 1),
            ));
        }
        pos += 1;
    }
    let indent = pos;

    let mut tokens = Vec::new();
    let mut content_end = indent;

    while pos < bytes.len() {
        let c = bytes[pos];
        if c == b' ' || c == b'\t' {
            pos += 1;
            continue;
        }
        if c == b'/' && bytes.get(pos + 1) == Some(&b'/') {
            break;
        }

        let start = pos;
        let kind = match c {
            b':' => {
                pos += 1;
                TokenKind::Colon
            }
            b',' => {
                pos += 1;
                TokenKind::Comma
            }
            b'(' => {
                pos += 1;
                TokenKind::LParen
            }
            b')' => {
                pos += 1;
                TokenKind::RParen
            }
            b'<' | b'>' | b'=' | b'!' if bytes.get(pos + 1) == Some(&b'=') => {
                pos += 2;
                match c {
                    b'<' => TokenKind::LtEq,
                    b'>' => TokenKind::GtEq,
                    b'=' => TokenKind::EqEq,
                    _ => TokenKind::NotEq,
                }
            }
            b'<' => {
                pos += 1;
                TokenKind::Lt
            }
            b'>' => {
                pos += 1;
                TokenKind::Gt
            }
            b'=' => {
                pos += 1;
                TokenKind::Assign
            }
            b'+' => {
                pos += 1;
                TokenKind::Plus
            }
            b'*' => {
                pos += 1;
                TokenKind::Star
            }
            b'/' => {
                pos += 1;
                TokenKind::Slash
            }
            b'[' => {
                pos += 1;
                TokenKind::LBracket
            }
            b']' => {
                pos += 1;
                TokenKind::RBracket
            }
            b'-' if !bytes.get(pos + 1).is_some_and(|b| b.is_ascii_digit() || *b == b'>') => {
                pos += 1;
                TokenKind::Minus
            }
            b'.' => {
                pos += 1;
                TokenKind::Dot
            }
            b'-' if bytes.get(pos + 1) == Some(&b'>') => {
                pos += 2;
                TokenKind::Arrow
            }
            b'"' => match lex_string(text, pos) {
                Ok((value, end)) => {
                    pos = end;
                    tokens.push(Token {
                        kind: TokenKind::Str,
                        text: value,
                        span: span_at(line_start, line_no, start, pos),
                    });
                    content_end = pos;
                    continue;
                }
                Err(message) => {
                    diagnostics.push(Diagnostic::error(
                        message,
                        span_at(line_start, line_no, start, bytes.len()),
                    ));
                    pos = bytes.len();
                    content_end = pos;
                    break;
                }
            },
            b'-' | b'0'..=b'9' => {
                pos += 1;
                while pos < bytes.len() && (bytes[pos].is_ascii_digit() || bytes[pos] == b'.') {
                    pos += 1;
                }
                TokenKind::Number
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_')
                {
                    pos += 1;
                }
                TokenKind::Ident
            }
            _ => {
                let ch = text[pos..].chars().next().unwrap_or('?');
                pos += ch.len_utf8();
                diagnostics.push(Diagnostic::error(
                    format!("unexpected character '{ch}'"),
                    span_at(line_start, line_no, start, pos),
                ));
                continue;
            }
        };

        tokens.push(Token {
            kind,
            text: text[start..pos].to_string(),
            span: span_at(line_start, line_no, start, pos),
        });
        content_end = pos;
    }

    if tokens.is_empty() {
        return None;
    }

    Some(Line {
        indent,
        tokens,
        span: span_at(line_start, line_no, indent, content_end),
    })
}

/// Lexes a string literal starting at `start` (the opening quote).
/// Returns the unescaped value and the position after the closing quote.
fn lex_string(text: &str, start: usize) -> Result<(String, usize), String> {
    let mut value = String::new();
    let mut chars = text[start + 1..].char_indices();
    while let Some((i, ch)) = chars.next() {
        match ch {
            '"' => return Ok((value, start + 1 + i + 1)),
            '\\' => match chars.next() {
                Some((_, '"')) => value.push('"'),
                Some((_, '\\')) => value.push('\\'),
                Some((_, 'n')) => value.push('\n'),
                Some((_, other)) => {
                    // Keep unknown escapes as written, so regex patterns like
                    // "\d" survive without double escaping.
                    value.push('\\');
                    value.push(other);
                }
                None => break,
            },
            _ => value.push(ch),
        }
    }
    Err("unterminated string".to_string())
}

/// Writes `text` as a string literal that `lex_string` reads back unchanged.
pub(crate) fn quote(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

fn span_at(line_start: usize, line_no: u32, start: usize, end: usize) -> Span {
    Span {
        start: (line_start + start) as u32,
        end: (line_start + end) as u32,
        line: line_no,
        col: start as u32 + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &Line) -> Vec<TokenKind> {
        line.tokens.iter().map(|t| t.kind).collect()
    }

    #[test]
    fn skips_blank_and_comment_lines() {
        let (lines, diags) = lex("\n// comment\n   \nmodule shop\n");
        assert!(diags.is_empty());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].span.line, 4);
    }

    #[test]
    fn records_indentation_and_columns() {
        let (lines, _) = lex("entity A\n    name: text\n");
        assert_eq!(lines[1].indent, 4);
        assert_eq!(lines[1].tokens[0].span.col, 5);
        assert_eq!(lines[1].tokens[0].span.start, 13);
    }

    #[test]
    fn lexes_field_line() {
        let (lines, diags) = lex("name: text(80), required, label \"Full name\" // trailing");
        assert!(diags.is_empty());
        assert_eq!(
            kinds(&lines[0]),
            vec![
                TokenKind::Ident,
                TokenKind::Colon,
                TokenKind::Ident,
                TokenKind::LParen,
                TokenKind::Number,
                TokenKind::RParen,
                TokenKind::Comma,
                TokenKind::Ident,
                TokenKind::Comma,
                TokenKind::Ident,
                TokenKind::Str,
            ]
        );
        assert_eq!(lines[0].tokens[10].text, "Full name");
    }

    #[test]
    fn lexes_arrow_and_generics() {
        let (lines, _) = lex("on open -> Form\nlines: list<Line>");
        assert_eq!(lines[0].tokens[2].kind, TokenKind::Arrow);
        assert_eq!(
            kinds(&lines[1])[2..],
            [
                TokenKind::Ident,
                TokenKind::Lt,
                TokenKind::Ident,
                TokenKind::Gt
            ]
        );
    }

    #[test]
    fn lexes_paths() {
        let (lines, diags) = lex("bind customer.name");
        assert!(diags.is_empty());
        assert_eq!(
            kinds(&lines[0]),
            vec![
                TokenKind::Ident,
                TokenKind::Ident,
                TokenKind::Dot,
                TokenKind::Ident
            ]
        );
    }

    #[test]
    fn lexes_negative_and_decimal_numbers() {
        let (lines, _) = lex("min -10.5");
        assert_eq!(lines[0].tokens[1].kind, TokenKind::Number);
        assert_eq!(lines[0].tokens[1].text, "-10.5");
    }

    #[test]
    fn keeps_unknown_escapes_in_strings() {
        let (lines, diags) = lex(r#"pattern "^\d+ \"x\"$""#);
        assert!(diags.is_empty());
        assert_eq!(lines[0].tokens[1].text, r#"^\d+ "x"$"#);
    }

    #[test]
    fn reports_unterminated_string() {
        let (_, diags) = lex("label \"oops");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("unterminated"));
    }

    #[test]
    fn quote_round_trips_through_the_lexer() {
        for text in [
            "plain",
            "with \"quotes\"",
            "back\\slash",
            "line\nbreak",
            r"^\d+$",
        ] {
            let (lines, diags) = lex(&quote(text));
            assert!(diags.is_empty(), "{text:?}: {diags:?}");
            assert_eq!(lines[0].tokens[0].text, text);
        }
    }

    #[test]
    fn reports_tabs() {
        let (_, diags) = lex("\tname: text");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("tabs"));
    }

    #[test]
    fn reports_unexpected_characters() {
        let (_, diags) = lex("name: text; €");
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn lexes_operators() {
        let (lines, diags) = lex("x = a == b != c <= d >= e + f - g * h / i [ ] < >");
        assert!(diags.is_empty(), "{diags:?}");
        let ops: Vec<TokenKind> = kinds(&lines[0]).into_iter().filter(|k| *k != TokenKind::Ident).collect();
        assert_eq!(
            ops,
            [
                TokenKind::Assign,
                TokenKind::EqEq,
                TokenKind::NotEq,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Lt,
                TokenKind::Gt,
            ]
        );
    }
}
