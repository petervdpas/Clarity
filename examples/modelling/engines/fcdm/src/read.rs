//! Reading the fcdm repository: the C# of its data objects, their versions
//! and its enums, and the packages of its domains, into flat rows the
//! application stores. What is tidied on the way (a casing, a typo, a
//! property without `init`, a method that is no data) becomes a note, for
//! the import report; a note that changes a public name says so.

use serde::{Deserialize, Serialize};

/// A file of the repository: its path in the folder (`Fontys.Fcdm.Hr/v2/Medewerker.cs`) and its text.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFile {
    pub path: String,
    pub text: String,
}

/// A row of what was read, by `kind`: a `domain` (its package, `role` model
/// or messages, and version), an `object` (a version of a data object), a
/// `property` of the object read before it, an `enum`, a `value` of the enum
/// read before it, or a `note` for the report.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReadRow {
    pub kind: String,
    pub path: String,
    pub domain: String,
    pub name: String,
    pub version: i64,
    pub root: bool,
    /// Whether a data object has a business key (`Bk`).
    pub keyed: bool,
    /// For a domain's model package: whether its project uses Newtonsoft.Json.
    pub newtonsoft: bool,
    pub obsolete: String,
    pub role: String,
    pub package_id: String,
    pub property_kind: String,
    pub nullable: bool,
    pub type_name: String,
    pub type_version: i64,
    pub initial: String,
    pub computed: String,
    pub number: i64,
    /// For a note: what was found and what was done; `renames` when it changes a public name.
    pub note: String,
    pub renames: bool,
    /// For an event: its namespace as published, its kind (Changed, Deleted, ...), and whether it is a base event.
    pub namespace: String,
    pub event_kind: String,
    pub base: bool,
    /// For an event field: how a list is typed (IEnumerable, Array, List) and its accessors (init, get, set).
    pub list: String,
    pub access: String,
    /// For an event or a data object's version: why someone should look at it; empty when nothing asks for it.
    pub review: String,
}

/// Whether a path is one the import reads: a domain's model project, not its tests, interfaces or build output.
fn model_file(path: &str) -> bool {
    let mut parts = path.split('/');
    let project = parts.next().unwrap_or("");
    let rest: Vec<&str> = parts.collect();
    project.starts_with("Fontys.Fcdm.")
        && project.matches('.').count() == 2
        && !["Fontys.Fcdm.EventDispatcher", "Fontys.Fcdm.Data"].iter().any(|p| project.starts_with(p))
        && path.ends_with(".cs")
        && !rest.iter().any(|p| matches!(*p, "Interfaces" | "obj" | "bin" | "CICD" | "Properties"))
}

/// Whether a path is a root's interface in a domain's model project.
fn interface_file(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    parts.len() >= 3 && parts[0].starts_with("Fontys.Fcdm.") && parts[0].matches('.').count() == 2 && parts[1] == "Interfaces" && path.ends_with(".cs")
}

/// The domain a project or namespace names: `Fontys.Fcdm.Hr.v2` gives Hr.
pub(crate) fn domain_of(namespace: &str) -> String {
    namespace.split('.').nth(2).unwrap_or("").to_string()
}

/// The version a namespace names: `.v3` gives 3, none gives 1.
pub(crate) fn version_of(namespace: &str) -> i64 {
    namespace.rsplit('.').next().and_then(|last| last.strip_prefix('v').or_else(|| last.strip_prefix('V'))).and_then(|n| n.parse().ok()).unwrap_or(1)
}

/// The text between the first quotes of an attribute, for the events' reader.
pub(crate) fn quoted_text(line: &str) -> String {
    quoted(line)
}

/// The text between the first quotes of an attribute: `[Obsolete("Use v2.")]` gives `Use v2.`.
fn quoted(line: &str) -> String {
    let Some(start) = line.find('"') else { return String::new() };
    let rest = &line[start + 1..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            '"' => break,
            c => out.push(c),
        }
    }
    out
}

