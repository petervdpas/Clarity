//! The ERD designer's model: what the canvas edits, read from Mermaid text
//! and written back as plain Mermaid. Each entity keeps its place on the
//! canvas in a `%% clarity: Customer at 120,80` line, a comment Mermaid
//! ignores, so the file still opens anywhere Mermaid does. Writing what
//! was read keeps its meaning; the text comes back in Clarity's own words.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::diagnostic::Diagnostic;
use crate::dictionary::{FieldSpec, field_line};

use super::{Cardinality, DiagramField, DiagramKind, parse};

/// What the ERD designer edits.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Erd {
    pub entities: Vec<ErdEntity>,
    pub relations: Vec<ErdRelation>,
}

/// An entity on the canvas: its place when it has one, and its fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ErdEntity {
    pub name: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub fields: Vec<ErdField>,
}

/// A field as the designer shows it: its type in Clarity's words, the two
/// flags it offers as boxes, and the other modifiers as Clarity writes them
/// (`min 1, default 1`, `owned`, `computed`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ErdField {
    pub name: String,
    #[serde(rename = "typeText")]
    #[ts(rename = "typeText")]
    pub type_text: String,
    pub required: bool,
    pub unique: bool,
    pub extra: String,
}

/// A relation between two entities, as an ERD draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ErdRelation {
    pub left: String,
    pub right: String,
    #[serde(rename = "leftCount")]
    #[ts(rename = "leftCount")]
    pub left_count: Cardinality,
    #[serde(rename = "rightCount")]
    #[ts(rename = "rightCount")]
    pub right_count: Cardinality,
    /// A solid line: the records at the many end are owned by the one.
    pub identifying: bool,
    pub label: Option<String>,
    /// The points the line bends at, in order from left to right; none: straight.
    #[serde(default)]
    pub via: Vec<(i32, i32)>,
}

/// An ERD as the designer opens it, with the problems of its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ErdRead {
    pub erd: Erd,
    pub problems: Vec<Diagnostic>,
}

/// The comment line that keeps an entity's place.
pub(crate) const PLACE: &str = "%% clarity: ";

/// Reads an ERD for the designer, with the problems of its text.
pub fn read(text: &str) -> (Erd, Vec<Diagnostic>) {
    let (diagram, problems) = parse(text);
    if diagram.kind != Some(DiagramKind::Erd) {
        return (Erd::default(), problems);
    }
    let places = places(text);
    let entities = diagram
        .entities
        .into_iter()
        .map(|e| {
            let place = places.iter().find(|(name, _, _)| *name == e.name);
            ErdEntity {
                x: place.map(|p| p.1),
                y: place.map(|p| p.2),
                fields: e.fields.iter().map(field).collect(),
                name: e.name,
            }
        })
        .collect();
    let relations = diagram
        .relations
        .into_iter()
        .map(|r| ErdRelation {
            left: r.left,
            right: r.right,
            left_count: r.left_count,
            right_count: r.right_count,
            identifying: r.identifying,
            label: r.label,
            via: Vec::new(),
        })
        .collect();
    let mut erd = Erd { entities, relations };
    let mut bent = bends(text);
    for r in &mut erd.relations {
        r.via = take_bend(&mut bent, &r.left, &r.right, r.label.as_deref());
    }
    (erd, problems)
}

/// The places the `%% clarity:` lines give: name, x and y.
pub fn places(text: &str) -> Vec<(String, i32, i32)> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix(PLACE)?;
            if rest.contains(" -> ") {
                return None;
            }
            let (name, at) = rest.split_once(" at ")?;
            let (x, y) = at.trim().split_once(',')?;
            Some((name.trim().to_string(), x.trim().parse().ok()?, y.trim().parse().ok()?))
        })
        .collect()
}

/// The points lines bend at, from `%% clarity: Left -> Right : label via x,y x,y`
/// lines: the line's ends, its label, and the points in order.
pub(super) fn bends(text: &str) -> Vec<(String, String, String, Vec<(i32, i32)>)> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix(PLACE)?;
            let (ends, points) = rest.rsplit_once(" via ")?;
            let (ends, label) = ends.split_once(" : ").unwrap_or((ends, ""));
            let (left, right) = ends.split_once(" -> ")?;
            let points: Option<Vec<(i32, i32)>> = points
                .split_whitespace()
                .map(|p| {
                    let (x, y) = p.split_once(',')?;
                    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
                })
                .collect();
            Some((left.trim().to_string(), right.trim().to_string(), label.trim().to_string(), points?))
        })
        .collect()
}

