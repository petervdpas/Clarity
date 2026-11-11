//! Import for a data source that is a file (docs/FORMATS-PLAN.md, "Import,
//! and the XML Data Binding wizard made generic"): the tables Clarity.Formats
//! finds in the file, or an XSD schema declares, written as entities. A
//! file's columns are text (types are not guessed from values); a schema's
//! are typed, with enums, sizes and `required`. A file source is one flat
//! table: records inside its records stay in the file, unmapped. Beside the
//! entity, Import writes a mapping (docs/FORMATS-PLAN.md, step 4) with the
//! `at` and the paths it found, and the source is laid over it, so the
//! paths are kept in one place and changed on the mapping page.

use std::path::Path;

use clarity_db::{entity_name, field_name};
use clarity_spec::code::mapping::{MappedFieldForm, MappingForm, MappingOptionForm, text as mapping_text};
use clarity_spec::dictionary::{FieldSpec, field_line};
use clarity_spec::model::{Modifiers, snake_case};
use serde::Deserialize;

use crate::{DataSource, FileFormat, SourceTable, file_options_json, sqlite_path};

/// A table as `clarity_formats::describe` gives it.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Table {
    pub path: String,
    #[allow(dead_code)]
    pub count: usize,
    pub columns: Vec<Column>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Column {
    pub name: String,
    pub column: String,
    pub kind: String,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default, rename = "type")]
    pub type_name: Option<String>,
    #[serde(default)]
    pub size: Option<u32>,
    #[serde(default)]
    pub precision: Option<u32>,
    #[serde(default)]
    pub scale: Option<u32>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub list: bool,
}

/// What an import of a file source writes: the module, the tables left out,
/// and the mapping the source is laid over from then on, when it wrote one.
pub(crate) struct FileImport {
    pub text: String,
    pub skipped: Vec<(String, String)>,
    pub mapping: Option<String>,
}

