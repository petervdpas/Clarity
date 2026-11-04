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

/// The program's files a browser host was given, owned: the `AppFiles` made
/// of them borrow from here, and are given up with them. The IDE's designer
/// reloads the files after every change; each reload's are dropped once the
/// screen made of them is (`BrowserHost::reload`), so nothing builds up.
#[derive(Default)]
pub struct OwnedFiles {
    texts: Vec<Box<str>>,
    pairs: Vec<Box<[(&'static str, &'static str)]>>,
    resources: Option<Box<[(&'static str, &'static [u8])]>>,
    alive: std::sync::Arc<()>,
}

impl OwnedFiles {
    /// Lives as long as these files: a test sees them given up by it.
    pub fn alive(&self) -> std::sync::Weak<()> {
        std::sync::Arc::downgrade(&self.alive)
    }

    /// Keeps `text`; the text, borrowed for as long as these files are kept.
    fn keep(&mut self, text: &str) -> &'static str {
        let owned: Box<str> = text.into();
        // The box's text stays where it is when the box moves into the list.
        let borrowed: &'static str = unsafe { &*(owned.as_ref() as *const str) };
        self.texts.push(owned);
        borrowed
    }

    fn pairs(&mut self, value: Option<&Value>) -> &'static [(&'static str, &'static str)] {
        let mut list = Vec::new();
        for (name, text) in value.and_then(Value::as_object).into_iter().flatten() {
            let text = match text {
                Value::String(text) => text.clone(),
                Value::Null => continue,
                other => other.to_string(),
            };
            list.push((self.keep(name), self.keep(&text)));
        }
        let owned = list.into_boxed_slice();
        let borrowed: &'static [(&'static str, &'static str)] = unsafe { &*(owned.as_ref() as *const [(&'static str, &'static str)]) };
        self.pairs.push(owned);
        borrowed
    }
}

/// A resource's bytes kept for the run, once for each name and contents:
/// fonts the screen took stay in use, and a reload brings the same bytes again.
fn kept_resource(name: &str, bytes: Vec<u8>) -> (&'static str, &'static [u8]) {
    type Kept = Vec<(&'static str, &'static [u8])>;
    static KEPT: std::sync::Mutex<Kept> = std::sync::Mutex::new(Vec::new());
    let mut kept = KEPT.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(found) = kept.iter().find(|(n, b)| *n == name && *b == bytes.as_slice()) {
        return *found;
    }
    let found = (&*Box::leak(name.to_string().into_boxed_str()), &*Box::leak(bytes.into_boxed_slice()));
    kept.push(found);
    found
}

/// The program's files as the server serves them (`files.json`):
/// `{"title", "width", "height", "windows": {name: markup}, "apps": {name:
/// markup}, "themes": {"shop.light": json}, "model": json, "resources": [name]}`,
/// a theme and the model as a JSON value or as its text. A resource's bytes
/// come from `resource` by its name (fetched from the server's
/// `resources/<name>`); one it cannot give is left out. Kept for the run.
pub fn files_from_json(files: &Value, resource: &dyn Fn(&str) -> Option<Vec<u8>>) -> AppFiles {
    let (app, owned) = owned_files_from_json(files, resource);
    std::mem::forget(owned);
    app
}

/// The program's files as `files_from_json` reads them, with what owns
/// them: the files may be used for as long as the owner is kept.
pub fn owned_files_from_json(files: &Value, resource: &dyn Fn(&str) -> Option<Vec<u8>>) -> (AppFiles, OwnedFiles) {
    let mut owned = OwnedFiles::default();
    let text = |owned: &mut OwnedFiles, name: &str| -> &'static str {
        match files.get(name) {
            Some(Value::String(text)) => owned.keep(text),
            Some(Value::Null) | None => "",
            Some(other) => owned.keep(&other.to_string()),
        }
    };
    let number = |name: &str| files.get(name).and_then(Value::as_f64).unwrap_or(0.0);
    let resources: Vec<(&'static str, &'static [u8])> = files
        .get("resources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|name| Some(kept_resource(name, resource(name)?)))
        .collect();
    let resources = resources.into_boxed_slice();
    let borrowed: &'static [(&'static str, &'static [u8])] = unsafe { &*(resources.as_ref() as *const [(&'static str, &'static [u8])]) };
    owned.resources = Some(resources);
    let app = AppFiles {
        windows: owned.pairs(files.get("windows")),
        apps: owned.pairs(files.get("apps")),
        themes: owned.pairs(files.get("themes")),
        model: text(&mut owned, "model"),
        resources: borrowed,
        title: text(&mut owned, "title"),
        width: number("width"),
        height: number("height"),
    };
    (app, owned)
}
