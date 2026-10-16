//! Edits to code blocks. They work on the text, not the line tree, so the
//! comments and blank lines a developer writes inside handlers stay exactly
//! as written. A block is addressed by the window or app it belongs to.

use crate::edit::{EditError, EditResult};
use crate::parser::parse;

/// Indentation of a handler inside its code block.
const BLOCK_INDENT: &str = "    ";

fn fail<T>(message: impl Into<String>) -> EditResult<T> {
    Err(EditError(message.into()))
}

/// The byte range of `owner`'s code block: its `code` line and every
/// indented or blank line after it, without trailing blank lines.
fn block_range(source: &str, owner: &str) -> Option<(usize, usize)> {
    let (file, _) = parse(source, "");
    let header = file.code.iter().find(|b| b.name.name == owner)?.span;
    let start = source[..header.start as usize].rfind('\n').map_or(0, |i| i + 1);
    Some((start, crate::source::block_end(source, start)))
}

/// The handlers of `owner`'s code block as the code panel shows them:
/// without the `code` line and one level less indented. Empty when the
/// window has no code yet.
pub fn code_text(source: &str, owner: &str) -> String {
    let Some((start, end)) = block_range(source, owner) else {
        return String::new();
    };
    let block = &source[start..end];
    let body = block.split_once('\n').map_or("", |(_, rest)| rest);
    body.lines()
        .map(|line| line.strip_prefix(BLOCK_INDENT).unwrap_or(line.trim_start()))
        .map(|line| format!("{line}\n"))
        .collect()
}

fn owner_exists(source: &str, owner: &str) -> bool {
    let (file, _) = parse(source, "");
    file.pages.iter().any(|p| p.name.name == owner) || file.apps.iter().any(|a| a.name.name == owner)
}

/// Replaces `owner`'s code with `text` (as `code_text` gives it), adds the
/// block at the end of the file when there is none, and removes it when
/// `text` is empty.
pub fn set_code(source: &str, owner: &str, text: &str) -> EditResult<String> {
    if !owner_exists(source, owner) {
        return fail(format!("there is no window or app '{owner}' in this file"));
    }
    if text.lines().any(|l| l.starts_with(|c: char| c.is_whitespace()) && l.contains('\t')) {
        return fail("tabs are not allowed for indentation, use spaces");
    }
    let lines: Vec<&str> = text.trim_end().lines().collect();
    let block = if lines.iter().all(|l| l.trim().is_empty()) {
        None
    } else {
        let mut block = format!("code {owner}\n");
        for line in lines {
            if line.trim().is_empty() {
                block.push('\n');
            } else {
                block.push_str(BLOCK_INDENT);
                block.push_str(line.trim_end());
                block.push('\n');
            }
        }
        Some(block)
    };
    let text = match (block_range(source, owner), block) {
        (Some((start, end)), Some(block)) => format!("{}{block}{}", &source[..start], &source[end..]),
        (Some((start, end)), None) => {
            // Take the blank line that separated the block with it.
            let before = source[..start].trim_end_matches('\n');
            let after = &source[end..];
            if after.trim().is_empty() {
                format!("{before}\n")
            } else {
                format!("{before}\n\n{}", after.trim_start_matches('\n'))
            }
        }
        (None, Some(block)) => format!("{}\n\n{block}", source.trim_end_matches('\n')),
        (None, None) => source.to_string(),
    };
    Ok(text)
}

/// Adds an empty `handler name` at the end of `owner`'s code, unless it is
/// there already.
pub fn add_handler(source: &str, owner: &str, name: &str) -> EditResult<String> {
    if !crate::widgets::is_identifier(name) {
        return fail(format!("'{name}' is not a valid handler name"));
    }
    let (file, _) = parse(source, "");
    let exists = file
        .code
        .iter()
        .filter(|b| b.name.name == owner)
        .flat_map(|b| &b.handlers)
        .any(|h| h.name.name == name);
    if exists {
        return Ok(source.to_string());
    }
    let current = code_text(source, owner);
    let text = if current.trim().is_empty() {
        format!("handler {name}\n")
    } else {
        format!("{}\n\nhandler {name}\n", current.trim_end())
    };
    set_code(source, owner, &text)
}

/// The problems inside `owner`'s code block, with line and column as the
/// code panel shows them (`code_text`): line 1 is the first handler line,
/// and columns lose the block's indentation.
pub fn problems_in_code(
    source: &str,
    owner: &str,
    path: &str,
    diagnostics: &[crate::Diagnostic],
) -> Vec<crate::Diagnostic> {
    let Some((start, end)) = block_range(source, owner) else {
        return Vec::new();
    };
    let header_line = source[..start].matches('\n').count() as u32 + 1;
    let indent = BLOCK_INDENT.len() as u32;
    diagnostics
        .iter()
        .filter(|d| d.path == path && (d.span.start as usize) >= start && (d.span.start as usize) < end.max(start + 1))
        .map(|d| {
            let mut d = d.clone();
            d.span.line = d.span.line.saturating_sub(header_line).max(1);
            d.span.col = d.span.col.saturating_sub(indent).max(1);
            d
        })
        .collect()
}