/// The tables of file source `found`: from the file, or from the XSD
/// `schema` (relative to the project's folder `dir`).
pub(crate) fn tables(dir: &Path, found: &DataSource, file: &FileFormat, schema: Option<&Path>) -> Result<Vec<Table>, String> {
    let json = match schema {
        Some(schema) => {
            // The format says whether it has schemas (an XSD for XML).
            let path = if schema.is_absolute() { schema.to_path_buf() } else { dir.join(schema) };
            let text = std::fs::read_to_string(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
            clarity_formats::describe_schema(&file.format, &text).map_err(|p| format!("data source {}: {}", found.name, p.text))?
        }
        None => {
            let path = sqlite_path(dir, found);
            let bytes = std::fs::read(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
            clarity_formats::describe(&file.format, &bytes, &file_options_json(file)).map_err(|p| format!("{}: {}", path.display(), p.text))?
        }
    };
    serde_json::from_str(&json).map_err(|err| format!("the tables of data source {} cannot be read: {err}", found.name))
}

/// The name a table is listed by: its path, or the file's name for the one table at its root.
pub(crate) fn table_name(table: &Table, found: &DataSource) -> String {
    if table.path.is_empty() {
        Path::new(&found.url).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| found.name.clone())
    } else {
        table.path.clone()
    }
}

fn at_of(file: &FileFormat) -> Option<&str> {
    file.options.iter().find(|o| o.name == "at").map(|o| o.value.as_str()).filter(|a| !a.is_empty())
}

/// What separates the steps of a path in the format, as its catalog entry says.
fn separator(file: &FileFormat) -> char {
    clarity_formats::format(&file.format).and_then(|f| f.path_separator).unwrap_or('/')
}

/// Whether `path` is inside `outer`.
fn inside(path: &str, outer: &str, sep: char) -> bool {
    path.strip_prefix(outer).is_some_and(|rest| rest.starts_with(sep))
}

/// The Import list: a table elsewhere than the source's `at` cannot be imported.
pub(crate) fn listed(tables: &[Table], found: &DataSource, file: &FileFormat) -> Vec<SourceTable> {
    let at = at_of(file);
    tables
        .iter()
        .map(|t| SourceTable {
            name: table_name(t, found),
            columns: t.columns.len(),
            not_importable: at.filter(|a| t.path != *a).map(|a| format!("data source {} reads '{a}' (its at)", found.name)),
        })
        .collect()
}

/// Whether a value can be an enum member: a name, which reading matches ignoring case.
fn is_member(value: &str) -> bool {
    value.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The member for a value: the value with a capital first letter.
fn member(value: &str) -> String {
    let mut chars = value.chars();
    chars.next().map(|f| f.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
}

fn unique(base: String, taken: &[String]) -> String {
    if !taken.contains(&base) {
        return base;
    }
    (2..).map(|n| format!("{base}{n}")).find(|n| !taken.contains(n)).expect("a free name")
}

/// Writes `chosen` of `tables` as module `module` of file source `found`:
/// one table, the source's records (at its `at`), as one entity with its
/// own fields. Records inside it stay in the file and are named as skipped.
/// A source not laid over a mapping yet gets one, `<Entity>Feed`, written
/// after the entity with the `at`, the paths that differ from the fields'
/// names and the source's options; `mappings` are the names already taken.
#[allow(clippy::too_many_arguments)]
pub(crate) fn import(tables: &[Table], chosen: &[String], found: &DataSource, file: &FileFormat, module: &str, taken: &[String], mappings: &[String], from: Option<&str>) -> Result<FileImport, String> {
    let source = &found.name;
    let mut skipped = Vec::new();
    for name in chosen {
        if !tables.iter().any(|t| table_name(t, found) == *name) {
            skipped.push((name.clone(), "the file has no such table".to_string()));
        }
    }
    let picked: Vec<&Table> = tables.iter().filter(|t| chosen.contains(&table_name(t, found))).collect();
    let sep = separator(file);
    let tops: Vec<&Table> = picked.iter().copied().filter(|t| !picked.iter().any(|o| !o.path.is_empty() && inside(&t.path, &o.path, sep))).collect();
    let top = match tops.as_slice() {
        [] => return Err("choose a table to import".to_string()),
        [one] => *one,
        many => {
            let names: Vec<String> = many.iter().map(|t| table_name(t, found)).collect();
            return Err(format!("data source {source} is one file, one table: choose one of {}, with tables inside it", names.join(", ")));
        }
    };
    let at = at_of(file);
    let has_at = clarity_formats::format(&file.format).is_some_and(|f| f.path_separator.is_some());
    match at {
        Some(a) if a == top.path => {}
        Some(a) if top.path.is_empty() => return Err(format!("data source {source} reads '{a}' (its at), but the table is the whole file; change its at first")),
        Some(a) => return Err(format!("data source {source} reads '{a}' (its at), not '{}'; import '{a}', or change its at first", top.path)),
        None => {}
    }
    // A source laid over a mapping already keeps it: the entity's columns say where its fields are.
    let writes_mapping = file.mapping.is_none();

    for t in picked.iter().filter(|t| t.path != top.path) {
        skipped.push((table_name(t, found), format!("its records are inside {}, and stay in the file untouched", table_name(top, found))));
    }
    let last = table_name(top, found).rsplit(sep).next().unwrap_or_default().to_string();
    let mut entity = entity_name(&last);
    if entity.is_empty() {
        entity = "Record".to_string();
    }
    if taken.contains(&entity) {
        entity.push_str(&entity_name(source));
    }
    let entity = unique(entity, taken);
    let mut names: Vec<String> = taken.to_vec();
    names.push(entity.clone());

    let mut text = match from {
        Some(schema) => format!("// Imported by Clarity from data source {source}, with the schema {schema}.\nmodule {module}\n\nsource {source}\n"),
        None => format!("// Imported by Clarity from data source {source}.\nmodule {module}\n\nsource {source}\n"),
    };
    // (name, values, the schema's type), each enum once.
    let mut enums: Vec<(String, Vec<String>, Option<String>)> = Vec::new();
    text.push_str(&format!("\nentity {entity}\n"));
    let mut fields: Vec<String> = vec!["id".to_string()];
    let mut notes = Vec::new();
    // Where each field is in the file, where that is not its name.
    let mut paths: Vec<MappedFieldForm> = Vec::new();
    for c in &top.columns {
        let base = field_name(&c.name);
        if base == "id" {
            notes.push(format!("// The file's {} is not imported: every record has its own id.", c.column));
            continue;
        }
        if c.kind == "record" {
            skipped.push((c.name.clone(), format!("field '{}' holds records, which stay in the file untouched", c.name)));
            continue;
        }
        let name = unique(base, &fields);
        let text_type = || match c.size {
            Some(n) if n > 0 => format!("text({n})"),
            _ => "text".to_string(),
        };
        let value = match c.kind.as_str() {
            "integer" | "date" | "datetime" | "bool" => c.kind.clone(),
            "decimal" => match (c.precision, c.scale) {
                (Some(p), s) if p > 0 => format!("decimal({p}, {})", s.unwrap_or(0).min(p)),
                _ => "decimal(18, 6)".to_string(),
            },
            // Values that are no names would be members that never match: text, with the values named.
            "enum" if !c.values.iter().all(|v| is_member(v)) => {
                let values: Vec<String> = c.values.iter().map(|v| format!("\"{v}\"")).collect();
                notes.push(format!("// {name} takes one of {}.", values.join(", ")));
                text_type()
            }
            "enum" => {
                let wanted = c.type_name.as_deref().map(entity_name).unwrap_or_else(|| format!("{entity}{}", entity_name(&c.name)));
                match enums.iter().find(|(_, values, from)| *values == c.values && from.is_some() && *from == c.type_name) {
                    Some((existing, _, _)) => existing.clone(),
                    None => {
                        let enum_name = unique(wanted, &names);
                        names.push(enum_name.clone());
                        enums.push((enum_name.clone(), c.values.clone(), c.type_name.clone()));
                        enum_name
                    }
                }
            }
            _ => text_type(),
        };
        let type_text = match (c.list, value.as_str()) {
            (false, _) => value.clone(),
            (true, "integer" | "date") => format!("list<{value}>"),
            (true, v) if v.starts_with("decimal") => "list<decimal>".to_string(),
            (true, _) if c.kind == "enum" && c.values.iter().all(|v| is_member(v)) => format!("list<{value}>"),
            (true, _) => "list<text>".to_string(),
        };
        let spec = FieldSpec {
            name: name.clone(),
            type_text,
            modifiers: Modifiers {
                required: c.required && !c.list,
                // A field finds its value by its name; a column only where that would not, and no mapping says where.
                column: (!writes_mapping && !c.list && snake_case(&name) != c.column).then(|| c.column.clone()),
                ..Modifiers::default()
            },
        };
        text.push_str(&format!("    {}\n", field_line(&spec)));
        if c.column != name {
            paths.push(MappedFieldForm { field: name.clone(), at: c.column.clone(), inside: None });
        }
        fields.push(name);
    }
    for note in notes {
        text.push_str(&format!("    {note}\n"));
    }
    for (name, values, _) in &enums {
        text.push_str(&format!("\nenum {name}\n"));
        let mut members: Vec<String> = Vec::new();
        for value in values {
            let m = unique(member(value), &members);
            text.push_str(&format!("    {m}\n"));
            members.push(m);
        }
    }
    if !writes_mapping {
        return Ok(FileImport { text, skipped, mapping: None });
    }
    let mapping = unique(format!("{entity}Feed"), &[mappings, &names].concat());
    let form = MappingForm {
        name: mapping.clone(),
        entity,
        format: file.format.clone(),
        at: if has_at { top.path.clone() } else { String::new() },
        fields: paths,
        check: String::new(),
        options: file.options.iter().filter(|o| o.name != "at").map(|o| MappingOptionForm { name: o.name.clone(), value: o.value.clone() }).collect(),
    };
    text.push_str(&format!("\n{}", mapping_text(&form)));
    Ok(FileImport { text, skipped, mapping: Some(mapping) })
}