/// The points of the first bend line kept for a relation with these ends and label; it is used up.
pub(super) fn take_bend(bent: &mut Vec<(String, String, String, Vec<(i32, i32)>)>, left: &str, right: &str, label: Option<&str>) -> Vec<(i32, i32)> {
    let label = label.unwrap_or("").trim();
    match bent.iter().position(|(l, r, t, _)| l == left && r == right && t == label) {
        Some(i) => bent.remove(i).3,
        None => Vec::new(),
    }
}

/// The bend line of a relation that bends: `%% clarity: Left -> Right : label via x,y x,y`.
pub(super) fn bend_line(left: &str, right: &str, label: Option<&str>, via: &[(i32, i32)]) -> String {
    let points: Vec<String> = via.iter().map(|(x, y)| format!("{x},{y}")).collect();
    format!("    {PLACE}{left} -> {right} : {} via {}\n", label.unwrap_or("").trim(), points.join(" "))
}

/// A field of a diagram as the designers show it.
pub(super) fn field(f: &DiagramField) -> ErdField {
    let mut rest = f.modifiers.clone();
    rest.required = false;
    rest.unique = false;
    rest.label = None;
    let line = field_line(&FieldSpec { name: "x".into(), type_text: f.type_text.clone(), modifiers: rest });
    let extra = line.strip_prefix(&format!("x: {}", f.type_text)).unwrap_or("").trim_start_matches(',').trim().to_string();
    ErdField { name: f.name.clone(), type_text: f.type_text.clone(), required: f.modifiers.required, unique: f.modifiers.unique, extra }
}

/// Writes the designer's ERD as Mermaid: entities with their attributes,
/// relations, then the places.
pub fn write(erd: &Erd) -> String {
    let mut out = String::from("erDiagram\n");
    for entity in &erd.entities {
        out.push_str(&format!("    {} {{\n", entity.name));
        for f in &entity.fields {
            out.push_str(&format!("        {}\n", attribute(f)));
        }
        out.push_str("    }\n");
    }
    for r in &erd.relations {
        let left = match r.left_count {
            Cardinality::ZeroOrOne => "|o",
            Cardinality::ExactlyOne => "||",
            Cardinality::ZeroOrMore => "}o",
            Cardinality::OneOrMore => "}|",
        };
        let right = match r.right_count {
            Cardinality::ZeroOrOne => "o|",
            Cardinality::ExactlyOne => "||",
            Cardinality::ZeroOrMore => "o{",
            Cardinality::OneOrMore => "|{",
        };
        let line = if r.identifying { "--" } else { ".." };
        let label = match r.label.as_deref().map(str::trim) {
            Some(label) if !label.is_empty() && !label.contains(char::is_whitespace) => label.to_string(),
            Some(label) if !label.is_empty() => format!("\"{label}\""),
            _ => "\"\"".to_string(),
        };
        out.push_str(&format!("    {} {left}{line}{right} {} : {label}\n", r.left, r.right));
    }
    for entity in &erd.entities {
        if let (Some(x), Some(y)) = (entity.x, entity.y) {
            out.push_str(&format!("    {PLACE}{} at {x},{y}\n", entity.name));
        }
    }
    for r in erd.relations.iter().filter(|r| !r.via.is_empty()) {
        out.push_str(&bend_line(&r.left, &r.right, r.label.as_deref(), &r.via));
    }
    out
}

/// A field as an attribute: `type name keys "comment"`; the comment holds
/// the modifiers, and a decimal's full type, which Mermaid's word cannot.
fn attribute(f: &ErdField) -> String {
    let mut comment: Vec<&str> = Vec::new();
    let mut keys: Vec<&str> = Vec::new();
    let word = if f.type_text.starts_with("decimal") {
        comment.push(&f.type_text);
        "decimal".to_string()
    } else if let Some(entity) = f.type_text.strip_prefix("ref ") {
        keys.push("FK");
        entity.trim().to_string()
    } else if let Some(item) = f.type_text.strip_prefix("list<").and_then(|t| t.strip_suffix('>')) {
        format!("{}[]", item.trim())
    } else {
        f.type_text.clone()
    };
    if f.unique {
        keys.push("UK");
    }
    if f.required {
        comment.push("required");
    }
    if !f.extra.trim().is_empty() {
        comment.push(f.extra.trim());
    }
    let mut line = format!("{word} {}", f.name);
    if !keys.is_empty() {
        line.push_str(&format!(" {}", keys.join(", ")));
    }
    if !comment.is_empty() {
        line.push_str(&format!(" \"{}\"", comment.join(", ")));
    }
    line
}