/// Every row of the files: the packages first, then the enums (so a property knows an enum by its name), then the data objects.
pub fn read_files(files: &[SourceFile]) -> Vec<ReadRow> {
    let mut rows = Vec::new();
    for f in files.iter().filter(|f| f.path.ends_with(".csproj")) {
        rows.extend(read_project(f));
    }
    let models: Vec<&SourceFile> = files.iter().filter(|f| model_file(&f.path)).collect();
    let mut enums: Vec<String> = Vec::new();
    for f in &models {
        let text = clean(&f.text);
        if text.contains("public enum ") {
            let read = read_enum(f, &text);
            enums.extend(read.iter().filter(|r| r.kind == "enum").map(|r| r.name.clone()));
            rows.extend(read);
        }
    }
    // The domains whose project uses Newtonsoft.Json, whose enums get its converter too.
    let newtonsoft: Vec<String> = rows.iter().filter(|r| r.kind == "domain" && r.role == "model" && r.newtonsoft).map(|r| r.domain.clone()).collect();
    for f in &models {
        let text = clean(&f.text);
        if text.contains("public class ") || text.contains("public record ") {
            rows.extend(read_class(f, &text, &enums, &newtonsoft));
        }
    }
    resolve_keys(&mut rows);
    // The interfaces, which the app writes from the classes: how each differs from what it gets.
    for f in files.iter().filter(|f| interface_file(&f.path)) {
        let notes = read_interface(f, &clean(&f.text), &rows, &newtonsoft);
        rows.extend(notes);
    }
    // The events, which carry the data objects read.
    let objects: Vec<(String, String, i64)> = rows.iter().filter(|r| r.kind == "object").map(|r| (r.domain.clone(), r.name.clone(), r.version)).collect();
    rows.extend(crate::events::read_events(files, &objects));
    // A data object's version whose public names the import changed asks for a review.
    let renamed: Vec<(String, String)> = rows.iter().filter(|r| r.kind == "note" && r.renames).map(|r| (r.path.clone(), r.note.clone())).collect();
    for o in rows.iter_mut().filter(|r| r.kind == "object") {
        let mine: Vec<&str> = renamed.iter().filter(|(p, _)| *p == o.path).map(|(_, n)| n.as_str()).collect();
        if !mine.is_empty() {
            o.review = format!("The import changed a public name: {}", mine.join("; "));
        }
    }
    // A package is a domain's only when the domain has data objects or enums: not a dispatcher's or a test's.
    let has: Vec<String> = rows.iter().filter(|r| r.kind == "object" || r.kind == "enum").map(|r| r.domain.clone()).collect();
    rows.retain(|r| r.kind != "domain" || has.contains(&r.domain));
    rows
}

/// The text without its byte order mark, its lines ending in `\n`.
fn clean(text: &str) -> String {
    text.trim_start_matches('\u{feff}').replace("\r\n", "\n")
}

/// The package a project file names, with its version: a domain row.
fn read_project(f: &SourceFile) -> Vec<ReadRow> {
    let tag = |name: &str| {
        let open = format!("<{name}>");
        f.text.find(&open).and_then(|at| {
            let rest = &f.text[at + open.len()..];
            rest.find(&format!("</{name}>")).map(|end| rest[..end].trim().to_string())
        })
    };
    let Some(package) = tag("PackageId") else { return Vec::new() };
    let parts: Vec<&str> = package.split('.').collect();
    let role = match parts.as_slice() {
        ["Fontys", "Fcdm", _] => "model",
        ["Fontys", "Messages", _] => "messages",
        _ => return Vec::new(),
    };
    let newtonsoft = f.text.contains("Include=\"Newtonsoft.Json\"");
    vec![ReadRow { kind: "domain".into(), path: f.path.clone(), domain: parts[2].to_string(), role: role.into(), package_id: package.clone(), name: tag("Version").unwrap_or_default(), newtonsoft, ..ReadRow::default() }]
}

fn namespace_in(text: &str) -> String {
    text.lines().find_map(|l| l.trim().strip_prefix("namespace ")).map(|n| n.trim_end_matches(';').trim().to_string()).unwrap_or_default()
}

fn note(path: &str, note: String, renames: bool) -> ReadRow {
    ReadRow { kind: "note".into(), path: path.into(), note, renames, ..ReadRow::default() }
}

/// An enum and its values, numbered as written or counting on from the one before.
fn read_enum(f: &SourceFile, text: &str) -> Vec<ReadRow> {
    let namespace = namespace_in(text);
    let mut rows = Vec::new();
    let mut lines = text.lines();
    let Some(head) = lines.by_ref().find(|l| l.contains("public enum ")) else { return rows };
    let name = head.split("public enum ").nth(1).unwrap_or("").split_whitespace().next().unwrap_or("").to_string();
    rows.push(ReadRow { kind: "enum".into(), path: f.path.clone(), domain: domain_of(&namespace), name: name.clone(), ..ReadRow::default() });
    let mut next = 0;
    let mut unnumbered = false;
    for line in lines {
        let line = line.trim().trim_end_matches(',');
        if line.starts_with('}') {
            break;
        }
        if line.is_empty() || line.starts_with('{') || line.starts_with("//") || line.starts_with('[') {
            continue;
        }
        let (value, number) = match line.split_once('=') {
            Some((v, n)) => (v.trim().to_string(), n.trim().parse().unwrap_or(next)),
            None => {
                unnumbered = true;
                (line.to_string(), next)
            }
        };
        next = number + 1;
        rows.push(ReadRow { kind: "value".into(), path: f.path.clone(), domain: domain_of(&namespace), name: value, number, ..ReadRow::default() });
    }
    if text.lines().any(|l| l.trim().starts_with("using ")) {
        rows.push(note(&f.path, format!("{name}: usings an enum does not need are left out"), false));
    }
    if unnumbered {
        rows.push(note(&f.path, format!("{name}: its values get their numbers written, as the adapter keeps them"), false));
    }
    rows
}

