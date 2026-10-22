//! Views generated from the project, always true: Mermaid text written
//! from the data dictionary, each line linked to what it shows so a click
//! on the drawing opens it. A view read back proposes nothing.

use serde::Serialize;
use ts_rs::TS;

use crate::model::{Entity, Field, FieldType, Project};
use crate::span::Span;

/// A view: its Mermaid text, and what its lines show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct View {
    pub text: String,
    pub links: Vec<Link>,
}

/// A line of a view and the declaration it shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Link {
    /// The line in the view's text, from 1.
    pub line: u32,
    /// The module file of the declaration.
    pub path: String,
    pub span: Span,
}

struct Writer {
    text: String,
    links: Vec<Link>,
    line: u32,
}

impl Writer {
    fn line(&mut self, text: &str, shows: Option<(&str, Span)>) {
        self.line += 1;
        self.text.push_str(text);
        self.text.push('\n');
        if let Some((path, span)) = shows {
            self.links.push(Link { line: self.line, path: path.to_string(), span });
        }
    }
}

/// The Data model: every entity with its fields, and a relationship for
/// every list and for every ref no list stands for.
pub fn data_model(project: &Project) -> View {
    let mut out = Writer { text: String::new(), links: Vec::new(), line: 0 };
    out.line("erDiagram", None);
    let entities: Vec<(&str, &Entity)> =
        project.files.iter().flat_map(|f| f.entities.iter().map(move |e| (f.path.as_str(), e))).collect();
    for (path, entity) in &entities {
        out.line(&format!("    {} {{", entity.name.name), Some((path, entity.span)));
        for field in &entity.fields {
            out.line(&format!("        {}", attribute(field)), Some((path, field.span)));
        }
        out.line("    }", None);
    }
    for (path, entity) in &entities {
        for field in &entity.fields {
            match &field.ty {
                FieldType::List { item } => {
                    let line = if field.modifiers.owned { "--" } else { ".." };
                    out.line(&format!("    {} ||{line}o{{ {} : {}", entity.name.name, item.name, field.name.name), Some((path, field.span)));
                }
                FieldType::Ref { entity: other } => {
                    let listed = entities.iter().any(|(_, e)| {
                        e.name.name == other.name
                            && e.fields.iter().any(|f| matches!(&f.ty, FieldType::List { item } if item.name == entity.name.name))
                    });
                    if !listed {
                        out.line(&format!("    {} }}o..|| {} : {}", entity.name.name, other.name, field.name.name), Some((path, field.span)));
                    }
                }
                _ => {}
            }
        }
    }
    View { text: out.text, links: out.links }
}

/// A field as an ERD attribute: `type name keys "comment"`, the comment
/// holding the modifiers, and the type where Mermaid's word cannot.
fn attribute(field: &Field) -> String {
    let m = &field.modifiers;
    let mut comment: Vec<String> = Vec::new();
    let (word, key) = match &field.ty {
        FieldType::Decimal { .. } => {
            comment.push(field.type_text.clone());
            ("decimal".to_string(), None)
        }
        FieldType::Ref { entity } => (entity.name.clone(), Some("FK")),
        FieldType::List { item } => (format!("{}[]", item.name), None),
        _ => (field.type_text.clone(), None),
    };
    let key = key.or(m.unique.then_some("UK"));
    for (on, word) in [(m.required, "required"), (m.computed, "computed"), (m.owned, "owned")] {
        if on {
            comment.push(word.to_string());
        }
    }
    let mut line = format!("{word} {}", field.name.name);
    if let Some(key) = key {
        line.push_str(&format!(" {key}"));
        if key == "FK" && m.unique {
            line.push_str(", UK");
        }
    }
    if !comment.is_empty() {
        line.push_str(&format!(" \"{}\"", comment.join(", ")));
    }
    line
}
