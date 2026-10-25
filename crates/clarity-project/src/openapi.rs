//! OpenAPI descriptions in the project (DESIGN 5.23, D2). A web API data
//! source names its description with `openapi "forms.yaml"`, a file of the
//! project's `apis` folder, and picks the operations it uses with `pick`
//! (and schemas of its own with `pick schema`). Clarity writes the module
//! `apis/<source>.clarity` from it: an entity per schema the picks need,
//! living in the web API, and an action per picked operation, in plain
//! Clarity on `http ... from source` and `read ... from json`. A new revision
//! of the description writes it again, so its entities follow the service.
//!
//! OpenAPI 3 and Swagger 2, in YAML or JSON (which is YAML too).

use yaml_rust2::{Yaml, YamlLoader};

/// A description, as far as Clarity reads it: in the order it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Description {
    pub title: String,
    pub version: String,
    pub operations: Vec<Operation>,
    pub schemas: Vec<Schema>,
}

/// An operation: a method on a path, by its id (`operationId`, else
/// `GET /path`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    pub id: String,
    /// `get`, `post`, `put`, `delete` or `patch`.
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub parameters: Vec<Parameter>,
    /// What it sends, as JSON.
    pub body: Option<Shape>,
    /// What it answers with on success, as JSON.
    pub result: Option<Shape>,
}

/// A parameter of an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    /// `path`, `query` or `header`.
    pub at: String,
    pub required: bool,
    pub shape: Shape,
    /// The value the service takes when it is left out, as written.
    pub default: Option<String>,
}

/// A schema of `components/schemas` (`definitions` in Swagger 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    pub name: String,
    pub properties: Vec<(String, Shape)>,
}

/// What a value is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    Text,
    Integer,
    Number,
    Bool,
    Date,
    DateTime,
    /// A text that is one of these values.
    Enum(Vec<String>),
    List(Box<Shape>),
    /// A schema, by its name.
    Named(String),
    /// An object or anything else written in place.
    Other,
}

/// A description's text as an ordered tree: YAML, and JSON, which is YAML.
fn tree(text: &str) -> Result<Yaml, String> {
    let mut documents = YamlLoader::load_from_str(text).map_err(|e| format!("the description is no YAML or JSON: {e}"))?;
    if documents.is_empty() {
        return Err("the description is empty".to_string());
    }
    Ok(documents.swap_remove(0))
}

fn text_of(node: &Yaml) -> Option<String> {
    match node {
        Yaml::String(s) | Yaml::Real(s) => Some(s.clone()),
        Yaml::Integer(n) => Some(n.to_string()),
        Yaml::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

fn get<'y>(node: &'y Yaml, key: &str) -> &'y Yaml {
    &node[key]
}

fn entries(node: &Yaml) -> Vec<(String, &Yaml)> {
    match node {
        Yaml::Hash(hash) => hash.iter().filter_map(|(k, v)| text_of(k).map(|k| (k, v))).collect(),
        _ => Vec::new(),
    }
}

fn items(node: &Yaml) -> &[Yaml] {
    match node {
        Yaml::Array(list) => list,
        _ => &[],
    }
}

/// The schema a `$ref` names: `#/components/schemas/Order` or `#/definitions/Order`.
fn named(reference: &str) -> String {
    reference.rsplit('/').next().unwrap_or(reference).to_string()
}

fn shape(node: &Yaml) -> Shape {
    if let Some(reference) = text_of(get(node, "$ref")) {
        return Shape::Named(named(&reference));
    }
    let kind = text_of(get(node, "type")).unwrap_or_default();
    let format = text_of(get(node, "format")).unwrap_or_default();
    match kind.as_str() {
        "string" if !items(get(node, "enum")).is_empty() => Shape::Enum(items(get(node, "enum")).iter().filter_map(text_of).collect()),
        "string" if format == "date" => Shape::Date,
        "string" if format == "date-time" => Shape::DateTime,
        "string" => Shape::Text,
        "integer" => Shape::Integer,
        "number" => Shape::Number,
        "boolean" => Shape::Bool,
        "array" => Shape::List(Box::new(shape(get(node, "items")))),
        _ => Shape::Other,
    }
}