/// A version of a data object: the class, then its properties in the order they are declared.
fn read_class(f: &SourceFile, text: &str, enums: &[String], newtonsoft: &[String]) -> Vec<ReadRow> {
    let namespace = namespace_in(text);
    let (domain, version) = (domain_of(&namespace), version_of(&namespace));
    let both = newtonsoft.contains(&domain);
    let mut rows = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = lines.iter().position(|l| l.contains("public class ") || l.contains("public record ")) else { return rows };
    let head = lines[at];
    let after = head.split(if head.contains("public class ") { "public class " } else { "public record " }).nth(1).unwrap_or("");
    let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    let implements = after.split(':').nth(1).map(|s| s.trim().to_string()).unwrap_or_default();
    let obsolete = lines[..at].iter().rev().take_while(|l| l.trim().starts_with('[')).find(|l| l.contains("Obsolete")).map(|l| quoted(l)).unwrap_or_default();
    // Data declared as an NServiceBus event (`IEvent`) in a model project: read as the data object it is, which drops the IEvent, a public change.
    if implements.split(',').any(|i| i.trim() == "IEvent") {
        rows.push(note(&f.path, format!("{name} v{version}: is data (events carry it) but declared an NServiceBus IEvent; written as a data object without IEvent"), true));
    }
    let has_bk = lines.iter().any(|l| l.trim().starts_with("public string Bk "));
    let root = has_bk || implements.split(',').any(|i| i.trim() == format!("I{name}"));
    rows.push(ReadRow { kind: "object".into(), path: f.path.clone(), domain: domain.clone(), name: name.clone(), version, root, keyed: has_bk, obsolete, ..ReadRow::default() });

    // The constructor's defaults (`string? brinBk = null`), and the parameter it gives each property (`Alias = @alias;`).
    let mut defaults: Vec<(String, String)> = Vec::new();
    let mut given: Vec<(String, String)> = Vec::new();
    let mut order: Vec<String> = Vec::new();
    let mut layout: Option<(bool, usize, String)> = None;
    if let Some(start) = text.find(&format!("public {name}(")) {
        let rest = &text[start + name.len() + 8..];
        let end = rest.find(')').unwrap_or(rest.len());
        // How it is laid out: on one line (and how long that line is), or a parameter per line (and their indent).
        let line_start = text[..start].rfind('\n').map_or(0, |n| n + 1);
        let one_line = !rest[..end].contains('\n');
        let line_length = if one_line { start + name.len() + 8 + end + 1 - line_start } else { 0 };
        let indent = rest[..end].lines().nth(1).map(|l| l.len() - l.trim_start().len()).unwrap_or(0);
        layout = Some((one_line, line_length, " ".repeat(indent)));
        for parameter in rest[..end].split(',') {
            let parameter = parameter.trim();
            let declared = parameter.split('=').next().unwrap_or("").trim();
            let pname = declared.rsplit(char::is_whitespace).next().unwrap_or("").trim_start_matches('@').to_string();
            if !pname.is_empty() {
                order.push(pname.to_lowercase());
            }
            if let Some((_, initial)) = parameter.split_once('=') {
                defaults.push((pname.to_lowercase(), initial.trim().to_string()));
            }
        }
        if let Some(body) = rest[end..].find('{').map(|b| &rest[end + b + 1..]) {
            for line in body.lines().map(str::trim).take_while(|l| !l.starts_with('}')) {
                if let Some((property, value)) = line.trim_end_matches(';').split_once(" = ") {
                    given.push((property.trim().to_string(), value.trim().to_string()));
                }
            }
        }
    }

    let mut attributes: Vec<String> = Vec::new();
    let mut props: Vec<ReadRow> = Vec::new();
    let mut skip_until = 0usize;
    for (i, raw) in lines.iter().enumerate().skip(at + 1) {
        if skip_until > 0 && i <= skip_until {
            continue;
        }
        let line = raw.trim();
        if line.starts_with('[') {
            attributes.push(line.to_string());
            continue;
        }
        // A property whose getter is a block: `public string? Naam`, then `{ get { ... } }` over lines.
        let block = line.starts_with("public ") && !line.contains('(') && !line.contains('{') && !line.contains("=>") && !line.contains(" class ") && lines.get(i + 1).is_some_and(|n| n.trim() == "{");
        if block {
            let indent = raw.len() - raw.trim_start().len();
            let mut depth = 0;
            let mut body: Vec<String> = Vec::new();
            for next in &lines[i + 1..] {
                depth += next.matches('{').count() as i32 - next.matches('}').count() as i32;
                body.push(if next.trim().is_empty() { String::new() } else { next.get(indent..).unwrap_or(next.trim_start()).to_string() });
                if depth == 0 {
                    break;
                }
            }
            skip_until = i + body.len();
            let mut words: Vec<&str> = line.trim_start_matches("public ").split_whitespace().collect();
            let pname = words.pop().unwrap_or("").to_string();
            let mut row = property_row(&f.path, &domain, &name, version, &pname, &words.join(" "), enums);
            row.computed = body.join("\n");
            if !ignores_as_written(&attributes, both) {
                rows.push(note(&f.path, format!("{name} v{version}: {pname} is computed and gets {}", ignores_word(both)), false));
            }
            props.push(row);
            attributes.clear();
            continue;
        }
        if i <= skip_until && skip_until > 0 {
            continue;
        }
        let property = line.starts_with("public ") && (line.contains(" { get") || line.contains(" => ")) && !line.contains('(');
        let method = line.starts_with("public ") && line.contains('(') && !line.starts_with(&format!("public {name}("));
        if method {
            let signature: String = line.trim_start_matches("public ").chars().take_while(|c| *c != '(').collect();
            rows.push(note(&f.path, format!("{name} v{version}: the method {} is code, not data, and is left out", signature.split_whitespace().last().unwrap_or("")), false));
        }
        if property {
            let declaration = line.trim_start_matches("public ");
            let (left, computed) = match declaration.split_once(" => ") {
                Some((l, c)) => (l.trim(), c.trim().trim_end_matches(';').to_string()),
                None => (declaration.split(" {").next().unwrap_or("").trim(), String::new()),
            };
            let mut words: Vec<&str> = left.split_whitespace().collect();
            let pname = words.pop().unwrap_or("").to_string();
            let type_text = words.join(" ");
            if pname == "Bk" && has_bk {
                attributes.clear();
                continue;
            }
            let mut row = property_row(&f.path, &domain, &name, version, &pname, &type_text, enums);
            row.computed = computed.clone();
            row.obsolete = attributes.iter().find(|a| a.contains("Obsolete")).map(|a| quoted(a)).unwrap_or_default();
            if let Some((_, initial)) = defaults.iter().find(|(p, _)| *p == pname.to_lowercase()) {
                row.initial = initial.clone();
            }
            // Tidied: the business key's casing, a property without `init`.
            if pname.ends_with("BK") {
                let tidy = format!("{}Bk", &pname[..pname.len() - 2]);
                rows.push(note(&f.path, format!("{name} v{version}: {pname} is written {tidy}"), true));
                row.name = tidy;
            }
            if computed.is_empty() && (line.contains("{ get; }") || line.contains("{ get;  }")) {
                rows.push(note(&f.path, format!("{name} v{version}: {pname} had no init; it gets one"), false));
            }
            if line.contains("set; }") {
                rows.push(note(&f.path, format!("{name} v{version}: {pname} had a setter; it gets init, as every property has"), false));
            }
            // An enum's converters, both and in full; a computed property's JsonIgnore, both.
            if row.property_kind == "EnumChoice" && !converters_as_written(&attributes, both) {
                rows.push(note(&f.path, format!("{name} v{version}: {pname} gets {}", converters_word(both)), false));
            }
            if !computed.is_empty() && !ignores_as_written(&attributes, both) {
                rows.push(note(&f.path, format!("{name} v{version}: {pname} is computed and gets {}", ignores_word(both)), false));
            }
            // The constructor's parameter for it, written as it is named.
            if let Some((_, value)) = given.iter().find(|(p, _)| *p == pname) {
                let wanted = super::parameter(&row.name);
                if value.trim_start_matches('@') != wanted.trim_start_matches('@') && !pname.ends_with("BK") {
                    // A parameter's name is public: a caller can pass it by name (`voltooiingTijdstaatdatum: x`).
                    rows.push(note(&f.path, format!("{name} v{version}: the constructor's parameter for {pname} was {value}; it is written {wanted}"), true));
                }
            }
            props.push(row);
            let _ = i;
        }
        attributes.clear();
    }
    // The properties follow the constructor's parameters, as callers pass them; computed ones come last.
    let declared: Vec<String> = props.iter().map(|p| p.name.clone()).collect();
    // The parameter a property is given, by the constructor's assignment (`Alias = @alias;`), else by its name.
    let parameter_of = |p: &ReadRow| -> String {
        given.iter().find(|(prop, _)| prop.eq_ignore_ascii_case(&p.name) || prop.to_lowercase() == format!("{}k", p.name.to_lowercase().trim_end_matches('k'))).map(|(_, v)| v.trim_start_matches('@').to_lowercase()).unwrap_or_else(|| p.name.to_lowercase())
    };
    let place = |p: &ReadRow| order.iter().position(|o| *o == parameter_of(p)).unwrap_or(usize::MAX - 1);
    // The stored ones in the constructor's order; a computed one stays after the property it followed.
    let mut stored: Vec<ReadRow> = props.iter().filter(|p| p.computed.is_empty()).cloned().collect();
    stored.sort_by_key(place);
    let mut placed: Vec<ReadRow> = Vec::new();
    let mut before: Option<String> = None;
    let mut after: Vec<(Option<String>, ReadRow)> = Vec::new();
    for p in &props {
        if p.computed.is_empty() {
            before = Some(p.name.clone());
        } else {
            after.push((before.clone(), p.clone()));
        }
    }
    for (b, c) in after.iter().filter(|(b, _)| b.is_none()) {
        let _ = b;
        placed.push(c.clone());
    }
    for s in stored {
        let name = s.name.clone();
        placed.push(s);
        for (_, c) in after.iter().filter(|(b, _)| b.as_deref() == Some(name.as_str())) {
            placed.push(c.clone());
        }
    }
    props = placed;
    if props.iter().map(|p| p.name.clone()).collect::<Vec<_>>() != declared {
        rows.push(note(&f.path, format!("{name} v{version}: its properties were declared in another order than the constructor takes them; they follow the constructor"), false));
    }
    // The constructor gives the properties in its own order.
    let given_order: Vec<String> = given.iter().map(|(p, _)| p.to_lowercase()).filter(|p| p != "bk").collect();
    let taken_order: Vec<String> = order.iter().filter(|p| *p != "bk" && given_order.contains(p)).cloned().collect();
    if given_order != taken_order {
        rows.push(note(&f.path, format!("{name} v{version}: the constructor assigned its properties in another order; it assigns them as it takes them"), false));
    }
    // Its layout: on one line up to three parameters that fit in 120 characters, else a parameter per line, indented by 12.
    if let Some((one_line, length, indent)) = layout {
        let fits = order.len() <= super::ONE_LINE_PARAMETERS && length <= super::ONE_LINE;
        if one_line && !fits {
            rows.push(note(&f.path, format!("{name} v{version}: its constructor stood on one line; it takes a parameter per line"), false));
        } else if !one_line && fits {
            rows.push(note(&f.path, format!("{name} v{version}: its constructor took a parameter per line; it stands on one line"), false));
        } else if !one_line && indent != "            " {
            rows.push(note(&f.path, format!("{name} v{version}: its constructor's parameters were indented by {}; they are indented by 12", indent.len()), false));
        }
    }
    // The usings it needs, and no others.
    let kinds: Vec<&str> = props.iter().map(|p| p.property_kind.as_str()).collect();
    let mut needed: Vec<String> = Vec::new();
    if kinds.iter().any(|k| matches!(*k, "Moment" | "MomentWithOffset" | "DayOnly")) || lines[..at].iter().any(|l| l.contains("Obsolete")) || props.iter().any(|p| !p.obsolete.is_empty()) || !obsolete_of(&lines, at).is_empty() {
        needed.push("System".into());
    }
    if kinds.contains(&"HeldObjects") {
        needed.push("System.Collections.Generic".into());
    }
    if kinds.contains(&"EnumChoice") || props.iter().any(|p| !p.computed.is_empty()) {
        needed.push("System.Text.Json.Serialization".into());
    }
    if root {
        needed.push(if version > 1 { format!("Fontys.Fcdm.{domain}.Interfaces.v{version}") } else { format!("Fontys.Fcdm.{domain}.Interfaces") });
    }
    if kinds.contains(&"EnumChoice") && both {
        needed.push("Newtonsoft.Json.Converters".into());
    }
    let written: Vec<String> = lines[..at].iter().filter_map(|l| l.trim().strip_prefix("using ")).map(|u| u.trim_end_matches(';').to_string()).collect();
    for extra in written.iter().filter(|u| !needed.contains(u)) {
        rows.push(note(&f.path, format!("{name} v{version}: using {extra} is left out, as nothing needs it"), false));
    }
    for missing in needed.iter().filter(|u| !written.contains(u)) {
        rows.push(note(&f.path, format!("{name} v{version}: using {missing} is added, as it is needed"), false));
    }
    let sorted = {
        let mut s = written.clone();
        s.sort_by(|a, b| {
            let system = |n: &str| !(n == "System" || n.starts_with("System."));
            (system(a), a.as_str()).cmp(&(system(b), b.as_str()))
        });
        s
    };
    if written == needed.iter().filter(|u| written.contains(u)).cloned().collect::<Vec<_>>() && sorted != written {
        rows.push(note(&f.path, format!("{name} v{version}: its usings are put in order, System first"), false));
    }
    rows.extend(props);
    if !f.text.ends_with('\n') {
        rows.push(note(&f.path, format!("{name} v{version}: the file ended without a newline; it ends with one"), false));
    }
    if f.text.contains("\r\n") {
        rows.push(note(&f.path, format!("{name} v{version}: its lines ended in CRLF; they end in LF"), false));
    }
    rows
}

