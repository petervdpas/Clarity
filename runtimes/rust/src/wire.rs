//! The program over the web server's wire (DESIGN 7.3 and 7.9): Clarity.UI
//! in the browser asks the program on the server each call of
//! [`Program`] as a command with its arguments as JSON, and this answers it
//! with the program: `{"value": ...}` or `{"error": Text}`. The commands and
//! their arguments are the ones Clarity.UI's `program_wire` sends.

use base64::Engine as _;
use serde::de::DeserializeOwned;
use serde_json::{json, Map, Value};

use crate::program::{self, Answer, Program};
use crate::screen::{Drawn, ScreenInput};
use crate::ui::{Text, WindowState};

fn arg<T: DeserializeOwned>(args: &Value, name: &str) -> Answer<T> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null)).map_err(|err| Text::plain(format!("{name}: {err}")))
}

fn given<T: serde::Serialize>(answer: Answer<T>) -> Value {
    match answer {
        Ok(value) => json!({ "value": value }),
        Err(error) => json!({ "error": error }),
    }
}

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn id(args: &Value) -> u64 {
    args.get("id").and_then(Value::as_u64).unwrap_or(0)
}

/// What `started` answers: the main window, and what to show while the databases are not open.
fn started(started: program::Started) -> Value {
    let mut out = json!({ "main": started.main });
    if let Some(unconnected) = started.unconnected {
        out["unconnected"] = json!({ "handler": unconnected.handler, "window": unconnected.window, "problem": unconnected.problem });
    }
    if let Some(signin) = started.signin {
        out["signin"] = json!(signin);
    }
    out
}

/// An engine's input read from the wire: `{"pointer":{kind,x,y}}`, `{"key":{kind,name}}`, `{"hear":{name,values}}`.
pub fn screen_input(value: &Value) -> Option<ScreenInput> {
    let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    if let Some(p) = value.get("pointer") {
        return Some(ScreenInput::Pointer { kind: text(p, "kind")?, x: p.get("x")?.as_i64()?, y: p.get("y")?.as_i64()? });
    }
    if let Some(k) = value.get("key") {
        return Some(ScreenInput::Key { kind: text(k, "kind")?, name: text(k, "name")? });
    }
    let h = value.get("hear")?;
    Some(ScreenInput::Hear { name: text(h, "name")?, values: h.get("values").and_then(Value::as_array).cloned().unwrap_or_default() })
}

/// A frame on the wire: `{"shapes", "tells": [{"tell", "values"}]}`.
pub fn drawn(drawn: &Drawn) -> Value {
    let tells: Vec<Value> = drawn.tells.iter().map(|t| json!({ "tell": t.tell, "values": t.values })).collect();
    json!({ "shapes": drawn.shapes, "tells": tells })
}

/// Answers `command` with `args` (a JSON object) by `program`. A command
/// that panics answers its message as the error, and the program goes on.
/// How a program across the wire asks the agent its command runs for
/// (Clarity.UI's `clarity_agent_ask`); set once by the host that has it.
pub static AGENT_ASK: std::sync::OnceLock<fn(&str, &str) -> Option<String>> = std::sync::OnceLock::new();

pub fn answer<P: Program + ?Sized>(program: &P, command: &str, args: Value) -> Value {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| answered(program, command, &args))) {
        Ok(value) => value,
        Err(panic) => {
            let why = panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_else(|| format!("{command} stopped"));
            json!({ "error": Text::plain(why) })
        }
    }
}

/// Runs a handler's call under the watch its request names (`"watch": id`),
/// on this request's thread: how far it is and Stop come by other requests
/// meanwhile, and a Stop that came first holds. Without one it runs unwatched.
fn watched<P: Program + ?Sized>(app: &P, args: &Value, run: impl FnOnce() -> Answer<crate::ui::Reply>) -> Value {
    let Some(watch) = args.get("watch").and_then(Value::as_u64) else { return given(run()) };
    struct Ends<'a, P: Program + ?Sized>(&'a P, u64);
    impl<P: Program + ?Sized> Drop for Ends<'_, P> {
        fn drop(&mut self) {
            self.0.end_watch(self.1);
        }
    }
    app.start_watch(watch);
    let _ends = Ends(app, watch);
    given(run())
}

