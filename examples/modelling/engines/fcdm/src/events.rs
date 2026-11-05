//! The events of a domain, as its `Fontys.Messages.X` project keeps them
//! under `Events/` (version 1) and `Events/vN`. An event is written one of
//! two ways. A base event is one line on fcdm's own base classes, carrying
//! one version of a data object:
//! `public class MedewerkerChangedEvent(Medewerker data, DateTimeOffset changedTimestamp) : BaseChangedEvent<Medewerker>(data, changedTimestamp)`.
//! A field event is an NServiceBus `IEvent` with a constructor and a
//! property per field: business keys, data objects or lists of them, values
//! and its timestamp. Its namespace is kept as it is, since NServiceBus
//! routes a message by its full type name: one that strays from the
//! folder's is flagged for review, never moved.

use serde::Deserialize;

use crate::read::{ReadRow, SourceFile};
use crate::{parameter, quoted, INDENT, ONE_LINE, ONE_LINE_PARAMETERS};

/// An event, as the application hands it over.
#[derive(Debug, Clone, Deserialize)]
pub struct EventSpec {
    pub domain: String,
    pub name: String,
    /// Its event version: 1 for `Events/`, N for `Events/vN`.
    pub version: i64,
    /// Its namespace, as it was published.
    pub namespace: String,
    /// Changed, Deleted, Created, Updated or Other.
    pub kind: String,
    /// Whether it is a base event: one line on BaseChangedEvent or BaseDeletedEvent.
    pub base: bool,
    /// For a base event: the data object it carries, and which version of it.
    #[serde(default)]
    pub type_name: String,
    #[serde(default)]
    pub type_version: i64,
    /// Why its constructor is no longer to be used; empty while it is.
    #[serde(default)]
    pub obsolete: String,
}

/// A field of a field event, in its constructor's order.
#[derive(Debug, Clone, Deserialize)]
pub struct EventFieldSpec {
    pub name: String,
    /// A property kind: TextValue, BusinessKey, HeldObject, HeldObjects, Moment, MomentWithOffset, ...
    pub kind: String,
    #[serde(default)]
    pub nullable: bool,
    /// The data object it holds (or whose key it is), and which version of it.
    #[serde(default)]
    pub type_name: String,
    #[serde(default)]
    pub type_version: i64,
    /// How a list is typed: IEnumerable, Array or List; empty for one value.
    #[serde(default)]
    pub list: String,
    /// Its accessors: `init` (get; init;), `get` (get;) or `set` (get; set;).
    #[serde(default)]
    pub access: String,
    #[serde(default)]
    pub obsolete: String,
}

/// Where an event's file goes: `Fontys.Messages.Hr/Events/v4/MedewerkerChangedEvent.cs`.
pub fn event_path(e: &EventSpec) -> String {
    if e.version > 1 { format!("Fontys.Messages.{}/Events/v{}/{}.cs", e.domain, e.version, e.name) } else { format!("Fontys.Messages.{}/Events/{}.cs", e.domain, e.name) }
}

/// The namespace an event's folder gives: `Fontys.Messages.Hr.Events.v4`.
pub fn folder_namespace(domain: &str, version: i64) -> String {
    if version > 1 { format!("Fontys.Messages.{domain}.Events.v{version}") } else { format!("Fontys.Messages.{domain}.Events") }
}

/// The namespace of a version of a domain's data objects: `Fontys.Fcdm.Hr.v2`.
fn model_namespace(domain: &str, version: i64) -> String {
    if version > 1 { format!("Fontys.Fcdm.{domain}.v{version}") } else { format!("Fontys.Fcdm.{domain}") }
}

/// A data object named as its domain (Account in Account) is taken in by an alias, `AccountFcdm`, since its name is a namespace there too.
fn alias(domain: &str, name: &str) -> Option<String> {
    (domain == name).then(|| format!("{name}Fcdm"))
}

/// The C# of an event.
pub fn write_event(e: &EventSpec, fields: &[EventFieldSpec]) -> String {
    if e.base { write_base(e) } else { write_fields(e, fields) }
}

fn obsolete_line(indent: &str, text: &str) -> String {
    if text.is_empty() { String::new() } else { format!("{indent}[Obsolete(\"{}\")]\n", quoted(text)) }
}

