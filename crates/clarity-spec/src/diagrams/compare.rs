//! A written diagram compared with the project: what it would add, change
//! or leave out, as proposals, each with the ordinary edit that makes it.
//! Apply turns the ticked ones into one batch of edits per module, with the
//! texts they write keyed, as any designer edit.
//!
//! Only what the diagram draws is compared: an entity drawn without fields
//! says nothing about its fields, and what the diagram does not name at all
//! is never proposed for removal. Removals start unticked.

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

use crate::dictionary::{FieldSpec, field_line};
use crate::edit::{Edit, EditError, apply_in};
use crate::model::{Entity, EnumDecl, Modifiers, Project, SpecFile, default_label, snake_case};
use crate::span::Span;
use crate::templates::plural;

use super::{Diagram, DiagramField, Relation};

/// What a proposal does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Change {
    Add,
    Change,
    Remove,
}

/// One change a diagram proposes, to tick or leave.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Proposal {
    pub change: Change,
    /// What it is about: `Order`, `Customer.email`, `OrderStatus.Shipped`.
    pub subject: String,
    /// What it would write, in Clarity's words.
    pub description: String,
    /// The module file it changes.
    pub file: String,
    /// Whether it starts ticked: everything but removals.
    pub ticked: bool,
    /// Its line in the diagram; a removal's is the drawn entity's or enum's.
    pub span: Span,
    #[serde(skip)]
    #[ts(skip)]
    pub edit: Edit,
    /// Modules the file must use for what the edit writes.
    #[serde(skip)]
    #[ts(skip)]
    pub uses: Vec<String>,
}

/// A module's new text after Apply, and the keys its new texts became.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleText {
    pub path: String,
    pub text: String,
    pub keys: Vec<(String, String)>,
}

/// What the project declares, by name.
struct Known<'a> {
    project: &'a Project,
    entities: BTreeMap<&'a str, (&'a SpecFile, &'a Entity)>,
    enums: BTreeMap<&'a str, (&'a SpecFile, &'a EnumDecl)>,
}

impl<'a> Known<'a> {
    fn new(project: &'a Project) -> Self {
        let mut entities = BTreeMap::new();
        let mut enums = BTreeMap::new();
        for file in &project.files {
            for entity in &file.entities {
                entities.insert(entity.name.name.as_str(), (file, entity));
            }
            for decl in &file.enums {
                enums.insert(decl.name.name.as_str(), (file, decl));
            }
        }
        Known { project, entities, enums }
    }

    fn module_file(&self, module: &str) -> Option<&'a SpecFile> {
        self.project.files.iter().find(|f| f.module.as_ref().is_some_and(|m| m.name == module))
    }

    fn first_module(&self) -> Option<&'a SpecFile> {
        self.project.files.iter().find(|f| f.module.is_some())
    }

    fn module_of(&self, name: &str) -> Option<&'a SpecFile> {
        self.entities.get(name).map(|(f, _)| *f).or_else(|| self.enums.get(name).map(|(f, _)| *f))
    }
}

