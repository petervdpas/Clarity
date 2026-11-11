//! The program's calls as JSON, in the shapes of the web server's wire
//! (DESIGN 7.3, clarity-gen's `served`): what each `Program` call carries
//! becomes the arguments of its command, and what the program answers is
//! read back. The runtime's types that have no JSON of their own (a move, a
//! range move, a canvas's arguments, an engine's input) are written here.

use clarity_runtime::canvas::CanvasArgs;
use clarity_runtime::list_fields::ItemKind;
use clarity_runtime::program::{CanvasEdit, Edit, ItemEdit, RangeEdit, Reorder, StepPress, Started, Unconnected};
use clarity_runtime::ranges::{RangeKey, RangeMove};
use clarity_runtime::reorder::{DropAt, Move, MoveKey};
use clarity_runtime::screen::{Drawn, Hears, ScreenInput, ScreenTell};
use clarity_runtime::ui::{Text, WindowState};
use serde_json::{json, Map, Value};

fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

fn state(state: &WindowState) -> Value {
    serde_json::to_value(state).unwrap_or(Value::Null)
}

/// `dispatch_event`'s arguments.
pub fn dispatch(at: &WindowState, handler: &str) -> Map<String, Value> {
    object(json!({ "state": state(at), "handler": handler }))
}

/// `answer_event`'s arguments: the answer to a question goes to `handler` with `values`.
pub fn answer(at: &WindowState, handler: &str, values: &[Value]) -> Map<String, Value> {
    object(json!({ "state": state(at), "handler": handler, "values": values }))
}

/// `edit_event`'s arguments.
pub fn edit(at: &WindowState, edit: &Edit) -> Map<String, Value> {
    object(json!({
        "state": state(at),
        "handler": edit.handler,
        "part": edit.part,
        "scope": edit.scope,
        "ends": edit.ends,
        "readonly": edit.readonly,
        "path": edit.path,
        "editable": edit.editable,
    }))
}

fn drop_at(at: DropAt) -> &'static str {
    match at {
        DropAt::Before => "before",
        DropAt::After => "after",
        DropAt::Into => "into",
    }
}

fn move_key(key: MoveKey) -> &'static str {
    match key {
        MoveKey::Up => "up",
        MoveKey::Down => "down",
        MoveKey::Left => "left",
        MoveKey::Right => "right",
    }
}

/// A row's move as the wire's `move`.
pub fn moved(moved: &Move) -> Value {
    match *moved {
        Move::Drag { row, at, target } => json!({ "row": row, "where": drop_at(at), "target": target }),
        Move::Key { row, key } => json!({ "row": row, "key": move_key(key) }),
    }
}

/// `reorder_event`'s arguments.
pub fn reorder(at: &WindowState, reorder: &Reorder) -> Map<String, Value> {
    object(json!({
        "state": state(at),
        "part": reorder.part,
        "scope": reorder.scope,
        "list": reorder.list,
        "by": reorder.by,
        "parent": reorder.parent,
        "move": moved(&reorder.moved),
        "handler": reorder.handler,
        "readonly": reorder.readonly,
    }))
}

fn item_kind(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Text => "text",
        ItemKind::Integer => "integer",
        ItemKind::Decimal => "decimal",
        ItemKind::Date => "date",
        ItemKind::Enum => "enum",
    }
}

/// `item_event`'s arguments.
pub fn item(at: &WindowState, item: &ItemEdit) -> Map<String, Value> {
    object(json!({
        "state": state(at),
        "part": item.part,
        "scope": item.scope,
        "path": item.path,
        "item": item_kind(item.item),
        "values": item.values,
        "typed": item.typed,
        "handler": item.handler,
        "readonly": item.readonly,
    }))
}

fn range_key(key: RangeKey) -> &'static str {
    match key {
        RangeKey::Up => "up",
        RangeKey::Down => "down",
        RangeKey::PageUp => "pageUp",
        RangeKey::PageDown => "pageDown",
        RangeKey::Home => "home",
        RangeKey::End => "end",
    }
}

/// `range_event`'s arguments: the move as one of `key`, `at` and `typed`.
pub fn range(at: &WindowState, range: &RangeEdit) -> Map<String, Value> {
    let mut args = object(json!({
        "state": state(at),
        "part": range.part,
        "scope": range.scope,
        "path": range.path,
        "min": range.range.min,
        "max": range.range.max,
        "step": range.range.step,
        "whole": range.whole,
        "handler": range.handler,
        "readonly": range.readonly,
    }));
    match &range.moved {
        RangeMove::Key(key) => args.insert("key".into(), json!(range_key(*key))),
        RangeMove::At(place) => args.insert("at".into(), json!(place)),
        RangeMove::Typed(typed) => args.insert("typed".into(), json!(typed)),
    };
    args
}

fn canvas_args(canvas: &CanvasArgs) -> Map<String, Value> {
    object(json!({
        "part": canvas.part,
        "mode": canvas.mode,
        "list": canvas.list,
        "at": canvas.at,
        "path": canvas.path,
        "units": canvas.units,
        "grid": canvas.grid,
        "snap": canvas.snap,
    }))
}

