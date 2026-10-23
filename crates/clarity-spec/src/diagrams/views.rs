//! Views generated from the project, always true: Mermaid text written
//! from the data dictionary, each line linked to what it shows so a click
//! on the drawing opens it. A view read back proposes nothing.

use serde::Serialize;
use ts_rs::TS;

use crate::code::{ExprKind, Param, TypeRef, TypeRefKind, each_expr_in};
use crate::model::{Entity, Field, FieldType, Project};

use super::class_model::{ClassBox, ClassBoxKind, ClassDesign, ClassLink, ClassRelation};
use super::{Cardinality, DiagramField, DiagramKind};
use crate::span::Span;

/// A view: its Mermaid text, and what its lines show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct View {
    pub text: String,
    pub links: Vec<Link>,
}

/// A line of a view and the declaration it shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Link {
    /// The line in the view's text, from 1.
    pub line: u32,
    /// The module file of the declaration.
    pub path: String,
    pub span: Span,
}

struct Writer {
    text: String,
    links: Vec<Link>,
    line: u32,
}

impl Writer {
    fn line(&mut self, text: &str, shows: Option<(&str, Span)>) {
        self.line += 1;
        self.text.push_str(text);
        self.text.push('\n');
        if let Some((path, span)) = shows {
            self.links.push(Link { line: self.line, path: path.to_string(), span });
        }
    }
}

/// The Data model with the places of its boxes: a `%% clarity:` line for
/// each entity it draws that has a place.
pub fn data_model_placed(project: &Project, only: Option<&[String]>, places: &[(String, i32, i32)]) -> View {
    let mut view = data_model_of(project, only);
    // The settings are kept per person, not data of the project.
    for entity in project.files.iter().flat_map(|f| &f.entities).filter(|e| !e.settings && drawn(only, &e.name.name)) {
        if let Some((_, x, y)) = places.iter().find(|(name, _, _)| *name == entity.name.name) {
            view.text.push_str(&format!("    {}{} at {x},{y}\n", super::erd_model::PLACE, entity.name.name));
        }
    }
    view
}

/// Whether `name` is drawn: every entity when `only` is None.
fn drawn(only: Option<&[String]>, name: &str) -> bool {
    only.is_none_or(|names| names.iter().any(|n| n == name))
}

/// The Data model: every entity with its fields, and a relationship for
/// every list and for every ref no list stands for.
pub fn data_model(project: &Project) -> View {
    data_model_of(project, None)
}

/// The Data model of the entities `only` names (all when None), with the
/// relationships between them.
pub fn data_model_of(project: &Project, only: Option<&[String]>) -> View {
    let mut out = Writer { text: String::new(), links: Vec::new(), line: 0 };
    out.line("erDiagram", None);
    let entities: Vec<(&str, &Entity)> = project
        .files
        .iter()
        .flat_map(|f| f.entities.iter().filter(|e| !e.settings).map(move |e| (f.path.as_str(), e)))
        .filter(|(_, e)| drawn(only, &e.name.name))
        .collect();
    for (path, entity) in &entities {
        out.line(&format!("    {} {{", entity.name.name), Some((path, entity.span)));
        for field in &entity.fields {
            out.line(&format!("        {}", attribute(field)), Some((path, field.span)));
        }
        out.line("    }", None);
    }
    let shown = |name: &str| entities.iter().any(|(_, e)| e.name.name == name);
    for (path, entity) in &entities {
        for field in &entity.fields {
            match &field.ty {
                FieldType::List { item } if shown(&item.name) => {
                    let line = if field.modifiers.owned { "--" } else { ".." };
                    out.line(&format!("    {} ||{line}o{{ {} : {}", entity.name.name, item.name, field.name.name), Some((path, field.span)));
                }
                FieldType::Ref { entity: other } if shown(&other.name) && !listed(&entities, &other.name, &entity.name.name) => {
                    let one = if field.modifiers.required { "||" } else { "o|" };
                    out.line(&format!("    {} }}o..{one} {} : {}", entity.name.name, other.name, field.name.name), Some((path, field.span)));
                }
                _ => {}
            }
        }
    }
    View { text: out.text, links: out.links }
}

/// Whether entity `one` lists the records of `many`, so a list stands for the ref back.
fn listed(entities: &[(&str, &Entity)], one: &str, many: &str) -> bool {
    entities.iter().any(|(_, e)| {
        e.name.name == one && e.fields.iter().any(|f| matches!(&f.ty, FieldType::List { item } if item.name == many))
    })
}

