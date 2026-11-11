//! Mappings (docs/FORMATS-PLAN.md, step 4): how one kind of file is laid
//! over an entity, per case. The checker holds a mapping to its entity and to
//! the catalog's entry for its format; nothing here is particular to a format.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{MappedField, MappedInside, MappingDecl, MappingText, MappingValue};
use crate::diagnostic::Diagnostic;
use crate::model::{Entity, FieldType, SpecFile};

/// A mapping as the IDE's mapping page edits it: plain texts, an empty one
/// for a line the mapping leaves out. `text` writes it as the language
/// writes a mapping; `form_of` reads one into it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MappingForm {
    pub name: String,
    pub entity: String,
    pub format: String,
    /// Where the records are; empty for the format's own place.
    pub at: String,
    pub fields: Vec<MappedFieldForm>,
    /// The schema file the file is checked against; empty for none.
    pub check: String,
    /// The format's options, each value as text (`true` for a flag).
    pub options: Vec<MappingOptionForm>,
}

/// A field's line: its path (empty: found by its name), or the records inside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MappedFieldForm {
    pub field: String,
    pub at: String,
    pub inside: Option<MappedInsideForm>,
}

/// The records inside a field: their entity, the field of theirs naming the
/// record around (`order is ".."`, empty for none), and their fields' lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MappedInsideForm {
    pub entity: String,
    pub parent: String,
    pub fields: Vec<MappedFieldForm>,
}

/// An option of the format: its name and its value as text, empty to leave it out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MappingOptionForm {
    pub name: String,
    pub value: String,
}

/// The form of a mapping as it is written.
pub fn form_of(mapping: &MappingDecl) -> MappingForm {
    fn lines(fields: &[MappedField]) -> Vec<MappedFieldForm> {
        fields
            .iter()
            .map(|f| MappedFieldForm {
                field: f.field.name.clone(),
                at: f.at.as_ref().map(|a| a.value.clone()).unwrap_or_default(),
                inside: f.inside.as_ref().map(|i| MappedInsideForm {
                    entity: i.entity.name.clone(),
                    parent: i.parent.as_ref().map(|p| p.field.name.clone()).unwrap_or_default(),
                    fields: lines(&i.fields),
                }),
            })
            .collect()
    }
    MappingForm {
        name: mapping.name.name.clone(),
        entity: mapping.entity.name.clone(),
        format: mapping.format.name.clone(),
        at: mapping.at.as_ref().map(|a| a.value.clone()).unwrap_or_default(),
        fields: lines(&mapping.fields),
        check: mapping.check.as_ref().map(|c| c.value.clone()).unwrap_or_default(),
        options: mapping
            .options
            .iter()
            .map(|o| MappingOptionForm {
                name: o.name.name.clone(),
                value: match &o.value {
                    MappingValue::Text(text) => text.clone(),
                    MappingValue::Bool(yes) => yes.to_string(),
                },
            })
            .collect(),
    }
}

/// The mapping `form` describes, as the language writes it: its first line,
/// `at`, a line per field that has a path or records inside, `check`, and
/// the options that have a value (a flag of the format bare, others quoted).
pub fn text(form: &MappingForm) -> String {
    fn lines(out: &mut String, fields: &[MappedFieldForm], indent: &str) {
        for f in fields {
            let at = if f.at.is_empty() { String::new() } else { format!(" at {:?}", f.at) };
            match &f.inside {
                Some(inside) => {
                    out.push_str(&format!("{indent}{} of {}{at}\n", f.field, inside.entity));
                    let inner = format!("{indent}    ");
                    if !inside.parent.is_empty() {
                        out.push_str(&format!("{inner}{} is \"..\"\n", inside.parent));
                    }
                    lines(out, &inside.fields, &inner);
                }
                None if !at.is_empty() => out.push_str(&format!("{indent}{}{at}\n", f.field)),
                None => {}
            }
        }
    }
    let mut out = format!("mapping {} of {} from {}\n", form.name, form.entity, form.format);
    if !form.at.is_empty() {
        out.push_str(&format!("    at {:?}\n", form.at));
    }
    lines(&mut out, &form.fields, "    ");
    if !form.check.is_empty() {
        out.push_str(&format!("    check {:?}\n", form.check));
    }
    let info = clarity_formats::format(&form.format);
    for option in form.options.iter().filter(|o| !o.value.is_empty()) {
        let flag = match info.as_ref().and_then(|i| i.options.iter().find(|i| i.name == option.name)) {
            Some(declared) => declared.kind == clarity_formats::OptionKind::Bool,
            None => option.value == "true" || option.value == "false",
        };
        let value = if flag && (option.value == "true" || option.value == "false") { option.value.clone() } else { format!("{:?}", option.value) };
        out.push_str(&format!("    {} {value}\n", option.name));
    }
    out
}

