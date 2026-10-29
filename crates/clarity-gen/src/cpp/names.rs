//! Names and types as the C++ output writes them: Clarity's snake-case
//! names stay as they are, with a trailing `_` where C++ reserves the word.

use clarity_spec::code::check::{TypeInfo, ValueItem};
use clarity_spec::model::FieldType;

/// The words C++ reserves, and the names a record or window already uses.
const RESERVED: [&str; 97] = [
    "alignas", "alignof", "and", "and_eq", "asm", "auto", "bitand", "bitor", "bool", "break", "case", "catch", "char", "char8_t", "char16_t",
    "char32_t", "class", "compl", "concept", "const", "consteval", "constexpr", "constinit", "const_cast", "continue", "co_await",
    "co_return", "co_yield", "decltype", "default", "delete", "do", "double", "dynamic_cast", "else", "enum", "explicit", "export",
    "extern", "false", "float", "for", "friend", "goto", "if", "inline", "int", "long", "mutable", "namespace", "new", "noexcept", "not",
    "not_eq", "nullptr", "operator", "or", "or_eq", "private", "protected", "public", "register", "reinterpret_cast", "requires", "return",
    "short", "signed", "sizeof", "static", "static_assert", "static_cast", "struct", "switch", "template", "this", "thread_local", "throw",
    "true", "try", "typedef", "typeid", "typename", "union", "unsigned", "using", "virtual", "void", "volatile", "wchar_t", "while", "xor",
    "xor_eq",
    // What every record has, and the two arguments of every handler.
    "table", "key", "source", "db", "ui",
];

/// A Clarity name as a C++ name.
pub fn ident(name: &str) -> String {
    if RESERVED.contains(&name) || ["columns", "values", "pictures", "from_values", "load"].contains(&name) { format!("{name}_") } else { name.to_string() }
}

/// `order_status` and `My shop` as `OrderStatus` and `MyShop`.
pub fn pascal(name: &str) -> String {
    name.split(['_', ' ', '-'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
        })
        .collect()
}

/// `OrderStatus` as `order_status`.
pub fn snake(name: &str) -> String {
    clarity_spec::model::snake_case(name)
}

/// Text as a C++ string literal.
pub fn string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // A `??` could read as a trigraph in old compilers.
            '?' => out.push_str("\\?"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The C++ type of a stored field.
pub fn field_type(ty: &FieldType) -> String {
    match ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret => "std::string".into(),
        FieldType::Integer | FieldType::Id | FieldType::Duration => "int64_t".into(),
        FieldType::Decimal { .. } | FieldType::Money => "clarity::Decimal".into(),
        FieldType::Bool => "bool".into(),
        FieldType::Date => "std::optional<clarity::Date>".into(),
        FieldType::DateTime => "std::optional<clarity::DateTime>".into(),
        FieldType::Enum { name } => name.name.clone(),
        FieldType::Ref { .. } => "std::optional<int64_t>".into(),
        FieldType::Image => "std::optional<clarity::Picture>".into(),
        FieldType::File => "std::optional<clarity::RecordFile>".into(),
        FieldType::List { item } => format!("std::vector<{}>", item.name),
        FieldType::Record { entity } => entity.name.clone(),
        FieldType::Values { item } => format!("std::vector<{}>", value_type(&ValueItem::of_list(item).info())),
    }
}

/// The C++ type of a value in code.
pub fn value_type(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Text => "std::string".into(),
        TypeInfo::Integer | TypeInfo::Duration => "int64_t".into(),
        TypeInfo::Decimal | TypeInfo::Money => "clarity::Decimal".into(),
        TypeInfo::Bool => "bool".into(),
        TypeInfo::Date => "std::optional<clarity::Date>".into(),
        TypeInfo::DateTime => "std::optional<clarity::DateTime>".into(),
        TypeInfo::Image => "std::optional<clarity::Picture>".into(),
        TypeInfo::RecordFile => "std::optional<clarity::RecordFile>".into(),
        TypeInfo::File => "clarity::FileEntry".into(),
        TypeInfo::Files => "std::vector<clarity::FileEntry>".into(),
        TypeInfo::Bytes => "clarity::Bytes".into(),
        TypeInfo::HttpReply => "clarity::HttpReply".into(),
        TypeInfo::Enum(name) | TypeInfo::Record(name) => name.clone(),
        TypeInfo::List(item) => format!("std::vector<{item}>"),
        TypeInfo::Values(item) => format!("std::vector<{}>", value_type(&item.info())),
        TypeInfo::Unknown => "auto".into(),
    }
}

/// A parameter of a function or action: a record is changed in place, as
/// on the other stacks; everything else is a value.
pub fn param_type(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Record(name) => format!("{name}&"),
        other => value_type(other),
    }
}

/// A decimal written in Clarity (`9.95`) as a C++ expression.
pub fn decimal(text: &str) -> String {
    format!("clarity::Decimal::parse({})", string(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_literals() {
        assert_eq!(ident("delete"), "delete_");
        assert_eq!(ident("table"), "table_");
        assert_eq!(ident("credit_limit"), "credit_limit");
        assert_eq!(pascal("order_status"), "OrderStatus");
        assert_eq!(string("say \"hi\"\n?"), "\"say \\\"hi\\\"\\n\\?\"");
    }
}
