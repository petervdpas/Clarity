//! Clarity names and types as C# names and types, the way a C# developer
//! writes them: `CreditLimit` for a field, `creditLimit` for a local,
//! `WithinCredit` for a function.

use clarity_spec::Project;
use clarity_spec::code::check::{TypeInfo, ValueItem};
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

/// Classes and namespaces that generated code names by their simple name:
/// the program's own and the runtime's. A method of `Logic` with one of
/// these names would hide it there.
const TYPE_NAMES: [&str; 37] = [
    "App", "AppThemes", "Choices", "Edit", "Labels", "Logic", "Program", "Schema", "Shown", "Texts", "Views", "Engines",
    "Clarity", "CommunityToolkit", "Microsoft", "System", "Avalonia", "Clock", "Cond", "Contracts", "Db", "Dialect", "Engine",
    "Files", "IRecord", "Numbers", "Op", "Picture", "Query", "RefusedException", "Show", "Slot", "Sql", "Text", "Math", "List",
    "Enum",
];

/// A function or action as a method of `Logic`: `within_credit` as
/// `WithinCredit`. A name that a type already has (an enum, a record, a
/// window's classes, a class of the program or the runtime) takes a
/// trailing `_`: `size` beside `enum Size` is `Size_`.
pub fn function(project: &Project, name: &str) -> String {
    let cs = pascal(name);
    let types = project.entities().map(|e| e.name.name.clone()).chain(project.enums().map(|e| e.name.name.clone()));
    let windows = project.pages().flat_map(|p| [format!("{}View", p.name.name), format!("{}ViewModel", p.name.name)]);
    let apps = project.apps().map(|a| a.name.name.clone());
    let taken = TYPE_NAMES.contains(&cs.as_str()) || types.chain(windows).chain(apps).any(|n| n == cs);
    if taken { format!("{cs}_") } else { cs }
}

/// Names the generated code uses itself, for locals.
const TAKEN: [&str; 1] = ["db"];

/// Members every record has, which a field cannot be called.
const RECORD_MEMBERS: [&str; 21] = [
    "Id",
    // The settings' own members (model::settings_class).
    "Current",
    "Start",
    "ToJson",
    "Described",
    "ProgramName",
    "Pictures",
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

/// Members every window's ViewModel has, which data or a part cannot be called.
const VIEW_MODEL_MEMBERS: [&str; 26] = [
    "Db", "Title", "IsDialog", "Messages", "Say", "Handle", "Open", "Close", "Opened", "Closing", "Returned", "OnOpen", "OnClose", "OnReturn",
    "QuitApplication", "Selected", "OpenRequested", "CloseRequested", "PropertyChanged", "PropertyChanging",
    "OnPropertyChanged", "OnPropertyChanging", "SetProperty", "Equals", "GetHashCode", "ToString",
];

/// Window data or a part as a member of the window's ViewModel: `orders`
/// as `Orders`, never a member every ViewModel has.
pub fn member(name: &str) -> String {
    let name = pascal(name);
    if VIEW_MODEL_MEMBERS.contains(&name.as_str()) { format!("{name}_") } else { name }
}

/// A part as a control's `x:Name` in AXAML: `number_input` as `numberInput`.
/// Camel case never meets a member of the window, which are all Pascal case.
pub fn control(name: &str) -> String {
    let local = camel(name);
    match local.strip_prefix('@') {
        Some(keyword) => format!("{keyword}Part"),
        None => local,
    }
}

/// The private field behind a property: `credit_limit` as `_creditLimit`,
/// never a keyword.
pub fn backing(name: &str) -> String {
    let pascal = pascal(name);
    let mut chars = pascal.chars();
    let first: String = chars.next().map(|c| c.to_lowercase().collect()).unwrap_or_default();
    format!("_{first}{}", chars.as_str())
}

/// The C# type of a stored field.
pub fn field_type(ty: &FieldType) -> String {
    match ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme | FieldType::Secret => "string".into(),
        FieldType::Integer | FieldType::Id | FieldType::Duration => "long".into(),
        FieldType::Decimal { .. } | FieldType::Money => "decimal".into(),
        FieldType::Bool => "bool".into(),
        FieldType::Date => "DateOnly?".into(),
        FieldType::DateTime => "DateTime?".into(),
        FieldType::Enum { name } => name.name.clone(),
        FieldType::Ref { .. } => "long?".into(),
        FieldType::Image => "Picture?".into(),
        FieldType::File => "RecordFile?".into(),
        FieldType::List { item } => format!("List<{}>", item.name),
        FieldType::Record { entity } => entity.name.clone(),
        FieldType::Values { item } => format!("List<{}>", value_type(&ValueItem::of_list(item).info())),
    }
}

/// The C# type of a value in code.
pub fn value_type(ty: &TypeInfo) -> String {
    match ty {
        TypeInfo::Text => "string".into(),
        TypeInfo::Integer | TypeInfo::Duration => "long".into(),
        TypeInfo::Decimal | TypeInfo::Money => "decimal".into(),
        TypeInfo::Bool => "bool".into(),
        TypeInfo::Date => "DateOnly?".into(),
        TypeInfo::DateTime => "DateTime?".into(),
        TypeInfo::Image => "Picture?".into(),
        TypeInfo::RecordFile => "RecordFile?".into(),
        TypeInfo::File => "FileEntry".into(),
        TypeInfo::Files => "List<FileEntry>".into(),
        TypeInfo::Bytes => "byte[]".into(),
        TypeInfo::HttpReply => "HttpReply".into(),
        TypeInfo::Enum(name) | TypeInfo::Record(name) => name.clone(),
        TypeInfo::List(item) => format!("List<{item}>"),
        TypeInfo::Values(item) => format!("List<{}>", value_type(&item.info())),
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
        assert_eq!(backing("credit_limit"), "_creditLimit");
        assert_eq!(backing("event"), "_event");
        assert_eq!(member("orders"), "Orders");
        assert_eq!(member("messages"), "Messages_");
    }

    #[test]
    fn literals() {
        assert_eq!(string("say \"hi\"\n"), "\"say \\\"hi\\\"\\n\"");
        assert_eq!(verbatim("CREATE TABLE \"order\""), "@\"CREATE TABLE \"\"order\"\"\"");
    }
}
