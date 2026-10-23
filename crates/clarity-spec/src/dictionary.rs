//! The data dictionary as the designer edits it: fields written back as
//! canonical spec lines, from a name, a type as written and modifiers.

use serde::Deserialize;
use ts_rs::TS;

use crate::diagnostic::Severity;
use crate::lexer::{quote, text_token};
use crate::model::{FieldType, Literal, Modifiers};
use crate::parser::parse;
use crate::widgets::is_identifier;

/// A field as the designer sends it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FieldSpec {
    pub name: String,
    /// The type as written in spec files: `text(80)`, `ref Customer`.
    pub type_text: String,
    pub modifiers: Modifiers,
}

/// The type of a field as written in spec files.
pub fn type_text(ty: &FieldType) -> String {
    match ty {
        FieldType::Text { max_length: None } => "text".to_string(),
        FieldType::Text {
            max_length: Some(n),
        } => format!("text({n})"),
        FieldType::Integer => "integer".to_string(),
        FieldType::Decimal { precision, scale } => format!("decimal({precision}, {scale})"),
        FieldType::Money => "money".to_string(),
        FieldType::Bool => "bool".to_string(),
        FieldType::Date => "date".to_string(),
        FieldType::DateTime => "datetime".to_string(),
        FieldType::Duration => "duration".to_string(),
        FieldType::Image => "image".to_string(),
        FieldType::Email => "email".to_string(),
        FieldType::Url => "url".to_string(),
        FieldType::Color => "color".to_string(),
        FieldType::File => "file".to_string(),
        FieldType::Language => "language".to_string(),
        FieldType::Theme => "theme".to_string(),
        FieldType::Id => "id".to_string(),
        FieldType::Enum { name } => name.name.clone(),
        FieldType::Ref { entity } => format!("ref {}", entity.name),
        FieldType::List { item } => format!("list<{}>", item.name),
        FieldType::Values { item } => format!("list<{}>", item.word()),
    }
}

/// A literal value as written in spec files.
pub fn literal_text(literal: &Literal) -> String {
    match literal {
        Literal::Number(n) => n.clone(),
        Literal::Str(s) => text_token(s),
        Literal::Bool(b) => b.to_string(),
        Literal::Ident(name) => name.clone(),
    }
}

/// Writes a field as one canonical spec line, modifiers in a fixed order.
pub fn field_line(spec: &FieldSpec) -> String {
    let m = &spec.modifiers;
    let mut parts = vec![format!("{}: {}", spec.name, spec.type_text.trim())];
    for (on, word) in [
        (m.required, "required"),
        (m.unique, "unique"),
        (m.owned, "owned"),
        (m.computed, "computed"),
    ] {
        if on {
            parts.push(word.to_string());
        }
    }
    if let Some(format) = &m.format {
        parts.push(format!("format {format}"));
    }
    for (value, word) in [(&m.min, "min"), (&m.max, "max"), (&m.default, "default")] {
        if let Some(value) = value {
            parts.push(format!("{word} {}", literal_text(value)));
        }
    }
    for (value, word) in [
        (&m.pattern, "pattern"),
        (&m.label, "label"),
        (&m.help, "help"),
        (&m.column, "column"),
    ] {
        if let Some(value) = value {
            // A label and a help text are read by people: keys.
            let written = if matches!(word, "label" | "help") { text_token(value) } else { quote(value) };
            parts.push(format!("{word} {written}"));
        }
    }
    if let Some(store) = &m.store {
        parts.push(format!("store {store}"));
    }
    parts.join(", ")
}

/// Checks a field before it is written: the name is an identifier and the
/// line parses as a field.
pub fn validate_field(spec: &FieldSpec) -> Result<String, String> {
    if !is_identifier(&spec.name) {
        return Err(format!("'{}' is not a valid field name", spec.name));
    }
    let line = field_line(spec);
    let probe = format!("module m\nentity E\n    {line}\n");
    let (_, diagnostics) = parse(&probe, "");
    match diagnostics.iter().find(|d| d.severity == Severity::Error) {
        Some(error) => Err(error.message.clone()),
        None => Ok(line),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_fields_canonically() {
        let spec = FieldSpec {
            name: "credit_limit".into(),
            type_text: "money".into(),
            modifiers: Modifiers {
                required: true,
                min: Some(Literal::Number("0".into())),
                default: Some(Literal::Number("0".into())),
                label: Some("Credit \"limit\"".into()),
                ..Modifiers::default()
            },
        };
        assert_eq!(
            field_line(&spec),
            "credit_limit: money, required, min 0, default 0, label \"Credit \\\"limit\\\"\""
        );
        assert!(validate_field(&spec).is_ok());
    }

    #[test]
    fn rejects_bad_names_and_types() {
        let mut spec = FieldSpec {
            name: "bad name".into(),
            type_text: "text".into(),
            modifiers: Modifiers::default(),
        };
        assert!(validate_field(&spec).is_err());
        spec.name = "ok".into();
        spec.type_text = "decimal".into();
        assert!(validate_field(&spec).unwrap_err().contains("precision"));
    }
}