fn answered<P: Program + ?Sized>(app: &P, command: &str, args: &Value) -> Value {
    let args = args.clone();
    match command {
        "started" => json!({ "value": started(app.started()) }),
        "start_app" => given(app.start_app()),
        "connect" => given(app.connect()),
        "signin" => given((|| {
            let (strategy, inputs): (String, Option<Map<String, Value>>) = (arg(&args, "strategy")?, arg(&args, "inputs")?);
            app.signin(&strategy, inputs.unwrap_or_default())
        })()),
        "signout" => given(app.signout()),
        "open_window" => given((|| {
            let (window, with): (String, Option<Map<String, Value>>) = (arg(&args, "window")?, arg(&args, "with")?);
            app.open_window(&window, with.unwrap_or_default())
        })()),
        "close_window" => given((|| app.close_window(arg(&args, "state")?))()),
        "return_to" => given((|| app.return_to(arg(&args, "state")?))()),
        "dispatch_event" => watched(app, &args, || {
            let (state, handler): (WindowState, String) = (arg(&args, "state")?, arg(&args, "handler")?);
            app.dispatch_event(state, &handler)
        }),
        "edit_event" => given((|| {
            let readonly: Option<bool> = arg(&args, "readonly")?;
            let edit = program::Edit { handler: arg(&args, "handler")?, part: arg(&args, "part")?, scope: arg(&args, "scope")?, ends: arg(&args, "ends")?, readonly: readonly.unwrap_or_default(), path: arg(&args, "path")?, editable: arg(&args, "editable")? };
            app.edit_event(arg(&args, "state")?, edit)
        })()),
        "undo_edit" => given((|| {
            let (state, scope, back): (WindowState, String, bool) = (arg(&args, "state")?, arg(&args, "scope")?, arg(&args, "back")?);
            app.undo_edit(state, &scope, back)
        })()),
        "reorder_event" => given((|| {
            let readonly: Option<bool> = arg(&args, "readonly")?;
            let reorder = program::Reorder { part: arg(&args, "part")?, scope: arg(&args, "scope")?, list: arg(&args, "list")?, by: arg(&args, "by")?, parent: arg(&args, "parent")?, moved: arg(&args, "move")?, handler: arg(&args, "handler")?, readonly: readonly.unwrap_or_default() };
            app.reorder_event(arg(&args, "state")?, reorder)
        })()),
        "item_event" => given((|| {
            let values: Option<Vec<String>> = arg(&args, "values")?;
            let readonly: Option<bool> = arg(&args, "readonly")?;
            let item = program::ItemEdit { part: arg(&args, "part")?, scope: arg(&args, "scope")?, path: arg(&args, "path")?, item: arg(&args, "item")?, values: values.unwrap_or_default(), typed: arg(&args, "typed")?, handler: arg(&args, "handler")?, readonly: readonly.unwrap_or_default() };
            app.item_event(arg(&args, "state")?, item)
        })()),
        "range_event" => given((|| {
            let range = crate::ranges::Range { min: arg(&args, "min")?, max: arg(&args, "max")?, step: arg(&args, "step")? };
            let moved = crate::ranges::RangeMove::of(arg(&args, "key")?, arg(&args, "at")?, arg(&args, "typed")?).map_err(|err| Text::plain(err.to_string()))?;
            let readonly: Option<bool> = arg(&args, "readonly")?;
            let range = program::RangeEdit { part: arg(&args, "part")?, scope: arg(&args, "scope")?, path: arg(&args, "path")?, range, whole: arg(&args, "whole")?, moved, handler: arg(&args, "handler")?, readonly: readonly.unwrap_or_default() };
            app.range_event(arg(&args, "state")?, range)
        })()),
        "canvas_event" => given((|| {
            let (list, path, grid, snap): (Option<String>, Option<String>, Option<i64>, Option<bool>) = (arg(&args, "list")?, arg(&args, "path")?, arg(&args, "grid")?, arg(&args, "snap")?);
            let canvas = crate::canvas::CanvasArgs { part: arg(&args, "part")?, mode: arg(&args, "mode")?, list: list.unwrap_or_default(), at: arg(&args, "at")?, path: path.unwrap_or_default(), units: arg(&args, "units")?, grid: grid.unwrap_or_default(), snap: snap.unwrap_or_default(), readonly: false };
            let readonly: Option<bool> = arg(&args, "readonly")?;
            let canvas = program::CanvasEdit { canvas, action: arg(&args, "action")?, scope: arg(&args, "scope")?, handler: arg(&args, "handler")?, readonly: readonly.unwrap_or_default() };
            app.canvas_event(arg(&args, "state")?, canvas)
        })()),
        "step_event" => given((|| {
            let step = program::StepPress { part: arg(&args, "part")?, button: arg(&args, "button")?, steps: arg(&args, "steps")?, next: arg(&args, "next")?, finish: arg(&args, "finish")? };
            app.step_event(arg(&args, "state")?, step)
        })()),
        "view_event" => given((|| {
            let (state, part, action): (WindowState, String, Map<String, Value>) = (arg(&args, "state")?, arg(&args, "part")?, arg(&args, "action")?);
            app.view_event(state, &part, action)
        })()),
        "answer_event" => watched(app, &args, || {
            let (state, handler, values): (WindowState, String, Vec<Value>) = (arg(&args, "state")?, arg(&args, "handler")?, arg(&args, "values")?);
            app.answer_event(state, &handler, values)
        }),
        "ref_options" => given((|| app.ref_options(&arg::<String>(&args, "entity")?))()),
        "field_values" => given((|| {
            let (entity, field): (String, String) = (arg(&args, "entity")?, arg(&args, "field")?);
            app.field_values(&entity, &field)
        })()),
        "find_options" => given((|| {
            let (kind, entity, field, show, typed): (String, String, Option<String>, Option<Vec<String>>, String) =
                (arg(&args, "kind")?, arg(&args, "entity")?, arg(&args, "field")?, arg(&args, "show")?, arg(&args, "typed")?);
            app.find_options(&kind, &entity, field.as_deref(), &show.unwrap_or_default(), &typed)
        })()),
        // A record's file or picture, and a store's file, as base64.
        "field_bytes" => given((|| {
            let (entity, id, field): (String, i64, String) = (arg(&args, "entity")?, arg(&args, "id")?, arg(&args, "field")?);
            Ok(app.field_bytes(&entity, id, &field)?.map(|bytes| base64(&bytes)))
        })()),
        "store_file" => given((|| {
            let (store, path): (String, String) = (arg(&args, "store")?, arg(&args, "path")?);
            Ok(base64(&app.store_file(&store, &path)?))
        })()),
        "read_settings" => json!({ "value": app.read_settings() }),
        "keep_settings" => given((|| app.keep_settings(arg::<Option<Map<String, Value>>>(&args, "settings")?.unwrap_or_default()))()),
        "set_language" => given((|| {
            let language: Option<String> = arg(&args, "language")?;
            app.set_language(language.as_deref());
            Ok(Value::Null)
        })()),
        "watches" => json!({ "value": app.watches() }),
        "start_watch" | "stop_watch" | "end_watch" => {
            match command {
                "start_watch" => app.start_watch(id(&args)),
                "stop_watch" => app.stop_watch(id(&args)),
                _ => app.end_watch(id(&args)),
            }
            json!({ "value": null })
        }
        "splash_status" => json!({ "value": app.splash_status() }),
        "watch_progress" => json!({ "value": app.watch_progress(id(&args)).map(|(done, all)| json!([done, all])) }),
        "facts" => json!({ "value": app.facts_json() }),
        // The commands for AI agents while the app runs (docs/ledger/app-mcp.md).
        "agents" => json!({ "value": app.agents() }),
        "agent_undo" => given((|| app.agent_undo(arg(&args, "step")?))()),
        "agent_call" => given((|| {
            let line: Vec<String> = arg(&args, "line")?;
            let mut ask = |question: &str, form: &str| AGENT_ASK.get().and_then(|ask| ask(question, form));
            Ok(app.agent_call(&line, &mut ask))
        })()),
        "engine_open" => match app.engine_open(&arg::<String>(&args, "name").unwrap_or_default()) {
            None => json!({ "value": null }),
            Some(Ok(id)) => json!({ "value": id }),
            Some(Err(why)) => json!({ "error": Text::plain(why) }),
        },
        "engine_hears" => {
            let hears = app.engine_hears(id(&args));
            json!({ "value": { "pointer": hears.pointer, "keys": hears.keys } })
        }
        "engine_frame" => {
            let input: Vec<ScreenInput> = args.get("input").and_then(Value::as_array).into_iter().flatten().filter_map(screen_input).collect();
            let ms = args.get("ms").and_then(Value::as_f64).unwrap_or(0.0);
            match app.engine_frame(id(&args), ms, input) {
                Ok(frame) => json!({ "value": drawn(&frame) }),
                Err(why) => json!({ "error": Text::plain(why) }),
            }
        }
        "engine_close" => {
            app.engine_close(id(&args));
            json!({ "value": null })
        }
        _ => json!({ "error": format!("there is no command {command}") }),
    }
}
