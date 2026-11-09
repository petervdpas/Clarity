//! The Fcdm engine: the C# of the canonical data model, as the fcdm
//! repository writes it. A data object's version becomes a class (and a
//! root's an interface), an enum a C# enum, each in its domain's namespace
//! (`Fontys.Fcdm.Hr`) or its version's (`Fontys.Fcdm.Hr.v3`). The layout is
//! the repository's: block namespaces, four spaces, `System` usings first,
//! a constructor that takes every property and `{ get; init; }` properties
//! with a blank line between. A constructor of up to three parameters that
//! fits in 120 characters stands on one line, any other has a parameter per
//! line. The text has no byte order mark: the file it goes into gets one.

use serde::Deserialize;

pub mod diff;
pub mod events;
pub mod layout;
pub mod read;

clarity_engine::interface!();

/// A version of a data object, as the application hands it over.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSpec {
    pub domain: String,
    pub name: String,
    /// A root has an interface; a child none.
    pub root: bool,
    /// Whether it has a business key (`Bk`): every root has, but a few.
    #[serde(default = "yes")]
    pub keyed: bool,
    /// Whether its domain's project uses Newtonsoft.Json, so an enum gets its converter too.
    #[serde(default = "yes")]
    pub newtonsoft: bool,
    /// 1 for the domain's own namespace, 2 and up for its `vN`.
    pub version: i64,
    #[serde(default)]
    pub summary: String,
    /// Why it is no longer to be used; empty while it is.
    #[serde(default)]
    pub obsolete: String,
}

fn yes() -> bool {
    true
}

/// A property of a version, in its order.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertySpec {
    pub name: String,
    /// The kind by its name in the app: `TextValue`, `HeldObjects`, ...
    pub kind: String,
    #[serde(default)]
    pub nullable: bool,
    /// The enum or data object it names, for an enum choice or a held object.
    #[serde(default)]
    pub type_name: String,
    /// The version of the held object it takes (the newest up to its own).
    #[serde(default)]
    pub type_version: i64,
    /// What it starts as when it was added later: `null`, `0`.
    #[serde(default)]
    pub initial: String,
    /// An expression in C#: a property worked out from the others.
    #[serde(default)]
    pub computed: String,
    #[serde(default)]
    pub obsolete: String,
}

/// An enum of a domain.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumSpec {
    pub domain: String,
    pub name: String,
}

/// A value of an enum, with the number it is kept as.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumValueSpec {
    pub name: String,
    pub number: i64,
}

clarity_engine::function!(classCode, |(object, properties): (ObjectSpec, Vec<PropertySpec>)| Ok(write_class(&object, &properties)));
clarity_engine::function!(interfaceCode, |(object, properties): (ObjectSpec, Vec<PropertySpec>)| Ok(write_interface(&object, &properties)));
clarity_engine::function!(readFiles, |(files,): (Vec<read::SourceFile>,)| Ok(read::read_files(&files)));
clarity_engine::function!(arrangeBoxes, |(boxes, links, grid, width): (Vec<layout::LayoutBox>, Vec<layout::LayoutLink>, i64, i64)| Ok(layout::arrange(&boxes, &links, grid, width)));
clarity_engine::function!(routeLinks, |(boxes, links, grid, width): (Vec<layout::LayoutBox>, Vec<layout::LayoutLink>, i64, i64)| Ok(layout::route(&boxes, &links, grid, width)));
clarity_engine::function!(diffText, |(path, on_disk, written): (String, String, String)| Ok(diff::unified(&path, &on_disk, &written)));
clarity_engine::function!(fileBytes, |(text,): (String,)| Ok(clarity_engine::Bytes(with_mark(&text))));
clarity_engine::function!(eventCode, |(spec, fields): (events::EventSpec, Vec<events::EventFieldSpec>)| Ok(events::write_event(&spec, &fields)));
clarity_engine::function!(enumCode, |(spec, values): (EnumSpec, Vec<EnumValueSpec>)| Ok(write_enum(&spec, &values)));