/// A mapping as the options object every stack passes to Clarity.Formats:
/// `{"at":"shop/orders/order","fields":{"number":"@number"},"nested":{"lines":
/// {"at":"lines/line","fields":{...},"parent":"order"}},"indent":true}`, keyed
/// by field names. Its `check` is the schema file's text (`"check":"<xs:schema
/// ...>"`), read when the project was loaded with its folder, so no stack
/// reads files for Clarity.Formats: the schema is part of the program, and
/// a changed schema needs the program generated again. Without the text
/// (no folder, or a file that is not there, which the checker reports) the
/// check is left out.
/// The generator writes it into programs; the IDE's preview reads with it.
pub fn options_json(mapping: &MappingDecl) -> String {
    let mut pairs = records_pairs(mapping.at.as_ref(), &mapping.fields);
    if let Some(schema) = &mapping.schema {
        pairs.push(format!("\"check\":{}", json_text(schema)));
    }
    for option in &mapping.options {
        let value = match &option.value {
            MappingValue::Text(text) => json_text(text),
            MappingValue::Bool(yes) => yes.to_string(),
        };
        pairs.push(format!("{}:{value}", json_text(&option.name.name)));
    }
    format!("{{{}}}", pairs.join(","))
}

/// `at`, `fields` and `nested` of records: those of a mapping, or of the
/// records inside a field.
fn records_pairs(at: Option<&MappingText>, fields: &[MappedField]) -> Vec<String> {
    let mut pairs = Vec::new();
    if let Some(at) = at {
        pairs.push(format!("\"at\":{}", json_text(&at.value)));
    }
    let plain: Vec<String> = fields.iter().filter(|f| f.inside.is_none()).filter_map(|f| Some(format!("{}:{}", json_text(&f.field.name), json_text(&f.at.as_ref()?.value)))).collect();
    if !plain.is_empty() {
        pairs.push(format!("\"fields\":{{{}}}", plain.join(",")));
    }
    let nested: Vec<String> = fields
        .iter()
        .filter_map(|f| {
            let inside = f.inside.as_ref()?;
            let mut inner = records_pairs(f.at.as_ref(), &inside.fields);
            if let Some(parent) = &inside.parent {
                inner.push(format!("\"parent\":{}", json_text(&parent.field.name)));
            }
            Some(format!("{}:{{{}}}", json_text(&f.field.name), inner.join(",")))
        })
        .collect();
    if !nested.is_empty() {
        pairs.push(format!("\"nested\":{{{}}}", nested.join(",")));
    }
    pairs
}

/// A text as a JSON string.
fn json_text(text: &str) -> String {
    serde_json::Value::String(text.to_string()).to_string()
}

/// Whether paths in `format` can name an attribute (`@number`).
///
/// The catalog does not say yet: until `clarity_formats::FormatInfo` has a
/// field of its own for it, XML is the one format of the catalog with attributes.
pub fn has_attributes(format: &clarity_formats::FormatInfo) -> bool {
    format.attributes
}

/// Every mapping of `file`, with `entity` finding an entity or record by name
/// among the modules the file sees, and `mappings` every mapping it sees.
pub(crate) fn check<'e>(file: &SpecFile, entity: &dyn Fn(&str) -> Option<&'e Entity>, mappings: &[&MappingDecl], out: &mut Vec<Diagnostic>) {
    for mapping in &file.mappings {
        let name = &mapping.name.name;
        if !name.starts_with(|c: char| c.is_ascii_uppercase()) {
            out.push(Diagnostic::error(format!("a mapping's name starts with a capital, as in 'mapping {}'", capitalized(name)), mapping.name.span));
        }
        if mappings.iter().filter(|m| m.name.name == *name).count() > 1 {
            out.push(Diagnostic::error(format!("two mappings are named {name}"), mapping.name.span));
        }
        let Some(format) = clarity_formats::format(&mapping.format.name) else {
            let formats: Vec<&str> = clarity_formats::catalog().iter().map(|f| f.name).collect();
            out.push(Diagnostic::error(format!("there is no format '{}'; the formats are {}", mapping.format.name, formats.join(", ")), mapping.format.span));
            continue;
        };
        let Some(over) = entity(&mapping.entity.name).filter(|e| !e.settings) else {
            out.push(Diagnostic::error(format!("unknown entity '{}'", mapping.entity.name), mapping.entity.span));
            continue;
        };
        let mut check = Check { format: &format, entity, out };
        if let Some(at) = &mapping.at {
            if format.path_separator.is_none() {
                check.out.push(Diagnostic::error(format!("{} holds its records in rows: a mapping from {} takes no 'at'", format.title, format.name), at.span));
            } else {
                check.path(at, false);
            }
        }
        check.fields(over, &mapping.fields, None);
        if let Some(schema) = &mapping.check {
            check.schema(schema);
        }
        check.options(mapping);
    }
}

