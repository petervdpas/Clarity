//! A class diagram (`classDiagram`): classes become entities, a class
//! marked `<<enumeration>>` an enum, `<<engine>>` an engine and `<<module>>`
//! a module's functions; a `namespace` names the module of the classes in
//! it. Associations, compositions and aggregations become relations, read
//! as an ERD's are; inheritance and dependencies are drawn only.

use crate::diagnostic::Diagnostic;
use crate::span::Span;

use super::fields::{field_type, read_field, split_type, value_type};
use super::{
    Cardinality, Diagram, DiagramEngine, DiagramEntity, DiagramEnum, DiagramField, DiagramFunction, DiagramKind, DiagramModule,
    DiagramParam, DiagramValue, Line, Relation,
};

const UNREADABLE: &str = "Clarity cannot read this line of a class diagram: write a class `class Name {`, a member `+name: text`, or a relation `A \"1\" --> \"*\" B : label`";

/// Lines Mermaid uses for how a diagram looks, which mean nothing to the model.
const LOOKS: [&str; 7] = ["note", "click", "link", "callback", "style", "classDef", "cssClass"];

/// What a class stands for, by its stereotype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stereotype {
    Enumeration,
    Engine,
    Module,
    /// A stereotype Clarity does not map: the class is drawn only.
    Other,
}

/// A class as it is read, before its stereotype says what it is.
struct Draft {
    name: String,
    stereotype: Option<Stereotype>,
    fields: Vec<DiagramField>,
    /// Members of one word: an enum's values, or fields without a type.
    words: Vec<DiagramValue>,
    functions: Vec<DiagramFunction>,
    module: Option<String>,
    span: Span,
}

enum Block {
    Namespace(String),
    Class(usize),
}

pub(super) fn parse<'a>(body: impl Iterator<Item = Line<'a>>) -> (Diagram, Vec<Diagnostic>) {
    let mut drafts: Vec<Draft> = Vec::new();
    let mut relations = Vec::new();
    let mut problems = Vec::new();
    let mut blocks: Vec<Block> = Vec::new();
    for line in body {
        let text = line.text;
        let namespace = blocks.iter().rev().find_map(|b| match b {
            Block::Namespace(name) => Some(name.clone()),
            Block::Class(_) => None,
        });
        if text == "}" {
            blocks.pop();
            continue;
        }
        if let Some(Block::Class(index)) = blocks.last() {
            let index = *index;
            match stereotype_of(text) {
                Some((word, _)) => set_stereotype(&mut drafts[index], word, &line, &mut problems),
                None => member(&mut drafts[index], text, &line, &mut problems),
            }
            continue;
        }
        let first = text.split_whitespace().next().unwrap_or_default();
        if first == "direction" || LOOKS.contains(&first) {
            continue;
        }
        if let Some(rest) = text.strip_prefix("namespace ") {
            blocks.push(Block::Namespace(rest.trim_end_matches('{').trim().to_string()));
        } else if let Some(rest) = text.strip_prefix("class ") {
            let name = class_name(rest);
            let index = draft_index(&mut drafts, name, namespace, line.span_of(name));
            if rest.trim_end().ends_with('{') {
                blocks.push(Block::Class(index));
            }
        } else if let Some((word, name)) = stereotype_of(text) {
            let index = draft_index(&mut drafts, class_name(name), namespace, line.span_of(name.trim()));
            set_stereotype(&mut drafts[index], word, &line, &mut problems);
        } else if let Some(found) = relation(&line, &mut problems) {
            if let Some(relation) = found {
                draft_index(&mut drafts, &relation.left, namespace.clone(), relation.span);
                draft_index(&mut drafts, &relation.right, namespace, relation.span);
                relations.push(relation);
            } else {
                for name in ends(text) {
                    draft_index(&mut drafts, name, namespace.clone(), line.span);
                }
            }
        } else if let Some((name, what)) = text.split_once(':') {
            let index = draft_index(&mut drafts, class_name(name), namespace, line.span_of(name.trim()));
            member(&mut drafts[index], what.trim(), &line, &mut problems);
        } else {
            problems.push(Diagnostic::error(UNREADABLE, line.span));
        }
    }
    (finish(drafts, relations, &mut problems), problems)
}

/// The name of a class, without generics, label, style or brace.
fn class_name(text: &str) -> &str {
    let text = text.trim();
    let end = text.find(|c: char| c.is_whitespace() || matches!(c, '{' | '~' | '[' | ':')).unwrap_or(text.len());
    &text[..end]
}

fn draft_index(drafts: &mut Vec<Draft>, name: &str, module: Option<String>, span: Span) -> usize {
    match drafts.iter().position(|d| d.name == name) {
        Some(index) => index,
        None => {
            drafts.push(Draft {
                name: name.to_string(),
                stereotype: None,
                fields: Vec::new(),
                words: Vec::new(),
                functions: Vec::new(),
                module,
                span,
            });
            drafts.len() - 1
        }
    }
}

