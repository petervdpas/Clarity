//! One line of a theme, a custom part or a library, set by its key: what the
//! IDE's forms write (`Edit::SetDeclLine`). Every line of such a block says
//! one thing, and its key names that thing:
//!
//! - a theme: `mode`, `title`, `padding`, `border`, `shadow`, `color accent`,
//!   `font text` (`font "Inter"` is the text font), `size small`,
//!   `radius all` (`radius 6`), `part button`, `part button hover`,
//!   `style primary`;
//! - a custom part: `library` (its native, crate, go, nuget or dll line),
//!   `binds`, `size`, `grows`, `preview`, `description`, `property speed`
//!   (`speed: integer`), `tells scored`, `hears restart`;
//! - a library: `source` (its nuget, dll, crate, go or cmake line),
//!   `function spell`;
//! - a toast: `text`, `lifespan`, `level`, `icon`, `corner`.
//!
//! A line is replaced where it is, a new one goes after the last line that
//! shares most of its key (a state of a part after that part's lines), or at
//! the end; comment lines and every other line stay as written.

use crate::edit::{EditError, EditResult};
use crate::lexer::{Token, TokenKind, lex};

fn fail<T>(message: impl Into<String>) -> EditResult<T> {
    Err(EditError(message.into()))
}

/// What a theme's part look sets, which is not a state.
const LOOK_WORDS: [&str; 8] = ["background", "text", "border", "radius", "weight", "size", "font", "align"];

/// The key of a line of a block that starts with `decl` (`theme`, `part`
/// or `library`); none for a comment, a blank line or a line it cannot read.
pub fn line_key(decl: &str, line: &str) -> Option<String> {
    let tokens: Vec<Token> = lex(line).0.into_iter().next().map(|l| l.tokens).unwrap_or_default();
    let word = |i: usize| tokens.get(i).filter(|t| t.kind == TokenKind::Ident).map(|t| t.text.as_str());
    let kind = |i: usize| tokens.get(i).map(|t| t.kind);
    let first = word(0)?;
    match decl {
        "theme" => match first {
            "mode" | "title" | "padding" | "border" | "shadow" => Some(first.to_string()),
            "color" | "style" => Some(format!("{first} {}", word(1)?)),
            "font" | "size" | "radius" => {
                let default = if first == "radius" { "all" } else { "text" };
                Some(format!("{first} {}", word(1).unwrap_or(default)))
            }
            "part" => {
                let part = word(1)?;
                match word(2) {
                    Some(state) if !LOOK_WORDS.contains(&state) => Some(format!("part {part} {state}")),
                    _ => Some(format!("part {part}")),
                }
            }
            _ => None,
        },
        "part" => match first {
            "native" | "crate" | "go" | "nuget" | "dll" => Some("library".to_string()),
            "binds" | "size" | "grows" | "preview" => Some(first.to_string()),
            "description" if matches!(kind(1), Some(TokenKind::Str | TokenKind::Key)) => Some("description".to_string()),
            "tells" | "hears" => Some(format!("{first} {}", word(1)?)),
            name if kind(1) == Some(TokenKind::Colon) => Some(format!("property {name}")),
            _ => None,
        },
        "library" => match first {
            "nuget" | "dll" | "crate" | "go" | "cmake" => Some("source".to_string()),
            "function" => Some(format!("function {}", word(1)?)),
            _ => None,
        },
        "toast" => match first {
            "text" | "lifespan" | "level" | "icon" | "corner" => Some(first.to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// Sets the line with `key` of the theme, custom part, library or toast declared at
/// `item` to `line`, adds it when there is none, or takes it away (`None`).
/// A line with another key renames the thing (a property, a tell), unless
/// the block has a line of that key already.
pub fn set_decl_line(source: &str, item: u32, key: &str, line: Option<&str>) -> EditResult<String> {
    let (start, end) = crate::source::decl_range(source, item)?;
    let decl = source[start..].split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap_or("");
    if !["theme", "part", "library", "toast"].contains(&decl) {
        return fail("only a theme, a custom part, a library or a toast has lines set one by one");
    }
    let new_line = match line {
        Some(text) => {
            let text = text.trim();
            if text.contains('\n') {
                return fail("a line of a declaration is one line");
            }
            let Some(new_key) = line_key(decl, text) else {
                return fail(format!("'{text}' is not a line of a {decl}"));
            };
            Some((text, new_key))
        }
        None => None,
    };
    let block = &source[start..end];
    let mut lines: Vec<String> = block.split_inclusive('\n').map(str::to_string).collect();
    let keys: Vec<Option<String>> = lines.iter().enumerate().map(|(i, l)| if i == 0 { None } else { line_key(decl, l.trim()) }).collect();
    let at = keys.iter().position(|k| k.as_deref() == Some(key));
    if let Some((_, new_key)) = &new_line
        && new_key != key
        && keys.iter().any(|k| k.as_ref() == Some(new_key))
    {
        return fail(format!("the {decl} has a line '{new_key}' already"));
    }
    // Indented as its other lines are, four spaces when it has none.
    let indent = lines.iter().skip(1).find(|l| !l.trim().is_empty()).map(|l| l[..l.len() - l.trim_start().len()].to_string()).unwrap_or_else(|| "    ".to_string());
    match (at, new_line) {
        (Some(i), Some((text, _))) => {
            let old = &lines[i];
            let pad = &old[..old.len() - old.trim_start().len()];
            let newline = if old.ends_with('\n') { "\n" } else { "" };
            lines[i] = format!("{pad}{text}{newline}");
        }
        (Some(i), None) => {
            lines.remove(i);
        }
        (None, None) => return Ok(source.to_string()),
        (None, Some((text, new_key))) => {
            let words: Vec<&str> = new_key.split(' ').collect();
            let shared = |k: &str| k.split(' ').zip(&words).take_while(|(a, b)| a == *b).count();
            let best = keys.iter().enumerate().filter_map(|(i, k)| Some((i, shared(k.as_deref()?)))).filter(|(_, n)| *n > 0).max_by_key(|(i, n)| (*n, *i));
            let after = best.map(|(i, _)| i).unwrap_or(lines.len() - 1);
            if let Some(last) = lines.last_mut()
                && !last.ends_with('\n')
            {
                last.push('\n');
            }
            lines.insert(after + 1, format!("{indent}{text}\n"));
        }
    }
    let mut written = lines.concat();
    if !block.ends_with('\n') && written.ends_with('\n') {
        written.pop();
    }
    Ok(format!("{}{written}{}", &source[..start], &source[end..]))
}
