//! The data model's tools: entities, enums, their fields and values, and the
//! modules a module uses, each made as the designer's edit of it. Renames
//! are the `rename` tool's. See mod.rs for the structural tools as a whole.

use std::path::{Path, PathBuf};

use clarity_agent::ToolDef;
use clarity_spec::Project;
use clarity_spec::dictionary::FieldSpec;
use clarity_spec::edit::Edit;
use clarity_spec::model::{Literal, Modifiers, default_label};
use serde_json::{Map, Value, json};

use super::{places, schema, tool};
use crate::ProjectTools;
use crate::edits::{self, Applied};
use crate::files::FileChange;

// The field types, shared with the IDE's entity editor.
use clarity_spec::dictionary::{NAMED_TYPES, SIMPLE_TYPES};

/// The modifiers that are on or off.
const FLAGS: &[(&str, &str)] = &[
    ("required", "A value must be given."),
    ("unique", "No two records have the same value."),
    ("owned", "The records it holds belong to this one."),
    ("computed", "Worked out, not entered."),
    ("search", "In the record's full-text index."),
    ("cascade", "On a ref: deleting the record it refers to deletes this one first."),
];

/// The modifiers that hold a value: a number, a text, true or false, or an enum's value.
const VALUES: &[(&str, &str)] = &[("min", "The smallest value."), ("max", "The largest value."), ("default", "The value a new record starts with; an enum field's is one of its values.")];

/// The modifiers that hold a text.
const TEXTS: &[(&str, &str)] = &[
    ("label", "What people read for the field; a text, or a translation key like @customerEmail."),
    ("help", "The field's help text; a text, or a translation key."),
    ("pattern", "A regular expression a text value matches."),
    ("format", "How the value is shown, a format name."),
    ("column", "The existing column of an external entity the field is stored in."),
    ("store", "An image field's file store, from the project settings."),
];

pub fn definitions() -> Vec<ToolDef> {
    let name = |what: &str| json!({ "type": "string", "description": what });
    vec![
        tool(
            "add_entity",
            "Adds an entity (a table of records) to a module. It starts with the field `name: text(80)`: change it with change_field, add more with add_field.",
            schema(json!({ "module": name("The module, by name."), "name": name("The new entity's name, like Customer.") }), &["module", "name"]),
        ),
        tool(
            "add_enum",
            "Adds an enum (a fixed set of values) to a module, with its values in order.",
            schema(
                json!({
                    "module": name("The module, by name."),
                    "name": name("The new enum's name, like Status."),
                    "values": { "type": "array", "items": { "type": "string" }, "minItems": 1, "description": "Its values, like Open and Closed." }
                }),
                &["module", "name", "values"],
            ),
        ),
        tool("add_field", "Adds a field at the end of an entity.", schema(Value::Object(field_properties(true)), &["entity", "name", "type"])),
        tool(
            "change_field",
            "Changes a field of an entity: its type or modifiers; what is not given stays. null (or false) takes a modifier away. Renaming is the rename tool's.",
            schema(Value::Object(field_properties(false)), &["entity", "field"]),
        ),
        tool(
            "add_enum_value",
            "Adds a value at the end of an enum.",
            schema(json!({ "enum": name("The enum, by name."), "name": name("The new value, like Archived."), "label": name("What people read for it; a text or a translation key. Left out, it is made from the name.") }), &["enum", "name"]),
        ),
        tool(
            "change_enum_value",
            "Changes a value of an enum: its name or its label; what is not given stays. A null label is made from the name.",
            schema(json!({ "enum": name("The enum, by name."), "value": name("The value, by name."), "name": name("Its new name."), "label": name("What people read for it; a text or a translation key.") }), &["enum", "value"]),
        ),
        tool(
            "set_uses",
            "Sets the modules a module uses (its `use` lines), replacing the ones it has.",
            schema(json!({ "module": name("The module, by name."), "uses": { "type": "array", "items": { "type": "string" }, "description": "The modules it uses, by name; empty for none." } }), &["module", "uses"]),
        ),
        tool(
            "remove_model_item",
            "Removes an entity, an enum, a field of an entity or a value of an enum. Refused while something still uses it.",
            schema(
                json!({
                    "kind": { "type": "string", "enum": ["entity", "enum", "field", "value"] },
                    "name": name("The entity, enum, field or value, by name."),
                    "entity": name("For a field: its entity."),
                    "enum": name("For a value: its enum.")
                }),
                &["kind", "name"],
            ),
        ),
    ]
}