/// `<<enumeration>>` and what follows it.
fn stereotype_of(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("<<")?;
    let (word, after) = rest.split_once(">>")?;
    Some((word.trim(), after))
}

fn set_stereotype(draft: &mut Draft, word: &str, line: &Line, problems: &mut Vec<Diagnostic>) {
    let stereotype = match word.to_lowercase().as_str() {
        "enumeration" | "enum" => Stereotype::Enumeration,
        "engine" => Stereotype::Engine,
        "module" => Stereotype::Module,
        _ => {
            problems.push(Diagnostic::warning(
                format!("Clarity does not map a <<{word}>> class; it is drawn only. Clarity reads <<enumeration>>, <<engine>> and <<module>>"),
                line.span,
            ));
            Stereotype::Other
        }
    };
    draft.stereotype = Some(stereotype);
}

/// A member of a class: a field, a function, or a word (an enum's value).
fn member(draft: &mut Draft, text: &str, line: &Line, problems: &mut Vec<Diagnostic>) {
    let text = text.trim_start_matches(['+', '-', '#', '~']).trim().trim_end_matches(['$', '*']).trim_end();
    let bracket = text.find('(');
    let is_function = bracket.is_some_and(|b| text.find(':').is_none_or(|colon| b < colon));
    if let (true, Some((name, rest))) = (is_function, text.split_once('(')) {
        let (params, result) = rest.rsplit_once(')').unwrap_or((rest, ""));
        let result = result.trim().trim_start_matches(':').trim();
        draft.functions.push(DiagramFunction {
            name: name.trim().to_string(),
            params: params.split(',').map(str::trim).filter(|p| !p.is_empty()).map(param).collect(),
            result: (!result.is_empty()).then(|| value_type(result)),
            span: line.span,
        });
        return;
    }
    let field = if let Some((name, rest)) = text.split_once(':') {
        let (written, modifiers) = split_type(rest.trim());
        typed(name.trim(), written, modifiers, line, problems)
    } else {
        let words: Vec<&str> = text.split_whitespace().collect();
        match words[..] {
            [word] => {
                draft.words.push(DiagramValue { name: word.to_string(), span: line.span_of(word) });
                return;
            }
            [written, name] => typed(name, written, None, line, problems),
            _ => {
                problems.push(Diagnostic::error(UNREADABLE, line.span));
                return;
            }
        }
    };
    if let Some(field) = field {
        draft.fields.push(field);
    }
}

/// A field from its name, its type as written and its modifiers.
fn typed(name: &str, written: &str, modifiers: Option<&str>, line: &Line, problems: &mut Vec<Diagnostic>) -> Option<DiagramField> {
    let ty = match field_type(written, false) {
        Ok(ty) => ty,
        Err(message) => {
            problems.push(Diagnostic::error(message, line.span_of(written)));
            return None;
        }
    };
    let field_text = match modifiers {
        Some(modifiers) => format!("{ty}, {modifiers}"),
        None => ty,
    };
    read_field(name, &field_text, line, problems)
}

/// `square: integer` or `int square`.
fn param(text: &str) -> DiagramParam {
    if let Some((name, ty)) = text.split_once(':') {
        return DiagramParam { name: name.trim().to_string(), type_text: value_type(ty) };
    }
    match text.split_whitespace().collect::<Vec<_>>()[..] {
        [ty, name] => DiagramParam { name: name.to_string(), type_text: value_type(ty) },
        _ => DiagramParam { name: text.to_string(), type_text: String::new() },
    }
}

/// The words of a relation line: names, quoted counts and the arrow.
fn tokens(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if rest.starts_with('"') {
            let end = rest[1..].find('"').map_or(rest.len(), |i| i + 2);
            out.push(&rest[..end]);
            rest = rest[end..].trim_start();
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            out.push(&rest[..end]);
            rest = rest[end..].trim_start();
        }
    }
    out
}

/// The classes a relation line names, whether or not Clarity maps it.
fn ends(text: &str) -> Vec<&str> {
    let words = tokens(text.split_once(':').map_or(text, |(ends, _)| ends));
    match (words.first(), words.last()) {
        (Some(first), Some(last)) => vec![class_name(first), class_name(last)],
        _ => Vec::new(),
    }
}

