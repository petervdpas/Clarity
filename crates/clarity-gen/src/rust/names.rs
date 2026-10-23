//! Clarity names and types as Rust names and types.

use clarity_spec::code::check::TypeInfo;
use clarity_spec::model::FieldType;

/// Rust keywords, which a Clarity name may still use.
const KEYWORDS: [&str; 50] = [
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "abstract", "become",
    "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield",
];

/// Names the generated code uses itself.
const TAKEN: [&str; 3] = ["db", "Ok", "Err"];

/// A Clarity name as a Rust identifier.
pub fn ident(name: &str) -> String {
    if TAKEN.contains(&name) || matches!(name, "self" | "Self" | "crate" | "super") {
        format!("{name}_")
    } else if KEYWORDS.contains(&name) || name == "try" || name == "gen" {
        format!("r#{name}")
    } else {
        name.to_string()
    }
}

/// `CustomerKind` and `My shop` as `customer_kind` and `my_shop`.
pub fn snake(name: &str) -> String {
    clarity_spec::model::snake_case(name)
}
/// The Rust type of a stored field.
pub fn field_type(ty: &FieldType) -> String {
    match ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme => "String".into(),
        FieldType::Integer | FieldType::Id | FieldType::Duration => "i64".into(),
        FieldType::Decimal { .. } | FieldType::Money => "Decimal".into(),
        FieldType::Bool => "bool".into(),
        FieldType::Date => "Option<NaiveDate>".into(),
        FieldType::DateTime => "Option<NaiveDateTime>".into(),
        FieldType::Enum { name } => name.name.clone(),
        FieldType::Ref { .. } => "Option<Id>".into(),
        FieldType::Image => "Option<Picture>".into(),
        FieldType::File => "Option<RecordFile>".into(),
        FieldType::List { item } => format!("Vec<{}>", item.name),
    }
}

/// The Rust type of a value in code.
pub fn value_type(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Text => "String".into(),
        TypeInfo::Integer | TypeInfo::Duration => "i64".into(),
        TypeInfo::Decimal | TypeInfo::Money => "Decimal".into(),
        TypeInfo::Bool => "bool".into(),
        TypeInfo::Date => "Option<NaiveDate>".into(),
        TypeInfo::DateTime => "Option<NaiveDateTime>".into(),
        TypeInfo::Image => "Option<Picture>".into(),
        TypeInfo::RecordFile => "Option<RecordFile>".into(),
        TypeInfo::File => "FileEntry".into(),
        TypeInfo::Files => "Vec<FileEntry>".into(),
        TypeInfo::Bytes => "Vec<u8>".into(),
        TypeInfo::Enum(name) | TypeInfo::Record(name) => name.clone(),
        TypeInfo::List(item) => format!("Vec<{item}>"),
        TypeInfo::Values(item) => format!("Vec<{}>", value_type(&item.info())),
        TypeInfo::Unknown => "()".into(),
    }
}

/// A decimal written in Clarity (`9.95`) as a Rust `Decimal` expression.
pub fn decimal(text: &str) -> String {
    let negative = text.starts_with('-');
    let digits = text.trim_start_matches('-');
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let mantissa = format!("{whole}{fraction}").trim_start_matches('0').to_string();
    let mantissa = if mantissa.is_empty() { "0".to_string() } else { mantissa };
    let sign = if negative { "-" } else { "" };
    format!("Decimal::new({sign}{mantissa}, {})", fraction.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(snake("CustomerKind"), "customer_kind");
        assert_eq!(snake("My shop"), "my_shop");
        assert_eq!(snake("Order"), "order");
        assert_eq!(snake("2026 plan"), "app_2026_plan");
        assert_eq!(ident("type"), "r#type");
        assert_eq!(ident("db"), "db_");
        assert_eq!(ident("self"), "self_");
        assert_eq!(ident("total"), "total");
    }

    #[test]
    fn decimals() {
        assert_eq!(decimal("9.95"), "Decimal::new(995, 2)");
        assert_eq!(decimal("100"), "Decimal::new(100, 0)");
        assert_eq!(decimal("-0.5"), "Decimal::new(-5, 1)");
        assert_eq!(decimal("0"), "Decimal::new(0, 0)");
    }
}
