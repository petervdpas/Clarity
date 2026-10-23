//! A top-level declaration as text: a window, the app, an entity, an enum,
//! a function, an action, a code block, a theme, an engine, a custom part or
//! a library. The code panels show one
//! declaration, check it while it is typed and write it back, keeping the
//! rest of the file exactly as it was.

use crate::diagnostic::Diagnostic;
use crate::edit::{EditError, EditResult};
use crate::lexer::lex;

/// The words a declaration that can be edited as text starts with.
const DECL_WORDS: [&str; 11] = ["page", "app", "entity", "enum", "function", "action", "code", "theme", "engine", "part", "library"];

fn fail<T>(message: impl Into<String>) -> EditResult<T> {
    Err(EditError(message.into()))
}

/// The end of the block that starts on the line at `start`: every
/// indented or blank line after it, without trailing blank lines.
pub(crate) fn block_end(source: &str, start: usize) -> usize {
    let mut end = start + source[start..].find('\n').map_or(source.len() - start, |i| i + 1);
    let mut content_end = end;
    while end < source.len() {
        let line_end = source[end..].find('\n').map_or(source.len(), |i| end + i + 1);
        let line = &source[end..line_end];
        if line.trim().is_empty() {
            end = line_end;
            continue;
        }
        if !line.starts_with([' ', '\t']) {
            break;
        }
        end = line_end;
        content_end = line_end;
    }
    content_end
}

/// The byte range of the declaration whose first line starts at `item`.
fn decl_range(source: &str, item: u32) -> EditResult<(usize, usize)> {
    let start = item as usize;
    let at_line_start = start == 0 || source[..start].ends_with('\n');
    let word = source.get(start..).and_then(|rest| rest.split(|c: char| !c.is_ascii_alphanumeric()).next());
    match word {
        Some(word) if at_line_start && DECL_WORDS.contains(&word) => Ok((start, block_end(source, start))),
        _ => fail("there is no declaration at this place; the file may have changed"),
    }
}

/// The text of the declaration at `item`, ending in a newline.
pub fn decl_text(source: &str, item: u32) -> EditResult<String> {
    let (start, end) = decl_range(source, item)?;
    let text = &source[start..end];
    Ok(if text.ends_with('\n') { text.to_string() } else { format!("{text}\n") })
}

/// Checks that `text` is exactly one declaration, and returns it ending in
/// one newline.
fn one_decl(text: &str) -> EditResult<String> {
    let (lines, _) = lex(text);
    let top: Vec<_> = lines.iter().filter(|l| l.indent == 0).collect();
    let starts_right = lines.first().is_some_and(|l| l.indent == 0);
    let word = top.first().and_then(|l| l.tokens.first()).map(|t| t.text.as_str());
    if top.len() != 1 || !starts_right || !word.is_some_and(|w| DECL_WORDS.contains(&w)) {
        return fail("the text must be one declaration: a line like 'page Name: window' with its indented lines");
    }
    Ok(format!("{}\n", text.trim_end()))
}

/// Replaces the declaration at `item` with `text`.
pub fn replace_decl(source: &str, item: u32, text: &str) -> EditResult<String> {
    let (start, end) = decl_range(source, item)?;
    let text = one_decl(text)?;
    Ok(format!("{}{text}{}", &source[..start], &source[end..]))
}

/// Adds `text` as a new declaration at the end of the file.
pub fn append_decl(source: &str, text: &str) -> EditResult<String> {
    let text = one_decl(text)?;
    Ok(format!("{}\n\n{text}", source.trim_end_matches('\n')))
}

/// Where `append_decl` puts the new declaration.
pub(crate) fn appended_at(source: &str) -> u32 {
    (source.trim_end_matches('\n').len() + 2) as u32
}

/// The problems inside the declaration at `item`, with lines counted from
/// its first line, as the code panel shows it.
pub fn problems_in_decl(source: &str, item: u32, path: &str, diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    let Ok((start, end)) = decl_range(source, item) else {
        return Vec::new();
    };
    let first_line = source[..start].matches('\n').count() as u32 + 1;
    diagnostics
        .iter()
        .filter(|d| d.path == path && (start..end.max(start + 1)).contains(&(d.span.start as usize)))
        .map(|d| {
            let mut d = d.clone();
            d.span.line = d.span.line + 1 - first_line;
            d
        })
        .collect()
}
