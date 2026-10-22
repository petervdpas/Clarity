//! Diagrams (DESIGN section 5.13): Mermaid text read into what Clarity maps
//! it to. A diagram is a view of the project or a proposal to change it;
//! the data dictionary stays the one source of truth.
//!
//! The kinds Clarity maps are read here: an ERD into entities, fields and
//! relationships. Other Mermaid kinds are only drawn.

mod class;
mod compare;
mod erd;
pub mod erd_model;
mod fields;
mod files;
pub mod views;

pub use compare::{Change, ModuleText, Proposal, apply, propose};
pub use files::{DiagramFile, diagram_files, starter};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::diagnostic::Diagnostic;
use crate::model::Modifiers;
use crate::span::Span;

/// The folder of written diagrams in a project's folder, beside
/// `resources`, `engines` and `translations`. Diagrams are design: no
/// generated application carries them.
pub const DIAGRAMS_DIR: &str = "diagrams";

/// The extension of a diagram file.
pub const DIAGRAM_EXTENSION: &str = "mmd";

/// A kind of diagram Clarity maps onto the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DiagramKind {
    /// Entities, their fields and relationships.
    Erd,
    /// Entities, enums, engines and modules with their functions.
    Class,
    /// The values of an enum and the steps between them.
    State,
    /// What a handler calls, in order.
    Sequence,
    /// The flow of a handler or function.
    Activity,
    /// Modules, engines, data sources and the stack.
    Architecture,
}

/// A kind as the tree, the New diagram dialog and the agent list it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiagramKindInfo {
    pub kind: DiagramKind,
    pub name: String,
    /// The Mermaid word a diagram of this kind starts with.
    pub header: String,
}

/// The kinds of diagram, in the order they are offered.
pub fn kinds() -> Vec<DiagramKindInfo> {
    let kind = |kind, name: &str, header: &str| DiagramKindInfo { kind, name: name.into(), header: header.into() };
    vec![
        kind(DiagramKind::Erd, "ERD", "erDiagram"),
        kind(DiagramKind::Class, "Class diagram", "classDiagram"),
        kind(DiagramKind::State, "State diagram", "stateDiagram-v2"),
        kind(DiagramKind::Sequence, "Sequence diagram", "sequenceDiagram"),
        kind(DiagramKind::Activity, "Activity diagram", "flowchart"),
        kind(DiagramKind::Architecture, "Architecture", "architecture-beta"),
    ]
}

/// Other words Mermaid starts these kinds with.
const OTHER_HEADERS: [(&str, DiagramKind); 2] = [("stateDiagram", DiagramKind::State), ("graph", DiagramKind::Activity)];

/// What a diagram says, as far as Clarity maps it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Diagram {
    /// None for a kind Clarity only draws.
    pub kind: Option<DiagramKind>,
    pub entities: Vec<DiagramEntity>,
    pub enums: Vec<DiagramEnum>,
    pub engines: Vec<DiagramEngine>,
    /// Modules drawn as a `<<module>>` class, with their functions.
    pub modules: Vec<DiagramModule>,
    pub relations: Vec<Relation>,
}

/// An entity of a diagram, in the order it is first named.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiagramEntity {
    pub name: String,
    pub fields: Vec<DiagramField>,
    /// The module a class diagram's namespace puts it in.
    pub module: Option<String>,
    /// Where it is first named.
    pub span: Span,
}

/// A field as Clarity reads it: its type in Clarity's words, its modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiagramField {
    pub name: String,
    #[serde(rename = "typeText")]
    #[ts(rename = "typeText")]
    pub type_text: String,
    pub modifiers: Modifiers,
    pub span: Span,
}

/// An enum of a class diagram, `<<enumeration>>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiagramEnum {
    pub name: String,
    pub values: Vec<DiagramValue>,
    pub module: Option<String>,
    pub span: Span,
}

/// A value of an enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiagramValue {
    pub name: String,
    pub span: Span,
}

/// An engine of a class diagram, `<<engine>>`, with its functions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiagramEngine {
    pub name: String,
    pub functions: Vec<DiagramFunction>,
    pub module: Option<String>,
    pub span: Span,
}

/// A module of a class diagram, `<<module>>`, with its functions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiagramModule {
    pub name: String,
    pub functions: Vec<DiagramFunction>,
    pub span: Span,
}

/// A function's signature as a diagram draws it, its types in Clarity's words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiagramFunction {
    pub name: String,
    pub params: Vec<DiagramParam>,
    pub result: Option<String>,
    pub span: Span,
}