/// The data object each business key names: by its name (`MedewerkerBk`),
/// in any casing (`LesPlaatsBk`), without a genitive s (`AfdelingsBk`), or
/// by the end of its name, a role in front (`StartStudiejaarBk`,
/// `ParentProjectBk`), the longest that fits. A match by the end of the name
/// gets a note to check; a key of an enum or of nothing found gets one too.
fn resolve_keys(rows: &mut Vec<ReadRow>) {
    let objects: Vec<(String, String)> = rows.iter().filter(|r| r.kind == "object").map(|r| (r.name.clone(), r.domain.clone())).collect();
    let enums: Vec<String> = rows.iter().filter(|r| r.kind == "enum").map(|r| r.name.clone()).collect();
    let mut notes = Vec::new();
    for row in rows.iter_mut().filter(|r| r.kind == "property" && r.property_kind == "BusinessKey") {
        let base = row.type_name.clone();
        let lower = base.to_lowercase();
        let named = |wanted: &str| objects.iter().filter(|(n, _)| n.to_lowercase() == wanted).min_by_key(|(_, d)| *d != row.domain).map(|(n, _)| n.clone());
        let found = named(&lower).or_else(|| lower.strip_suffix('s').and_then(|l| named(l)));
        if let Some(name) = found {
            row.type_name = name;
            continue;
        }
        let ending = objects
            .iter()
            .filter(|(n, _)| lower.ends_with(&n.to_lowercase()) && n.len() < base.len())
            .max_by_key(|(n, d)| (n.len(), *d == row.domain))
            .map(|(n, _)| n.clone());
        match ending {
            Some(name) => {
                notes.push(note(&row.path, format!("{} v{}: {} is taken as the key of {name}, by the end of its name; check it", row.note, row.version, row.name), false));
                row.type_name = name;
            }
            None if enums.iter().any(|e| e.to_lowercase() == lower) => {
                notes.push(note(&row.path, format!("{} v{}: {} names the enum {base}, not a data object", row.note, row.version, row.name), false));
                row.type_name = String::new();
            }
            // A key of what the originating system keeps outside the repository (Locatie, Stichting): it keeps its name.
            None => {
                notes.push(note(&row.path, format!("{} v{}: {} points at {base}, which the originating system keeps outside the repository", row.note, row.version, row.name), false));
            }
        }
    }
    rows.extend(notes);
}