/// The properties of add_field (`adding`) and change_field.
fn field_properties(adding: bool) -> Map<String, Value> {
    let mut properties = Map::new();
    properties.insert("entity".into(), json!({ "type": "string", "description": "The entity, by name." }));
    if adding {
        properties.insert("name".into(), json!({ "type": "string", "description": "The new field's name, like email." }));
    } else {
        properties.insert("field".into(), json!({ "type": "string", "description": "The field, by name." }));
    }
    let types: Vec<&str> = SIMPLE_TYPES.iter().chain(NAMED_TYPES).copied().collect();
    properties.insert("type".into(), json!({ "type": "string", "enum": types, "description": "ref, enum, record and list name what they hold in `of`." }));
    properties.insert("of".into(), json!({ "type": "string", "description": "For ref: the entity; enum: the enum; record: the record; list: an entity, a record or a simple type such as text." }));
    properties.insert("length".into(), json!({ "type": "integer", "minimum": 1, "description": "For text: the most characters, like 80." }));
    properties.insert("precision".into(), json!({ "type": "integer", "minimum": 1, "description": "For decimal: the digits in all, like 10." }));
    properties.insert("scale".into(), json!({ "type": "integer", "minimum": 0, "description": "For decimal: the digits after the point, like 2." }));
    for (key, description) in FLAGS {
        properties.insert((*key).into(), json!({ "type": ["boolean", "null"], "description": description }));
    }
    for (key, description) in VALUES {
        properties.insert((*key).into(), json!({ "type": ["string", "number", "boolean", "null"], "description": description }));
    }
    for (key, description) in TEXTS {
        properties.insert((*key).into(), json!({ "type": ["string", "null"], "description": description }));
    }
    properties
}

pub fn call(tools: &mut ProjectTools, name: &str, input: &Value) -> Option<Result<String, String>> {
    let tool = match name {
        "add_entity" => add_entity,
        "add_enum" => add_enum,
        "add_field" => add_field,
        "change_field" => change_field,
        "add_enum_value" => add_enum_value,
        "change_enum_value" => change_enum_value,
        "set_uses" => set_uses,
        "remove_model_item" => remove_model_item,
        _ => return None,
    };
    Some(tool(tools, input))
}

/// The text `key` of the input, trimmed; empty when it is not given.
fn text(input: &Value, key: &str) -> String {
    input[key].as_str().unwrap_or_default().trim().to_string()
}

/// The text `key` of the input, which must be given.
fn needed(input: &Value, key: &str) -> Result<String, String> {
    let value = text(input, key);
    if value.is_empty() { Err(format!("give {key}")) } else { Ok(value) }
}

/// The texts of the array `key` of the input.
fn texts(input: &Value, key: &str) -> Result<Vec<String>, String> {
    match &input[key] {
        Value::Array(items) => items.iter().map(|i| i.as_str().map(|s| s.trim().to_string()).ok_or_else(|| format!("{key} holds names (texts)"))).collect(),
        Value::Null => Ok(Vec::new()),
        _ => Err(format!("{key} is a list of names")),
    }
}

/// The project's folder and its checked model.
fn opened(tools: &ProjectTools) -> Result<(PathBuf, Project), String> {
    let (_, dir) = tools.open()?.clone();
    let project = places::model_project(&dir)?;
    Ok((dir, project))
}

fn add_entity(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = opened(tools)?;
    let file = places::module_file(&project, &needed(input, "module")?)?;
    let name = needed(input, "name")?;
    let applied = edits::apply(&dir, &file, &Edit::AddEntity { name: name.clone() })?;
    tools.applied(&format!("Added entity {name}"), applied)
}

