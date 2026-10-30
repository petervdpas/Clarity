//! The class diagram designer's model: boxes for entities, enums, engines
//! and modules, each with its place, its namespace (the module it goes to)
//! and its members, and relations as association, composition or
//! aggregation. Read from Mermaid text and written back as plain Mermaid,
//! the places in `%% clarity:` lines as the ERD's are. Writing what was
//! read keeps its meaning; the text comes back in Clarity's own words.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::diagnostic::Diagnostic;

use super::erd_model::{ErdField, PLACE, bend_line, bends, field, places, take_bend};
use super::{Cardinality, DiagramKind, parse};

/// What the class diagram designer edits.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ClassDesign {
    pub boxes: Vec<ClassBox>,
    pub relations: Vec<ClassRelation>,
}

/// What a box stands for: its stereotype in Mermaid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ClassBoxKind {
    Entity,
    Enum,
    Engine,
    Module,
}

/// A box on the canvas. An entity has fields, an enum values, an engine
/// and a module functions, each written as Clarity writes a signature:
/// `play(square: integer) -> text`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ClassBox {
    pub name: String,
    pub kind: ClassBoxKind,
    /// The module an entity, enum or engine goes to: its namespace.
    pub module: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub fields: Vec<ErdField>,
    pub values: Vec<String>,
    pub functions: Vec<String>,
}

/// How a relation is drawn and what it makes: an association a ref on the
/// left box, a composition an owned list on the left box, an aggregation
/// a plain list. Uses (`..>`) makes nothing: the left box's code uses the
/// right one, and the code says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ClassLink {
    Association,
    Composition,
    Aggregation,
    Uses,
}

/// A relation between two boxes. The left box refers (an association) or
/// is the whole (a composition or aggregation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ClassRelation {
    pub left: String,
    pub right: String,
    pub link: ClassLink,
    #[serde(rename = "leftCount")]
    #[ts(rename = "leftCount")]
    pub left_count: Cardinality,
    #[serde(rename = "rightCount")]
    #[ts(rename = "rightCount")]
    pub right_count: Cardinality,
    pub label: Option<String>,
    /// The points the line bends at, in order from left to right; none: straight.
    #[serde(default)]
    pub via: Vec<(i32, i32)>,
}

/// A class diagram as the designer opens it, with the problems of its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ClassRead {
    pub design: ClassDesign,
    pub problems: Vec<Diagnostic>,
}

fn new_box(name: String, kind: ClassBoxKind, module: Option<String>) -> ClassBox {
    ClassBox { name, kind, module, x: None, y: None, fields: Vec::new(), values: Vec::new(), functions: Vec::new() }
}

/// Reads a class diagram for the designer, with the problems of its text.
/// Boxes come in the order the text first names them.
pub fn read(text: &str) -> (ClassDesign, Vec<Diagnostic>) {
    let (diagram, problems) = parse(text);
    if diagram.kind != Some(DiagramKind::Class) {
        return (ClassDesign::default(), problems);
    }
    let mut found: Vec<(u32, ClassBox)> = Vec::new();
    for e in diagram.entities {
        let mut b = new_box(e.name, ClassBoxKind::Entity, e.module);
        b.fields = e.fields.iter().map(field).collect();
        found.push((e.span.start, b));
    }
    for e in diagram.enums {
        let mut b = new_box(e.name, ClassBoxKind::Enum, e.module);
        b.values = e.values.into_iter().map(|v| v.name).collect();
        found.push((e.span.start, b));
    }
    for e in diagram.engines {
        let mut b = new_box(e.name, ClassBoxKind::Engine, e.module);
        b.functions = e.functions.iter().map(|f| f.signature()).collect();
        found.push((e.span.start, b));
    }
    for m in diagram.modules {
        let mut b = new_box(m.name, ClassBoxKind::Module, None);
        b.functions = m.functions.iter().map(|f| f.signature()).collect();
        found.push((m.span.start, b));
    }
    found.sort_by_key(|(start, _)| *start);
    let places = places(text);
    let boxes = found
        .into_iter()
        .map(|(_, mut b)| {
            if let Some((_, x, y)) = places.iter().find(|(name, _, _)| *name == b.name) {
                b.x = Some(*x);
                b.y = Some(*y);
            }
            b
        })
        .collect();
    let uses = diagram.uses.into_iter().map(|u| ClassRelation {
        left: u.from,
        right: u.to,
        link: ClassLink::Uses,
        left_count: Cardinality::ExactlyOne,
        right_count: Cardinality::ExactlyOne,
        label: None,
        via: Vec::new(),
    });
    let relations = diagram
        .relations
        .into_iter()
        .map(|r| ClassRelation {
            link: match (r.lists, r.identifying) {
                (true, true) => ClassLink::Composition,
                (true, false) => ClassLink::Aggregation,
                (false, _) => ClassLink::Association,
            },
            left: r.left,
            right: r.right,
            left_count: r.left_count,
            right_count: r.right_count,
            label: r.label,
            via: Vec::new(),
        })
        .chain(uses)
        .collect();
    let mut design = ClassDesign { boxes, relations };
    let mut bent = bends(text);
    for r in &mut design.relations {
        r.via = take_bend(&mut bent, &r.left, &r.right, r.label.as_deref());
    }
    (design, problems)
}