/// `canvas_event`'s arguments.
pub fn canvas(at: &WindowState, canvas: &CanvasEdit) -> Map<String, Value> {
    let mut args = canvas_args(&canvas.canvas);
    args.insert("state".into(), state(at));
    args.insert("action".into(), Value::Object(canvas.action.clone()));
    args.insert("scope".into(), json!(canvas.scope));
    args.insert("handler".into(), json!(canvas.handler));
    args.insert("readonly".into(), json!(canvas.readonly || canvas.canvas.readonly));
    args
}

/// `step_event`'s arguments.
pub fn step(at: &WindowState, step: &StepPress) -> Map<String, Value> {
    object(json!({
        "state": state(at),
        "part": step.part,
        "button": step.button,
        "steps": step.steps,
        "next": step.next,
        "finish": step.finish,
    }))
}

/// What `started` answers: the main window, what to show when the databases
/// did not open, and the sign-in window when people sign in first.
pub fn started(value: &Value) -> Started {
    let main = value.get("main").and_then(Value::as_str).unwrap_or("Main").to_string();
    let unconnected = value.get("unconnected").filter(|u| u.is_object()).map(|u| Unconnected {
        handler: u.get("handler").and_then(Value::as_str).map(str::to_string),
        window: u.get("window").and_then(Value::as_str).map(str::to_string),
        problem: u.get("problem").cloned().and_then(|p| serde_json::from_value::<Text>(p).ok()).unwrap_or_default(),
    });
    let signin = value.get("signin").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
    Started { main, unconnected, signin }
}

/// An engine's input as JSON: `{"pointer":{kind,x,y}}`, `{"key":{kind,name}}`, `{"hear":{name,values}}`.
pub fn screen_input(input: &ScreenInput) -> Value {
    match input {
        ScreenInput::Pointer { kind, x, y } => json!({ "pointer": { "kind": kind, "x": x, "y": y } }),
        ScreenInput::Key { kind, name } => json!({ "key": { "kind": kind, "name": name } }),
        ScreenInput::Hear { name, values } => json!({ "hear": { "name": name, "values": values } }),
    }
}

/// An engine's input as the canvas gives it (clarity-ui's own type), as JSON.
pub fn ui_screen_input(input: &crate::parts::data::canvas::engine::ScreenInput) -> Value {
    use crate::parts::data::canvas::engine::ScreenInput as Ui;
    match input {
        Ui::Pointer { kind, x, y } => json!({ "pointer": { "kind": kind, "x": x, "y": y } }),
        Ui::Key { kind, name } => json!({ "key": { "kind": kind, "name": name } }),
        Ui::Hear { name, values } => json!({ "hear": { "name": name, "values": values } }),
    }
}

/// An engine's input read from JSON; none for one that is no input.
pub fn screen_input_of(value: &Value) -> Option<ScreenInput> {
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    if let Some(p) = value.get("pointer") {
        return Some(ScreenInput::Pointer { kind: text(p, "kind")?, x: p.get("x")?.as_i64()?, y: p.get("y")?.as_i64()? });
    }
    if let Some(k) = value.get("key") {
        return Some(ScreenInput::Key { kind: text(k, "kind")?, name: text(k, "name")? });
    }
    if let Some(h) = value.get("hear") {
        return Some(ScreenInput::Hear { name: text(h, "name")?, values: h.get("values").and_then(Value::as_array).cloned().unwrap_or_default() });
    }
    None
}

fn tells_of(value: &Value) -> Vec<(String, Vec<Value>)> {
    value
        .get("tells")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|t| Some((t.get("tell")?.as_str()?.to_string(), t.get("values").and_then(Value::as_array).cloned().unwrap_or_default())))
        .collect()
}

fn shapes_of(value: &Value) -> Vec<Value> {
    value.get("shapes").and_then(Value::as_array).cloned().unwrap_or_default()
}

/// A frame as the runtime has it, read from `{"shapes","tells"}`.
pub fn drawn(value: &Value) -> Drawn {
    Drawn { shapes: shapes_of(value), tells: tells_of(value).into_iter().map(|(tell, values)| ScreenTell { tell, values }).collect() }
}

/// A frame as clarity-ui has it, read from `{"shapes","tells"}`.
pub fn ui_drawn(value: &Value) -> crate::parts::data::canvas::engine::Drawn {
    use crate::parts::data::canvas::engine::{Drawn as UiDrawn, ScreenTell as UiTell};
    UiDrawn { shapes: shapes_of(value), tells: tells_of(value).into_iter().map(|(tell, values)| UiTell { tell, values }).collect() }
}

/// What an engine takes, read from `{"pointer","keys"}`.
pub fn hears(value: &Value) -> Hears {
    Hears { pointer: value.get("pointer").and_then(Value::as_bool).unwrap_or(false), keys: value.get("keys").and_then(Value::as_bool).unwrap_or(false) }
}
