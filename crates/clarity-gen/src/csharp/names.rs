//! Clarity names and types as C# names and types, the way a C# developer
//! writes them: `CreditLimit` for a field, `creditLimit` for a local,
//! `WithinCredit` for a function.

use clarity_spec::code::check::TypeInfo;
use clarity_spec::model::FieldType;

/// C# keywords, which a Clarity name may still use; a local escapes them with `@`.
const KEYWORDS: [&str; 77] = [
    "abstract", "as", "base", "bool", "break", "byte", "case", "catch", "char", "checked", "class", "const", "continue",
    "decimal", "default", "delegate", "do", "double", "else", "enum", "event", "explicit", "extern", "false", "finally",
    "fixed", "float", "for", "foreach", "goto", "if", "implicit", "in", "int", "interface", "internal", "is", "lock",
    "long", "namespace", "new", "null", "object", "operator", "out", "override", "params", "private", "protected",
    "public", "readonly", "ref", "return", "sbyte", "sealed", "short", "sizeof", "stackalloc", "static", "string",
    "struct", "switch", "this", "throw", "true", "try", "typeof", "uint", "ulong", "unchecked", "unsafe", "ushort",
    "using", "virtual", "void", "volatile", "while",
];

/// Names the generated code uses itself, for locals.
const TAKEN: [&str; 1] = ["db"];

/// Members every record has, which a field cannot be called.
const RECORD_MEMBERS: [&str; 15] = [
    "Id",
    "SourceName",
    "TableName",
    "KeyColumn",
    "ColumnNames",
    "HasEvents",
    "ToValues",
    "FromValues",
    "BeforeSave",
    "AfterSave",
    "BeforeDelete",
    "AfterDelete",
    "Equals",
    "GetHashCode",
    "ToString",
];

/// `credit_limit`, `My shop` and `CustomerKind` as `CreditLimit`, `MyShop`
/// and `CustomerKind`.
pub fn pascal(name: &str) -> String {
    let mut out = String::new();
    for part in name.split(|c: char| !c.is_alphanumeric()).filter(|p| !p.is_empty()) {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) || out.is_empty() {
        out.insert(0, '_');
    }
    out
}

/// A Clarity name as a C# local or parameter: `credit_limit` as `creditLimit`.
pub fn camel(name: &str) -> String {
    let pascal = pascal(name);
    let mut chars = pascal.chars();
    let mut out: String = chars.next().map(|c| c.to_lowercase().collect()).unwrap_or_default();
    out.push_str(chars.as_str());
    if TAKEN.contains(&out.as_str()) {
        format!("{out}_")
    } else if KEYWORDS.contains(&out.as_str()) {
        format!("@{out}")
    } else {
        out
    }
}

/// A field of `entity` as a C# property: never the name of its class or of a
/// member every record has.
pub fn property(entity: &str, field: &str) -> String {
    let name = pascal(field);
    if name == entity || RECORD_MEMBERS.contains(&name.as_str()) { format!("{name}_") } else { name }
}

/// The C# type of a stored field.
pub fn field_type(ty: &FieldType) -> String {
    match ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url => "string".into(),
        FieldType::Integer | FieldType::Id => "long".into(),
        FieldType::Decimal { .. } | FieldType::Money => "decimal".into(),
        FieldType::Bool => "bool".into(),
        FieldType::Date => "DateOnly?".into(),
        FieldType::DateTime => "DateTime?".into(),
        FieldType::Enum { name } => name.name.clone(),
        FieldType::Ref { .. } => "long?".into(),
        FieldType::List { item } => format!("List<{}>", item.name),
    }
}

/// The C# type of a value in code.
pub fn value_type(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Text => "string".into(),
        TypeInfo::Integer => "long".into(),
        TypeInfo::Decimal | TypeInfo::Money => "decimal".into(),
        TypeInfo::Bool => "bool".into(),
        TypeInfo::Date => "DateOnly?".into(),
        TypeInfo::DateTime => "DateTime?".into(),
        TypeInfo::Enum(name) | TypeInfo::Record(name) => name.clone(),
        TypeInfo::List(item) => format!("List<{item}>"),
        TypeInfo::Unknown => "void".into(),
    }
}

/// Text as a C# string literal.
pub fn string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Text, such as SQL over several lines, as a C# verbatim string.
pub fn verbatim(text: &str) -> String {
    format!("@\"{}\"", text.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(pascal("credit_limit"), "CreditLimit");
        assert_eq!(pascal("CustomerKind"), "CustomerKind");
        assert_eq!(pascal("My shop"), "MyShop");
        assert_eq!(pascal("2026 plan"), "_2026Plan");
        assert_eq!(camel("credit_limit"), "creditLimit");
        assert_eq!(camel("event"), "@event");
        assert_eq!(camel("db"), "db_");
        assert_eq!(property("Order", "order"), "Order_");
        assert_eq!(property("Order", "key_column"), "KeyColumn_");
        assert_eq!(property("Order", "event"), "Event");
    }

    #[test]
    fn literals() {
        assert_eq!(string("say \"hi\"\n"), "\"say \\\"hi\\\"\\n\"");
        assert_eq!(verbatim("CREATE TABLE \"order\""), "@\"CREATE TABLE \"\"order\"\"\"");
    }
}