fn write_base(e: &EventSpec) -> String {
    let (usings, carried) = match alias(&e.domain, &e.type_name) {
        Some(a) => (format!("using System;\nusing Fontys.Fcdm;\nusing {a} = {}.{};\n", model_namespace(&e.domain, e.type_version), e.type_name), a),
        None => (format!("using System;\nusing Fontys.Fcdm;\nusing {};\n", model_namespace(&e.domain, e.type_version)), e.type_name.clone()),
    };
    let i1 = INDENT;
    let head = if e.kind == "Deleted" {
        format!("public class {}(string bk, DateTimeOffset deletedTimestamp) : BaseDeletedEvent<{carried}>(bk, deletedTimestamp)", e.name)
    } else {
        format!("public class {}({carried} data, DateTimeOffset changedTimestamp) : BaseChangedEvent<{carried}>(data, changedTimestamp)", e.name)
    };
    format!("{usings}\nnamespace {}\n{{\n{}{i1}{head}\n{i1}{{\n{i1}}}\n}}\n", e.namespace, obsolete_line(i1, &e.obsolete))
}

/// The version namespace most of an event's data objects are named from; the others are named in full.
fn main_version(e: &EventSpec, fields: &[EventFieldSpec]) -> Option<i64> {
    let held: Vec<i64> = fields.iter().filter(|f| f.kind.starts_with("Held") && alias(&e.domain, &f.type_name).is_none()).map(|f| f.type_version.max(1)).collect();
    let mut versions = held.clone();
    versions.sort();
    versions.dedup();
    versions.into_iter().max_by_key(|v| (held.iter().filter(|h| *h == v).count(), -v))
}

fn field_type(e: &EventSpec, f: &EventFieldSpec, main: Option<i64>) -> String {
    let inner = match f.kind.as_str() {
        "TextValue" | "BusinessKey" => "string".to_string(),
        "WholeNumber" => "int".to_string(),
        "DecimalNumber" => "decimal".to_string(),
        "YesNo" => "bool".to_string(),
        "Moment" => "DateTime".to_string(),
        "MomentWithOffset" => "DateTimeOffset".to_string(),
        "DayOnly" => "DateOnly".to_string(),
        "HeldObject" | "HeldObjects" => match alias(&e.domain, &f.type_name) {
            Some(a) => a,
            None if Some(f.type_version.max(1)) == main => f.type_name.clone(),
            None => format!("{}.{}", model_namespace(&e.domain, f.type_version).trim_start_matches("Fontys."), f.type_name),
        },
        other => other.to_string(),
    };
    let listed = match f.list.as_str() {
        "IEnumerable" => format!("IEnumerable<{inner}>"),
        "List" => format!("List<{inner}>"),
        "Array" => format!("{inner}[]"),
        _ => inner,
    };
    if f.nullable { format!("{listed}?") } else { listed }
}

fn accessors(access: &str) -> &'static str {
    match access {
        "get" => "{ get; }",
        "set" => "{ get; set; }",
        _ => "{ get; init; }",
    }
}

/// The usings a field event needs: `System`, its lists', its data objects' namespace, NServiceBus, and the aliases last.
pub fn field_usings(e: &EventSpec, fields: &[EventFieldSpec]) -> Vec<String> {
    let mut names = vec!["System".to_string()];
    if fields.iter().any(|f| matches!(f.list.as_str(), "IEnumerable" | "List")) {
        names.push("System.Collections.Generic".to_string());
    }
    if let Some(v) = main_version(e, fields) {
        names.push(model_namespace(&e.domain, v));
    }
    names.push("NServiceBus".to_string());
    let mut lines: Vec<String> = names.iter().map(|n| format!("using {n};")).collect();
    let mut aliases: Vec<String> = fields
        .iter()
        .filter(|f| f.kind.starts_with("Held"))
        .filter_map(|f| alias(&e.domain, &f.type_name).map(|a| format!("using {a} = {}.{};", model_namespace(&e.domain, f.type_version), f.type_name)))
        .collect();
    aliases.dedup();
    lines.extend(aliases);
    lines
}