/// A relation line: None when the line is no relation, Some(None) when it
/// is one Clarity only draws, with the warning noted.
fn relation(line: &Line, problems: &mut Vec<Diagnostic>) -> Option<Option<Relation>> {
    let (ends, label) = match line.text.split_once(':') {
        Some((ends, label)) => (ends, Some(label.trim().trim_matches('"').to_string()).filter(|l| !l.is_empty())),
        None => (line.text, None),
    };
    let words = tokens(ends);
    let (a, a_count, arrow, b_count, b) = match words[..] {
        [a, arrow, b] => (a, None, arrow, None, b),
        [a, count, arrow, b] if count.starts_with('"') => (a, Some(count), arrow, None, b),
        [a, arrow, count, b] if count.starts_with('"') => (a, None, arrow, Some(count), b),
        [a, a_count, arrow, b_count, b] => (a, Some(a_count), arrow, Some(b_count), b),
        _ => return None,
    };
    let body = arrow.find("--").or_else(|| arrow.find(".."))?;
    let (head_a, line_kind, head_b) = (&arrow[..body], &arrow[body..body + 2], &arrow[body + 2..]);
    if !["", "<|", "*", "o", "<"].contains(&head_a) || !["", "|>", "*", "o", ">"].contains(&head_b) {
        return None;
    }
    if head_a == "<|" || head_b == "|>" {
        problems.push(Diagnostic::warning("Clarity has no inheritance; this line is drawn only", line.span));
        return Some(None);
    }
    let whole_at_a = matches!(head_a, "*" | "o");
    let whole_at_b = matches!(head_b, "*" | "o");
    if line_kind == ".." && !whole_at_a && !whole_at_b {
        problems.push(Diagnostic::warning(
            "a dependency is drawn only; Clarity reads associations (-->), compositions (*--) and aggregations (o--)",
            line.span,
        ));
        return Some(None);
    }
    let mut count = |written: Option<&str>, default: Cardinality| match written.map(|w| w.trim_matches('"').trim()) {
        None => default,
        Some(written) => multiplicity(written).unwrap_or_else(|| {
            problems.push(Diagnostic::warning(
                format!("Clarity reads the counts \"1\", \"0..1\", \"*\" and \"1..*\", not \"{written}\""),
                line.span,
            ));
            default
        }),
    };
    // The whole of a composition or aggregation, or the side that refers, stands left.
    let (left, left_count, right, right_count, identifying) = if whole_at_a || whole_at_b {
        let identifying = head_a == "*" || head_b == "*";
        if whole_at_a {
            (a, count(a_count, Cardinality::ExactlyOne), b, count(b_count, Cardinality::ZeroOrMore), identifying)
        } else {
            (b, count(b_count, Cardinality::ExactlyOne), a, count(a_count, Cardinality::ZeroOrMore), identifying)
        }
    } else if head_a == "<" && head_b.is_empty() {
        (b, count(b_count, Cardinality::ZeroOrMore), a, count(a_count, Cardinality::ExactlyOne), false)
    } else {
        (a, count(a_count, Cardinality::ZeroOrMore), b, count(b_count, Cardinality::ExactlyOne), false)
    };
    Some(Some(Relation {
        left: class_name(left).to_string(),
        right: class_name(right).to_string(),
        left_count,
        right_count,
        identifying,
        lists: whole_at_a || whole_at_b,
        label,
        span: line.span,
    }))
}

fn multiplicity(written: &str) -> Option<Cardinality> {
    Some(match written {
        "1" | "1..1" => Cardinality::ExactlyOne,
        "0..1" => Cardinality::ZeroOrOne,
        "*" | "0..*" | "n" | "0..n" | "many" => Cardinality::ZeroOrMore,
        "1..*" | "1..n" => Cardinality::OneOrMore,
        _ => return None,
    })
}

/// Each class becomes what its stereotype says.
fn finish(drafts: Vec<Draft>, relations: Vec<Relation>, problems: &mut Vec<Diagnostic>) -> Diagram {
    let mut diagram = Diagram { kind: Some(DiagramKind::Class), relations, ..Diagram::default() };
    for draft in drafts {
        match draft.stereotype {
            Some(Stereotype::Enumeration) => {
                for field in &draft.fields {
                    problems.push(Diagnostic::warning("an enum has values only; write each on a line of its own", field.span));
                }
                diagram.enums.push(DiagramEnum { name: draft.name, values: draft.words, module: draft.module, span: draft.span });
            }
            Some(Stereotype::Engine) => {
                for field in &draft.fields {
                    problems.push(Diagnostic::warning("what an engine keeps is not read from a diagram yet; write it in the engine", field.span));
                }
                diagram.engines.push(DiagramEngine { name: draft.name, functions: draft.functions, module: draft.module, span: draft.span });
            }
            Some(Stereotype::Module) => {
                diagram.modules.push(DiagramModule { name: draft.name, functions: draft.functions, span: draft.span });
            }
            Some(Stereotype::Other) => {}
            None => {
                for word in &draft.words {
                    problems.push(Diagnostic::error(format!("'{}' needs a type, as {}: text", word.name, word.name), word.span));
                }
                for function in &draft.functions {
                    problems.push(Diagnostic::warning(
                        format!("functions belong to a module: draw {} on the module's class, marked <<module>>", function.name),
                        function.span,
                    ));
                }
                diagram.entities.push(DiagramEntity { name: draft.name, fields: draft.fields, module: draft.module, span: draft.span });
            }
        }
    }
    diagram
}
