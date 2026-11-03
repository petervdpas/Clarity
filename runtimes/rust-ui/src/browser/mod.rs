//! The browser host (DESIGN 7.9, round B): the same library compiled once
//! to WebAssembly (wasm32-unknown-emscripten, Skia from skia-safe's prebuilt
//! binaries), running in a Web Worker and drawing with Skia through WebGL2
//! on an OffscreenCanvas the page hands it. The program stays on the server
//! the page came from: every command of the library (`Program`) is a
//! synchronous POST of `{"command", "args"}` to the server's wire, which a
//! worker may make, answered `{"value"}` or `{"error"}` as the desktop's
//! program callback answers it (`program_wire`).
//!
//! The worker and page scripts are carried in the repository
//! (runtimes/rust-ui-browser/js), not generated; the crate
//! runtimes/rust-ui-browser links this host into `clarity-ui.js` and
//! `clarity-ui.wasm` and gives the worker its entry points. What the page
//! sends (pointer, wheel, keys, text, an input method, size and device
//! pixel ratio, light or dark, pasted text, a chosen file) reaches the
//! screen through `host`; what only the page can do goes out as messages
//! (`page`).
//!
//! A browser build has no threads (its Skia is built without them). A
//! long call (a program that `watches`) is therefore not waited for: its
//! handler's request goes out without waiting, its arguments naming the
//! call's `watch`, and the worker draws on; the bar asks `watch_progress`
//! and Stop sends `stop_watch` as requests of their own, which the server
//! answers while the handler runs; the answer is taken when it comes, and
//! `end_watch` follows. Every other request waits for its answer.
//!
//! The person's choices (language, theme, columns) are kept by the page in
//! the browser's storage, as the desktop keeps them in the program's folder.

pub mod host;
pub mod input;
pub mod page;

#[cfg(target_os = "emscripten")]
pub mod web;

use serde_json::Value;

use crate::app_files::AppFiles;

fn leak(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

fn pairs(value: Option<&Value>) -> &'static [(&'static str, &'static str)] {
    let list: Vec<(&'static str, &'static str)> = value
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(name, text)| {
            let text = match text {
                Value::String(text) => text.clone(),
                Value::Null => return None,
                other => other.to_string(),
            };
            Some((leak(name), leak(&text)))
        })
        .collect();
    Box::leak(list.into_boxed_slice())
}

/// The program's files as the server serves them (`files.json`):
/// `{"title", "width", "height", "windows": {name: markup}, "apps": {name:
/// markup}, "themes": {"shop.light": json}, "model": json, "resources": [name]}`,
/// a theme and the model as a JSON value or as its text. A resource's bytes
/// come from `resource` by its name (fetched from the server's
/// `resources/<name>`); one it cannot give is left out. Kept for the run.
pub fn files_from_json(files: &Value, resource: &dyn Fn(&str) -> Option<Vec<u8>>) -> AppFiles {
    let text = |name: &str| -> &'static str {
        match files.get(name) {
            Some(Value::String(text)) => leak(text),
            Some(Value::Null) | None => "",
            Some(other) => leak(&other.to_string()),
        }
    };
    let number = |name: &str| files.get(name).and_then(Value::as_f64).unwrap_or(0.0);
    let resources: Vec<(&'static str, &'static [u8])> = files
        .get("resources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|name| Some((leak(name), &*Box::leak(resource(name)?.into_boxed_slice()))))
        .collect();
    AppFiles {
        windows: pairs(files.get("windows")),
        apps: pairs(files.get("apps")),
        themes: pairs(files.get("themes")),
        model: text("model"),
        resources: Box::leak(resources.into_boxed_slice()),
        title: text("title"),
        width: number("width"),
        height: number("height"),
    }
}
