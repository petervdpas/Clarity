//! What the program knows of its fields and types (DESIGN 7.9), as it gives
//! them in plain JSON (`Program::facts_json`, the runtime's
//! `fields::Facts::to_json`): each window's values and their types, each
//! record's fields, the facts of every entity field keyed "Entity.field",
//! and each enum's values. Read once per window; the parts find a field by
//! its path in the window's data, as the C# host does.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::facts::format_written;
use crate::parts::FieldFacts;

/// A program's facts, read from its JSON.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProgramFacts {
    windows: BTreeMap<String, BTreeMap<String, String>>,
    records: BTreeMap<String, BTreeMap<String, String>>,
    fields: BTreeMap<String, FieldFacts>,
    enums: BTreeMap<String, Vec<String>>,
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

/// `{"Main": {"orders": "list<Order>"}}` as names and their types.
fn typed(value: &Value) -> BTreeMap<String, BTreeMap<String, String>> {
    let Some(all) = value.as_object() else { return BTreeMap::new() };
    all.iter()
        .map(|(name, types)| {
            let types = types.as_object().map(|t| t.iter().filter_map(|(n, ty)| Some((n.clone(), text(ty)?))).collect()).unwrap_or_default();
            (name.clone(), types)
        })
        .collect()
}

/// A field's facts as the program writes them, as the parts read them.
fn field_facts(value: &Value) -> FieldFacts {
    let format = value.get("format").filter(|f| f.is_object()).map(|f| {
        let kind = f.get("kind").and_then(Value::as_str).unwrap_or_default();
        format_written(kind, f.get("scale").and_then(Value::as_u64), f.get("format").and_then(Value::as_str)).unwrap_or_default()
    });
    let named = |name: &str| value.get(name).and_then(text);
    FieldFacts {
        label: named("label").unwrap_or_default(),
        format,
        required: value.get("required").and_then(Value::as_bool).unwrap_or(false),
        min: named("min"),
        max: named("max"),
        refers: named("refers"),
        parent: named("parent"),
        color: value.get("color").and_then(Value::as_bool).unwrap_or(false),
        help: None,
    }
}

/// The type of a list's rows: `Order` of `list<Order>`.
fn row_type(ty: &str) -> Option<&str> {
    ty.strip_prefix("list<").and_then(|t| t.strip_suffix('>'))
}

/// A path's names, each with whether a place follows it: `orders[0].total`
/// is `orders` (placed) and `total`.
fn names(path: &str) -> Vec<(&str, bool)> {
    path.split('.')
        .map(|step| match step.find('[') {
            Some(at) => (&step[..at], true),
            None => (step, false),
        })
        .collect()
}

impl ProgramFacts {
    /// The facts in `json`; a program that carries none (null) knows none.
    pub fn read(json: &Value) -> ProgramFacts {
        ProgramFacts {
            windows: typed(&json["windows"]),
            records: typed(&json["records"]),
            fields: json["fields"].as_object().map(|all| all.iter().map(|(key, facts)| (key.clone(), field_facts(facts))).collect()).unwrap_or_default(),
            enums: json["enums"]
                .as_object()
                .map(|all| all.iter().map(|(name, values)| (name.clone(), values.as_array().map(|v| v.iter().filter_map(text).collect()).unwrap_or_default())).collect())
                .unwrap_or_default(),
        }
    }

    /// The type of the value at `path` in the data of `window`: a row of a
    /// list by its place (`orders[0]`) is the list's record; none for a path
    /// the window's data does not have.
    pub fn type_of(&self, window: &str, path: &str) -> Option<&str> {
        let mut steps = names(path).into_iter();
        let (first, placed) = steps.next()?;
        let mut ty = self.windows.get(window)?.get(first)?.as_str();
        if placed {
            ty = row_type(ty)?;
        }
        for (name, placed) in steps {
            // A field of a list's rows is reached without a place too: `orders.total`.
            let record = row_type(ty).unwrap_or(ty);
            ty = self.records.get(record)?.get(name)?.as_str();
            if placed {
                ty = row_type(ty)?;
            }
        }
        Some(ty)
    }

    /// The facts of the field at `path` in the data of `window`, by the
    /// record it is in and its name: `order.total`, `orders[0].total` and
    /// `orders.total` are all "Order.total". None for a value that is no
    /// entity's field.
    pub fn field(&self, window: &str, path: &str) -> Option<FieldFacts> {
        let (owner, name) = path.rsplit_once('.')?;
        let ty = self.type_of(window, owner)?;
        let record = row_type(ty).unwrap_or(ty);
        // The entity is an open set of the program's, so its key is made here.
        let key = format!("{record}.{name}");
        self.fields.get(&key).cloned()
    }

    /// The values of the enum `name`, as written; none for a name that is no enum.
    pub fn enum_values(&self, name: &str) -> &[String] {
        self.enums.get(name).map(Vec::as_slice).unwrap_or(&[])
    }
}
