//! Types and fields as both kinds of diagram write them: type words of
//! other languages read as Clarity's, and fields read by Clarity's own
//! field parser, so their problems are the checker's.

use crate::diagnostic::{Diagnostic, Severity};
use crate::parser::parse as parse_spec;

use super::{DiagramField, Line};

/// Clarity's own type words.
pub(super) const CLARITY_TYPES: [&str; 13] =
    ["text", "integer", "decimal", "money", "bool", "date", "datetime", "duration", "image", "email", "url", "ref", "list"];

/// Type words of other languages and databases, read as Clarity's.
const FOREIGN_TYPES: [(&str, &str); 8] = [
    ("string", "text"),
    ("varchar", "text"),
    ("char", "text"),
    ("int", "integer"),
    ("long", "integer"),
    ("boolean", "bool"),
    ("timestamp", "datetime"),
    ("float", "decimal"),
];

/// Words for a decimal, which a field needs with its precision and scale.
const DECIMAL_WORDS: [&str; 4] = ["decimal", "float", "double", "number"];

/// A type word in Clarity's words, ignoring case for the words Clarity
/// knows; any other word starting with a capital is an enum's or an
/// entity's name.
fn known(word: &str) -> Option<&'static str> {
    let lower = word.to_lowercase();
    CLARITY_TYPES
        .iter()
        .find(|t| **t == lower)
        .copied()
        .or_else(|| FOREIGN_TYPES.iter().find(|(foreign, _)| *foreign == lower).map(|(_, ours)| *ours))
}

/// A field's type in Clarity's words; an entity's name is a ref when the
/// attribute is a foreign key.
pub(super) fn field_type(written: &str, foreign_key: bool) -> Result<String, String> {
    if let Some(item) = list_item(written) {
        return match field_type(item, false)? {
            entity if entity.starts_with(|c: char| c.is_uppercase()) => Ok(format!("list<{entity}>")),
            _ => Err(format!("'{written}' lists values; a field lists the records of an entity, as Order[]")),
        };
    }
    let (word, size) = match written.split_once('(') {
        Some((word, rest)) => (word, Some(rest.trim_end_matches(')'))),
        None => (written, None),
    };
    if DECIMAL_WORDS.contains(&word.to_lowercase().as_str()) && size.is_none_or(|s| !s.contains(',')) {
        return Err(format!("'{written}' needs its precision and scale, as \"decimal(10, 2)\""));
    }
    if let Some(ours) = known(word) {
        return Ok(match (ours, size) {
            ("text", Some(n)) => format!("text({n})"),
            ("decimal", Some(s)) => format!("decimal({s})"),
            (ours, _) => ours.to_string(),
        });
    }
    if word.starts_with(|c: char| c.is_uppercase()) {
        return Ok(if foreign_key { format!("ref {word}") } else { word.to_string() });
    }
    Err(format!(
        "'{written}' is not a type Clarity knows: use text, integer, money, bool, date, datetime, duration, image, email or url, or the name of an enum or entity"
    ))
}

/// A parameter's or result's type in Clarity's words: `String` is `text`,
/// `List~Order~` is `list<Order>`. Whether it fits is checked against the
/// project.
pub(super) fn value_type(written: &str) -> String {
    let written = written.trim();
    let generic = written.split_once('~').or_else(|| written.split_once('<'));
    if let Some((outer, inner)) = generic {
        let inner = inner.trim_end_matches(['~', '>']);
        if outer.eq_ignore_ascii_case("list") {
            return format!("list<{}>", value_type(inner));
        }
    }
    known(written).map_or_else(|| written.to_string(), str::to_string)
}

/// The item of a list as a diagram writes it: `Order[]`, `List~Order~`,
/// `list<Order>`.
fn list_item(written: &str) -> Option<&str> {
    if let Some(item) = written.strip_suffix("[]") {
        return Some(item);
    }
    let (outer, inner) = written.split_once('~').or_else(|| written.split_once('<'))?;
    outer.eq_ignore_ascii_case("list").then(|| inner.trim_end_matches(['~', '>']))
}

/// `text(80), required` split into the type and what follows its first
/// comma outside brackets.
pub(super) fn split_type(text: &str) -> (&str, Option<&str>) {
    let mut depth = 0i32;
    for (i, c) in text.char_indices() {
        match c {
            '(' | '<' => depth += 1,
            ')' | '>' => depth -= 1,
            ',' if depth == 0 => return (text[..i].trim(), Some(text[i + 1..].trim())),
            _ => {}
        }
    }
    (text.trim(), None)
}

/// Reads `name: field_text` as Clarity reads a field line; on an error the
/// problem is noted on the diagram's line and the field is left out.
pub(super) fn read_field(name: &str, field_text: &str, line: &Line, problems: &mut Vec<Diagnostic>) -> Option<DiagramField> {
    let probe = format!("module m\nentity E\n    {name}: {field_text}\n");
    let (file, diagnostics) = parse_spec(&probe, "");
    if let Some(error) = diagnostics.iter().find(|d| d.severity == Severity::Error) {
        problems.push(Diagnostic::error(error.message.clone(), line.span));
        return None;
    }
    let field = file.entities.into_iter().next()?.fields.into_iter().next()?;
    Some(DiagramField { name: field.name.name, type_text: field.type_text, modifiers: field.modifiers, span: line.span })
}
