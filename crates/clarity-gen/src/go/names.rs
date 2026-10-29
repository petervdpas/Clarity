//! Clarity names and types as Go names and types, the way a Go developer
//! writes them: `CreditLimit` for an exported field, `creditLimit` for a
//! local, `WithinCredit` for a function.

use clarity_spec::Project;
use clarity_spec::code::check::{TypeInfo, ValueItem};
use clarity_spec::model::FieldType;

/// Go's keywords and predeclared names, which a local must not take.
const RESERVED: [&str; 62] = [
    "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough", "for", "func", "go", "goto", "if",
    "import", "interface", "map", "package", "range", "return", "select", "struct", "switch", "type", "var", "any", "append", "bool",
    "byte", "cap", "clear", "close", "complex", "copy", "delete", "error", "false", "float32", "float64", "imag", "int", "int8",
    "int16", "int32", "int64", "iota", "len", "make", "max", "min", "new", "nil", "panic", "print", "println", "real", "recover",
    "rune", "string", "true", "uint", "uintptr",
];

/// Names the generated code uses itself.
const TAKEN: [&str; 12] = ["db", "err", "result", "body", "read", "r", "w", "ui", "app", "clarity", "decimal", "slices"];

/// Members every record has, which a field cannot be called.
const RECORD_MEMBERS: [&str; 13] =
    ["ID", "Table", "Columns", "GetID", "SetID", "Values", "Load", "Source", "Key", "BeforeSave", "AfterSave", "BeforeDelete", "AfterDelete"];

/// `credit_limit` and `My shop` as `CreditLimit` and `MyShop`: exported.
pub fn pascal(name: &str) -> String {
    let mut out = String::new();
    for part in name.split(|c: char| !c.is_alphanumeric()).filter(|p| !p.is_empty()) {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, 'N');
    }
    out
}

/// Top-level names every generated program declares itself.
const PROGRAM_NAMES: [&str; 3] = ["Clarity", "Sources", "SourceSteps"];

/// A function or action as a Go function: `within_credit` as `WithinCredit`.
/// A name that another top-level declaration of the program already has (a
/// type, an enum value, a `NewX` constructor, a window's struct) takes a
/// trailing `_`: `size` beside `enum Size` is `Size_`.
pub fn function(project: &Project, name: &str) -> String {
    let go = pascal(name);
    let types = project.entities().map(|e| e.name.name.clone()).chain(project.enums().map(|e| e.name.name.clone()));
    let constructors = project.entities().map(|e| format!("New{}", e.name.name));
    let constants = project.enums().flat_map(|e| e.variants.iter().map(|v| super::model::constant(&e.name.name, &v.name.name)));
    let windows = project.pages().map(|p| format!("{}Window", p.name.name)).chain(project.apps().map(|a| format!("{}App", a.name.name)));
    let taken = PROGRAM_NAMES.iter().map(|n| n.to_string()).chain(types).chain(constructors).chain(constants).chain(windows).any(|n| n == go);
    if taken { format!("{go}_") } else { go }
}

/// A Clarity name as a Go local or parameter: `credit_limit` as `creditLimit`.
pub fn local(name: &str) -> String {
    let pascal = pascal(name);
    let mut chars = pascal.chars();
    let mut out: String = chars.next().map(|c| c.to_lowercase().collect()).unwrap_or_default();
    out.push_str(chars.as_str());
    let lifted = out.strip_prefix('v').is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    if RESERVED.contains(&out.as_str()) || TAKEN.contains(&out.as_str()) || lifted { format!("{out}_") } else { out }
}

/// A field of a record: exported, never the name of a member every record has.
pub fn field(name: &str) -> String {
    let name = pascal(name);
    if RECORD_MEMBERS.contains(&name.as_str()) { format!("{name}_") } else { name }
}

/// The Go module and package name of a project: `My shop` as `myshop`.
pub fn module(name: &str) -> String {
    let out: String = name.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) { format!("app{out}") } else { out }
}

/// Text as a Go string literal.
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

/// The Go type of a field.
pub fn field_type(ty: &FieldType) -> String {
    match ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret => "string".into(),
        FieldType::Integer | FieldType::Id | FieldType::Duration => "int64".into(),
        FieldType::Decimal { .. } | FieldType::Money => "decimal.Decimal".into(),
        FieldType::Bool => "bool".into(),
        FieldType::Date => "*clarity.Date".into(),
        FieldType::DateTime => "*clarity.DateTime".into(),
        FieldType::Enum { name } => name.name.clone(),
        FieldType::Ref { .. } => "*int64".into(),
        FieldType::Image => "*clarity.Picture".into(),
        FieldType::File => "*clarity.RecordFile".into(),
        FieldType::List { item } => format!("[]{}", item.name),
        FieldType::Record { entity } => entity.name.clone(),
        FieldType::Values { item } => format!("[]{}", value_type(&ValueItem::of_list(item).info())),
    }
}

/// The Go type of a value in code; records are worked on through pointers.
pub fn value_type(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Text => "string".into(),
        TypeInfo::Integer | TypeInfo::Duration => "int64".into(),
        TypeInfo::Decimal | TypeInfo::Money => "decimal.Decimal".into(),
        TypeInfo::Bool => "bool".into(),
        TypeInfo::Date => "*clarity.Date".into(),
        TypeInfo::DateTime => "*clarity.DateTime".into(),
        TypeInfo::Image => "*clarity.Picture".into(),
        TypeInfo::RecordFile => "*clarity.RecordFile".into(),
        TypeInfo::File => "clarity.FileEntry".into(),
        TypeInfo::Files => "[]clarity.FileEntry".into(),
        TypeInfo::Bytes => "[]byte".into(),
        TypeInfo::HttpReply => "clarity.HttpReply".into(),
        TypeInfo::Enum(name) => name.clone(),
        TypeInfo::Record(name) => format!("*{name}"),
        TypeInfo::List(name) => format!("[]{name}"),
        TypeInfo::Values(item) => format!("[]{}", value_type(&item.info())),
        TypeInfo::Unknown => "any".into(),
    }
}

/// The zero value of a type, for a return with an error.
pub fn zero(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Text | TypeInfo::Enum(_) => "\"\"".into(),
        TypeInfo::Integer | TypeInfo::Duration => "0".into(),
        TypeInfo::Decimal | TypeInfo::Money => "decimal.Zero".into(),
        TypeInfo::Bool => "false".into(),
        TypeInfo::File => "clarity.FileEntry{}".into(),
        TypeInfo::HttpReply => "clarity.HttpReply{}".into(),
        _ => "nil".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(pascal("credit_limit"), "CreditLimit");
        assert_eq!(local("credit_limit"), "creditLimit");
        assert_eq!(local("len"), "len_");
        assert_eq!(local("v1"), "v1_");
        assert_eq!(field("values"), "Values_");
        assert_eq!(module("My Shop"), "myshop");
        assert_eq!(string("a \"b\"\n"), "\"a \\\"b\\\"\\n\"");
    }
}
