//! The edit bus against the cases every runtime shares (conformance/edit-bus.json).

use clarity_runtime::edits::EditBus;
use clarity_runtime::Error;
use serde_json::{Map, Value};

fn object(value: &Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

#[test]
fn the_edit_bus_keeps_the_record_as_every_runtime_does() {
    let file: Value = serde_json::from_str(include_str!("../../../conformance/edit-bus.json")).unwrap();
    for c in file["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let mut bus = EditBus::default();
        let mut data = object(&c["start"]);
        bus.reset(&data);
        for (i, s) in c["steps"].as_array().unwrap().iter().enumerate() {
            let steps = |m: &Value| -> Vec<u64> { m["steps"].as_array().map(|a| a.iter().filter_map(Value::as_u64).collect()).unwrap_or_default() };
            if let Some(m) = s.get("edit") {
                data = object(&m["data"]);
                bus.edit(m["part"].as_str().unwrap(), m["scope"].as_str().unwrap(), &data, &steps(m));
            } else if let Some(m) = s.get("drag") {
                data = object(&m["data"]);
                bus.drag(m["part"].as_str().unwrap(), m["scope"].as_str().unwrap(), &steps(m));
            } else if let Some(m) = s.get("reply") {
                data = object(&m["data"]);
                bus.reset(&data);
            } else {
                let back = s.get("undo").is_some();
                let scope = s.get("undo").or(s.get("redo")).and_then(Value::as_str).unwrap();
                let refuse = s.get("refused").is_some();
                let mut taken = vec![];
                let after = bus.undo(scope, &data, back, |steps, _| {
                    if refuse {
                        return Err(Error::Invalid("changed since".into()));
                    }
                    taken = steps.to_vec();
                    Ok(())
                });
                if s.get("nothing").is_some() {
                    assert!(matches!(after, Ok(None)), "{name}, step {}: something to take back", i + 1);
                } else if refuse {
                    assert!(after.is_err(), "{name}, step {}: not refused", i + 1);
                } else {
                    let after = after.unwrap().unwrap_or_else(|| panic!("{name}, step {}: nothing taken back", i + 1));
                    assert_eq!(after, object(&s["data"]), "{name}, step {}", i + 1);
                    assert_eq!(taken, steps(s), "{name}, step {}: steps", i + 1);
                    data = after;
                }
            }
        }
    }
}