/// How an interface differs from the one the app writes for its class: each a note.
fn read_interface(f: &SourceFile, text: &str, rows: &[ReadRow], newtonsoft: &[String]) -> Vec<ReadRow> {
    let namespace = namespace_in(text);
    let version = version_of(&namespace);
    let both = newtonsoft.contains(&domain_of(&namespace));
    let Some(head) = text.lines().find(|l| l.contains("public interface ")) else { return Vec::new() };
    let iname: String = head.split("public interface ").nth(1).unwrap_or("").chars().take_while(|c| c.is_alphanumeric()).collect();
    let name = iname.strip_prefix('I').unwrap_or(&iname).to_string();
    let mut notes = Vec::new();
    let mut say = |text: String| notes.push(note(&f.path, format!("{iname} v{version}: {text}"), false));
    // The class's properties, in the order the app writes them.
    let Some(at) = rows.iter().position(|r| r.kind == "object" && r.name == name && r.version == version) else {
        say("its class is not read, so the app does not write it".into());
        return notes;
    };
    let class: Vec<&ReadRow> = rows[at + 1..].iter().take_while(|r| r.kind == "property" || r.kind == "note").filter(|r| r.kind == "property").collect();
    let keyed = rows[at].keyed;
    let lines: Vec<&str> = text.lines().collect();
    let mut declared: Vec<String> = Vec::new();
    let mut attributes: Vec<String> = Vec::new();
    for raw in &lines {
        let line = raw.trim();
        if line.starts_with('[') {
            attributes.push(line.to_string());
            continue;
        }
        if line.starts_with("public ") && !line.contains("interface") && (line.contains("{ get") || line.contains("=>")) {
            let left = line.trim_start_matches("public ").split(" {").next().unwrap_or("").split(" =>").next().unwrap_or("").trim().to_string();
            let pname = left.rsplit(' ').next().unwrap_or("").to_string();
            let type_text = left.rsplit_once(' ').map(|(t, _)| t).unwrap_or("");
            if line.contains("=>") {
                say(format!("{pname} carried its computed body; the interface declares it, {{ get; }}"));
            }
            if line.contains("set;") {
                say(format!("{pname} had a setter; it is read only"));
            }
            if pname.ends_with("BK") {
                say(format!("{pname} is written {}Bk", &pname[..pname.len() - 2]));
            }
            if let Some(p) = class.iter().find(|p| p.name.eq_ignore_ascii_case(&pname)) {
                if p.property_kind == "EnumChoice" && !converters_as_written(&attributes, both) {
                    say(format!("{pname} gets {}, as most interfaces have them", converters_word(both)));
                }
                if p.property_kind.starts_with("Held") && p.type_version > 1 && !type_text.contains(&format!(".v{}.", p.type_version)) {
                    say(format!("{pname} names its type of v{} in full, as the Onderwijs interfaces do", p.type_version));
                }
            }
            declared.push(pname);
        }
        attributes.clear();
    }
    let written: Vec<String> = class.iter().map(|p| p.name.clone()).collect();
    let lower = |v: &[String]| v.iter().map(|s| s.to_lowercase()).collect::<Vec<_>>();
    for missing in written.iter().filter(|p| !lower(&declared).contains(&p.to_lowercase())) {
        say(format!("{missing} was not in it; it is, as in the class"));
    }
    for extra in declared.iter().filter(|p| !lower(&written).contains(&p.to_lowercase())) {
        say(format!("{extra} is not in the class; it is left out"));
    }
    let shared: Vec<String> = lower(&declared).into_iter().filter(|p| lower(&written).contains(p)).collect();
    let in_class: Vec<String> = lower(&written).into_iter().filter(|p| shared.contains(p)).collect();
    if shared != in_class {
        say("its properties were in another order than the class's; they follow the class".into());
    }
    if keyed != head.contains("IFcdmModel") {
        say(if keyed { "it gets IFcdmModel, as its class has a business key".into() } else { "it is no IFcdmModel, as its class has no business key".into() });
    }
    // Its usings: Fontys.Fcdm.Interfaces when keyed, System for dates and obsolete, lists, the enum converters.
    let usings: Vec<String> = lines.iter().filter_map(|l| l.trim().strip_prefix("using ")).map(|u| u.trim_end_matches(';').to_string()).collect();
    let kinds: Vec<&str> = class.iter().map(|p| p.property_kind.as_str()).collect();
    let mut needed: Vec<String> = Vec::new();
    if keyed {
        needed.push("Fontys.Fcdm.Interfaces".into());
    }
    if kinds.iter().any(|k| matches!(*k, "Moment" | "MomentWithOffset" | "DayOnly")) || class.iter().any(|p| !p.obsolete.is_empty()) || !rows[at].obsolete.is_empty() {
        needed.push("System".into());
    }
    if kinds.contains(&"HeldObjects") {
        needed.push("System.Collections.Generic".into());
    }
    if kinds.contains(&"EnumChoice") {
        needed.push("System.Text.Json.Serialization".into());
        if both {
            needed.push("Newtonsoft.Json.Converters".into());
        }
    }
    for extra in usings.iter().filter(|u| !needed.contains(u)) {
        say(format!("using {extra} is left out, as nothing needs it"));
    }
    for missing in needed.iter().filter(|u| !usings.contains(u)) {
        say(format!("using {missing} is added, as it is needed"));
    }
    if !f.text.ends_with('\n') {
        say("the file ended without a newline; it ends with one".into());
    }
    if f.text.contains("\r\n") {
        say("its lines ended in CRLF; they end in LF".into());
    }
    notes
}