/// AddEnum makes the enum with one value, `First`: a second edit makes
/// the values asked for. Both are one change; when the second is refused,
/// the first is taken back.
fn add_enum(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = opened(tools)?;
    let file = places::module_file(&project, &needed(input, "module")?)?;
    let name = needed(input, "name")?;
    let values = texts(input, "values")?;
    let Some((first, rest)) = values.split_first() else {
        return Err("give values: the enum's values, at least one".into());
    };
    let made = edits::apply(&dir, &file, &Edit::AddEnum { name: name.clone() })?;
    let valued = places::model_project(&dir).and_then(|project| {
        let (_, decl) = places::enum_decl(&project, &name)?;
        let placeholder = decl.variants.first().ok_or("the new enum has no value")?;
        let mut batch = vec![Edit::UpdateVariant { item: placeholder.name.span.start, name: first.clone(), label: None }];
        batch.extend(rest.iter().map(|value| Edit::AddVariant { owner: decl.span.start, name: value.clone(), label: None }));
        edits::apply(&dir, &file, &Edit::Batch { edits: batch })
    });
    match valued {
        Ok(valued) => tools.applied(&format!("Added enum {name} with {}", crate::listed(&values.iter().map(String::as_str).collect::<Vec<_>>())), merged(made, valued)),
        Err(why) => {
            undo(&made)?;
            Err(why)
        }
    }
}

/// `a` then `b`, as one change: each file once, from before `a` to after `b`.
fn merged(mut a: Applied, b: Applied) -> Applied {
    for change in b.changes {
        match a.changes.iter_mut().find(|c| c.file == change.file) {
            Some(made) => made.after = change.after,
            None => a.changes.push(change),
        }
    }
    a.focus = b.focus.or(a.focus);
    a
}

/// Takes back what `applied` wrote.
fn undo(applied: &Applied) -> Result<(), String> {
    for change in applied.changes.iter().rev() {
        FileChange { file: change.file.clone(), before: change.after.clone(), after: change.before.clone() }.make()?;
    }
    Ok(())
}

fn add_field(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = opened(tools)?;
    let (file, entity) = places::entity(&project, &needed(input, "entity")?)?;
    let name = needed(input, "name")?;
    let type_text = type_text(&project, input)?.ok_or_else(|| format!("give type: one of {}", crate::listed(&all_types())))?;
    let mut modifiers = Modifiers::default();
    set_modifiers(&project, input, &type_text, &mut modifiers)?;
    let edit = Edit::AddField { entity: entity.span.start, spec: FieldSpec { name: name.clone(), type_text, modifiers } };
    let applied = edits::apply(&dir, &file, &edit)?;
    tools.applied(&format!("Added field {name} to {}", entity.name.name), applied)
}

fn change_field(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = opened(tools)?;
    let (file, entity) = places::entity(&project, &needed(input, "entity")?)?;
    let field = places::field(entity, &needed(input, "field")?)?;
    if input.get("name").is_some_and(|n| !n.is_null()) {
        return Err("a field is renamed with the rename tool, so its data follows".into());
    }
    let type_text = match type_text(&project, input)? {
        Some(written) => written,
        None => resized(&field.type_text, input)?,
    };
    let mut modifiers = field.modifiers.clone();
    set_modifiers(&project, input, &type_text, &mut modifiers)?;
    let edit = Edit::UpdateField { item: field.span.start, spec: FieldSpec { name: field.name.name.clone(), type_text, modifiers } };
    let applied = edits::apply(&dir, &file, &edit)?;
    tools.applied(&format!("Changed field {} of {}", field.name.name, entity.name.name), applied)
}

/// The field's own type with a new `length` (text) or `precision` and `scale`
/// (decimal) when change_field gives those without a type.
fn resized(type_text: &str, input: &Value) -> Result<String, String> {
    let number = |key: &str| input.get(key).and_then(Value::as_u64);
    let (length, precision, scale) = (number("length"), number("precision"), number("scale"));
    if length.is_none() && precision.is_none() && scale.is_none() {
        return Ok(type_text.to_string());
    }
    let base = type_text.split('(').next().unwrap_or(type_text).trim();
    let sizes: Vec<u64> = type_text.split_once('(').map(|(_, rest)| rest.trim_end_matches(')').split(',').filter_map(|n| n.trim().parse().ok()).collect()).unwrap_or_default();
    match base {
        "text" if precision.is_none() && scale.is_none() => Ok(match length {
            Some(n) => format!("text({n})"),
            None => type_text.to_string(),
        }),
        "decimal" if length.is_none() => {
            let p = precision.or(sizes.first().copied()).ok_or("a decimal field needs precision and scale, like 10 and 2")?;
            let s = scale.or(sizes.get(1).copied()).ok_or("a decimal field needs precision and scale, like 10 and 2")?;
            Ok(format!("decimal({p}, {s})"))
        }
        _ => Err(format!("a field of type {type_text} has no length, precision or scale; give type to change it")),
    }
}