/// The JSON a request body or a response holds: OpenAPI 3's `content`, else
/// Swagger 2's `schema`.
fn json_shape(node: &Yaml) -> Option<Shape> {
    let content = get(node, "content");
    if let Some((_, media)) = entries(content).into_iter().find(|(t, _)| t.contains("json")) {
        let schema = get(media, "schema");
        return (!schema.is_badvalue()).then(|| shape(schema));
    }
    let schema = get(node, "schema");
    (!schema.is_badvalue()).then(|| shape(schema))
}

const METHODS: [&str; 5] = ["get", "post", "put", "delete", "patch"];

/// Reads a description, OpenAPI 3 or Swagger 2, in YAML or JSON.
pub fn read(text: &str) -> Result<Description, String> {
    let root = tree(text)?;
    if get(&root, "openapi").is_badvalue() && get(&root, "swagger").is_badvalue() {
        return Err("the description is no OpenAPI or Swagger document: it has neither 'openapi' nor 'swagger'".to_string());
    }
    let info = get(&root, "info");
    let mut operations = Vec::new();
    for (path, item) in entries(get(&root, "paths")) {
        let shared = items(get(item, "parameters")).to_vec();
        for (method, op) in entries(item) {
            if !METHODS.contains(&method.as_str()) {
                continue;
            }
            let mut parameters = Vec::new();
            let mut body = None;
            for p in shared.iter().chain(items(get(op, "parameters"))) {
                let at = text_of(get(p, "in")).unwrap_or_default();
                // Swagger 2 sends a body as a parameter.
                if at == "body" {
                    body = Some(shape(get(p, "schema")));
                    continue;
                }
                let Some(name) = text_of(get(p, "name")) else { continue };
                let schema = get(p, "schema");
                let shape = if schema.is_badvalue() { shape(p) } else { shape(schema) };
                let required = matches!(get(p, "required"), Yaml::Boolean(true)) || at == "path";
                let default = text_of(get(if schema.is_badvalue() { p } else { schema }, "default"));
                parameters.retain(|q: &Parameter| !(q.name == name && q.at == at));
                parameters.push(Parameter { name, at, required, shape, default });
            }
            if let Some(found) = json_shape(get(op, "requestBody")) {
                body = Some(found);
            }
            let responses = entries(get(op, "responses"));
            let result = responses.iter().find(|(code, _)| code.starts_with('2')).and_then(|(_, r)| json_shape(r));
            let id = text_of(get(op, "operationId")).unwrap_or_else(|| format!("{} {path}", method.to_uppercase()));
            operations.push(Operation { id, method, path: path.clone(), summary: text_of(get(op, "summary")), parameters, body, result });
        }
    }
    let components = get(get(&root, "components"), "schemas");
    let definitions = if components.is_badvalue() { get(&root, "definitions") } else { components };
    let schemas = entries(definitions)
        .into_iter()
        .map(|(name, node)| Schema { name, properties: entries(get(node, "properties")).into_iter().map(|(p, n)| (p, shape(n))).collect() })
        .collect();
    Ok(Description {
        title: text_of(get(info, "title")).unwrap_or_else(|| "the API".to_string()),
        version: text_of(get(info, "version")).unwrap_or_default(),
        operations,
        schemas,
    })
}

fn json_text(node: &Yaml, out: &mut String) {
    match node {
        Yaml::Hash(hash) => {
            out.push('{');
            for (i, (k, v)) in hash.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                json_string(&text_of(k).unwrap_or_default(), out);
                out.push(':');
                json_text(v, out);
            }
            out.push('}');
        }
        Yaml::Array(list) => {
            out.push('[');
            for (i, v) in list.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                json_text(v, out);
            }
            out.push(']');
        }
        Yaml::String(s) => json_string(s, out),
        Yaml::Real(r) => out.push_str(r),
        Yaml::Integer(n) => out.push_str(&n.to_string()),
        Yaml::Boolean(b) => out.push_str(if *b { "true" } else { "false" }),
        _ => out.push_str("null"),
    }
}

fn json_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A description written as JSON, in the order it is written: the same
/// description as YAML read.
pub fn json_of(text: &str) -> Result<String, String> {
    let mut out = String::new();
    json_text(&tree(text)?, &mut out);
    Ok(out)
}

/// A name of the description as a Clarity name in snake case:
/// `createdAt` and `created-at` are `created_at`.
fn snake(name: &str) -> String {
    let mut out = String::new();
    let mut previous_lower = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && previous_lower && !out.ends_with('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            previous_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            previous_lower = false;
        }
    }
    let out = out.trim_matches('_').to_string();
    let out = if out.starts_with(|c: char| c.is_ascii_digit()) || out.is_empty() { format!("n_{out}") } else { out };
    // A word Clarity keeps for itself gets an underscore; JSON still finds it,
    // as names match without underscores.
    if RESERVED.contains(&out.as_str()) { format!("{out}_") } else { out }
}

