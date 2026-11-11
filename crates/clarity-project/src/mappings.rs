//! The mappings of a project for the IDE's mapping page (docs/FORMATS-PLAN.md,
//! step 4): each one as a form to edit, and a preview of a sample file laid
//! over it, read by Clarity.Formats with the mapping's own options object,
//! as a program reads the file.

use std::path::Path;

use clarity_spec::Project;
use clarity_spec::code::MappingDecl;
use clarity_spec::code::mapping::{MappingForm, form_of, options_json};
use clarity_spec::model::{Entity, FieldType, ListItem};
use serde::Serialize;
use serde_json::{Value, json};
use ts_rs::TS;

use crate::load_modules;

/// A mapping of the project: where it is declared and its form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MappingEntry {
    /// The module file that declares it.
    pub path: String,
    /// Where its declaration starts in that file, for writing it back.
    pub item: u32,
    pub form: MappingForm,
}

/// A sample file laid over a mapping: its tables (the records, then the
/// records inside them), or why the file cannot be read through it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MappingPreview {
    pub tables: Vec<PreviewTable>,
    pub problem: Option<String>,
}

/// The records of one entity as its fields hold them: the first rows, and how many there are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PreviewTable {
    /// The field the records are inside, empty for the mapping's own records.
    pub name: String,
    pub entity: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub count: usize,
}

fn project_of(dir: &Path) -> Result<Project, String> {
    load_modules(dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))
}

/// Every mapping the project declares, in file order.
pub fn mappings(dir: &Path) -> Result<Vec<MappingEntry>, String> {
    let project = project_of(dir)?;
    Ok(project
        .files
        .iter()
        .flat_map(|f| f.mappings.iter().map(move |m| MappingEntry { path: f.path.clone(), item: m.span.start, form: form_of(m) }))
        .collect())
}

/// The mapping called `name`.
pub fn mapping(dir: &Path, name: &str) -> Result<MappingEntry, String> {
    mappings(dir)?.into_iter().find(|m| m.form.name == name).ok_or_else(|| format!("the project has no mapping {name}"))
}

