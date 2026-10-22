//! An ERD (`erDiagram`): entity blocks with attributes, and relationships
//! between entities. Attributes become fields in Clarity's types, read by
//! Clarity's own field parser so their problems are the checker's.

use crate::diagnostic::Diagnostic;

use super::fields::{CLARITY_TYPES, field_type, read_field};
use super::{Cardinality, Diagram, DiagramEntity, DiagramField, DiagramKind, Line, Relation};

const UNREADABLE: &str =
    "Clarity cannot read this line of an ERD: write an entity `Name {`, an attribute `type name`, or a relationship `A ||--o{ B : label`";

pub(super) fn parse<'a>(body: impl Iterator<Item = Line<'a>>) -> (Diagram, Vec<Diagnostic>) {
    let mut diagram = Diagram { kind: Some(DiagramKind::Erd), ..Diagram::default() };
    let mut problems = Vec::new();
    let mut open: Option<usize> = None;
    for line in body {
        let text = line.text;
        if let Some(entity) = open {
            if text == "}" {
                open = None;
            } else if let Some(field) = attribute(&line, &mut problems) {
                diagram.entities[entity].fields.push(field);
            }
            continue;
        }
        if text.starts_with("direction ") {
            continue;
        }
        if let Some(head) = text.strip_suffix('{') {
            let name = entity_name(head.trim());
            open = Some(entity_index(&mut diagram, name, line.span_of(name)));
        } else if let Some(relation) = relation(&line) {
            entity_index(&mut diagram, &relation.left, relation.span);
            entity_index(&mut diagram, &relation.right, relation.span);
            diagram.relations.push(relation);
        } else if is_name(text) {
            entity_index(&mut diagram, entity_name(text), line.span);
        } else {
            problems.push(Diagnostic::error(UNREADABLE, line.span));
        }
    }
    (diagram, problems)
}

/// The name of an entity without its alias: `Customer["Our customers"]`.
fn entity_name(text: &str) -> &str {
    text.split('[').next().unwrap_or(text).trim()
}

fn is_name(text: &str) -> bool {
    let name = entity_name(text);
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

/// The entity's place in the diagram, adding it where it is first named.
fn entity_index(diagram: &mut Diagram, name: &str, span: crate::span::Span) -> usize {
    match diagram.entities.iter().position(|e| e.name == name) {
        Some(index) => index,
        None => {
            diagram.entities.push(DiagramEntity { name: name.to_string(), fields: Vec::new(), module: None, span });
            diagram.entities.len() - 1
        }
    }
}

/// `Customer ||--o{ Order : places`.
fn relation(line: &Line) -> Option<Relation> {
    let (ends, label) = match line.text.split_once(':') {
        Some((ends, label)) => (ends, Some(label.trim().trim_matches('"').to_string()).filter(|l| !l.is_empty())),
        None => (line.text, None),
    };
    let words: Vec<&str> = ends.split_whitespace().collect();
    let [left, link, right] = words[..] else { return None };
    if link.len() != 6 || !link.is_ascii() {
        return None;
    }
    let left_count = match &link[..2] {
        "|o" => Cardinality::ZeroOrOne,
        "||" => Cardinality::ExactlyOne,
        "}o" => Cardinality::ZeroOrMore,
        "}|" => Cardinality::OneOrMore,
        _ => return None,
    };
    let identifying = match &link[2..4] {
        "--" => true,
        ".." => false,
        _ => return None,
    };
    let right_count = match &link[4..] {
        "o|" => Cardinality::ZeroOrOne,
        "||" => Cardinality::ExactlyOne,
        "o{" => Cardinality::ZeroOrMore,
        "|{" => Cardinality::OneOrMore,
        _ => return None,
    };
    if !is_name(left) || !is_name(right) {
        return None;
    }
    Some(Relation {
        left: entity_name(left).to_string(),
        right: entity_name(right).to_string(),
        left_count,
        right_count,
        identifying,
        lists: identifying,
        label,
        span: line.span,
    })
}

/// `type name PK, FK "comment"`: a field, or None with the problem noted.
fn attribute(line: &Line, problems: &mut Vec<Diagnostic>) -> Option<DiagramField> {
    let (head, comment) = match line.text.split_once('"') {
        Some((head, rest)) => (head, rest.trim_end().trim_end_matches('"').trim()),
        None => (line.text, ""),
    };
    let words: Vec<&str> = head.split_whitespace().collect();
    let [written, name, keys @ ..] = &words[..] else {
        problems.push(Diagnostic::error(UNREADABLE, line.span));
        return None;
    };
    let keys: Vec<String> = keys.join(" ").split(',').map(|k| k.trim().to_uppercase()).filter(|k| !k.is_empty()).collect();
    let has = |key: &str| keys.iter().any(|k| k == key);
    if has("PK") {
        if *name == "id" {
            return None;
        }
        problems.push(Diagnostic::warning(
            format!("Clarity gives every entity its own key, id; '{name}' becomes a unique field"),
            line.span_of(name),
        ));
    }

    // The comment may give the whole type, where Mermaid's type word cannot hold it.
    let comment_type = comment.split(|c: char| c == '(' || c == ',' || c.is_whitespace()).next().is_some_and(|w| CLARITY_TYPES.contains(&w));
    let field_text = if comment_type {
        comment.to_string()
    } else {
        let ty = match field_type(written, has("FK")) {
            Ok(ty) => ty,
            Err(message) => {
                problems.push(Diagnostic::error(message, line.span_of(written)));
                return None;
            }
        };
        if comment.is_empty() { ty } else { format!("{ty}, {comment}") }
    };

    let mut field = read_field(name, &field_text, line, problems)?;
    if has("UK") || has("PK") {
        field.modifiers.unique = true;
    }
    Some(field)
}
