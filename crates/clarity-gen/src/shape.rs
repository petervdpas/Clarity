//! What a program reads from a format or writes in one, as Clarity.Formats
//! is told it (docs/FORMATS-PLAN.md): the kind, as JSON, the same on every
//! stack. A record lists every field of its entity in order with what it
//! takes: a reference, a picture, a file and a list of stored records are
//! never read (`other`), and a record held inside an entity of a web API,
//! and its lists of records, are read as the entity they hold (DESIGN
//! 5.25), down to where an entity would repeat inside itself. What a record
//! leaves out keeps its value in a new record of it, its `base`, which only
//! the stack's own codec writes; so a kind comes as pieces, each stack
//! putting its codec's new record where a base goes.

use clarity_spec::Project;
use clarity_spec::code::check::{TypeInfo, ValueItem};
use clarity_spec::code::{Expr, MappingDecl, MappingUse, TypeRef, TypeRefKind};
use clarity_spec::model::{Entity, FieldType, ListItem};

/// A piece of a kind's JSON: text as it is, or where a new record of an entity goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Piece {
    Text(String),
    Base(String),
    /// A value of the stack's code, already JSON text (an option's value, quoted by the stack).
    Code(String),
}

/// A format's options as a JSON object, each value given as the stack's
/// code that writes it as JSON; empty when there are none.
pub(crate) fn options_pieces(options: &[(String, String)]) -> Vec<Piece> {
    if options.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (i, (name, code)) in options.iter().enumerate() {
        out.push(Piece::Text(format!("{}{}:", if i == 0 { "{" } else { "," }, quoted(name))));
        out.push(Piece::Code(code.clone()));
    }
    out.push(Piece::Text("}".into()));
    out
}

/// The format a reading or writing is in, and its options as pieces
/// (docs/FORMATS-PLAN.md, step 4): a mapping written in place gives each
/// option's value as the stack's code, which `code` writes as JSON; a
/// declared mapping is its options object whole. None for a mapping the
/// project does not declare.
pub(crate) fn mapping_pieces(project: &Project, mapping: &MappingUse, mut code: impl FnMut(&Expr) -> String) -> Option<(String, Vec<Piece>)> {
    match mapping {
        MappingUse::Inline { format, options } => {
            let given: Vec<(String, String)> = options.iter().map(|o| (o.name.name.clone(), code(&o.value))).collect();
            Some((format.name.clone(), options_pieces(&given)))
        }
        MappingUse::Named { name } => {
            let declared = project.mapping(&name.name)?;
            Some((declared.format.name.clone(), vec![Piece::Text(mapping_options(declared))]))
        }
    }
}

/// A mapping as the options object every stack passes to Clarity.Formats,
/// written once in clarity-spec, where the IDE's preview reads with it too.
pub(crate) fn mapping_options(mapping: &MappingDecl) -> String {
    clarity_spec::code::mapping::options_json(mapping)
}

/// What `read T from <format>` reads: one record of an entity or a list of
/// them, with each record's base left to the stack. None for anything else.
pub(crate) fn read_kind(project: &Project, ty: &TypeRef) -> Option<Vec<Piece>> {
    let (entity, many) = match &ty.kind {
        TypeRefKind::Named { name } => (name.as_str(), false),
        TypeRefKind::List { item } => (item.as_str(), true),
        _ => return None,
    };
    let decl = project.entity(entity)?;
    let mut out = Kinds::default();
    out.record(project, decl, many, &mut vec![entity.to_string()]);
    Some(out.done())
}