/// What a diagram proposes for the project, in the order the diagram draws it.
pub fn propose(project: &Project, diagram: &Diagram) -> Vec<Proposal> {
    let known = Known::new(project);
    let is_enum = |name: &str| known.enums.contains_key(name) || diagram.enums.iter().any(|e| e.name == name);
    let is_entity = |name: &str| known.entities.contains_key(name) || diagram.entities.iter().any(|e| e.name == name);

    // Fields the relations imply, per entity.
    let mut implied: BTreeMap<&str, Vec<DiagramField>> = BTreeMap::new();
    for relation in &diagram.relations {
        for (owner, field) in relation_fields(relation, &is_enum, &is_entity) {
            implied.entry(owner).or_default().push(field);
        }
    }

    let mut groups: Vec<(u32, Vec<Proposal>)> = Vec::new();
    for entity in &diagram.entities {
        let mut fields: Vec<DiagramField> = entity.fields.iter().map(|f| with_refs(f.clone(), &is_entity, &is_enum)).collect();
        for field in implied.get(entity.name.as_str()).into_iter().flatten() {
            let drawn = fields.iter().any(|f| f.name == field.name || f.type_text == field.type_text);
            if !drawn {
                fields.push(field.clone());
            }
        }
        let proposals = match known.entities.get(entity.name.as_str()) {
            Some((file, existing)) => {
                compare_entity(&entity.name, &fields, !entity.fields.is_empty(), file, existing, entity.span, &known)
            }
            None => {
                let related = diagram.relations.iter().filter_map(|r| {
                    if r.left == entity.name {
                        Some(r.right.as_str())
                    } else if r.right == entity.name {
                        Some(r.left.as_str())
                    } else {
                        None
                    }
                });
                let by_type = fields.iter().filter_map(|f| f.type_text.strip_prefix("ref ").or(Some(f.type_text.as_str())));
                let target = entity
                    .module
                    .as_deref()
                    .and_then(|m| known.module_file(m))
                    .or_else(|| related.chain(by_type).find_map(|n| known.module_of(n)))
                    .or_else(|| known.first_module());
                target.map(|file| vec![new_entity(&entity.name, &fields, file, entity.span, &known)]).unwrap_or_default()
            }
        };
        groups.push((entity.span.line, proposals));
    }
    for drawn in &diagram.enums {
        let values: Vec<&str> = drawn.values.iter().map(|v| v.name.as_str()).collect();
        let proposals = match known.enums.get(drawn.name.as_str()) {
            Some((file, decl)) => {
                let mut out = Vec::new();
                for value in &drawn.values {
                    if !decl.variants.iter().any(|v| v.name.name == value.name) {
                        out.push(proposal(
                            Change::Add,
                            format!("{}.{}", drawn.name, value.name),
                            value.name.clone(),
                            file,
                            value.span,
                            Edit::AddVariant { owner: decl.span.start, name: value.name.clone(), label: Some(default_label(&value.name)) },
                        ));
                    }
                }
                if !values.is_empty() {
                    for variant in decl.variants.iter().filter(|v| !values.contains(&v.name.name.as_str())) {
                        out.push(proposal(
                            Change::Remove,
                            format!("{}.{}", drawn.name, variant.name.name),
                            format!("remove {}", variant.name.name),
                            file,
                            drawn.span,
                            Edit::Remove { item: variant.name.span.start },
                        ));
                    }
                }
                out
            }
            None => {
                let users = diagram.entities.iter().filter(|e| e.fields.iter().any(|f| f.type_text == drawn.name)).map(|e| e.name.as_str());
                let target = drawn
                    .module
                    .as_deref()
                    .and_then(|m| known.module_file(m))
                    .or_else(|| users.into_iter().find_map(|n| known.module_of(n)))
                    .or_else(|| known.first_module());
                let mut text = format!("enum {}", drawn.name);
                for value in &values {
                    text.push_str(&format!("\n    {value} label {}", crate::lexer::text_token(&default_label(value))));
                }
                target
                    .map(|file| {
                        vec![proposal(
                            Change::Add,
                            drawn.name.clone(),
                            format!("enum {} in module {}: {}", drawn.name, module_name(file), values.join(", ")),
                            file,
                            drawn.span,
                            Edit::AppendDecl { text },
                        )]
                    })
                    .unwrap_or_default()
            }
        };
        groups.push((drawn.span.line, proposals));
    }
    groups.sort_by_key(|(line, _)| *line);
    groups.into_iter().flat_map(|(_, p)| p).collect()
}

