//! Names and types as the C output writes them: Clarity's snake-case names
//! stay as they are, with a trailing `_` where C or its library already
//! uses the word, since C has one space of names for the whole program.

use clarity_spec::code::check::TypeInfo;
use clarity_spec::model::FieldType;

/// The words C reserves, the names of the C library a program may well
/// define by accident, and the names generated code gives its own values.
const RESERVED: [&str; 98] = [
    "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "enum", "extern", "float", "for", "goto", "if",
    "inline", "int", "long", "register", "restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef", "union",
    "unsigned", "void", "volatile", "while", "bool", "true", "false", "NULL", "main",
    // The C library and POSIX.
    "abs", "atoi", "calloc", "ceil", "clock", "close", "cos", "difftime", "div", "errno", "exit", "exp", "fabs", "fclose", "floor", "fopen",
    "free", "getenv", "index", "isalpha", "isdigit", "labs", "link", "log", "malloc", "memcpy", "mkdir", "open", "pow", "printf", "puts",
    "qsort", "rand", "random", "read", "realloc", "remove", "rename", "round", "select", "signal", "sin", "sleep", "sqrt", "stat",
    "strcmp", "strlen", "system", "tan", "time", "unlink", "write",
    // What generated code names itself: the database, the screen, the
    // window, a function's result, and its lifted values.
    "db", "ui", "w", "result", "in", "out", "record",
];

/// A Clarity name as a C name.
pub fn ident(name: &str) -> String {
    let lifted = name.len() > 1 && (name.starts_with('v') || name.starts_with('i')) && name[1..].chars().all(|c| c.is_ascii_digit());
    if RESERVED.contains(&name) || lifted { format!("{name}_") } else { name.to_string() }
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

/// The prefix of what belongs to a type: `order_status` of `OrderStatus`,
/// as in `order_status_label` and `customer_save`.
pub fn prefix(type_name: &str) -> String {
    snake(type_name)
}

/// `OrderStatus` and `Approved` as `ORDER_STATUS_APPROVED`.
pub fn constant(enum_name: &str, value: &str) -> String {
    format!("{}_{}", snake(enum_name).to_uppercase(), snake(value).to_uppercase())
}

/// Text as a C string literal.
pub fn string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // A `??` could read as a trigraph.
            '?' => out.push_str("\\?"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}\"\"", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A string literal, or NULL for none.
pub fn string_or_null(text: Option<&str>) -> String {
    text.map_or_else(|| "NULL".to_string(), string)
}

/// The C type of a stored field.
pub fn field_type(ty: &FieldType) -> String {
    match ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme => "const char*".into(),
        // A reference holds the id of its record; 0 is none.
        FieldType::Integer | FieldType::Id | FieldType::Duration | FieldType::Ref { .. } => "int64_t".into(),
        FieldType::Decimal { .. } | FieldType::Money => "ClarityDecimal".into(),
        FieldType::Bool => "bool".into(),
        FieldType::Date => "ClarityDate".into(),
        FieldType::DateTime => "ClarityDateTime".into(),
        FieldType::Enum { name } => name.name.clone(),
        FieldType::Image => "ClarityPicture*".into(),
        FieldType::File => "ClarityRecordFile*".into(),
        FieldType::List { item } => list_type(&item.name),
    }
}

/// The struct holding records of `entity`: `CustomerList`.
pub fn list_type(entity: &str) -> String {
    format!("{entity}List")
}

/// The C type of a list of plain values: `ClarityIntegers`.
pub fn values_type(item: clarity_spec::code::check::ValueItem) -> String {
    use clarity_spec::code::check::ValueItem;
    match item {
        ValueItem::Text => "ClarityTexts",
        ValueItem::Integer => "ClarityIntegers",
        ValueItem::Decimal => "ClarityDecimals",
        ValueItem::Bool => "ClarityBools",
    }
    .into()
}

/// The C type of a value in code. A record is its struct; code passes and
/// keeps records through pointers to them.
pub fn value_type(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Text => "const char*".into(),
        TypeInfo::Integer | TypeInfo::Duration => "int64_t".into(),
        TypeInfo::Decimal | TypeInfo::Money => "ClarityDecimal".into(),
        TypeInfo::Bool => "bool".into(),
        TypeInfo::Date => "ClarityDate".into(),
        TypeInfo::DateTime => "ClarityDateTime".into(),
        TypeInfo::Image => "ClarityPicture*".into(),
        TypeInfo::RecordFile => "ClarityRecordFile*".into(),
        TypeInfo::File => "ClarityFile".into(),
        TypeInfo::Files => "ClarityFileList".into(),
        TypeInfo::Bytes => "ClarityBytes".into(),
        TypeInfo::Enum(name) | TypeInfo::Record(name) => name.clone(),
        TypeInfo::List(item) => list_type(item),
        TypeInfo::Values(item) => values_type(*item),
        TypeInfo::Unknown => "int64_t".into(),
    }
}

/// A parameter of a function or action: a record by pointer, changed in
/// place as on the other stacks; everything else a value.
pub fn param_type(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Record(name) => format!("{name}*"),
        other => value_type(other),
    }
}

/// The empty value of a type: no date, no picture, empty text.
pub fn zero(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Text => "\"\"".into(),
        TypeInfo::Image | TypeInfo::RecordFile => "NULL".into(),
        TypeInfo::Bool => "false".into(),
        TypeInfo::Integer | TypeInfo::Duration | TypeInfo::Unknown => "0".into(),
        other => format!("({}){{0}}", value_type(other)),
    }
}

/// A decimal written in Clarity (`9.95`) as a C expression: its units and scale.
pub fn decimal(text: &str) -> String {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let digits = format!("{whole}{fraction}");
    match digits.parse::<i64>() {
        Ok(units) => format!("clarity_decimal({units}, {})", fraction.len()),
        // Too long for 64 bits: read from its text, which the checker found a number.
        Err(_) => format!("clarity_decimal_from_text({})", string(text)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_literals() {
        assert_eq!(ident("switch"), "switch_");
        assert_eq!(ident("remove"), "remove_");
        assert_eq!(ident("v1"), "v1_");
        assert_eq!(ident("credit_limit"), "credit_limit");
        assert_eq!(constant("OrderStatus", "InStock"), "ORDER_STATUS_IN_STOCK");
        assert_eq!(decimal("1250.50"), "clarity_decimal(125050, 2)");
        assert_eq!(decimal("7"), "clarity_decimal(7, 0)");
        assert_eq!(string("say \"hi\"\n?"), "\"say \\\"hi\\\"\\n\\?\"");
    }
}
