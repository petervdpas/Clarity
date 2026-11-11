//! A mapping (docs/FORMATS-PLAN.md, step 4): how one kind of file is laid
//! over an entity, configured per case, given in the options every stack
//! passes. Valid for every format beside its own options:
//! - `"at": "shop/orders/order"`, where the records are;
//! - `"fields": {"number": "@number", "customer": "customer/name"}`, where
//!   a field is, relative to its record (a field not listed is found by its name);
//! - `"nested": {"lines": {"at": "lines/line", "fields": {...}, "nested": {...},
//!   "parent": "order"}}`, a field holding records inside, with their own
//!   paths and the field of the inner entity that names the record around
//!   it (a file source's link column), where a format nests.
//!
//! - `"check": "<xs:schema ...>"`, the text of a schema the file is checked
//!   against before it is fitted (check.rs), where a format has schema kinds.

use crate::path::{Path, Why};
use crate::tree::Tree;
use crate::{FormatInfo, Problem, ProblemKind};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mapping {
    pub at: Option<Path>,
    pub fields: Vec<(String, Path)>,
    pub nested: Vec<(String, Nested)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Nested {
    /// Where the records are, relative to the record around them.
    pub at: Option<Path>,
    pub map: Mapping,
    pub parent: Option<String>,
}

/// The generic keys a format takes beside its own options.
pub fn keys(info: &FormatInfo) -> &'static [&'static str] {
    match (info.nests, info.schema_kinds.is_empty()) {
        (true, true) => &["fields", "nested"],
        (true, false) => &["fields", "nested", "check"],
        (false, true) => &["fields"],
        (false, false) => &["fields", "check"],
    }
}

fn bad(text: String) -> Problem {
    Problem::new(ProblemKind::BadOptions, text)
}

impl Mapping {
    /// The mapping the options give.
    pub fn of(options: &[(String, Tree)], info: &FormatInfo) -> Result<Mapping, Problem> {
        let get = |name: &str| options.iter().find(|(n, _)| n == name).map(|(_, v)| v);
        let at = get("at").and_then(Tree::as_text).filter(|a| !a.is_empty()).map(Path::at);
        Mapping::from(at, get("fields"), get("nested"), info)
    }

    fn from(at: Option<Path>, fields: Option<&Tree>, nested: Option<&Tree>, info: &FormatInfo) -> Result<Mapping, Problem> {
        let name = info.name;
        let mut map = Mapping { at, ..Mapping::default() };
        match fields {
            None | Some(Tree::Null) => {}
            Some(Tree::Object(pairs)) => {
                for (field, path) in pairs {
                    let Some(text) = path.as_text() else { return Err(bad(format!("the option 'fields' of {name} is an object of field names and paths, like {{\"number\": \"@number\"}}"))) };
                    let path = Path::field(text, info.nodes, info.attributes).map_err(|why| match why {
                        Why::NoPath => bad(format!("the path '{text}' of field '{field}' is no {} path", info.title)),
                        Why::Attribute => bad(format!("the path '{text}' of field '{field}' names an attribute, which {} does not have", info.title)),
                    })?;
                    map.fields.push((field.clone(), path));
                }
            }
            Some(_) => return Err(bad(format!("the option 'fields' of {name} is an object of field names and paths, like {{\"number\": \"@number\"}}"))),
        }
        let shape = || bad(format!("the option 'nested' of {name} is an object of fields holding records, each with its at, fields, nested and parent"));
        match nested {
            None | Some(Tree::Null) => {}
            Some(Tree::Object(pairs)) => {
                for (field, inner) in pairs {
                    let Tree::Object(keys) = inner else { return Err(shape()) };
                    if keys.iter().any(|(k, _)| !matches!(k.as_str(), "at" | "fields" | "nested" | "parent")) {
                        return Err(shape());
                    }
                    let text = |key: &str| -> Result<Option<String>, Problem> {
                        match inner.get(key) {
                            None | Some(Tree::Null) => Ok(None),
                            Some(Tree::Text(t)) => Ok(Some(t.clone())),
                            Some(_) => Err(shape()),
                        }
                    };
                    let at = text("at")?.filter(|a| !a.is_empty()).map(|a| Path::at(&a));
                    let parent = text("parent")?;
                    let inner_map = Mapping::from(None, inner.get("fields"), inner.get("nested"), info)?;
                    map.nested.push((field.clone(), Nested { at, map: inner_map, parent }));
                }
            }
            Some(_) => return Err(shape()),
        }
        Ok(map)
    }

    /// Where a field is, when the mapping says.
    pub fn path(&self, field: &str) -> Option<&Path> {
        self.fields.iter().find(|(f, _)| f == field).map(|(_, p)| p)
    }

    /// The records inside a field, when the mapping says.
    pub fn nested(&self, field: &str) -> Option<&Nested> {
        self.nested.iter().find(|(f, _)| f == field).map(|(_, n)| n)
    }

    /// The mapping inside a field: its nested one, else none.
    pub fn inner(&self, field: &str) -> Mapping {
        self.nested(field).map(|n| n.map.clone()).unwrap_or_default()
    }
}