/// The project as a class diagram of the boxes `only` names (all when
/// None): each module's enums, entities and engines in its namespace, and
/// a `<<module>>` box for its functions and actions. A ref or list between
/// two drawn entities is a line (an association, a composition when the
/// list is owned, an aggregation when not); one to an entity left out
/// stays a field.
pub fn classes_of(project: &Project, only: Option<&[String]>) -> ClassDesign {
    let mut design = ClassDesign::default();
    let mut used: Vec<(String, Vec<String>)> = Vec::new();
    let entities: Vec<(&str, &Entity)> = project
        .files
        .iter()
        .flat_map(|f| f.entities.iter().map(move |e| (f.path.as_str(), e)))
        .filter(|(_, e)| !e.settings && drawn(only, &e.name.name))
        .collect();
    let shown = |name: &str| entities.iter().any(|(_, e)| e.name.name == name);
    for file in &project.files {
        let module = file.module.as_ref().map(|m| m.name.clone());
        for e in file.enums.iter().filter(|e| drawn(only, &e.name.name)) {
            let mut b = class_box(&e.name.name, ClassBoxKind::Enum, module.clone());
            b.values = e.variants.iter().map(|v| v.name.name.clone()).collect();
            design.boxes.push(b);
        }
        for entity in file.entities.iter().filter(|e| !e.settings && drawn(only, &e.name.name)) {
            let mut b = class_box(&entity.name.name, ClassBoxKind::Entity, module.clone());
            for field in &entity.fields {
                match &field.ty {
                    FieldType::List { item } if shown(&item.name) => design.relations.push(ClassRelation {
                        left: entity.name.name.clone(),
                        right: item.name.clone(),
                        link: if field.modifiers.owned { ClassLink::Composition } else { ClassLink::Aggregation },
                        left_count: Cardinality::ExactlyOne,
                        right_count: Cardinality::ZeroOrMore,
                        label: Some(field.name.name.clone()),
                    }),
                    FieldType::Ref { entity: other } if shown(&other.name) => {
                        if !listed(&entities, &other.name, &entity.name.name) {
                            design.relations.push(ClassRelation {
                                left: entity.name.name.clone(),
                                right: other.name.clone(),
                                link: ClassLink::Association,
                                left_count: Cardinality::ZeroOrMore,
                                right_count: if field.modifiers.required { Cardinality::ExactlyOne } else { Cardinality::ZeroOrOne },
                                label: Some(field.name.name.clone()),
                            });
                        }
                    }
                    _ => b.fields.push(super::erd_model::field(&DiagramField {
                        name: field.name.name.clone(),
                        type_text: field.type_text.clone(),
                        modifiers: field.modifiers.clone(),
                        span: field.span,
                    })),
                }
            }
            design.boxes.push(b);
        }
        for engine in file.engines.iter().filter(|e| drawn(only, &e.name.name)) {
            let mut b = class_box(&engine.name.name, ClassBoxKind::Engine, module.clone());
            b.functions = engine.functions.iter().map(|f| signature(&f.name.name, &f.params, f.returns.as_ref())).collect();
            design.boxes.push(b);
        }
        if let Some(name) = &module
            && !file.functions.is_empty()
            && drawn(only, name)
        {
            let mut b = class_box(name, ClassBoxKind::Module, None);
            b.functions = file.functions.iter().map(|f| signature(&f.name.name, &f.params, f.returns.as_ref())).collect();
            design.boxes.push(b);
            used.push((name.clone(), uses_of(&file.functions)));
        }
    }
    // A uses arrow from each module to what its functions use that is drawn.
    for (module, names) in used {
        for name in names {
            if name != module && design.boxes.iter().any(|b| b.name == name) {
                design.relations.push(ClassRelation {
                    left: module.clone(),
                    right: name,
                    link: ClassLink::Uses,
                    left_count: Cardinality::ExactlyOne,
                    right_count: Cardinality::ExactlyOne,
                    label: None,
                });
            }
        }
    }
    design
}