fn write_fields(e: &EventSpec, fields: &[EventFieldSpec]) -> String {
    let (i1, i2, i3) = (INDENT, INDENT.repeat(2), INDENT.repeat(3));
    let main = main_version(e, fields);
    let mut out = field_usings(e, fields).join("\n") + "\n\n";
    out += &format!("namespace {}\n{{\n{i1}public class {} : IEvent\n{i1}{{\n", e.namespace, e.name);
    out += &obsolete_line(&i2, &e.obsolete);
    let parameters: Vec<String> = fields.iter().map(|f| format!("{} {}", field_type(e, f, main), parameter(&f.name))).collect();
    let one_line = format!("{i2}public {}({})", e.name, parameters.join(", "));
    if parameters.len() <= ONE_LINE_PARAMETERS && one_line.len() <= ONE_LINE {
        out += &one_line;
        out += "\n";
    } else {
        out += &format!("{i2}public {}(\n", e.name);
        out += &parameters.iter().map(|p| format!("{i3}{p}")).collect::<Vec<_>>().join(",\n");
        out += ")\n";
    }
    out += &format!("{i2}{{\n");
    for f in fields {
        out += &format!("{i3}{} = {};\n", f.name, parameter(&f.name));
    }
    out += &format!("{i2}}}\n");
    for f in fields {
        out += "\n";
        out += &obsolete_line(&i2, &f.obsolete);
        out += &format!("{i2}public {} {} {}\n", field_type(e, f, main), f.name, accessors(&f.access));
    }
    out += &format!("{i1}}}\n}}\n");
    out
}

// -- Reading -----------------------------------------------------------------

/// Whether a path is an event of a domain's messages project.
pub fn event_file(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    parts.len() >= 3 && parts[0].starts_with("Fontys.Messages.") && parts[0].matches('.').count() == 2 && parts[1] == "Events" && path.ends_with(".cs")
}

/// What an event's name says it tells: `MedewerkerDeletedEvent` is a Deleted.
fn kind_of(name: &str) -> &'static str {
    for k in ["Changed", "Deleted", "Created", "Updated"] {
        if name.ends_with(&format!("{k}Event")) {
            return k;
        }
    }
    "Other"
}

fn note(path: &str, note: String, renames: bool) -> ReadRow {
    ReadRow { kind: "note".into(), path: path.into(), note, renames, ..ReadRow::default() }
}

/// The usings of a file: aliases by their name, and the namespaces taken in.
struct Usings {
    lines: Vec<String>,
    aliases: Vec<(String, String)>,
    namespaces: Vec<String>,
}

fn usings_of(text: &str) -> Usings {
    let mut u = Usings { lines: Vec::new(), aliases: Vec::new(), namespaces: Vec::new() };
    for line in text.lines().map(str::trim).take_while(|l| !l.starts_with("namespace")) {
        let Some(rest) = line.strip_prefix("using ").map(|r| r.trim_end_matches(';').trim()) else { continue };
        u.lines.push(line.to_string());
        match rest.split_once('=') {
            Some((a, full)) => u.aliases.push((a.trim().to_string(), full.trim().to_string())),
            None => u.namespaces.push(rest.to_string()),
        }
    }
    u
}

/// A data object a type names, in full (`Fontys.Fcdm.Hr.v3.Arbeidscontract`): through an alias, a full name, or the usings in which one of that name exists.
fn resolve(name: &str, u: &Usings, objects: &[(String, String, i64)]) -> String {
    if let Some((_, full)) = u.aliases.iter().find(|(a, _)| a == name) {
        return full.clone();
    }
    if name.starts_with("Fcdm.") {
        return format!("Fontys.{name}");
    }
    if name.starts_with("Fontys.") {
        return name.to_string();
    }
    let models: Vec<&String> = u.namespaces.iter().filter(|n| n.starts_with("Fontys.Fcdm.") && n.split('.').count() >= 3).collect();
    let hit = models.iter().find(|ns| objects.iter().any(|(d, o, v)| o == name && *d == crate::read::domain_of(ns) && *v == crate::read::version_of(ns)));
    match hit.or(models.first()) {
        Some(ns) => format!("{ns}.{name}"),
        None => name.to_string(),
    }
}