impl DiagramFunction {
    /// The signature as Clarity writes it: `play(square: integer) -> text`.
    pub fn signature(&self) -> String {
        let params: Vec<String> = self.params.iter().map(|p| format!("{}: {}", p.name, p.type_text)).collect();
        match &self.result {
            Some(result) => format!("{}({}) -> {result}", self.name, params.join(", ")),
            None => format!("{}({})", self.name, params.join(", ")),
        }
    }
}

/// A parameter of a function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiagramParam {
    pub name: String,
    #[serde(rename = "typeText")]
    #[ts(rename = "typeText")]
    pub type_text: String,
}

/// How many records stand at one end of a relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Cardinality {
    ZeroOrOne,
    ExactlyOne,
    ZeroOrMore,
    OneOrMore,
}

impl Cardinality {
    /// Whether more than one record may stand at this end.
    pub fn is_many(self) -> bool {
        matches!(self, Cardinality::ZeroOrMore | Cardinality::OneOrMore)
    }
}

/// A relationship between two entities, with how many stand at each end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Relation {
    pub left: String,
    pub right: String,
    #[serde(rename = "leftCount")]
    #[ts(rename = "leftCount")]
    pub left_count: Cardinality,
    #[serde(rename = "rightCount")]
    #[ts(rename = "rightCount")]
    pub right_count: Cardinality,
    /// A solid line or a composition: the records at the many end live and
    /// die with the one, which lists them as owned.
    pub identifying: bool,
    /// Whether the one end lists the records at the many end: an ERD's
    /// solid line, a class diagram's composition or aggregation.
    pub lists: bool,
    pub label: Option<String>,
    pub span: Span,
}

/// The kind of a diagram by its header, after frontmatter and comments;
/// None for a kind Clarity only draws.
pub fn kind_of(text: &str) -> Option<DiagramKind> {
    lines(text).next().and_then(|line| header_kind(line.text))
}

/// Reads a diagram, with its problems placed on their lines, in the order of the lines.
pub fn parse(text: &str) -> (Diagram, Vec<Diagnostic>) {
    let mut body = lines(text);
    let Some(kind) = body.next().and_then(|line| header_kind(line.text)) else {
        return (Diagram::default(), Vec::new());
    };
    let (diagram, mut problems) = match kind {
        DiagramKind::Erd => erd::parse(body),
        DiagramKind::Class => class::parse(body),
        _ => (Diagram { kind: Some(kind), ..Diagram::default() }, Vec::new()),
    };
    problems.sort_by_key(|p| (p.span.line, p.span.col));
    (diagram, problems)
}

fn header_kind(line: &str) -> Option<DiagramKind> {
    let word = line.split_whitespace().next()?;
    kinds()
        .into_iter()
        .map(|k| (k.header, k.kind))
        .find(|(header, _)| header == word)
        .map(|(_, kind)| kind)
        .or_else(|| OTHER_HEADERS.iter().find(|(header, _)| *header == word).map(|(_, kind)| *kind))
}

/// A line of a diagram with content: its text without the indentation,
/// and where that text starts.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Line<'a> {
    pub text: &'a str,
    pub span: Span,
}

impl Line<'_> {
    /// The span of a piece of this line's text.
    pub fn span_of(&self, piece: &str) -> Span {
        let from = piece.as_ptr() as usize - self.text.as_ptr() as usize;
        Span {
            start: self.span.start + from as u32,
            end: self.span.start + (from + piece.len()) as u32,
            line: self.span.line,
            col: self.span.col + self.text[..from].chars().count() as u32,
        }
    }
}

/// The lines with content, past a leading frontmatter block; blank lines
/// and `%%` comments are left out.
pub(crate) fn lines(text: &str) -> impl Iterator<Item = Line<'_>> {
    let mut offset = 0usize;
    let mut all = Vec::new();
    for (index, raw) in text.split('\n').enumerate() {
        let start = offset;
        offset += raw.len() + 1;
        let content = raw.trim_end_matches('\r').trim_end();
        let trimmed = content.trim_start();
        let indent = content.len() - trimmed.len();
        all.push(Line {
            text: trimmed,
            span: Span {
                start: (start + indent) as u32,
                end: (start + content.len()) as u32,
                line: index as u32 + 1,
                col: content[..indent].chars().count() as u32 + 1,
            },
        });
    }
    let first = all.iter().position(|l| !l.text.is_empty());
    let skip = match first {
        Some(i) if all[i].text == "---" => all[i + 1..].iter().position(|l| l.text == "---").map_or(0, |end| i + end + 2),
        _ => 0,
    };
    all.into_iter().skip(skip).filter(|l| !l.text.is_empty() && !l.text.starts_with("%%"))
}