/// The fields a relation gives its ends: a ref on the side that refers, a
/// list on the side that lists, or a field of the enum's type.
fn relation_fields<'r>(
    relation: &'r Relation,
    is_enum: &dyn Fn(&str) -> bool,
    is_entity: &dyn Fn(&str) -> bool,
) -> Vec<(&'r str, DiagramField)> {
    let field = |name: String, type_text: String, owned: bool| DiagramField {
        name,
        type_text,
        modifiers: Modifiers { owned, ..Modifiers::default() },
        span: relation.span,
    };
    let (left, right) = (relation.left.as_str(), relation.right.as_str());
    match (is_enum(left), is_enum(right)) {
        (false, true) if is_entity(left) => return vec![(left, field(snake_case(right), right.to_string(), false))],
        (true, false) if is_entity(right) => return vec![(right, field(snake_case(left), left.to_string(), false))],
        (false, false) => {}
        _ => return Vec::new(),
    }
    let (many, one) = match (relation.left_count.is_many(), relation.right_count.is_many()) {
        (true, false) => (left, right),
        (false, true) => (right, left),
        (false, false) => (left, right),
        // Many on both sides needs an entity between them.
        (true, true) => return Vec::new(),
    };
    let mut out = vec![(many, field(snake_case(one), format!("ref {one}"), false))];
    if relation.lists && (relation.left_count.is_many() || relation.right_count.is_many()) {
        out.push((one, field(plural(&snake_case(many)), format!("list<{many}>"), relation.identifying)));
    }
    out
}

/// A class diagram's field typed with an entity's name is a ref to it.
fn with_refs(mut field: DiagramField, is_entity: &dyn Fn(&str) -> bool, is_enum: &dyn Fn(&str) -> bool) -> DiagramField {
    if field.type_text.starts_with(|c: char| c.is_uppercase()) && is_entity(&field.type_text) && !is_enum(&field.type_text) {
        field.type_text = format!("ref {}", field.type_text);
    }
    field
}

fn module_name(file: &SpecFile) -> &str {
    file.module.as_ref().map_or("", |m| m.name.as_str())
}

fn proposal(change: Change, subject: String, description: String, file: &SpecFile, span: Span, edit: Edit) -> Proposal {
    Proposal { change, subject, description, file: file.path.clone(), ticked: change != Change::Remove, span, edit, uses: Vec::new() }
}

/// The entity a referred or listed type names: `ref Customer`, `list<Order>`.
fn named_entity(type_text: &str) -> Option<&str> {
    type_text.strip_prefix("ref ").or_else(|| type_text.strip_prefix("list<").and_then(|t| t.strip_suffix('>'))).map(str::trim)
}

/// The modules `file` must use for fields of these types.
fn uses_for<'t>(types: impl IntoIterator<Item = &'t str>, file: &SpecFile, known: &Known) -> Vec<String> {
    let mut out = Vec::new();
    for ty in types {
        let name = named_entity(ty).unwrap_or(ty);
        let Some(other) = known.module_of(name) else { continue };
        let module = module_name(other);
        if other.path != file.path && !module.is_empty() && !file.uses.iter().any(|u| u.name == module) && !out.iter().any(|m| m == module) {
            out.push(module.to_string());
        }
    }
    out
}