/// The first `limit` records of `sample` (a path relative to the project's
/// folder, or absolute) read through mapping `name`, as its entity's fields
/// hold them; a file that does not fit gives the read's message.
pub fn preview_mapping(dir: &Path, name: &str, sample: &Path, limit: usize) -> Result<MappingPreview, String> {
    let project = project_of(dir)?;
    let mapping = project.mapping(name).ok_or_else(|| format!("the project has no mapping {name}"))?;
    let entity = project.entity(&mapping.entity.name).ok_or_else(|| format!("mapping {name} is laid over {}, which no module declares", mapping.entity.name))?;
    let path = if sample.is_absolute() { sample.to_path_buf() } else { dir.join(sample) };
    let bytes = std::fs::read(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    let fields = fields_json(&project, entity, &mapping.fields);
    let read = clarity_formats::read_table(&mapping.format.name, &bytes, &options_json(mapping), &fields.to_string());
    Ok(match read {
        Ok(json) => MappingPreview { tables: tables(&json, mapping, &fields, limit), problem: None },
        Err(problem) => MappingPreview { tables: Vec::new(), problem: Some(problem.text) },
    })
}

/// The fields of `entity` with their kinds, as `read_table` takes them, by
/// their names; the fields of records inside, where the mapping's lines say.
fn fields_json(project: &Project, entity: &Entity, lines: &[clarity_spec::code::MappedField]) -> Value {
    let values = |name: &str| -> Vec<String> { project.enum_decl(name).map(|e| e.variants.iter().map(|v| v.name.name.clone()).collect()).unwrap_or_default() };
    let mut out = Vec::new();
    for field in &entity.fields {
        let name = &field.name.name;
        let kind = match &field.ty {
            FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret => json!({ "kind": "text" }),
            FieldType::Integer | FieldType::Duration | FieldType::Id => json!({ "kind": "integer" }),
            FieldType::Decimal { .. } | FieldType::Money => json!({ "kind": "decimal" }),
            FieldType::Bool => json!({ "kind": "bool" }),
            FieldType::Date => json!({ "kind": "date" }),
            FieldType::DateTime => json!({ "kind": "datetime" }),
            FieldType::Enum { name } => json!({ "kind": "enum", "values": values(&name.name) }),
            FieldType::Values { item } => {
                let item = match item {
                    ListItem::Text => json!({ "kind": "text" }),
                    ListItem::Integer => json!({ "kind": "integer" }),
                    ListItem::Decimal => json!({ "kind": "decimal" }),
                    ListItem::Date => json!({ "kind": "date" }),
                    ListItem::Enum { name } => json!({ "kind": "enum", "values": values(&name.name) }),
                };
                json!({ "kind": "values", "item": item })
            }
            FieldType::List { .. } | FieldType::Record { .. } => {
                let inside = lines.iter().find(|l| l.field.name == *name).and_then(|l| l.inside.as_ref());
                let Some((inside, inner)) = inside.and_then(|i| project.entity(&i.entity.name).map(|e| (i, e))) else { continue };
                let Value::Array(fields) = fields_json(project, inner, &inside.fields) else { continue };
                json!({ "kind": "records", "entity": inner.name.name, "fields": fields, "base": {} })
            }
            FieldType::Ref { .. } | FieldType::Image | FieldType::File => continue,
        };
        let mut kind = kind;
        kind["name"] = Value::String(name.clone());
        out.push(kind);
    }
    Value::Array(out)
}

/// The kind of each field `fields` (as `fields_json` gives them) names,
/// those of records inside under the records field's name, by table name.
fn kinds_of(fields: &Value, table: &str) -> Vec<(String, clarity_formats::Kind)> {
    let Some(list) = fields.as_array() else { return Vec::new() };
    let (first, rest) = table.split_once('/').map_or((table, ""), |(f, r)| (f, r));
    if first.is_empty() {
        return list.iter().filter_map(|f| Some((f.get("name")?.as_str()?.to_string(), clarity_formats::Kind::from_json(&f.to_string()).ok()?))).collect();
    }
    list.iter()
        .find(|f| f.get("name").and_then(Value::as_str) == Some(first))
        .and_then(|f| f.get("fields"))
        .map(|inner| kinds_of(inner, rest))
        .unwrap_or_default()
}

/// The tables `read_table` gave, without each record's own text, cut to
/// `limit` rows, each cell turned back by its field's kind as the library
/// does when it writes a file (a yes or no, a decimal in its shortest form).
fn tables(json: &str, mapping: &MappingDecl, fields: &Value, limit: usize) -> Vec<PreviewTable> {
    let Ok(read) = serde_json::from_str::<Value>(json) else { return Vec::new() };
    let list = match read.get("tables").and_then(Value::as_array) {
        Some(list) => list.clone(),
        None => vec![read],
    };
    list.iter()
        .map(|t| {
            let columns: Vec<String> = t.get("columns").and_then(Value::as_array).map(|c| c.iter().map(|c| c.as_str().unwrap_or_default().to_string()).collect()).unwrap_or_default();
            let shown: Vec<usize> = (0..columns.len()).filter(|i| !columns[*i].starts_with('#')).collect();
            let rows = t.get("rows").and_then(Value::as_array).cloned().unwrap_or_default();
            let name = t.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
            let kinds = kinds_of(fields, &name);
            let kind = |column: usize| kinds.iter().find(|(n, _)| *n == columns[column]).map(|(_, k)| k);
            PreviewTable {
                name,
                entity: t.get("entity").and_then(Value::as_str).unwrap_or(&mapping.entity.name).to_string(),
                columns: shown.iter().map(|i| columns[*i].clone()).collect(),
                rows: rows
                    .iter()
                    .take(limit)
                    .map(|r| {
                        let cells = r.as_array().cloned().unwrap_or_default();
                        shown.iter().map(|i| cell(cells.get(*i).unwrap_or(&Value::Null), kind(*i))).collect()
                    })
                    .collect(),
                count: rows.len(),
            }
        })
        .collect()
}

/// A cell as the preview shows it, turned back by its field's kind
/// (`clarity_formats::table::back`): text as it is, `true` or `false` for a
/// yes or no, a decimal in its shortest form, nothing for none.
fn cell(value: &Value, kind: Option<&clarity_formats::Kind>) -> String {
    use clarity_formats::Tree;
    let stored = match value {
        Value::Null => Tree::Null,
        Value::Bool(yes) => Tree::Bool(*yes),
        Value::Number(n) => Tree::Number(n.to_string()),
        Value::String(text) => Tree::Text(text.clone()),
        other => Tree::Text(other.to_string()),
    };
    match clarity_formats::table::back(&stored, kind, true) {
        Tree::Null => String::new(),
        Tree::Bool(yes) => yes.to_string(),
        Tree::Number(n) | Tree::Text(n) => n,
        Tree::List(items) => items
            .into_iter()
            .map(|i| match i {
                Tree::Number(n) | Tree::Text(n) => n,
                Tree::Bool(b) => b.to_string(),
                _ => String::new(),
            })
            .collect::<Vec<_>>()
            .join(", "),
        Tree::Object(_) => value.to_string(),
    }
}

/// Where mapping `name` is used, each as a person reads it: the data
/// sources of the project file laid over it, and the code that reads or
/// writes `using` it, by file and line. Deleting it is refused while any is left.
pub fn mapping_uses(dir: &Path, name: &str) -> Result<Vec<String>, String> {
    use clarity_spec::code::{ExprKind, MappingUse, Stmt, each_expr_in};
    let project = project_of(dir)?;
    let mut uses = Vec::new();
    let settings_file = dir.with_extension(clarity_spec::SPEC_EXTENSION);
    if let Some(settings) = crate::load(&settings_file).settings {
        let file_name = settings_file.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        for source in settings.datasources.iter().filter(|s| s.file.as_ref().and_then(|f| f.mapping.as_deref()) == Some(name)) {
            uses.push(format!("data source {} in {file_name}", source.name));
        }
    }
    for file in &project.files {
        let file_name = Path::new(&file.path).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| file.path.clone());
        let mut lines: Vec<u32> = Vec::new();
        let mut look = |body: &[Stmt]| {
            each_expr_in(body, &mut |e| {
                let mapping = match &e.kind {
                    ExprKind::ReadFormat { mapping, .. } | ExprKind::FormatOf { mapping, .. } => mapping,
                    _ => return,
                };
                if matches!(mapping, MappingUse::Named { name: used } if used.name == name) {
                    lines.push(e.span.line);
                }
            });
        };
        for block in &file.code {
            block.handlers.iter().for_each(|h| look(&h.body));
            block.events.iter().for_each(|e| look(&e.body));
        }
        file.functions.iter().for_each(|f| look(&f.body));
        file.commands.iter().for_each(|c| look(&c.body));
        lines.sort_unstable();
        lines.dedup();
        uses.extend(lines.into_iter().map(|line| format!("code in {file_name} line {line}")));
    }
    Ok(uses)
}
