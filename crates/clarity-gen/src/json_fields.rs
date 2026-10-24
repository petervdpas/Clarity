//! The fields of an entity as JSON is fitted to them (DESIGN 5.23), which
//! every stack hands its runtime for `read Order from json text`: each
//! field's name and what it takes. A reference, a picture, a file and a list
//! of records are never read, so they are left out; so is the id, as the
//! records read are new.

use clarity_spec::Project;
use clarity_spec::model::{Entity, FieldType, ListItem};

/// What a field takes from JSON, as the runtimes name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    Text,
    Integer,
    Decimal,
    Bool,
    Date,
    DateTime,
    /// One of these values, matched in any case.
    Enum(Vec<String>),
    /// A list whose items fit the kind.
    Values(Box<Kind>),
}

impl Kind {
    /// As conformance/json.json and the Go and C# runtimes name it.
    pub(crate) fn word(&self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Integer => "integer",
            Kind::Decimal => "decimal",
            Kind::Bool => "bool",
            Kind::Date => "date",
            Kind::DateTime => "datetime",
            Kind::Enum(_) => "enum",
            Kind::Values(_) => "values",
        }
    }
}

fn values_of(project: &Project, name: &str) -> Vec<String> {
    project.enum_decl(name).map(|e| e.variants.iter().map(|v| v.name.name.clone()).collect()).unwrap_or_default()
}

fn kind(project: &Project, ty: &FieldType) -> Option<Kind> {
    Some(match ty {
        FieldType::Text { .. } | FieldType::Email | FieldType::Url | FieldType::Color | FieldType::Language | FieldType::Theme => Kind::Text,
        FieldType::Integer | FieldType::Duration | FieldType::Id => Kind::Integer,
        FieldType::Decimal { .. } | FieldType::Money => Kind::Decimal,
        FieldType::Bool => Kind::Bool,
        FieldType::Date => Kind::Date,
        FieldType::DateTime => Kind::DateTime,
        FieldType::Enum { name } => Kind::Enum(values_of(project, &name.name)),
        FieldType::Values { item } => Kind::Values(Box::new(match item {
            ListItem::Text => Kind::Text,
            ListItem::Integer => Kind::Integer,
            ListItem::Decimal => Kind::Decimal,
            ListItem::Date => Kind::Date,
            ListItem::Enum { name } => Kind::Enum(values_of(project, &name.name)),
        })),
        FieldType::Ref { .. } | FieldType::Image | FieldType::File | FieldType::List { .. } => return None,
    })
}

/// The fields of `entity` that JSON is read into, with what each takes.
pub(crate) fn fields(project: &Project, entity: &Entity) -> Vec<(String, Kind)> {
    entity.fields.iter().filter_map(|f| kind(project, &f.ty).map(|k| (f.name.name.clone(), k))).collect()
}

/// The entity `read ... from json` reads, and whether it reads a list of them.
pub(crate) fn target(ty: &clarity_spec::code::TypeRef) -> Option<(&str, bool)> {
    match &ty.kind {
        clarity_spec::code::TypeRefKind::Named { name } => Some((name, false)),
        clarity_spec::code::TypeRefKind::List { item } => Some((item, true)),
        _ => None,
    }
}