/// Every type add_field takes.
fn all_types() -> Vec<&'static str> {
    SIMPLE_TYPES.iter().chain(NAMED_TYPES).copied().collect()
}

/// The type as written in a module, from `type` and what goes with it;
/// none when the input gives no type.
fn type_text(project: &Project, input: &Value) -> Result<Option<String>, String> {
    let ty = match &input["type"] {
        Value::Null => return Ok(None),
        Value::String(ty) => ty.trim(),
        _ => return Err(format!("type is one of {}", crate::listed(&all_types()))),
    };
    let of = text(input, "of");
    let number = |key: &str| input.get(key).and_then(Value::as_u64);
    // `of` names one of `names`, said with them when it does not.
    let named = |kinds: &str, names: Vec<&str>| -> Result<String, String> {
        if names.contains(&of.as_str()) {
            return Ok(of.clone());
        }
        let there = if names.is_empty() { format!("there are no {kinds} yet") } else { format!("the {kinds} are {}", crate::listed(&names)) };
        Err(if of.is_empty() { format!("a {ty} field names what it holds in of; {there}") } else { format!("there is no {of} for a {ty} field; {there}") })
    };
    let written = match ty {
        "text" => match number("length") {
            Some(n) => format!("text({n})"),
            None => "text".into(),
        },
        "decimal" => match (number("precision"), number("scale")) {
            (Some(p), Some(s)) => format!("decimal({p}, {s})"),
            _ => return Err("a decimal field needs precision and scale, like 10 and 2".into()),
        },
        "ref" => format!("ref {}", named("entities", places::entity_names(project))?),
        "enum" => named("enums", places::enum_names(project))?,
        "record" => named("records", places::record_names(project))?,
        "list" => {
            let names: Vec<&str> = SIMPLE_TYPES.iter().copied().chain(places::entity_names(project)).chain(places::record_names(project)).collect();
            format!("list<{}>", named("simple types, entities and records", names)?)
        }
        simple if SIMPLE_TYPES.contains(&simple) => simple.to_string(),
        other => return Err(format!("there is no type {other}; the types are {}", crate::listed(&all_types()))),
    };
    Ok(Some(written))
}

/// Sets the modifiers the input gives on `modifiers`: null (or false, or an
/// empty text) takes one away; what the input leaves out stays.
fn set_modifiers(project: &Project, input: &Value, type_text: &str, modifiers: &mut Modifiers) -> Result<(), String> {
    for (key, _) in FLAGS {
        let on = match input.get(*key) {
            None => continue,
            Some(Value::Null) => false,
            Some(Value::Bool(on)) => *on,
            Some(_) => return Err(format!("{key} is true or false")),
        };
        let flag = match *key {
            "required" => &mut modifiers.required,
            "unique" => &mut modifiers.unique,
            "owned" => &mut modifiers.owned,
            "computed" => &mut modifiers.computed,
            "search" => &mut modifiers.search,
            _ => &mut modifiers.cascade,
        };
        *flag = on;
    }
    let is_enum = places::enum_names(project).contains(&type_text);
    let numeric = ["integer", "decimal", "money"].iter().any(|t| type_text.starts_with(t));
    for (key, _) in VALUES {
        let value = match input.get(*key) {
            None => continue,
            Some(Value::Null) => None,
            Some(Value::Bool(b)) => Some(Literal::Bool(*b)),
            Some(Value::Number(n)) => Some(Literal::Number(n.to_string())),
            Some(Value::String(s)) if s.trim().is_empty() => None,
            Some(Value::String(s)) if is_enum => Some(Literal::Ident(s.trim().to_string())),
            Some(Value::String(s)) if numeric && s.trim().parse::<f64>().is_ok() => Some(Literal::Number(s.trim().to_string())),
            Some(Value::String(s)) => Some(Literal::Str(s.clone())),
            Some(_) => return Err(format!("{key} is a number, a text, true or false")),
        };
        match *key {
            "min" => modifiers.min = value,
            "max" => modifiers.max = value,
            _ => modifiers.default = value,
        }
    }
    for (key, _) in TEXTS {
        let value = match input.get(*key) {
            None => continue,
            Some(Value::Null) => None,
            Some(Value::String(s)) if s.trim().is_empty() => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => return Err(format!("{key} is a text")),
        };
        match *key {
            "label" => modifiers.label = value,
            "help" => modifiers.help = value,
            "pattern" => modifiers.pattern = value,
            "format" => modifiers.format = value,
            "column" => modifiers.column = value,
            _ => modifiers.store = value,
        }
    }
    Ok(())
}