struct Check<'c, 'e> {
    format: &'c clarity_formats::FormatInfo,
    entity: &'c dyn Fn(&str) -> Option<&'e Entity>,
    out: &'c mut Vec<Diagnostic>,
}

impl Check<'_, '_> {
    /// The lines of `over`'s fields: each a field of it, once; records inside
    /// are a list or record of the entity named; `parent` is the line naming
    /// the record around, for the records inside `around`.
    fn fields(&mut self, over: &Entity, fields: &[MappedField], inside: Option<(&MappedInside, &Entity)>) {
        let mut seen: Vec<&str> = Vec::new();
        let parent = inside.and_then(|(i, around)| i.parent.as_ref().map(|p| (p, around)));
        if let Some((parent, around)) = parent {
            seen.push(&parent.field.name);
            match over.field(&parent.field.name) {
                None => self.out.push(Diagnostic::error(format!("{} has no field '{}'", over.name.name, parent.field.name), parent.field.span)),
                Some(field) if !matches!(&field.ty, FieldType::Ref { entity } if entity.name == around.name.name) => self.out.push(Diagnostic::error(
                    format!("'is \"..\"' names the {} a {} sits in: field '{}' is {}, not ref {}", around.name.name, over.name.name, field.name.name, field.type_text, around.name.name),
                    parent.field.span,
                )),
                Some(_) => {}
            }
            if parent.path.value != ".." {
                self.out.push(Diagnostic::error(format!("'is' names the record around, written \"..\", as in {} is \"..\"", parent.field.name), parent.path.span));
            }
        }
        for mapped in fields {
            let name = mapped.field.name.as_str();
            if seen.contains(&name) {
                self.out.push(Diagnostic::error(format!("field '{name}' is mapped twice"), mapped.field.span));
            }
            seen.push(name);
            let Some(field) = over.field(name) else {
                self.out.push(Diagnostic::error(format!("{} has no field '{name}'", over.name.name), mapped.field.span));
                continue;
            };
            if let Some(at) = &mapped.at {
                self.path(at, mapped.inside.is_none());
            }
            let Some(inner) = &mapped.inside else {
                if field.ty.is_held() {
                    let of = match &field.ty {
                        FieldType::List { item } => &item.name,
                        FieldType::Record { entity } => &entity.name,
                        _ => "",
                    };
                    self.out.push(Diagnostic::error(format!("field '{name}' holds records: write {name} of {of}, with their lines under it"), mapped.field.span));
                }
                continue;
            };
            if !self.format.nests {
                self.out.push(Diagnostic::error(format!("{} holds no records inside records: a mapping from {} has no '{name} of'", self.format.title, self.format.name), mapped.field.span));
                continue;
            }
            let holds = match &field.ty {
                FieldType::List { item } => Some(&item.name),
                FieldType::Record { entity } => Some(&entity.name),
                _ => None,
            };
            if holds != Some(&inner.entity.name) {
                let is = match holds {
                    Some(of) => format!("holds {of} records"),
                    None => format!("is {}", field.type_text),
                };
                self.out.push(Diagnostic::error(format!("field '{name}' {is}, not records of {}", inner.entity.name), inner.entity.span));
                continue;
            }
            let Some(entity) = (self.entity)(&inner.entity.name) else {
                self.out.push(Diagnostic::error(format!("unknown entity '{}'", inner.entity.name), inner.entity.span));
                continue;
            };
            self.fields(entity, &inner.fields, Some((inner, over)));
        }
    }

    /// A path as every format writes it: steps with slashes between, `@name`
    /// for an attribute as the last step where the format has them, `.` and
    /// `..`. A flat format's path is a column's name. `field` says whether it
    /// is a field's (which may name an attribute) or where records are.
    fn path(&mut self, path: &MappingText, field: bool) {
        let text = path.value.as_str();
        let title = self.format.title;
        if text.is_empty() {
            self.out.push(Diagnostic::error("a path names something: write the steps with slashes between, as in \"customer/name\"", path.span));
            return;
        }
        if self.format.path_separator.is_none() {
            if text.contains('/') || text.starts_with('@') || text == "." || text == ".." {
                self.out.push(Diagnostic::error(format!("{title} has columns only: a path in it is a column's name, not \"{text}\""), path.span));
            }
            return;
        }
        let steps: Vec<&str> = text.split('/').collect();
        if steps.iter().any(|s| s.is_empty()) {
            self.out.push(Diagnostic::error(format!("the path \"{text}\" has an empty step; write the steps with one slash between"), path.span));
            return;
        }
        for (i, step) in steps.iter().enumerate() {
            let Some(attribute) = step.strip_prefix('@') else { continue };
            if !has_attributes(self.format) {
                self.out.push(Diagnostic::error(format!("{title} has no attributes: '@{attribute}' in \"{text}\" names one"), path.span));
            } else if !field {
                self.out.push(Diagnostic::error(format!("records are elements, not an attribute: \"{text}\" ends at '@{attribute}'"), path.span));
            } else if i + 1 != steps.len() {
                self.out.push(Diagnostic::error(format!("an attribute is the last step of a path: \"{text}\" goes on after '@{attribute}'"), path.span));
            } else if attribute.is_empty() {
                self.out.push(Diagnostic::error("'@' names an attribute: write its name after it, as in \"@number\"", path.span));
            }
        }
    }