/// Words a field or parameter cannot be named.
const RESERVED: [&str; 14] = ["and", "or", "not", "in", "contains", "matches", "none", "true", "false", "table", "key", "column", "source", "search"];

/// A schema's name as its entity is named.
pub fn entity_name(name: &str) -> String {
    pascal(name)
}

/// A schema's name as a Clarity type name.
fn pascal(name: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(if upper { c.to_ascii_uppercase() } else { c });
            upper = false;
        } else {
            upper = true;
        }
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) || out.is_empty() { format!("N{out}") } else { out }
}

fn is_identifier(word: &str) -> bool {
    word.starts_with(|c: char| c.is_ascii_alphabetic()) && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !RESERVED.contains(&word)
}

/// What the module was written as, and what it could not take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub text: String,
    pub warnings: Vec<String>,
}

/// A field's type, the enum it needs, or why it is not read.
enum FieldOf {
    Type(String),
    Enum(String, Vec<String>),
    NotRead(String),
    /// A record that would hold itself (DESIGN 5.25), named for the developer.
    Circle(String),
}

fn field_of(entity: &str, property: &str, shape: &Shape) -> FieldOf {
    let plain = |s: &Shape| match s {
        Shape::Text => Some("text"),
        Shape::Integer => Some("integer"),
        Shape::Number => Some("decimal(18, 6)"),
        Shape::Bool => Some("bool"),
        Shape::Date => Some("date"),
        Shape::DateTime => Some("datetime"),
        _ => None,
    };
    if let Some(ty) = plain(shape) {
        return FieldOf::Type(ty.to_string());
    }
    match shape {
        Shape::Enum(values) if !values.is_empty() && values.iter().all(|v| is_identifier(v)) => FieldOf::Enum(format!("{entity}{}", pascal(property)), values.clone()),
        Shape::Enum(values) => FieldOf::Type(format!("text  // one of {}", values.join(", "))),
        Shape::List(item) => match item.as_ref() {
            Shape::Text | Shape::Enum(_) => FieldOf::Type("list<text>".into()),
            Shape::Integer => FieldOf::Type("list<integer>".into()),
            Shape::Number => FieldOf::Type("list<decimal>".into()),
            Shape::Date => FieldOf::Type("list<date>".into()),
            // Records inside the record (DESIGN 5.25); a list may hold its own kind.
            Shape::Named(name) => FieldOf::Type(format!("list<{}>", pascal(name))),
            _ => FieldOf::NotRead("a list of objects without a schema of their own".into()),
        },
        // A record inside the record (DESIGN 5.25), unless it would hold itself: see `module`.
        Shape::Named(name) => FieldOf::Type(pascal(name)),
        _ => FieldOf::NotRead("an object without a schema of its own".into()),
    }
}