/// What functions and actions use, each once in the order first met: the
/// entities and enums of their parameters and results, the engines they
/// call, and the entities they find and make.
fn uses_of(functions: &[crate::code::FunctionDecl]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut note = |name: &str| {
        if !names.iter().any(|n| n == name) {
            names.push(name.to_string());
        }
    };
    for f in functions {
        for ty in f.params.iter().map(|p| &p.ty).chain(f.returns.as_ref()) {
            match &ty.kind {
                TypeRefKind::Named { name } => note(name),
                TypeRefKind::List { item } => note(item),
                _ => {}
            }
        }
        each_expr_in(&f.body, &mut |expr| match &expr.kind {
            ExprKind::EngineCall { engine, .. } => note(&engine.name),
            ExprKind::Find { entity, .. } | ExprKind::New { entity } => note(&entity.name),
            _ => {}
        });
    }
    names
}

fn class_box(name: &str, kind: ClassBoxKind, module: Option<String>) -> ClassBox {
    ClassBox { name: name.to_string(), kind, module, x: None, y: None, fields: Vec::new(), values: Vec::new(), functions: Vec::new() }
}

/// A function's signature as Clarity writes it: `play(square: integer) -> text`.
fn signature(name: &str, params: &[Param], returns: Option<&TypeRef>) -> String {
    let params: Vec<String> = params.iter().map(|p| format!("{}: {}", p.name.name, type_word(&p.ty))).collect();
    match returns {
        Some(ty) => format!("{name}({}) -> {}", params.join(", "), type_word(ty)),
        None => format!("{name}({})", params.join(", ")),
    }
}

fn type_word(ty: &TypeRef) -> String {
    match &ty.kind {
        TypeRefKind::Text => "text".into(),
        TypeRefKind::Integer => "integer".into(),
        TypeRefKind::Decimal => "decimal".into(),
        TypeRefKind::Money => "money".into(),
        TypeRefKind::Bool => "bool".into(),
        TypeRefKind::Date => "date".into(),
        TypeRefKind::DateTime => "datetime".into(),
        TypeRefKind::Duration => "duration".into(),
        TypeRefKind::File => "file".into(),
        TypeRefKind::Bytes => "bytes".into(),
        TypeRefKind::Named { name } => name.clone(),
        TypeRefKind::List { item } => format!("list<{item}>"),
    }
}

/// An ERD or class diagram written again from the project, keeping the
/// places of its boxes: of every entity (and enum, engine and module), or
/// with `only_drawn` of those it draws that the project has. None for a
/// kind Clarity only draws.
pub fn generate(project: &Project, before: &str, only_drawn: bool) -> Option<String> {
    let places = super::erd_model::places(before);
    match super::kind_of(before)? {
        DiagramKind::Erd => {
            let names: Vec<String> = super::erd_model::read(before).0.entities.into_iter().map(|e| e.name).collect();
            Some(data_model_placed(project, only_drawn.then_some(names.as_slice()), &places).text)
        }
        DiagramKind::Class => {
            let names: Vec<String> = super::class_model::read(before).0.boxes.into_iter().map(|b| b.name).collect();
            let mut design = classes_of(project, only_drawn.then_some(names.as_slice()));
            for b in &mut design.boxes {
                if let Some((_, x, y)) = places.iter().find(|(name, _, _)| *name == b.name) {
                    b.x = Some(*x);
                    b.y = Some(*y);
                }
            }
            Some(super::class_model::write(&design))
        }
        _ => None,
    }
}

/// A field as an ERD attribute: `type name keys "comment"`, the comment
/// holding the modifiers, and the type where Mermaid's word cannot.
fn attribute(field: &Field) -> String {
    let m = &field.modifiers;
    let mut comment: Vec<String> = Vec::new();
    let (word, key) = match &field.ty {
        FieldType::Decimal { .. } => {
            comment.push(field.type_text.clone());
            ("decimal".to_string(), None)
        }
        FieldType::Ref { entity } => (entity.name.clone(), Some("FK")),
        FieldType::List { item } => (format!("{}[]", item.name), None),
        _ => (field.type_text.clone(), None),
    };
    let key = key.or(m.unique.then_some("UK"));
    for (on, word) in [(m.required, "required"), (m.computed, "computed"), (m.owned, "owned")] {
        if on {
            comment.push(word.to_string());
        }
    }
    let mut line = format!("{word} {}", field.name.name);
    if let Some(key) = key {
        line.push_str(&format!(" {key}"));
        if key == "FK" && m.unique {
            line.push_str(", UK");
        }
    }
    if !comment.is_empty() {
        line.push_str(&format!(" \"{}\"", comment.join(", ")));
    }
    line
}