fn add_enum_value(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = opened(tools)?;
    let (file, decl) = places::enum_decl(&project, &needed(input, "enum")?)?;
    let name = needed(input, "name")?;
    let label = Some(text(input, "label")).filter(|l| !l.is_empty());
    let applied = edits::apply(&dir, &file, &Edit::AddVariant { owner: decl.span.start, name: name.clone(), label })?;
    tools.applied(&format!("Added value {name} to {}", decl.name.name), applied)
}

fn change_enum_value(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = opened(tools)?;
    let (file, decl) = places::enum_decl(&project, &needed(input, "enum")?)?;
    let variant = places::variant(decl, &needed(input, "value")?)?;
    let name = Some(text(input, "name")).filter(|n| !n.is_empty()).unwrap_or_else(|| variant.name.name.clone());
    // The label as written: one made from the name was not written.
    let written = Some(variant.label.clone()).filter(|l| *l != default_label(&variant.name.name));
    let label = match input.get("label") {
        None => written,
        Some(Value::Null) => None,
        Some(Value::String(l)) => Some(l.trim().to_string()).filter(|l| !l.is_empty()),
        Some(_) => return Err("label is a text".into()),
    };
    let applied = edits::apply(&dir, &file, &Edit::UpdateVariant { item: variant.name.span.start, name, label })?;
    tools.applied(&format!("Changed value {} of {}", variant.name.name, decl.name.name), applied)
}

fn set_uses(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = opened(tools)?;
    let module = needed(input, "module")?;
    let file = places::module_file(&project, &module)?;
    let uses = texts(input, "uses")?;
    let others: Vec<&str> = places::module_names(&project).into_iter().filter(|m| *m != module).collect();
    if let Some(unknown) = uses.iter().find(|u| !others.contains(&u.as_str())) {
        let there = if others.is_empty() { "there are no other modules".to_string() } else { format!("the other modules are {}", crate::listed(&others)) };
        return Err(format!("{module} cannot use {unknown}; {there}"));
    }
    let applied = edits::apply(&dir, &file, &Edit::SetUses { uses: uses.clone() })?;
    let what = if uses.is_empty() { format!("{module} uses no modules") } else { format!("{module} uses {}", crate::listed(&uses.iter().map(String::as_str).collect::<Vec<_>>())) };
    tools.applied(&what, applied)
}