const STJ_CONVERTER: &str = "[System.Text.Json.Serialization.JsonConverter(typeof(JsonStringEnumConverter))]";
const NEWTONSOFT_CONVERTER: &str = "[Newtonsoft.Json.JsonConverter(typeof(StringEnumConverter))]";

/// Whether an enum property's attributes are those the app writes: System.Text.Json's converter in full, and Newtonsoft's where the project has it.
fn converters_as_written(attributes: &[String], both: bool) -> bool {
    let wanted: Vec<&str> = if both { vec![STJ_CONVERTER, NEWTONSOFT_CONVERTER] } else { vec![STJ_CONVERTER] };
    let had: Vec<&str> = attributes.iter().map(String::as_str).filter(|a| !a.contains("Obsolete")).collect();
    had == wanted
}

fn converters_word(both: bool) -> &'static str {
    if both { "both enum converters, written in full" } else { "the System.Text.Json enum converter, written in full" }
}

fn ignores_as_written(attributes: &[String], both: bool) -> bool {
    let had: Vec<&str> = attributes.iter().map(String::as_str).filter(|a| !a.contains("Obsolete")).collect();
    if both { had == ["[JsonIgnore]", "[Newtonsoft.Json.JsonIgnore]"] } else { had == ["[JsonIgnore]"] }
}