pub(crate) const INDENT: &str = "    ";
/// The longest constructor line that stays on one line.
pub(crate) const ONE_LINE: usize = 120;
/// The most parameters a constructor on one line takes.
pub(crate) const ONE_LINE_PARAMETERS: usize = 3;

/// The namespace of a version of something in `domain`: the domain's own for version 1, its `vN` after.
fn namespace(domain: &str, version: i64, under: &str) -> String {
    let base = format!("Fontys.Fcdm.{domain}{under}");
    if version > 1 { format!("{base}.v{version}") } else { base }
}

/// A property's C# type, as written in a class of version `own`.
fn type_text(p: &PropertySpec, own: i64) -> String {
    typed(p, own, None)
}

/// A property's C# type in an interface of `domain`, whose namespace holds no data object: a later version is named in full, `Onderwijs.v3.Opleiding`.
fn interface_type(p: &PropertySpec, domain: &str) -> String {
    typed(p, 0, Some(domain))
}

fn typed(p: &PropertySpec, own: i64, interface_of: Option<&str>) -> String {
    let held = |name: &str| match interface_of {
        Some(domain) if p.type_version > 1 => format!("{domain}.v{}.{name}", p.type_version),
        Some(_) => name.to_string(),
        // A held object of another version is named from the domain's namespace: `v3.Arbeidscontract`.
        None if p.type_version != own && p.type_version > 1 => format!("v{}.{name}", p.type_version),
        None => name.to_string(),
    };
    let base = match p.kind.as_str() {
        "TextValue" | "BusinessKey" => "string".to_string(),
        "WholeNumber" => "int".to_string(),
        "DecimalNumber" => "decimal".to_string(),
        "YesNo" => "bool".to_string(),
        "Moment" => "DateTime".to_string(),
        "MomentWithOffset" => "DateTimeOffset".to_string(),
        "DayOnly" => "DateOnly".to_string(),
        "EnumChoice" => p.type_name.clone(),
        "HeldObject" => held(&p.type_name),
        "HeldObjects" => format!("IEnumerable<{}>", held(&p.type_name)),
        other => other.to_string(),
    };
    if p.nullable { format!("{base}?") } else { base }
}

/// The words C# reserves or gives a meaning in places, which a parameter takes with `@` before it (`@alias`).
const KEYWORDS: [&str; 92] = [
    "abstract", "add", "alias", "and", "as", "ascending", "async", "await", "base", "bool", "break", "by", "byte", "case", "catch", "char", "checked", "class", "const", "continue",
    "decimal", "default", "delegate", "descending", "do", "double", "dynamic", "else", "enum", "equals", "event", "explicit", "extern", "false", "finally", "fixed", "float", "for",
    "foreach", "from", "get", "global", "goto", "group", "if", "implicit", "in", "init", "int", "interface", "internal", "into", "is", "join", "let", "lock", "long", "nameof",
    "namespace", "new", "not", "null", "object", "on", "operator", "or", "orderby", "out", "override", "params", "private", "protected", "public", "readonly", "record", "ref",
    "remove", "return", "sbyte", "sealed", "select", "set", "short", "sizeof", "static", "string", "struct", "switch", "this", "throw", "value", "var",
];

/// The parameter a constructor names a property by: its name with a small first letter, with `@` before a word C# keeps.
pub(crate) fn parameter(name: &str) -> String {
    let mut chars = name.chars();
    let word: String = match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    };
    if KEYWORDS.contains(&word.as_str()) { format!("@{word}") } else { word }
}

fn uses_time(properties: &[PropertySpec]) -> bool {
    properties.iter().any(|p| matches!(p.kind.as_str(), "Moment" | "MomentWithOffset" | "DayOnly"))
}

fn uses_lists(properties: &[PropertySpec]) -> bool {
    properties.iter().any(|p| p.kind == "HeldObjects")
}

pub(crate) fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The usings a file needs, `System` ones first, then the others by name.
fn usings(mut names: Vec<String>) -> String {
    names.sort_by(|a, b| {
        let system = |n: &str| !(n == "System" || n.starts_with("System."));
        (system(a), a.as_str()).cmp(&(system(b), b.as_str()))
    });
    names.dedup();
    if names.is_empty() {
        return String::new();
    }
    names.iter().map(|n| format!("using {n};\n")).collect::<String>() + "\n"
}