fn remove_model_item(tools: &mut ProjectTools, input: &Value) -> Result<String, String> {
    let (dir, project) = opened(tools)?;
    let name = needed(input, "name")?;
    let (file, item, what): (PathBuf, u32, String) = match text(input, "kind").as_str() {
        "entity" => {
            let (file, entity) = places::entity(&project, &name)?;
            (file, entity.span.start, format!("entity {name}"))
        }
        "enum" => {
            let (file, decl) = places::enum_decl(&project, &name)?;
            (file, decl.span.start, format!("enum {name}"))
        }
        "field" => {
            let (file, entity) = places::entity(&project, &needed(input, "entity")?)?;
            (file, places::field(entity, &name)?.span.start, format!("field {name} of {}", entity.name.name))
        }
        "value" => {
            let (file, decl) = places::enum_decl(&project, &needed(input, "enum")?)?;
            (file, places::variant(decl, &name)?.name.span.start, format!("value {name} of {}", decl.name.name))
        }
        _ => return Err("kind is entity, enum, field or value".into()),
    };
    let applied = edits::apply(&dir, Path::new(&file), &Edit::Remove { item })?;
    tools.applied(&format!("Removed {what}"), applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clarity_agent::Tools;

    /// A new project in a scratch folder, with the tools on it: its module is `todo`.
    fn project(name: &str) -> (PathBuf, PathBuf, ProjectTools) {
        let root = std::env::temp_dir().join(format!("clarity-model-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = crate::new_project::write_new_project(&root.join("todo"), clarity_project::starter("Todo", "todo").stack).unwrap();
        let dir = clarity_project::module_dir(&file);
        let tools = ProjectTools::new(Some((file, dir.clone())), root.clone());
        (root, dir, tools)
    }

    /// The tool's answer, which must not be an error.
    fn ok(tools: &mut ProjectTools, name: &str, input: Value) -> String {
        let result = tools.call(name, &input);
        assert!(!result.is_error, "{name}: {}", result.text);
        result.text
    }

    /// The tool's error.
    fn refused(tools: &mut ProjectTools, name: &str, input: Value) -> String {
        let result = tools.call(name, &input);
        assert!(result.is_error, "{name} was not refused: {}", result.text);
        result.text
    }

    #[test]
    fn every_tool_is_defined_once() {
        let names: Vec<String> = definitions().into_iter().map(|d| d.name).collect();
        assert_eq!(names, ["add_entity", "add_enum", "add_field", "change_field", "add_enum_value", "change_enum_value", "set_uses", "remove_model_item"]);
        let add = definitions().into_iter().find(|d| d.name == "add_field").unwrap();
        assert!(add.input_schema["properties"]["type"]["enum"].as_array().unwrap().iter().any(|t| t == "money"));
    }

    #[test]
    fn entities_and_fields_by_name() {
        let (root, dir, mut tools) = project("fields");
        let module = || std::fs::read_to_string(dir.join("todo.clarity")).unwrap();

        let added = ok(&mut tools, "add_entity", json!({"module": "todo", "name": "Customer"}));
        assert!(added.contains("Added entity Customer"), "{added}");
        assert!(module().contains("entity Customer\n    name: text(80)"), "{}", module());
        assert!(refused(&mut tools, "add_entity", json!({"module": "shop", "name": "Order"})).contains("there is no module shop; the modules are todo"));

        ok(&mut tools, "add_field", json!({"entity": "Customer", "name": "email", "type": "email", "required": true}));
        assert!(module().contains("    email: email, required"), "{}", module());
        ok(&mut tools, "add_field", json!({"entity": "Customer", "name": "limit", "type": "decimal", "precision": 10, "scale": 2, "min": 0}));
        assert!(module().contains("    limit: decimal(10, 2), min 0"), "{}", module());
        assert!(refused(&mut tools, "add_field", json!({"entity": "Client", "name": "x", "type": "text"})).contains("there is no entity Client; the entities are Customer"));
        assert!(refused(&mut tools, "add_field", json!({"entity": "Customer", "name": "x", "type": "texts"})).contains("there is no type texts; the types are text, integer"));
        assert!(refused(&mut tools, "add_field", json!({"entity": "Customer", "name": "boss", "type": "ref", "of": "Person"})).contains("there is no Person for a ref field; the entities are Customer"));
        ok(&mut tools, "add_field", json!({"entity": "Customer", "name": "referrer", "type": "ref", "of": "Customer"}));
        assert!(module().contains("    referrer: ref Customer"), "{}", module());

        ok(&mut tools, "change_field", json!({"entity": "Customer", "field": "email", "required": null, "unique": true}));
        assert!(module().contains("    email: email, unique,"), "{}", module());
        ok(&mut tools, "change_field", json!({"entity": "Customer", "field": "name", "type": "text", "length": 120}));
        assert!(module().contains("    name: text(120)"), "{}", module());
        // A length alone keeps the field's type.
        ok(&mut tools, "change_field", json!({"entity": "Customer", "field": "name", "length": 200}));
        assert!(module().contains("    name: text(200)"), "{}", module());
        ok(&mut tools, "change_field", json!({"entity": "Customer", "field": "limit", "scale": 4}));
        assert!(module().contains("    limit: decimal(10, 4), min 0"), "{}", module());
        assert!(refused(&mut tools, "change_field", json!({"entity": "Customer", "field": "email", "length": 40})).contains("length"));
        assert!(refused(&mut tools, "change_field", json!({"entity": "Customer", "field": "mail", "unique": true})).contains("there is no field mail in Customer; the fields of Customer are name, email, limit and referrer"));

        assert!(refused(&mut tools, "remove_model_item", json!({"kind": "field", "entity": "Customer", "name": "phone"})).contains("fields of Customer are"));
        ok(&mut tools, "remove_model_item", json!({"kind": "field", "entity": "Customer", "name": "limit"}));
        assert!(!module().contains("limit"), "{}", module());
        ok(&mut tools, "remove_model_item", json!({"kind": "field", "entity": "Customer", "name": "referrer"}));
        ok(&mut tools, "remove_model_item", json!({"kind": "entity", "name": "Customer"}));
        assert!(!module().contains("Customer"), "{}", module());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn enums_and_their_values_by_name() {
        let (root, dir, mut tools) = project("enums");
        let module = || std::fs::read_to_string(dir.join("todo.clarity")).unwrap();

        ok(&mut tools, "add_enum", json!({"module": "todo", "name": "Status", "values": ["Open", "Closed"]}));
        assert!(module().contains("enum Status\n    Open label @statusOpen\n    Closed label @statusClosed\n"), "{}", module());
        // One change for Undo.
        assert_eq!(tools.written.iter().filter(|w| w.file == dir.join("todo.clarity")).count(), 1);
        assert!(refused(&mut tools, "add_enum_value", json!({"enum": "State", "name": "Done"})).contains("there is no enum State; the enums are Status"));
        ok(&mut tools, "add_enum_value", json!({"enum": "Status", "name": "Archived"}));
        assert!(module().contains("    Closed label @statusClosed\n    Archived"), "{}", module());
        ok(&mut tools, "change_enum_value", json!({"enum": "Status", "value": "Archived", "label": "Put away"}));
        let line = |text: String, start: &str| text.lines().find(|l| l.trim_start().starts_with(start)).map(str::to_string).unwrap_or_default();
        let archived = line(module(), "Archived");
        assert!(archived.contains("label @"), "{}", module());
        assert!(std::fs::read_to_string(dir.join("translations/en.clarity")).unwrap().contains("\"Put away\""));
        // A new name keeps the label as written.
        ok(&mut tools, "change_enum_value", json!({"enum": "Status", "value": "Archived", "name": "Stored"}));
        assert_eq!(line(module(), "Stored"), archived.replacen("Archived", "Stored", 1), "{}", module());
        assert!(refused(&mut tools, "change_enum_value", json!({"enum": "Status", "value": "Gone"})).contains("the values of Status are Open, Closed and Stored"));

        ok(&mut tools, "add_entity", json!({"module": "todo", "name": "Task"}));
        assert!(refused(&mut tools, "add_field", json!({"entity": "Task", "name": "state", "type": "enum", "of": "State"})).contains("the enums are Status"));
        ok(&mut tools, "add_field", json!({"entity": "Task", "name": "status", "type": "enum", "of": "Status", "default": "Open"}));
        assert!(module().contains("    status: Status, default Open"), "{}", module());

        ok(&mut tools, "remove_model_item", json!({"kind": "value", "enum": "Status", "name": "Stored"}));
        assert!(!module().contains("Stored"), "{}", module());
        ok(&mut tools, "remove_model_item", json!({"kind": "field", "entity": "Task", "name": "status"}));
        ok(&mut tools, "remove_model_item", json!({"kind": "enum", "name": "Status"}));
        assert!(!module().contains("enum Status"), "{}", module());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_module_uses_others_by_name() {
        let (root, dir, mut tools) = project("uses");
        std::fs::write(dir.join("billing.clarity"), "module billing\n").unwrap();
        ok(&mut tools, "set_uses", json!({"module": "billing", "uses": ["todo"]}));
        assert!(std::fs::read_to_string(dir.join("billing.clarity")).unwrap().contains("use todo"));
        assert!(refused(&mut tools, "set_uses", json!({"module": "billing", "uses": ["stock"]})).contains("billing cannot use stock; the other modules are todo"));
        ok(&mut tools, "set_uses", json!({"module": "billing", "uses": []}));
        assert!(!std::fs::read_to_string(dir.join("billing.clarity")).unwrap().contains("use todo"));
        let _ = std::fs::remove_dir_all(root);
    }
}