fn ignores_word(both: bool) -> &'static str {
    if both { "both JsonIgnores" } else { "JsonIgnore" }
}

fn obsolete_of(lines: &[&str], at: usize) -> String {
    lines[..at].iter().rev().take_while(|l| l.trim().starts_with('[')).find(|l| l.contains("Obsolete")).map(|l| quoted(l)).unwrap_or_default()
}

/// A property as a row: its kind and type from its C# type.
fn property_row(path: &str, domain: &str, owner: &str, version: i64, name: &str, type_text: &str, enums: &[String]) -> ReadRow {
    let nullable = type_text.ends_with('?');
    let bare = type_text.trim_end_matches('?');
    let mut row = ReadRow { kind: "property".into(), path: path.into(), domain: domain.into(), name: name.into(), version, nullable, ..ReadRow::default() };
    row.note = owner.to_string();
    // A type named from another version: `Fcdm.Hr.v3.Arbeidscontract` or `v3.Arbeidscontract`.
    let named = |t: &str| -> (String, i64) {
        let parts: Vec<&str> = t.split('.').collect();
        let last = parts.last().copied().unwrap_or(t).to_string();
        let at = parts.iter().rev().skip(1).find_map(|p| p.strip_prefix('v').and_then(|n| n.parse::<i64>().ok())).unwrap_or(0);
        (last, at)
    };
    let kind = match bare {
        "string" if name.ends_with("Bk") || name.ends_with("BK") => {
            row.type_name = name[..name.len() - 2].to_string();
            "BusinessKey"
        }
        "string" => "TextValue",
        "int" | "long" => "WholeNumber",
        "decimal" | "double" => "DecimalNumber",
        "bool" => "YesNo",
        "DateTime" => "Moment",
        "DateTimeOffset" => "MomentWithOffset",
        "DateOnly" => "DayOnly",
        t if t.starts_with("IEnumerable<") || t.starts_with("List<") || t.starts_with("IReadOnlyCollection<") => {
            let inner = t.split_once('<').map(|(_, r)| r.trim_end_matches('>')).unwrap_or("");
            let (n, v) = named(inner.trim_end_matches('?'));
            row.type_name = n;
            row.type_version = v;
            "HeldObjects"
        }
        t => {
            let (n, v) = named(t);
            let is_enum = enums.contains(&n);
            row.type_name = n;
            row.type_version = v;
            if is_enum { "EnumChoice" } else { "HeldObject" }
        }
    };
    row.property_kind = kind.into();
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, text: &str) -> SourceFile {
        SourceFile { path: path.into(), text: text.into() }
    }

    #[test]
    fn a_project_file_names_its_domain_s_package_and_version() {
        let enum_file = file("Fontys.Fcdm.Hr/Soort.cs", "namespace Fontys.Fcdm.Hr\n{\n    public enum Soort\n    {\n        A = 0\n    }\n}\n");
        let dispatcher = file("Fontys.Fcdm.EventDispatcher/Fontys.Fcdm.EventDispatcher.csproj", "<PackageId>Fontys.Fcdm.EventDispatcher</PackageId><Version>1.0.0</Version>");
        let rows = read_files(&[file("Fontys.Fcdm.Hr/Fontys.Fcdm.Hr.csproj", "<Project><PropertyGroup><PackageId>Fontys.Fcdm.Hr</PackageId>\n<Version>10.2.1</Version></PropertyGroup></Project>"), enum_file, dispatcher]);
        assert_eq!(rows.iter().filter(|r| r.kind == "domain").count(), 1, "a package of no domain is left out");
        assert_eq!((rows[0].kind.as_str(), rows[0].domain.as_str(), rows[0].role.as_str(), rows[0].name.as_str()), ("domain", "Hr", "model", "10.2.1"));
    }

    #[test]
    fn a_class_gives_its_version_its_properties_and_what_was_tidied() {
        let enum_file = file("Fontys.Fcdm.Hr/Soort.cs", "\u{feff}namespace Fontys.Fcdm.Hr\n{\n    public enum Soort\n    {\n        A = 0,\n        B = 2\n    }\n}\n");
        let class = "\u{feff}using System;\n\nnamespace Fontys.Fcdm.Hr.v2\n{\n    [Obsolete(\"Use v3.\")]\n    public class Medewerker : IMedewerker\n    {\n        public Medewerker(string bk, string? functieBK, Soort soort, IEnumerable<Fcdm.Hr.v3.Contract> contracten, string? brinBk = null)\n        {\n        }\n\n        public string Bk { get; init; }\n\n        public string? FunctieBK { get; }\n\n        public Soort Soort { get; init; }\n\n        public IEnumerable<Fcdm.Hr.v3.Contract> Contracten { get; init; }\n\n        public string? BrinBk { get; init; }\n\n        [JsonIgnore]\n        public string Naam => $\"{Soort}\";\n\n        public bool IsNieuw(DateTime d)\n        {\n            return true;\n        }\n    }\n}\n";
        let rows = read_files(&[enum_file, file("Fontys.Fcdm.Hr/v2/Medewerker.cs", class), file("Fontys.Fcdm.Hr.Testing/X.cs", "public class X {}")]);
        let object = rows.iter().find(|r| r.kind == "object").unwrap();
        assert_eq!((object.name.as_str(), object.version, object.root, object.obsolete.as_str()), ("Medewerker", 2, true, "Use v3."));
        let props: Vec<&ReadRow> = rows.iter().filter(|r| r.kind == "property").collect();
        let names: Vec<&str> = props.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["FunctieBk", "Soort", "Contracten", "BrinBk", "Naam"]);
        // Functie is not in these files: the key points outside them, by its name.
        assert_eq!((props[0].property_kind.as_str(), props[0].type_name.as_str(), props[0].nullable), ("BusinessKey", "Functie", true));
        assert!(rows.iter().any(|r| r.kind == "note" && r.note.contains("FunctieBk points at Functie, which the originating system keeps outside the repository")), "{rows:#?}");
        assert_eq!((props[1].property_kind.as_str(), props[1].type_name.as_str()), ("EnumChoice", "Soort"));
        assert_eq!((props[2].property_kind.as_str(), props[2].type_name.as_str(), props[2].type_version), ("HeldObjects", "Contract", 3));
        assert_eq!(props[3].initial, "null");
        assert_eq!(props[4].computed, "$\"{Soort}\"");
        let values: Vec<(&str, i64)> = rows.iter().filter(|r| r.kind == "value").map(|r| (r.name.as_str(), r.number)).collect();
        assert_eq!(values, [("A", 0), ("B", 2)]);
        let notes: Vec<&ReadRow> = rows.iter().filter(|r| r.kind == "note").collect();
        assert!(notes.iter().any(|n| n.renames && n.note.contains("FunctieBK is written FunctieBk")), "{notes:#?}");
        assert!(notes.iter().any(|n| n.note.contains("FunctieBK had no init")), "{notes:#?}");
        assert!(notes.iter().any(|n| n.note.contains("the method IsNieuw is code")), "{notes:#?}");
        assert!(!rows.iter().any(|r| r.name == "X"), "a test project is not read");
    }
}