/// A field's row from its C# type and name.
fn field_row(name: &str, type_text: &str, u: &Usings, objects: &[(String, String, i64)]) -> ReadRow {
    let mut t = type_text.trim().to_string();
    let nullable = t.ends_with('?');
    t = t.trim_end_matches('?').to_string();
    let (list, inner) = if let Some(i) = t.strip_suffix("[]") {
        ("Array", i.to_string())
    } else if let Some(i) = t.strip_prefix("IEnumerable<").and_then(|r| r.strip_suffix('>')) {
        ("IEnumerable", i.to_string())
    } else if let Some(i) = t.strip_prefix("List<").and_then(|r| r.strip_suffix('>')) {
        ("List", i.to_string())
    } else {
        ("", t.clone())
    };
    let mut row = ReadRow { kind: "event_field".into(), name: name.into(), nullable, list: list.into(), ..ReadRow::default() };
    let value = match inner.as_str() {
        "string" if name.ends_with("Bk") && list.is_empty() => Some("BusinessKey"),
        "string" => Some("TextValue"),
        "int" => Some("WholeNumber"),
        "decimal" => Some("DecimalNumber"),
        "bool" => Some("YesNo"),
        "DateTime" => Some("Moment"),
        "DateTimeOffset" => Some("MomentWithOffset"),
        "DateOnly" => Some("DayOnly"),
        _ => None,
    };
    match value {
        Some(kind) => {
            row.property_kind = kind.into();
            if kind == "BusinessKey" {
                row.type_name = name.trim_end_matches("Bk").to_string();
            }
        }
        None => {
            let full = resolve(&inner, u, objects);
            row.property_kind = if list.is_empty() { "HeldObject" } else { "HeldObjects" }.into();
            row.type_name = full.rsplit('.').next().unwrap_or(&inner).to_string();
            let ns = full.rsplit_once('.').map(|(n, _)| n).unwrap_or("");
            row.type_version = crate::read::version_of(ns);
            row.domain = crate::read::domain_of(ns);
        }
    }
    row
}

