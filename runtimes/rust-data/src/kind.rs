//! What a program reads or writes, as the generator describes it once for
//! every stack: a record of an entity (its fields, each with its kind, and
//! a new record as the stack's codec writes it, which what is left out
//! keeps), a list of records, or a plain value. Given as JSON:
//! `{"kind":"record","entity":"Order","fields":[{"name":"total","kind":"decimal"}],"base":{...}}`.

use crate::tree::{self, Tree};

/// What a value is read as or written as.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Text,
    Integer,
    /// Decimals and money, as exact text.
    Decimal,
    Bool,
    Date,
    DateTime,
    /// One of these values, matched in any case, written as declared.
    Enum(Vec<String>),
    /// A list whose items are of the kind.
    Values(Box<Kind>),
    /// One record of an entity.
    Record(Box<Shape>),
    /// A list of records of an entity.
    Records(Box<Shape>),
    /// Never read, written as the stack writes it: a reference, a picture, a file, a list of stored records.
    Other,
}

/// An entity as it is read and written: its name, its fields in their order, and a new record of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub entity: String,
    pub fields: Vec<Field>,
    pub base: Tree,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub kind: Kind,
}

impl Kind {
    /// The kind JSON describes, or why it does not describe one.
    pub fn from_json(text: &str) -> Result<Kind, String> {
        let tree = tree::parse(text).ok_or("the kind is no JSON")?;
        Kind::from_tree(&tree)
    }

    pub fn from_tree(tree: &Tree) -> Result<Kind, String> {
        let word = tree.get("kind").and_then(Tree::as_text).ok_or("a kind has a 'kind'")?;
        Ok(match word {
            "text" => Kind::Text,
            "integer" => Kind::Integer,
            "decimal" => Kind::Decimal,
            "bool" => Kind::Bool,
            "date" => Kind::Date,
            "datetime" => Kind::DateTime,
            "other" => Kind::Other,
            "enum" => match tree.get("values") {
                Some(Tree::List(values)) => Kind::Enum(values.iter().filter_map(|v| v.as_text().map(str::to_string)).collect()),
                _ => return Err("an enum kind lists its 'values'".into()),
            },
            "values" => Kind::Values(Box::new(Kind::from_tree(tree.get("item").ok_or("a values kind has its 'item'")?)?)),
            "record" => Kind::Record(Box::new(Shape::from_tree(tree)?)),
            "records" => Kind::Records(Box::new(Shape::from_tree(tree)?)),
            other => return Err(format!("'{other}' is no kind")),
        })
    }

    /// What the kind is, as a message says it.
    pub fn wanted(&self) -> String {
        match self {
            Kind::Text => "text".into(),
            Kind::Integer => "an integer".into(),
            Kind::Decimal => "a decimal".into(),
            Kind::Bool => "true or false".into(),
            Kind::Date => "a date".into(),
            Kind::DateTime => "a date and time".into(),
            Kind::Enum(values) => match values.as_slice() {
                [] => "an enum value".into(),
                [one] => one.clone(),
                [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
            },
            Kind::Values(_) => "a list".into(),
            Kind::Record(shape) => format!("one {}", shape.entity),
            Kind::Records(shape) => format!("a list of {}", shape.entity),
            Kind::Other => "something else".into(),
        }
    }

    /// Whether records nest inside it: a record or list of records among its fields.
    pub fn nests(&self) -> bool {
        match self {
            Kind::Record(shape) | Kind::Records(shape) => shape.fields.iter().any(|f| matches!(f.kind, Kind::Record(_) | Kind::Records(_))),
            _ => false,
        }
    }
}

impl Shape {
    fn from_tree(tree: &Tree) -> Result<Shape, String> {
        let entity = tree.get("entity").and_then(Tree::as_text).ok_or("a record kind names its 'entity'")?.to_string();
        let fields = match tree.get("fields") {
            Some(Tree::List(fields)) => fields
                .iter()
                .map(|f| {
                    let name = f.get("name").and_then(Tree::as_text).ok_or("a field has a 'name'")?.to_string();
                    Ok(Field { name, kind: Kind::from_tree(f)? })
                })
                .collect::<Result<Vec<_>, String>>()?,
            _ => return Err(format!("the record kind of {entity} lists its 'fields'")),
        };
        let base = match tree.get("base") {
            Some(base @ Tree::Object(_)) => base.clone(),
            _ => return Err(format!("the record kind of {entity} has a 'base', a new record")),
        };
        Ok(Shape { entity, fields, base })
    }
}