/// Writes the designer's class diagram as Mermaid: the boxes, those of a
/// module inside its namespace, then the relations, then the places.
pub fn write(design: &ClassDesign) -> String {
    let mut out = String::from("classDiagram\n");
    let mut written: Vec<&str> = Vec::new();
    for b in &design.boxes {
        match b.module.as_deref() {
            Some(module) if b.kind != ClassBoxKind::Module => {
                if written.contains(&module) {
                    continue;
                }
                written.push(module);
                out.push_str(&format!("    namespace {module} {{\n"));
                for inside in design.boxes.iter().filter(|o| o.kind != ClassBoxKind::Module && o.module.as_deref() == Some(module)) {
                    class(&mut out, inside, "        ");
                }
                out.push_str("    }\n");
            }
            _ => class(&mut out, b, "    "),
        }
    }
    for r in &design.relations {
        let arrow = match r.link {
            ClassLink::Association => "-->",
            ClassLink::Composition => "*--",
            ClassLink::Aggregation => "o--",
            ClassLink::Uses => {
                out.push_str(&format!("    {} ..> {}\n", r.left, r.right));
                continue;
            }
        };
        let label = match r.label.as_deref().map(str::trim) {
            Some(label) if !label.is_empty() => format!(" : {label}"),
            _ => String::new(),
        };
        out.push_str(&format!(
            "    {} \"{}\" {arrow} \"{}\" {}{label}\n",
            r.left,
            multiplicity(r.left_count),
            multiplicity(r.right_count),
            r.right
        ));
    }
    for b in &design.boxes {
        if let (Some(x), Some(y)) = (b.x, b.y) {
            out.push_str(&format!("    {PLACE}{} at {x},{y}\n", b.name));
        }
    }
    for r in design.relations.iter().filter(|r| !r.via.is_empty()) {
        out.push_str(&bend_line(&r.left, &r.right, r.label.as_deref(), &r.via));
    }
    out
}

fn multiplicity(count: Cardinality) -> &'static str {
    match count {
        Cardinality::ExactlyOne => "1",
        Cardinality::ZeroOrOne => "0..1",
        Cardinality::ZeroOrMore => "*",
        Cardinality::OneOrMore => "1..*",
    }
}

/// A class with its stereotype and members; `class Name` alone when it has none.
fn class(out: &mut String, b: &ClassBox, indent: &str) {
    let stereotype = match b.kind {
        ClassBoxKind::Entity => None,
        ClassBoxKind::Enum => Some("enumeration"),
        ClassBoxKind::Engine => Some("engine"),
        ClassBoxKind::Module => Some("module"),
    };
    let mut members: Vec<String> = Vec::new();
    if let Some(word) = stereotype {
        members.push(format!("<<{word}>>"));
    }
    members.extend(b.fields.iter().map(member));
    members.extend(b.values.iter().cloned());
    members.extend(b.functions.iter().map(|f| method(f)));
    if members.is_empty() {
        out.push_str(&format!("{indent}class {}\n", b.name));
        return;
    }
    out.push_str(&format!("{indent}class {} {{\n", b.name));
    for m in members {
        out.push_str(&format!("{indent}    {m}\n"));
    }
    out.push_str(&format!("{indent}}}\n"));
}

/// A field as a member: `+name: text(80), required, min 1`.
fn member(f: &ErdField) -> String {
    let mut parts = vec![f.type_text.clone()];
    if f.required {
        parts.push("required".into());
    }
    if f.unique {
        parts.push("unique".into());
    }
    if !f.extra.trim().is_empty() {
        parts.push(f.extra.trim().to_string());
    }
    format!("+{}: {}", f.name, parts.join(", "))
}

/// A signature as a method: `play(square: integer) -> text` is written
/// `+play(square: integer) text`, Mermaid's way of giving the result.
fn method(signature: &str) -> String {
    let signature = signature.trim();
    match signature.rsplit_once("->") {
        Some((call, result)) => format!("+{} {}", call.trim(), result.trim()),
        None => format!("+{signature}"),
    }
}