/// The module `apis/<source>.clarity` for what data source `source` picks
/// from `description` (its file `file`): the operations `picks` names, the
/// schemas they take and give, and the schemas `pick_schemas` names.
pub fn module(source: &str, file: &str, description: &Description, picks: &[String], pick_schemas: &[String]) -> Written {
    let mut warnings = Vec::new();
    let about = format!("{file} ({} {})", description.title, description.version).replace(" )", ")");
    let mut operations = Vec::new();
    for pick in picks {
        match description.operations.iter().find(|o| &o.id == pick) {
            Some(op) if op.method == "patch" => warnings.push(format!("data source '{source}' picks '{pick}', a patch, which Clarity does not call yet")),
            Some(op) => operations.push(op),
            None => warnings.push(format!("data source '{source}' picks '{pick}', which {about} does not have")),
        }
    }
    // The schemas the operations take and give, then those picked on their own.
    let mut wanted: Vec<String> = Vec::new();
    let mut want = |shape: &Shape| {
        let name = match shape {
            Shape::Named(n) => n,
            Shape::List(item) => match item.as_ref() {
                Shape::Named(n) => n,
                _ => return,
            },
            _ => return,
        };
        if !wanted.contains(name) {
            wanted.push(name.clone());
        }
    };
    for op in &operations {
        op.body.iter().chain(&op.result).for_each(&mut want);
    }
    for pick in pick_schemas {
        if description.schemas.iter().any(|s| &s.name == pick) {
            want(&Shape::Named(pick.clone()));
        } else {
            warnings.push(format!("data source '{source}' picks schema '{pick}', which {about} does not have"));
        }
    }
    // And the schemas those hold, nested any depth (DESIGN 5.25).
    let mut at = 0;
    while at < wanted.len() {
        if let Some(schema) = description.schemas.iter().find(|s| s.name == wanted[at]) {
            for (_, shape) in &schema.properties {
                let held = match shape {
                    Shape::Named(n) => Some(n),
                    Shape::List(item) => match item.as_ref() {
                        Shape::Named(n) => Some(n),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(n) = held
                    && !wanted.contains(n)
                    && description.schemas.iter().any(|s| &s.name == n)
                {
                    wanted.push(n.clone());
                }
            }
        }
        at += 1;
    }
    let schemas: Vec<&Schema> = description.schemas.iter().filter(|s| wanted.contains(&s.name)).collect();
    // The records held inside others so far, as (holder, held): one that
    // would close a circle is not held, as a record cannot hold itself.
    let mut holds: Vec<(String, String)> = Vec::new();
    let reaches = |holds: &[(String, String)], from: &str, to: &str| {
        let mut todo = vec![from.to_string()];
        let mut seen = Vec::new();
        while let Some(at) = todo.pop() {
            if at == to {
                return true;
            }
            if seen.contains(&at) {
                continue;
            }
            seen.push(at.clone());
            todo.extend(holds.iter().filter(|(h, _)| *h == at).map(|(_, held)| held.clone()));
        }
        false
    };

    let mut out = format!(
        "// Written by Clarity from apis/{about}: the schemas and operations data\n// source {source} picks. Do not edit this file: change the picks in the project\n// file or the description, and Clarity writes it again.\nmodule {source}\nsource {source}\n"
    );
    for schema in &schemas {
        let entity = pascal(&schema.name);
        let mut fields = String::new();
        let mut enums = String::new();
        for (property, shape) in &schema.properties {
            let name = snake(property);
            // Every record has its own id; the service's is never read.
            if name == "id" {
                continue;
            }
            let held = match shape {
                Shape::Named(n) if description.schemas.iter().any(|s| &s.name == n) => Some(pascal(n)),
                Shape::Named(n) => {
                    fields.push_str(&format!("    // {name}: a {}, which the description does not have; not read\n", pascal(n)));
                    continue;
                }
                _ => None,
            };
            let field = match held {
                Some(held) if reaches(&holds, &held, &entity) => FieldOf::Circle(format!("{} {held}, which would hold itself; not read", article(&held))),
                Some(held) => {
                    holds.push((entity.clone(), held));
                    field_of(&entity, property, shape)
                }
                None => field_of(&entity, property, shape),
            };
            match field {
                FieldOf::Type(ty) => fields.push_str(&format!("    {name}: {ty}\n")),
                FieldOf::Enum(ty, values) => {
                    fields.push_str(&format!("    {name}: {ty}\n"));
                    enums.push_str(&format!("\nenum {ty}\n{}", values.iter().map(|v| format!("    {v}\n")).collect::<String>()));
                }
                FieldOf::NotRead(what) => fields.push_str(&format!("    // {name}: {what}, which is not read yet\n")),
                FieldOf::Circle(what) => fields.push_str(&format!("    // {name}: {what}\n")),
            }
        }
        out.push_str(&enums);
        out.push_str(&format!("\nentity {entity}\n{fields}"));
    }
    for op in operations {
        out.push('\n');
        out.push_str(&action(source, op));
    }
    Written { text: out, warnings }
}

/// "a" or "an" before a name: a User, an Order.
fn article(word: &str) -> &'static str {
    let lower = word.to_lowercase();
    if ["uni", "use", "usu", "uti", "eu", "one"].iter().any(|start| lower.starts_with(start)) {
        return "a";
    }
    if lower.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" }
}

/// A parameter's Clarity type, and how its value goes into text.
fn parameter_type(shape: &Shape) -> (&'static str, bool) {
    match shape {
        Shape::Integer => ("integer", true),
        Shape::Number => ("decimal", true),
        Shape::Bool => ("bool", true),
        _ => ("text", false),
    }
}

/// An optional parameter's default in Clarity: the description's own, else
/// the empty value of its type, which leaves it out of the call.
fn default_of(ty: &str, given: Option<&str>) -> String {
    match (ty, given) {
        ("integer", Some(n)) if n.parse::<i64>().is_ok() => n.to_string(),
        ("decimal", Some(n)) if n.parse::<f64>().is_ok() => n.to_string(),
        ("bool", Some("true")) => "true".into(),
        ("text", Some(t)) => format!("{t:?}"),
        ("integer" | "decimal", _) => "0".into(),
        ("bool", _) => "false".into(),
        _ => "\"\"".into(),
    }
}

/// The pairs that send parameter `name` of type `ty` as `at` (`query` or
/// `header`); `default` leaves it out while it holds that value. A bool is
/// sent as `true` or `false`, as a service reads it.
fn pairs_of(at: &str, wire: &str, name: &str, ty: &str, slotted: bool, default: Option<&str>) -> String {
    if ty == "bool" {
        let yes = format!(" {at} \"{wire}\": \"true\" when {name}");
        let no = format!(" {at} \"{wire}\": \"false\" when not {name}");
        return match default {
            Some("true") => no,
            Some(_) => yes,
            None => format!("{yes}{no}"),
        };
    }
    let value = if slotted { format!("\"{{{name}}}\"") } else { name.to_string() };
    match default {
        Some(default) => format!(" {at} \"{wire}\": {value} when {name} != {default}"),
        None => format!(" {at} \"{wire}\": {value}"),
    }
}

/// The action that calls `op`.
fn action(source: &str, op: &Operation) -> String {
    let mut out = String::new();
    if let Some(summary) = &op.summary {
        out.push_str(&format!("// {}\n", summary.lines().next().unwrap_or_default()));
    }
    let mut params = Vec::new();
    // Optional parameters have a default, so they come last (DESIGN 5.24).
    let mut optional = Vec::new();
    let mut path = op.path.clone();
    let mut clauses = String::new();
    let mut unpassed = Vec::new();
    for p in &op.parameters {
        let name = snake(&p.name);
        let (ty, slotted) = parameter_type(&p.shape);
        match p.at.as_str() {
            "path" => {
                path = path.replace(&format!("{{{}}}", p.name), &format!("{{{name}}}"));
                params.push(format!("{name}: {ty}"));
            }
            "query" | "header" if p.required => {
                params.push(format!("{name}: {ty}"));
                clauses.push_str(&pairs_of(&p.at, &p.name, &name, ty, slotted, None));
            }
            "query" | "header" => {
                let default = default_of(ty, p.default.as_deref());
                optional.push(format!("{name}: {ty} = {default}"));
                clauses.push_str(&pairs_of(&p.at, &p.name, &name, ty, slotted, Some(&default)));
            }
            at => unpassed.push(format!("// {name} ({at}) is not passed; Clarity sends path, query and header values\n")),
        }
    }
    let mut body = String::new();
    match &op.body {
        Some(Shape::Named(schema)) => {
            let name = snake(schema);
            params.push(format!("{name}: {}", pascal(schema)));
            body = format!(" with json of {name} header \"Content-Type\": \"application/json\"");
        }
        Some(_) => {
            params.push("body: text".into());
            body = " with body header \"Content-Type\": \"application/json\"".into();
        }
        None => {}
    }
    params.extend(optional);
    let (returns, read) = match &op.result {
        Some(Shape::Named(schema)) => (format!(" -> {}", pascal(schema)), Some(format!("read {} from json reply.text", pascal(schema)))),
        Some(Shape::List(item)) if matches!(item.as_ref(), Shape::Named(_)) => {
            let Shape::Named(schema) = item.as_ref() else { unreachable!() };
            (format!(" -> list<{}>", pascal(schema)), Some(format!("read list<{}> from json reply.text", pascal(schema))))
        }
        Some(_) => (" -> text".into(), Some("reply.text".into())),
        None => (String::new(), None),
    };
    for line in &unpassed {
        out.push_str(line);
    }
    out.push_str(&format!("action {}({}){returns}\n    effects net\n", snake(&op.id), params.join(", ")));
    out.push_str(&format!("    let reply = http {} \"{path}\" from {source}{body}{clauses}\n", op.method));
    out.push_str(&format!("    if not reply.ok\n        refuse \"{source} {} answered {{reply.status}}: {{reply.text}}\"\n", op.id));
    if let Some(read) = read {
        out.push_str(&format!("    return {read}\n"));
    }
    out
}