/// The C# class of a version of a data object.
pub fn write_class(o: &ObjectSpec, properties: &[PropertySpec]) -> String {
    let stored: Vec<&PropertySpec> = properties.iter().filter(|p| p.computed.is_empty()).collect();
    let computed: Vec<&PropertySpec> = properties.iter().filter(|p| !p.computed.is_empty()).collect();
    let _ = &stored;
    let enums = properties.iter().any(|p| p.kind == "EnumChoice");
    let mut names = Vec::new();
    if uses_time(properties) || !o.obsolete.is_empty() || properties.iter().any(|p| !p.obsolete.is_empty()) {
        names.push("System".to_string());
    }
    if uses_lists(properties) {
        names.push("System.Collections.Generic".to_string());
    }
    if enums || !computed.is_empty() {
        names.push("System.Text.Json.Serialization".to_string());
    }
    if o.root {
        names.push(interface_namespace(&o.domain, o.version));
    }
    if enums && o.newtonsoft {
        names.push("Newtonsoft.Json.Converters".to_string());
    }
    let mut out = usings(names);
    out += &format!("namespace {}\n{{\n", namespace(&o.domain, o.version, ""));
    let i1 = INDENT;
    let i2 = INDENT.repeat(2);
    let i3 = INDENT.repeat(3);
    if !o.obsolete.is_empty() {
        out += &format!("{i1}[Obsolete(\"{}\")]\n", quoted(&o.obsolete));
    }
    let interface = if o.root { format!(" : I{}", o.name) } else { String::new() };
    out += &format!("{i1}public class {}{interface}\n{i1}{{\n", o.name);

    // The constructor: the business key of a root first, then every stored property in order.
    let mut parameters: Vec<String> = Vec::new();
    let mut assignments: Vec<String> = Vec::new();
    if o.root && o.keyed {
        parameters.push("string bk".to_string());
        assignments.push("Bk = bk;".to_string());
    }
    for p in &stored {
        let initial = if p.initial.is_empty() { String::new() } else { format!(" = {}", p.initial) };
        parameters.push(format!("{} {}{initial}", type_text(p, o.version), parameter(&p.name)));
        assignments.push(format!("{} = {};", p.name, parameter(&p.name)));
    }
    let one_line = format!("{i2}public {}({})", o.name, parameters.join(", "));
    if parameters.len() <= ONE_LINE_PARAMETERS && one_line.len() <= ONE_LINE {
        out += &one_line;
        out += "\n";
    } else {
        out += &format!("{i2}public {}(\n", o.name);
        out += &parameters.iter().map(|p| format!("{i3}{p}")).collect::<Vec<_>>().join(",\n");
        out += ")\n";
    }
    out += &format!("{i2}{{\n");
    for a in &assignments {
        out += &format!("{i3}{a}\n");
    }
    out += &format!("{i2}}}\n");

    // The properties, a blank line before each.
    if o.root && o.keyed {
        out += &format!("\n{i2}public string Bk {{ get; init; }}\n");
    }
    // In their order, a computed one where it stands among them.
    for p in properties {
        out += "\n";
        if !p.obsolete.is_empty() {
            out += &format!("{i2}[Obsolete(\"{}\")]\n", quoted(&p.obsolete));
        }
        if !p.computed.is_empty() {
            out += &format!("{i2}[JsonIgnore]\n");
            if o.newtonsoft {
                out += &format!("{i2}[Newtonsoft.Json.JsonIgnore]\n");
            }
            if p.computed.starts_with('{') {
                // A getter written as a block: its lines as they were, under the property.
                out += &format!("{i2}public {} {}\n", type_text(p, o.version), p.name);
                for line in p.computed.lines() {
                    if line.is_empty() { out += "\n" } else { out += &format!("{i2}{line}\n") }
                }
            } else {
                out += &format!("{i2}public {} {} => {};\n", type_text(p, o.version), p.name, p.computed);
            }
            continue;
        }
        if p.kind == "EnumChoice" {
            out += &format!("{i2}[System.Text.Json.Serialization.JsonConverter(typeof(JsonStringEnumConverter))]\n");
            if o.newtonsoft {
                out += &format!("{i2}[Newtonsoft.Json.JsonConverter(typeof(StringEnumConverter))]\n");
            }
        }
        out += &format!("{i2}public {} {} {{ get; init; }}\n", type_text(p, o.version), p.name);
    }
    out += &format!("{i1}}}\n}}\n");
    out
}