/// What `<format> of value` writes, by the value's type: a record, a list of
/// records, or a plain value. A writing needs no base (it only puts the
/// fields in their order), so the kind is whole text, an empty base in it.
pub(crate) fn write_kind(project: &Project, ty: &TypeInfo) -> Option<String> {
    let mut out = Kinds { bases: false, ..Kinds::default() };
    match ty {
        TypeInfo::Record(entity) | TypeInfo::List(entity) => {
            let decl = project.entity(entity)?;
            out.record(project, decl, matches!(ty, TypeInfo::List(_)), &mut vec![entity.clone()]);
        }
        TypeInfo::Text => out.text(r#"{"kind":"text"}"#),
        TypeInfo::Integer | TypeInfo::Duration => out.text(r#"{"kind":"integer"}"#),
        TypeInfo::Decimal | TypeInfo::Money => out.text(r#"{"kind":"decimal"}"#),
        TypeInfo::Bool => out.text(r#"{"kind":"bool"}"#),
        TypeInfo::Date => out.text(r#"{"kind":"date"}"#),
        TypeInfo::DateTime => out.text(r#"{"kind":"datetime"}"#),
        TypeInfo::Enum(name) => out.enum_kind(&values_of(project, name)),
        TypeInfo::Values(item) => {
            out.text(r#"{"kind":"values","item":"#);
            match item {
                ValueItem::Text => out.text(r#"{"kind":"text"}"#),
                ValueItem::Integer => out.text(r#"{"kind":"integer"}"#),
                ValueItem::Decimal => out.text(r#"{"kind":"decimal"}"#),
                ValueItem::Bool => out.text(r#"{"kind":"bool"}"#),
                ValueItem::Date => out.text(r#"{"kind":"date"}"#),
                ValueItem::Enum(name) => out.enum_kind(&values_of(project, name)),
            }
            out.text("}");
        }
        _ => return None,
    }
    let pieces = out.done();
    Some(pieces.into_iter().map(|p| if let Piece::Text(t) = p { t } else { String::new() }).collect())
}

/// A text as a JSON string.
pub(crate) fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn values_of(project: &Project, name: &str) -> Vec<String> {
    project.enum_decl(name).map(|e| e.variants.iter().map(|v| v.name.name.clone()).collect()).unwrap_or_default()
}

/// A kind being written, piece by piece.
struct Kinds {
    pieces: Vec<Piece>,
    /// Whether records get their base (reading), or an empty one (writing).
    bases: bool,
}

impl Default for Kinds {
    fn default() -> Kinds {
        Kinds { pieces: Vec::new(), bases: true }
    }
}

impl Kinds {
    fn text(&mut self, text: &str) {
        match self.pieces.last_mut() {
            Some(Piece::Text(last)) => last.push_str(text),
            _ => self.pieces.push(Piece::Text(text.to_string())),
        }
    }

    fn done(self) -> Vec<Piece> {
        self.pieces
    }

    fn enum_kind(&mut self, values: &[String]) {
        let values: Vec<String> = values.iter().map(|v| quoted(v)).collect();
        self.text(&format!(r#"{{"kind":"enum","values":[{}]}}"#, values.join(",")));
    }

    /// One record of `entity` (or with `many` a list of them); `path` holds the entities around it.
    fn record(&mut self, project: &Project, entity: &Entity, many: bool, path: &mut Vec<String>) {
        let name = &entity.name.name;
        self.text(&format!(r#"{{"kind":"{}","entity":{},"fields":["#, if many { "records" } else { "record" }, quoted(name)));
        for (i, field) in entity.fields.iter().enumerate() {
            if i > 0 {
                self.text(",");
            }
            self.text(&format!(r#"{{"name":{},"#, quoted(&field.name.name)));
            self.field(project, entity, &field.ty, path);
            self.text("}");
        }
        self.text(r#"],"base":"#);
        if self.bases {
            self.pieces.push(Piece::Base(name.clone()));
        } else {
            self.text("{}");
        }
        self.text("}");
    }

    /// What a field takes, written inside its object after its name (without the braces).
    fn field(&mut self, project: &Project, owner: &Entity, ty: &FieldType, path: &mut Vec<String>) {
        let nested = |name: &str, path: &Vec<String>| (!path.iter().any(|p| p == name)).then(|| project.entity(name)).flatten();
        match ty {
            FieldType::Record { entity } | FieldType::List { item: entity } if !matches!(ty, FieldType::List { .. }) || project.in_web_api(&owner.name.name) => {
                let Some(decl) = nested(&entity.name, path) else { return self.text(r#""kind":"other""#) };
                // The nested record's own object, unwrapped into this field's.
                let mut inner = Kinds { pieces: Vec::new(), bases: self.bases };
                path.push(entity.name.clone());
                inner.record(project, decl, matches!(ty, FieldType::List { .. }), path);
                path.pop();
                let mut pieces = inner.done();
                if let Some(Piece::Text(first)) = pieces.first_mut() {
                    *first = first.trim_start_matches('{').to_string();
                }
                if let Some(Piece::Text(last)) = pieces.last_mut() {
                    last.pop();
                }
                for piece in pieces {
                    match piece {
                        Piece::Text(t) => self.text(&t),
                        other => self.pieces.push(other),
                    }
                }
            }
            FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Folder | FieldType::Language | FieldType::Theme | FieldType::Secret => self.text(r#""kind":"text""#),
            FieldType::Integer | FieldType::Duration | FieldType::Id => self.text(r#""kind":"integer""#),
            FieldType::Decimal { .. } | FieldType::Money => self.text(r#""kind":"decimal""#),
            FieldType::Bool => self.text(r#""kind":"bool""#),
            FieldType::Date => self.text(r#""kind":"date""#),
            FieldType::DateTime => self.text(r#""kind":"datetime""#),
            FieldType::Enum { name } => {
                let values: Vec<String> = values_of(project, &name.name).iter().map(|v| quoted(v)).collect();
                self.text(&format!(r#""kind":"enum","values":[{}]"#, values.join(",")));
            }
            FieldType::Values { item } => {
                let item = match item {
                    ListItem::Text => r#"{"kind":"text"}"#.to_string(),
                    ListItem::Integer => r#"{"kind":"integer"}"#.to_string(),
                    ListItem::Decimal => r#"{"kind":"decimal"}"#.to_string(),
                    ListItem::Date => r#"{"kind":"date"}"#.to_string(),
                    ListItem::Enum { name } => format!(r#"{{"kind":"enum","values":[{}]}}"#, values_of(project, &name.name).iter().map(|v| quoted(v)).collect::<Vec<_>>().join(",")),
                };
                self.text(&format!(r#""kind":"values","item":{item}"#));
            }
            FieldType::Ref { .. } | FieldType::Image | FieldType::File | FieldType::List { .. } | FieldType::Record { .. } => self.text(r#""kind":"other""#),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "module shop\n\nenum Kind\n    Person\n    Business\n\nentity Customer\n    name: text\n\nentity Order\n    number: text\n    kind: Kind\n    total: money\n    tags: list<text>\n    customer: ref Customer\n";

    #[test]
    fn a_record_lists_every_field_in_order_and_leaves_its_base_to_the_stack() {
        let project = clarity_spec::load_converted([("shop.clarity", SOURCE)]);
        let ty = TypeRef { kind: TypeRefKind::List { item: "Order".into() }, span: Default::default() };
        let pieces = read_kind(&project, &ty).unwrap();
        assert_eq!(
            pieces,
            [
                Piece::Text(r#"{"kind":"records","entity":"Order","fields":[{"name":"number","kind":"text"},{"name":"kind","kind":"enum","values":["Person","Business"]},{"name":"total","kind":"decimal"},{"name":"tags","kind":"values","item":{"kind":"text"}},{"name":"customer","kind":"other"}],"base":"#.into()),
                Piece::Base("Order".into()),
                Piece::Text("}".into()),
            ]
        );
        let written = write_kind(&project, &TypeInfo::Record("Order".into())).unwrap();
        assert!(written.starts_with(r#"{"kind":"record","entity":"Order""#) && written.ends_with(r#""base":{}}"#), "{written}");
        assert!(clarity_formats::Kind::from_json(&written).is_ok(), "{written}");
        assert_eq!(write_kind(&project, &TypeInfo::Values(ValueItem::Enum("Kind".into()))).unwrap(), r#"{"kind":"values","item":{"kind":"enum","values":["Person","Business"]}}"#);
    }
}