/// The flags a diagram can say, by name.
fn flags(m: &Modifiers) -> [(bool, &'static str); 4] {
    [(m.required, "required"), (m.unique, "unique"), (m.owned, "owned"), (m.computed, "computed")]
}

/// A field written with its default label.
fn spec_of(field: &DiagramField) -> FieldSpec {
    let mut modifiers = field.modifiers.clone();
    modifiers.label.get_or_insert_with(|| default_label(&field.name));
    FieldSpec { name: field.name.clone(), type_text: field.type_text.clone(), modifiers }
}

/// A field as a proposal describes it: without its label.
fn described(field: &DiagramField) -> String {
    let mut spec = spec_of(field);
    spec.modifiers.label = None;
    field_line(&spec)
}

fn compare_entity(
    name: &str,
    fields: &[DiagramField],
    draws_fields: bool,
    file: &SpecFile,
    existing: &Entity,
    span: Span,
    known: &Known,
) -> Vec<Proposal> {
    let mut out = Vec::new();
    for field in fields {
        let subject = format!("{name}.{}", field.name);
        // A ref or list a relationship implies is the project's field of that type, whatever its name.
        let have = existing.fields.iter().find(|f| f.name.name == field.name).or_else(|| {
            named_entity(&field.type_text).and_then(|_| existing.fields.iter().find(|f| f.type_text == field.type_text))
        });
        match have {
            Some(have) => {
                let mut changes = Vec::new();
                let type_changed = have.type_text != field.type_text;
                for ((now, word), (before, _)) in flags(&field.modifiers).into_iter().zip(flags(&have.modifiers)) {
                    if now != before {
                        changes.push(if now { word.to_string() } else { format!("no longer {word}") });
                    }
                }
                if !type_changed && changes.is_empty() {
                    continue;
                }
                let mut modifiers = have.modifiers.clone();
                modifiers.required = field.modifiers.required;
                modifiers.unique = field.modifiers.unique;
                modifiers.owned = field.modifiers.owned;
                modifiers.computed = field.modifiers.computed;
                let mut description = format!("{}: {}", field.name, field.type_text);
                if type_changed {
                    description.push_str(&format!(" instead of {}", have.type_text));
                }
                for change in &changes {
                    description.push_str(&format!(", {change}"));
                }
                let spec = FieldSpec { name: field.name.clone(), type_text: field.type_text.clone(), modifiers };
                let mut p = proposal(Change::Change, subject, description, file, field.span, Edit::UpdateField { item: have.span.start, spec });
                p.uses = uses_for([field.type_text.as_str()], file, known);
                out.push(p);
            }
            None => {
                let mut p = proposal(
                    Change::Add,
                    subject,
                    described(field),
                    file,
                    field.span,
                    Edit::AddField { entity: existing.span.start, spec: spec_of(field) },
                );
                p.uses = uses_for([field.type_text.as_str()], file, known);
                out.push(p);
            }
        }
    }
    if draws_fields {
        for have in &existing.fields {
            let covered = fields.iter().any(|f| f.name == have.name.name || (named_entity(&f.type_text).is_some() && f.type_text == have.type_text));
            if !covered {
                out.push(proposal(
                    Change::Remove,
                    format!("{name}.{}", have.name.name),
                    format!("remove {}: {}", have.name.name, have.type_text),
                    file,
                    span,
                    Edit::Remove { item: have.span.start },
                ));
            }
        }
    }
    out
}

fn new_entity(name: &str, fields: &[DiagramField], file: &SpecFile, span: Span, known: &Known) -> Proposal {
    let mut text = format!("entity {name}");
    for field in fields {
        text.push_str(&format!("\n    {}", field_line(&spec_of(field))));
    }
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    let description = format!("entity {name} in module {}: {}", module_name(file), names.join(", "));
    let mut p = proposal(Change::Add, name.to_string(), description, file, span, Edit::AppendDecl { text });
    p.uses = uses_for(fields.iter().map(|f| f.type_text.as_str()), file, known);
    p
}

/// Applies the proposals at `ticked`: one batch of edits per module, in the
/// order the modules first come, each checked and keyed as a designer edit.
/// `sources` are the modules' texts by path.
pub fn apply(project: &Project, sources: &[(String, String)], proposals: &[Proposal], ticked: &[usize]) -> Result<Vec<ModuleText>, EditError> {
    let mut by_file: Vec<(&str, Vec<&Proposal>)> = Vec::new();
    for proposal in ticked.iter().filter_map(|i| proposals.get(*i)) {
        match by_file.iter_mut().find(|(path, _)| *path == proposal.file) {
            Some((_, list)) => list.push(proposal),
            None => by_file.push((proposal.file.as_str(), vec![proposal])),
        }
    }
    let mut out = Vec::new();
    for (path, list) in by_file {
        let source = sources
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, text)| text)
            .ok_or_else(|| EditError(format!("{path} is not a module of the project")))?;
        let mut edits: Vec<Edit> = list.iter().map(|p| p.edit.clone()).collect();
        let mut uses: Vec<String> = Vec::new();
        for module in list.iter().flat_map(|p| &p.uses) {
            if !uses.contains(module) {
                uses.push(module.clone());
            }
        }
        if !uses.is_empty() {
            let file = project.files.iter().find(|f| f.path == path);
            let mut all: Vec<String> = file.map(|f| f.uses.iter().map(|u| u.name.clone()).collect()).unwrap_or_default();
            all.extend(uses);
            edits.push(Edit::SetUses { uses: all });
        }
        let outcome = apply_in(project, source, &Edit::Batch { edits })?;
        out.push(ModuleText { path: path.to_string(), text: outcome.text, keys: outcome.keys });
    }
    Ok(out)
}