    /// `check "orders.xsd"`: a format with schemas, and a file of one of its
    /// kinds. Whether the file is in the project is checked where its folder is known.
    fn schema(&mut self, schema: &MappingText) {
        let kinds = self.format.schema_kinds;
        if kinds.is_empty() {
            let with: Vec<String> = clarity_formats::catalog().iter().filter(|f| !f.schema_kinds.is_empty()).map(|f| format!("{} ({})", f.name, f.schema_kinds.join(", "))).collect();
            self.out.push(Diagnostic::error(format!("{} has no schemas to check a file against; the formats with schemas are {}", self.format.title, with.join(", ")), schema.span));
            return;
        }
        let kind = schema.value.rsplit_once('.').map(|(_, k)| k.to_ascii_lowercase()).unwrap_or_default();
        if !kinds.contains(&kind.as_str()) {
            self.out.push(Diagnostic::error(format!("a schema of {} is a {} file, not \"{}\"", self.format.title, kinds.join(" or "), schema.value), schema.span));
        }
    }

    /// The format's own options: each one of its catalog entry, once, of its kind.
    fn options(&mut self, mapping: &MappingDecl) {
        for (i, option) in mapping.options.iter().enumerate() {
            let name = option.name.name.as_str();
            if mapping.options[..i].iter().any(|o| o.name.name == name) {
                self.out.push(Diagnostic::error(format!("the option '{name}' is given twice"), option.name.span));
            }
            let Some(declared) = self.format.options.iter().find(|o| o.name == name) else {
                let names: Vec<&str> = self.format.options.iter().map(|o| o.name).filter(|n| *n != "at").collect();
                let has = if names.is_empty() { "no options".to_string() } else { format!("the options {}", names.join(", ")) };
                self.out.push(Diagnostic::error(format!("{} has no option '{name}'; it has {has}", self.format.name), option.name.span));
                continue;
            };
            let fits = matches!((declared.kind, &option.value), (clarity_formats::OptionKind::Text, MappingValue::Text(_)) | (clarity_formats::OptionKind::Bool, MappingValue::Bool(_)));
            if !fits {
                let (wanted, given) = match declared.kind {
                    clarity_formats::OptionKind::Text => ("text", "true or false"),
                    clarity_formats::OptionKind::Bool => ("true or false", "text"),
                };
                self.out.push(Diagnostic::error(format!("the option '{name}' of {} is {wanted}, not {given}", self.format.name), option.span));
            }
        }
    }
}

/// Whether `mapping` gives option `name`: the generic `at`, or one of its format's.
pub fn gives(mapping: &MappingDecl, name: &str) -> bool {
    (name == "at" && mapping.at.is_some()) || mapping.options.iter().any(|o| o.name.name == name)
}

/// The files the mappings of the project check against (`check "orders.xsd"`)
/// that `exists` says are missing, each with where it is named.
pub(crate) fn missing_schemas(project: &crate::model::Project, exists: &dyn Fn(&str) -> bool) -> Vec<Diagnostic> {
    project
        .files
        .iter()
        .flat_map(|f| f.mappings.iter().map(move |m| (f, m)))
        .filter_map(|(file, mapping)| {
            let schema = mapping.check.as_ref()?;
            (!exists(&schema.value)).then(|| {
                Diagnostic::error(format!("mapping {} checks against \"{}\", which is not in the project's folder", mapping.name.name, schema.value), schema.span).in_file(&file.path)
            })
        })
        .collect()
}

/// Gives each mapping with a `check` the text of its schema file, which
/// `read` reads (none when it is not there), for the generator to carry
/// in the program.
pub(crate) fn attach_schemas(project: &mut crate::model::Project, read: &dyn Fn(&str) -> Option<String>) {
    for mapping in project.files.iter_mut().flat_map(|f| f.mappings.iter_mut()) {
        mapping.schema = mapping.check.as_ref().and_then(|c| read(&c.value));
    }
}

fn capitalized(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|c| c.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
}