/// Splits a parameter list at its commas outside `<...>`.
fn split_parameters(text: &str) -> Vec<String> {
    let (mut out, mut depth, mut current) = (Vec::new(), 0, String::new());
    for c in text.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                out.push(current.trim().to_string());
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

/// Every event of the files, each followed by its fields, with notes; `objects` are the data objects read (domain, name, version).
pub fn read_events(files: &[SourceFile], objects: &[(String, String, i64)]) -> Vec<ReadRow> {
    let mut rows = Vec::new();
    for f in files.iter().filter(|f| event_file(&f.path)) {
        rows.extend(read_event(f, objects));
    }
    rows
}

fn read_event(f: &SourceFile, objects: &[(String, String, i64)]) -> Vec<ReadRow> {
    let raw = f.text.trim_start_matches('\u{feff}');
    let text = raw.replace("\r\n", "\n");
    let parts: Vec<&str> = f.path.split('/').collect();
    let domain = parts[0].trim_start_matches("Fontys.Messages.").to_string();
    let version = if parts.len() > 3 { parts[2].strip_prefix('v').and_then(|n| n.parse().ok()).unwrap_or(1) } else { 1 };
    let u = usings_of(&text);
    let namespace = text.lines().find_map(|l| l.trim().strip_prefix("namespace ")).map(|n| n.trim_end_matches(';').trim().to_string()).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let Some(class_at) = lines.iter().position(|l| l.trim_start().starts_with("public class ")) else {
        return vec![note(&f.path, format!("{}: no event class in it; it is left out", f.path), false)];
    };
    // The class's head, up to its brace, on one line: a base event may spread it over several.
    let head_lines: Vec<&str> = lines[class_at..].iter().take_while(|l| l.trim() != "{").copied().collect();
    let joined = head_lines.iter().map(|l| l.trim()).collect::<Vec<_>>().join(" ").replace("( ", "(");
    let class_line = joined.as_str();
    let name: String = class_line["public class ".len()..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    let kind = kind_of(&name);
    let mut event = ReadRow { kind: "event".into(), path: f.path.clone(), domain: domain.clone(), name: name.clone(), version, namespace: namespace.clone(), event_kind: kind.into(), ..ReadRow::default() };
    let mut notes = Vec::new();
    let mut fields = Vec::new();
    let wanted = folder_namespace(&domain, version);
    if namespace != wanted {
        event.review = format!("Its namespace {namespace} is not its folder's {wanted}; NServiceBus routes by it, so it stays as published.");
        notes.push(note(&f.path, format!("{name} v{version}: namespace {namespace} is kept (the folder gives {wanted}); flagged for review"), false));
    }
    let attributes_above = |at: usize| lines[..at].iter().rev().take_while(|l| l.trim().starts_with('[')).map(|l| l.trim().to_string()).collect::<Vec<_>>();
    let obsolete_in = |attrs: &[String]| attrs.iter().find(|a| a.contains("Obsolete")).map(|a| crate::read::quoted_text(a));
    if let Some(at) = class_line.find("Event<").filter(|_| class_line.contains("BaseChangedEvent<") || class_line.contains("BaseDeletedEvent<")) {
        event.base = true;
        let generic: String = class_line[at + "Event<".len()..].chars().take_while(|c| *c != '>').collect();
        let full = resolve(&generic, &u, objects);
        event.type_name = full.rsplit('.').next().unwrap_or(&generic).to_string();
        event.type_version = crate::read::version_of(full.rsplit_once('.').map(|(n, _)| n).unwrap_or(""));
        event.obsolete = obsolete_in(&attributes_above(class_at)).unwrap_or_default();
        if head_lines.len() > 1 {
            notes.push(note(&f.path, format!("{name} v{version}: the class's head is written on one line"), false));
        }
    } else {
        // The constructor, its parameters in order, and what each is put in.
        let ctor_start = format!("public {name}(");
        let Some(ctor_at) = lines.iter().position(|l| l.trim_start().starts_with(&ctor_start)) else {
            return vec![note(&f.path, format!("{name}: no constructor; it is left out"), false)];
        };
        if lines.get(class_at + 1).is_some_and(|l| l.trim().starts_with(':')) {
            notes.push(note(&f.path, format!("{name} v{version}: `: IEvent` is written on the class's line"), false));
        }
        let ctor_attrs = attributes_above(ctor_at);
        if let Some(o) = obsolete_in(&ctor_attrs) {
            event.obsolete = o;
        }
        let mut params_text = String::new();
        let mut end = ctor_at;
        for (i, l) in lines.iter().enumerate().skip(ctor_at) {
            params_text.push_str(l.trim());
            end = i;
            if l.trim_end().ends_with(')') {
                break;
            }
        }
        let inside = params_text.strip_prefix(&ctor_start).and_then(|r| r.strip_suffix(')')).unwrap_or("");
        let parameters: Vec<(String, String)> = split_parameters(inside).iter().filter_map(|p| p.rsplit_once(' ').map(|(t, n)| (t.trim().to_string(), n.trim().to_string()))).collect();
        let one_line = format!("{}public {name}({})", INDENT.repeat(2), parameters.iter().map(|(t, n)| format!("{t} {n}")).collect::<Vec<_>>().join(", "));
        let fits = parameters.len() <= ONE_LINE_PARAMETERS && one_line.len() <= ONE_LINE;
        if fits != (end == ctor_at) {
            notes.push(note(&f.path, format!("{name} v{version}: the constructor is laid out as the others ({})", if fits { "on one line" } else { "a parameter per line" }), false));
        }
        // Assignments: which property each parameter is put in.
        let mut given: Vec<(String, String)> = Vec::new();
        for l in lines.iter().skip(end + 1).map(|l| l.trim()) {
            if l == "}" {
                break;
            }
            if let Some((prop, value)) = l.trim_end_matches(';').split_once(" = ") {
                given.push((prop.trim().to_string(), value.trim().to_string()));
            }
        }
        // The properties, by name.
        let mut properties: Vec<(String, String, String, String)> = Vec::new();
        for (i, l) in lines.iter().enumerate().skip(end + 1) {
            let t = l.trim();
            let Some(rest) = t.strip_prefix("public ") else { continue };
            let Some(open) = rest.find(" {") else { continue };
            let Some((ty, pname)) = rest[..open].rsplit_once(' ') else { continue };
            let access = match rest[open..].replace(' ', "").as_str() {
                "{get;}" => "get",
                "{get;set;}" => "set",
                _ => "init",
            };
            let attrs = attributes_above(i);
            let obsolete = match obsolete_in(&attrs) {
                Some(o) if o.is_empty() => {
                    notes.push(note(&f.path, format!("{name} v{version}: {pname}'s [Obsolete] gets the constructor's message"), false));
                    if event.obsolete.is_empty() { "Obsolete.".to_string() } else { event.obsolete.clone() }
                }
                Some(o) => o,
                None => String::new(),
            };
            properties.push((pname.to_string(), ty.to_string(), access.to_string(), obsolete));
        }
        for (pty, pname) in &parameters {
            let property = given.iter().find(|(_, v)| v == pname).map(|(p, _)| p.clone());
            let Some(property) = property else {
                notes.push(note(&f.path, format!("{name} v{version}: parameter {pname} is put in no property; it is left out"), false));
                continue;
            };
            let found = properties.iter().find(|p| p.0 == property);
            let mut row = field_row(&property, pty, &u, objects);
            if let Some((_, ty, access, obsolete)) = found {
                row.access = access.clone();
                row.obsolete = obsolete.clone();
                if ty != pty {
                    notes.push(note(&f.path, format!("{name} v{version}: {property} is typed as its parameter ({pty})"), true));
                }
            }
            if parameter(&property) != *pname {
                notes.push(note(&f.path, format!("{name} v{version}: the parameter for {property} was {pname}; it is written {}", parameter(&property)), true));
            }
            fields.push(row);
        }
        let order: Vec<&String> = given.iter().map(|(p, _)| p).collect();
        let wanted_order: Vec<&String> = fields.iter().map(|r| &r.name).collect();
        if order != wanted_order {
            notes.push(note(&f.path, format!("{name} v{version}: the properties are given in the constructor's order"), false));
        }
        let declared: Vec<&String> = properties.iter().map(|p| &p.0).collect();
        if declared != wanted_order {
            notes.push(note(&f.path, format!("{name} v{version}: the properties stand in the constructor's order"), false));
        }
    }
    // What the writer makes of it, against the file: the usings and the file's own shape.
    let spec = spec_of(&event);
    let field_specs: Vec<EventFieldSpec> = fields.iter().map(field_spec_of).collect();
    let wanted_usings: Vec<String> = if event.base {
        write_base(&spec).lines().take_while(|l| l.starts_with("using ")).map(str::to_string).collect()
    } else {
        field_usings(&spec, &field_specs)
    };
    if u.lines != wanted_usings {
        notes.push(note(&f.path, format!("{name} v{version}: usings as needed: {}", wanted_usings.iter().map(|l| l.trim_start_matches("using ").trim_end_matches(';')).collect::<Vec<_>>().join(", ")), false));
    }
    if !event.base && text.contains("Fcdm.") && text.lines().any(|l| l.trim_start().starts_with("public") && l.contains(" Fcdm.")) {
        notes.push(note(&f.path, format!("{name} v{version}: data objects are named from the namespace most of them are in"), false));
    }
    if event.base {
        let canonical = if kind == "Deleted" { "(string bk, DateTimeOffset deletedTimestamp)" } else { "data, DateTimeOffset changedTimestamp)" };
        if !class_line.contains(canonical) {
            notes.push(note(&f.path, format!("{name} v{version}: written as the other base events: {canonical}"), false));
        }
    }
    if !raw.ends_with('\n') {
        notes.push(note(&f.path, format!("{name} v{version}: the file gets its final line end"), false));
    }
    if raw.contains("\r\n") {
        notes.push(note(&f.path, format!("{name} v{version}: the file's lines end in LF, not CRLF"), false));
    }
    let mut out = vec![event];
    out.extend(fields);
    out.extend(notes);
    out
}

/// The spec an event row gives, as the application builds it from what it stored.
pub fn spec_of(r: &ReadRow) -> EventSpec {
    EventSpec { domain: r.domain.clone(), name: r.name.clone(), version: r.version, namespace: r.namespace.clone(), kind: r.event_kind.clone(), base: r.base, type_name: r.type_name.clone(), type_version: r.type_version, obsolete: r.obsolete.clone() }
}

pub fn field_spec_of(r: &ReadRow) -> EventFieldSpec {
    EventFieldSpec { name: r.name.clone(), kind: r.property_kind.clone(), nullable: r.nullable, type_name: r.type_name.clone(), type_version: r.type_version, list: r.list.clone(), access: r.access.clone(), obsolete: r.obsolete.clone() }
}