/// The C# interface of a version of a root data object: every property but its business key, computed ones too, read only, with the attributes the class has.
pub fn write_interface(o: &ObjectSpec, properties: &[PropertySpec]) -> String {
    let enums = properties.iter().any(|p| p.kind == "EnumChoice");
    let mut names = Vec::new();
    if o.keyed {
        names.push("Fontys.Fcdm.Interfaces".to_string());
    }
    if uses_time(properties) || !o.obsolete.is_empty() || properties.iter().any(|p| !p.obsolete.is_empty()) {
        names.push("System".to_string());
    }
    if uses_lists(properties) {
        names.push("System.Collections.Generic".to_string());
    }
    if enums {
        names.push("System.Text.Json.Serialization".to_string());
        if o.newtonsoft {
            names.push("Newtonsoft.Json.Converters".to_string());
        }
    }
    let mut out = usings(names);
    let i1 = INDENT;
    let i2 = INDENT.repeat(2);
    out += &format!("namespace {}\n{{\n", interface_namespace(&o.domain, o.version));
    if !o.obsolete.is_empty() {
        out += &format!("{i1}[Obsolete(\"{}\")]\n", quoted(&o.obsolete));
    }
    let model = if o.keyed { " : IFcdmModel" } else { "" };
    out += &format!("{i1}public interface I{}{model}\n{i1}{{\n", o.name);
    let lines: Vec<String> = properties
        .iter()
        .map(|p| {
            let mut text = String::new();
            if !p.obsolete.is_empty() {
                text += &format!("{i2}[Obsolete(\"{}\")]\n", quoted(&p.obsolete));
            }
            if p.kind == "EnumChoice" {
                text += &format!("{i2}[System.Text.Json.Serialization.JsonConverter(typeof(JsonStringEnumConverter))]\n");
                if o.newtonsoft {
                    text += &format!("{i2}[Newtonsoft.Json.JsonConverter(typeof(StringEnumConverter))]\n");
                }
            }
            text + &format!("{i2}public {} {} {{ get; }}\n", interface_type(p, &o.domain), p.name)
        })
        .collect();
    out += &lines.join("\n");
    out += &format!("{i1}}}\n}}\n");
    out
}

/// The namespace of a root's interface: `Fontys.Fcdm.Hr.Interfaces`, or `.Interfaces.vN` for a later version.
fn interface_namespace(domain: &str, version: i64) -> String {
    let base = format!("Fontys.Fcdm.{domain}.Interfaces");
    if version > 1 { format!("{base}.v{version}") } else { base }
}

/// A file's bytes as the fcdm repository keeps them: UTF-8 with a byte order mark, its lines ending in LF.
pub fn with_mark(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.trim_start_matches('\u{feff}').replace("\r\n", "\n").as_bytes());
    bytes
}

/// The C# enum of a domain, each value with its number.
pub fn write_enum(e: &EnumSpec, values: &[EnumValueSpec]) -> String {
    let i1 = INDENT;
    let i2 = INDENT.repeat(2);
    let mut out = format!("namespace {}\n{{\n{i1}public enum {}\n{i1}{{\n", namespace(&e.domain, 1, ""), e.name);
    out += &values.iter().map(|v| format!("{i2}{} = {}", v.name, v.number)).collect::<Vec<_>>().join(",\n");
    out += &format!("\n{i1}}}\n}}\n");
    out
}

#[cfg(test)]
mod tests;
