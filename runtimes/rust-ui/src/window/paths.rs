//! A value in a window's data by its path, as the markup binds it:
//! `order.status`, `tasks[1].done`, `shown`.

use serde_json::{Map, Value};

/// One step of a path: a field by its name, or a row by its place.
#[derive(Debug, Clone, PartialEq)]
enum Step {
    Field(String),
    Row(usize),
}

fn steps(path: &str) -> Option<Vec<Step>> {
    let mut steps = Vec::new();
    for part in path.split('.') {
        let (name, mut rest) = match part.find('[') {
            Some(i) => (&part[..i], &part[i..]),
            None => (part, ""),
        };
        if name.is_empty() && steps.is_empty() {
            return None;
        }
        if !name.is_empty() {
            steps.push(Step::Field(name.to_string()));
        }
        while let Some(inner) = rest.strip_prefix('[') {
            let end = inner.find(']')?;
            steps.push(Step::Row(inner[..end].trim().parse().ok()?));
            rest = &inner[end + 1..];
        }
        if !rest.is_empty() {
            return None;
        }
    }
    Some(steps)
}

/// The value at `path` in `data`; none when there is none.
pub fn read(data: &Map<String, Value>, path: &str) -> Option<Value> {
    let steps = steps(path)?;
    let (first, rest) = steps.split_first()?;
    let Step::Field(name) = first else { return None };
    let mut at = data.get(name)?;
    for step in rest {
        at = match step {
            Step::Field(field) => at.get(field)?,
            Step::Row(row) => at.get(*row)?,
        };
    }
    Some(at.clone())
}

/// Puts `value` at `path` in `data`: a record or field that is not there yet is made; false when the path cannot be.
pub fn write(data: &mut Map<String, Value>, path: &str, value: Value) -> bool {
    let Some(steps) = steps(path) else { return false };
    let Some((Step::Field(name), rest)) = steps.split_first() else { return false };
    if rest.is_empty() {
        data.insert(name.clone(), value);
        return true;
    }
    let mut at = data.entry(name.clone()).or_insert(Value::Null);
    for (i, step) in rest.iter().enumerate() {
        let last = i == rest.len() - 1;
        at = match step {
            Step::Field(field) => {
                if !at.is_object() {
                    *at = Value::Object(Map::new());
                }
                let record = at.as_object_mut().expect("made a record above");
                if last {
                    record.insert(field.clone(), value);
                    return true;
                }
                record.entry(field.clone()).or_insert(Value::Null)
            }
            Step::Row(row) => {
                let Some(rows) = at.as_array_mut() else { return false };
                let Some(item) = rows.get_mut(*row) else { return false };
                if last {
                    *item = value;
                    return true;
                }
                item
            }
        };
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn values_are_read_and_written_by_their_path() {
        let mut data = json!({ "order": { "note": "a" }, "tasks": [{ "done": false }, { "done": false }] }).as_object().unwrap().clone();
        assert_eq!(Some(json!("a")), read(&data, "order.note"));
        assert_eq!(Some(json!(false)), read(&data, "tasks[1].done"));
        assert_eq!(None, read(&data, "tasks[5].done"));
        assert_eq!(None, read(&data, "nothing"));
        assert!(write(&mut data, "tasks[1].done", json!(true)));
        assert!(write(&mut data, "order.note", json!("b")));
        assert!(write(&mut data, "shown", json!("x")));
        assert!(write(&mut data, "new.field", json!(1)));
        assert!(!write(&mut data, "tasks[9].done", json!(true)));
        assert_eq!(json!({ "order": { "note": "b" }, "tasks": [{ "done": false }, { "done": true }], "shown": "x", "new": { "field": 1 } }), Value::Object(data));
    }
}
