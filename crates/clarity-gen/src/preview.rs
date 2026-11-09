//! The IDE's preview and design canvas on Clarity.UI (DESIGN 8.1): what is
//! designed and previewed is drawn by the same library, from the same
//! markup, as the program that runs. The IDE serves Clarity.UI's browser
//! files, the project's windows as `files.json` and a program that answers
//! the library's wire with the preview's sample data (`PreviewProgram`),
//! from a server of its own on this machine's loopback address (`Server`):
//! a worker of the IDE's page asks it with synchronous requests, which no
//! webview's custom scheme answers on every system.
//!
//! The preview's program runs no handler: the generated program does that
//! (Run). It opens windows with sample data, takes what the person changes,
//! and says which handler a click would run.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use clarity_spec::model::{DataKind, FieldType, Literal, Project};
use clarity_spec::sample::Row;
use serde_json::{json, Map, Value};

/// What the IDE shows: a window of the project, or an app's frame around an empty window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    Window(String),
    App(String),
}

impl Shown {
    /// What the query of a request names: `window=Main` or `app=Shop`.
    pub fn of_query(query: &BTreeMap<String, String>) -> Option<Shown> {
        if let Some(app) = query.get("app").filter(|a| !a.is_empty()) {
            return Some(Shown::App(app.clone()));
        }
        query.get("window").filter(|w| !w.is_empty()).map(|w| Shown::Window(w.clone()))
    }

    /// The window the program starts with.
    pub fn main(&self) -> &str {
        match self {
            Shown::Window(name) | Shown::App(name) => name,
        }
    }
}

/// `files.json` for the IDE: the project's windows, apps, themes, model
/// and resources, the title, and the size of the window shown. The window
/// picked shows at once: the app's splash is left out (Run shows it). For the
/// designer (`design`), every window, part, page and menu item carries its
/// `designItem`; an app is shown as an empty window of its name, framed by
/// it, which is the app itself in the design.
/// `showing` gives each tabs, steps or accordion part (by its item) the page
/// the designer switched to, from 0.
pub fn files_json(project: &Project, title: &str, shown: &Shown, design: bool, showing: &BTreeMap<u32, usize>) -> String {
    let size = match shown {
        Shown::Window(name) => crate::window_size(project, name),
        Shown::App(_) => (clarity_spec::layout::WINDOW_WIDTH, clarity_spec::layout::WINDOW_HEIGHT),
    };
    // The preview shows each view in its window's part; the designer shows the part as written, empty.
    // Texts that show the app's version carry it, as in the program.
    let filled = clarity_spec::translations::with_app_slots(project);
    let project = &filled;
    // The designer draws a window as it looks: each view in its content (designed in place
    // when it stands in the window's file), and a tree of levels as the tree of its rows.
    let mut placed = if design { clarity_spec::trees::lowered(project) } else { clarity_spec::views::placed(project) };
    if design {
        show_views_in_content(&mut placed, project);
    }
    // The window picked shows, not the app's start: no splash stands in for it (Run shows the splash).
    for app in placed.files.iter_mut().flat_map(|f| f.apps.iter_mut()) {
        app.splash = None;
        app.splash_for = None;
    }
    let write = || crate::web_browser::files_json_as(&placed, title, size);
    let text = if design { crate::web_windows::designed_showing(showing, write) } else { write() };
    // A view is no window of a program, but the designer and preview draw it on its own as one.
    let view = match shown {
        Shown::Window(name) => project.pages().find(|p| p.name.name == *name && p.kind == clarity_spec::model::PageKind::View),
        Shown::App(_) => None,
    };
    if let Some(view) = view {
        let Ok(mut files) = serde_json::from_str::<Value>(&text) else { return text };
        let module = project.files.iter().find(|f| f.pages.iter().any(|p| std::ptr::eq(p, view))).and_then(|f| f.module.as_ref()).map_or("", |m| m.name.as_str());
        let write = || crate::web_windows::window_markup_in(view, module, project.row_height);
        let markup = if design { crate::web_windows::designed_showing(showing, write) } else { write() };
        if let Some(windows) = files.get_mut("windows").and_then(Value::as_object_mut) {
            windows.insert(view.name.name.clone(), Value::String(markup));
        }
        return files.to_string() + "\n";
    }
    let Shown::App(name) = shown else { return text };
    let Ok(mut files) = serde_json::from_str::<Value>(&text) else { return text };
    let item = project.apps().find(|a| a.name.name == *name).filter(|_| design).map(|a| format!(" designItem=\"{}\"", a.span.start)).unwrap_or_default();
    if let Some(windows) = files.get_mut("windows").and_then(Value::as_object_mut) {
        let rows = crate::web_windows::row_height_attribute(project.row_height);
        windows.insert(name.clone(), Value::String(format!("<Window name=\"{}\"{rows}{item} />\n", escaped(name))));
    }
    files.to_string() + "\n"
}

/// Fills each content of `placed` with the parts of the views it lists, so
/// the window is designed as it looks: a view of the window's own file
/// keeps its places; one of its own file (a `.cxaml`) has its parts' places
/// as designer's items of that file (`cxaml::design_items`), so an edit of
/// one changes the view's file.
fn show_views_in_content(placed: &mut Project, project: &Project) {
    fn fill(widgets: &mut [clarity_spec::model::Widget], project: &Project, own: &str, depth: usize) {
        for w in widgets {
            if w.props.kind == clarity_spec::widgets::WidgetKind::Content && depth < 8 {
                for page in &mut w.pages {
                    let found = page.view.as_deref().and_then(|v| {
                        project.files.iter().enumerate().find_map(|(i, f)| f.pages.iter().find(|p| p.name.name == v && p.kind == clarity_spec::model::PageKind::View).map(|p| (i, f.path.as_str(), p)))
                    });
                    if let Some((index, path, view)) = found
                        && page.children.is_empty()
                    {
                        let mut parts = clarity_spec::trees::lowered_widgets(project, &view.widgets);
                        if path != own {
                            clarity_spec::cxaml::design_items(&mut parts, index);
                        }
                        fill(&mut parts, project, own, depth + 1);
                        page.children = parts;
                        page.columns.get_or_insert(view.columns);
                    }
                }
            }
            fill(&mut w.children, project, own, depth);
            for page in &mut w.pages {
                fill(&mut page.children, project, own, depth);
            }
        }
    }
    for file in &mut placed.files {
        let own = file.path.clone();
        for page in &mut file.pages {
            fill(&mut page.widgets, project, &own, 0);
            for sidebar in &mut page.frame.sidebars {
                fill(&mut sidebar.widgets, project, &own, 0);
            }
        }
    }
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// `pages=50.1,70.0` (a container's item and the page it shows, from 0) as a map.
pub fn showing_of(query: &BTreeMap<String, String>) -> BTreeMap<u32, usize> {
    query.get("pages").map(|pages| pages.split(',').filter_map(|pair| pair.split_once('.')).filter_map(|(item, page)| Some((item.trim().parse().ok()?, page.trim().parse().ok()?))).collect()).unwrap_or_default()
}

/// The number the next window that opens gets.
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

/// The preview's program: windows with sample data, no handlers run.
pub struct PreviewProgram {
    project: Project,
    samples: BTreeMap<String, Vec<Row>>,
    /// The rows each tree of levels shows, by the data that holds them.
    tree_rows: BTreeMap<String, Vec<Map<String, Value>>>,
    displays: BTreeMap<String, Vec<String>>,
    main: String,
    /// The popovers shown open, by name: the one picked in the designer.
    open: Vec<String>,
}

impl PreviewProgram {
    /// The program of `project`, starting with the window `main`.
    pub fn new(project: Project, main: &str) -> PreviewProgram {
        let tree_rows = clarity_spec::trees::sample_data(&project);
        let mut placed = clarity_spec::views::placed(&project);
        // A view the designer shows on its own is its window here, with its own code.
        for (file, own) in project.files.iter().zip(placed.files.iter_mut()) {
            if let Some(view) = file.pages.iter().find(|p| p.name.name == main && p.kind == clarity_spec::model::PageKind::View) {
                own.pages.push(view.clone());
                own.code.extend(file.code.iter().find(|c| c.name.name == main).cloned());
            }
        }
        let project = placed;
        let samples = clarity_spec::sample::sample_data(&project);
        let displays = clarity_spec::sample::display_values(&project);
        PreviewProgram { project, samples, tree_rows, displays, main: main.to_string(), open: Vec::new() }
    }

    /// The popovers named in `open` show open in their windows, as the designer shows the one picked.
    pub fn opening(mut self, open: Vec<String>) -> PreviewProgram {
        self.open = open;
        self
    }

    /// The program's answer to a command of the wire: `{"value"}` or `{"error"}`.
    pub fn answer(&self, command: &str, args: &Value) -> Value {
        match command {
            "started" => json!({ "value": { "main": self.main } }),
            "start_app" => reply(state("", Map::new()), Vec::new()),
            "connect" | "set_language" | "keep_settings" | "start_watch" | "stop_watch" | "end_watch" | "engine_close" => json!({ "value": null }),
            "watches" => json!({ "value": false }),
            "read_settings" => json!({ "value": {} }),
            "facts" => json!({ "value": serde_json::from_str::<Value>(&crate::rust::facts_json(&self.project)).unwrap_or(Value::Null) }),
            "open_window" => {
                let window = args.get("window").and_then(Value::as_str).unwrap_or_default();
                if !self.project.pages().any(|p| p.name.name == window) && window != self.main {
                    return json!({ "error": format!("The project has no window {window}.") });
                }
                let mut data = self.window_data(window);
                if let Some(with) = args.get("with").and_then(Value::as_object) {
                    data.extend(with.clone());
                }
                let mut opened = state(window, data);
                for name in self.open.iter().filter(|name| self.project.pages().filter(|p| p.name.name == window).any(|p| named_in(&p.widgets, name))) {
                    opened["parts"][name] = json!({ "visible": true });
                }
                reply(opened, Vec::new())
            }
            "dispatch_event" | "answer_event" => {
                let handler = args.get("handler").and_then(Value::as_str).unwrap_or_default();
                // A timer's tick is no action of the person's: nothing runs and nothing is said.
                if self.ticks(handler) {
                    return reply(sent_state(args), Vec::new());
                }
                // Clarity's own words, in the person's language where the project translates them.
                let (key, english) = clarity_spec::translations::PREVIEW_KEYS[0];
                let said = json!({ "key": key, "text": english, "values": { "handler": { "kind": "text", "value": handler } } });
                reply(sent_state(args), vec![json!({ "level": "info", "text": said })])
            }
            "ref_options" => {
                let entity = args.get("entity").and_then(Value::as_str).unwrap_or_default();
                let texts = self.displays.get(entity).cloned().unwrap_or_default();
                json!({ "value": texts.iter().enumerate().map(|(i, text)| json!({ "id": i + 1, "text": text })).collect::<Vec<_>>() })
            }
            "field_values" => {
                let entity = args.get("entity").and_then(Value::as_str).unwrap_or_default();
                let field = args.get("field").and_then(Value::as_str).unwrap_or_default();
                let mut values: Vec<String> = self.samples.get(entity).into_iter().flatten().filter_map(|row| row.get(field)).map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).collect();
                values.sort();
                values.dedup();
                json!({ "value": values })
            }
            "field_bytes" => json!({ "value": null }),
            "watch_progress" => json!({ "value": null }),
            _ if args.get("state").is_some() => reply(sent_state(args), Vec::new()),
            _ => json!({ "error": format!("The preview does not answer {command}.") }),
        }
    }

    /// A window's data when it opens: a list's every sample record, a record
    /// the first one, a plain value its default, as the preview shows them.
    pub fn window_data(&self, window: &str) -> Map<String, Value> {
        let mut data = Map::new();
        let decls = self.project.pages().filter(|p| p.name.name == window).flat_map(|p| &p.data).chain(self.project.apps().flat_map(|a| &a.data));
        for decl in decls {
            let name = decl.name.name.clone();
            if data.contains_key(&name) {
                continue;
            }
            // A tree of levels shows sample rows of its levels.
            if let Some(rows) = self.tree_rows.get(&name) {
                data.insert(name, Value::Array(rows.iter().cloned().map(Value::Object).collect()));
                continue;
            }
            let value = match &decl.of {
                DataKind::List { entity } => Value::Array(self.rows(&entity.name).into_iter().map(Value::Object).collect()),
                DataKind::Record { entity } => self.rows(&entity.name).into_iter().next().map(Value::Object).unwrap_or(Value::Null),
                DataKind::Value { ty, default } => start_value(&ty.kind, default.as_ref()),
            };
            data.insert(name, value);
        }
        data
    }

    /// An entity's sample records as a program holds them: each with its
    /// number, a reference by the number of the record it names.
    fn rows(&self, entity: &str) -> Vec<Map<String, Value>> {
        let Some(found) = self.project.entities().find(|e| e.name.name == entity) else { return Vec::new() };
        let rows = self.samples.get(entity).cloned().unwrap_or_default();
        rows.into_iter()
            .enumerate()
            .map(|(i, row)| {
                let mut record: Map<String, Value> = row.into_iter().collect();
                record.entry("id".to_string()).or_insert(json!(i + 1));
                for field in &found.fields {
                    if let FieldType::Ref { entity: target } = &field.ty {
                        let named = record.get(&field.name.name).and_then(Value::as_str).and_then(|text| self.displays.get(&target.name)?.iter().position(|d| d == text));
                        record.insert(field.name.name.clone(), named.map_or(Value::Null, |at| json!(at + 1)));
                    }
                }
                record
            })
            .collect()
    }
}

/// Whether a part named `name` is among `widgets` or inside them.
impl PreviewProgram {
    /// Whether a timer of the project runs `handler` on its tick.
    fn ticks(&self, handler: &str) -> bool {
        fn in_widgets(widgets: &[clarity_spec::model::Widget], handler: &str) -> bool {
            widgets.iter().any(|w| {
                w.props.handlers.iter().any(|h| h.handler == handler && h.event == clarity_spec::widgets::Event::Tick)
                    || in_widgets(&w.children, handler)
                    || w.pages.iter().any(|p| in_widgets(&p.children, handler))
            })
        }
        self.project.pages().any(|p| in_widgets(&p.widgets, handler))
    }
}

fn named_in(widgets: &[clarity_spec::model::Widget], name: &str) -> bool {
    widgets.iter().any(|w| w.props.name.as_deref() == Some(name) || named_in(&w.children, name) || w.pages.iter().any(|p| named_in(&p.children, name)))
}

/// A plain value when its window opens: its default as written, else its type's empty value.
fn start_value(ty: &clarity_spec::code::TypeRefKind, default: Option<&Literal>) -> Value {
    use clarity_spec::code::TypeRefKind as T;
    if let Some(default) = default {
        return match default {
            Literal::Number(n) => match ty {
                T::Integer => n.parse::<i64>().map(Value::from).unwrap_or_else(|_| Value::String(n.clone())),
                _ => Value::String(n.clone()),
            },
            Literal::Str(text) => Value::String(text.clone()),
            Literal::Bool(b) => Value::Bool(*b),
            Literal::Ident(name) => Value::String(name.clone()),
        };
    }
    match ty {
        T::Text => json!(""),
        T::Integer => json!(0),
        T::Decimal | T::Money => json!("0"),
        T::Bool => json!(false),
        T::List { .. } => json!([]),
        _ => Value::Null,
    }
}

fn state(window: &str, data: Map<String, Value>) -> Value {
    json!({ "window": window, "instance": NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed), "data": data, "parts": {}, "selected": {} })
}

/// The window's state as the screen sent it.
fn sent_state(args: &Value) -> Value {
    args.get("state").cloned().unwrap_or_else(|| state("", Map::new()))
}

fn reply(state: Value, messages: Vec<Value>) -> Value {
    json!({ "value": { "state": state, "messages": messages, "open": [], "close": false } })
}

/// What the preview shows, read again for each start of the drawing.
#[derive(Clone)]
pub struct Loaded {
    pub project: Project,
    pub title: String,
    /// The project's folder, whose resources the drawing reads.
    pub dir: PathBuf,
}

/// Gives the open project as it is on disk now; why not, when there is none.
pub type Loader = Arc<dyn Fn() -> Result<Loaded, String> + Send + Sync>;

/// Where Clarity.UI's browser files are, asked when they are served.
pub type Library = Arc<dyn Fn() -> Result<PathBuf, String> + Send + Sync>;

/// The IDE's preview server on the loopback address: Clarity.UI's browser
/// files, `files.json`, the project's resources and the wire, below a path
/// only the IDE knows (`address`). Each start of the drawing (files.json)
/// reads the project again; the wire answers from what was read last.
pub struct Server {
    address: String,
    serving: Arc<Serving>,
}

struct Serving {
    loader: Loader,
    library: Library,
    /// The program of what was read last, by the window it starts with.
    program: Mutex<Option<Arc<PreviewProgram>>>,
    loaded: Mutex<Option<Loaded>>,
    /// The project as the Design text being typed would make it (`set_draft`), drawn for `draft=1`.
    draft: Mutex<Option<Project>>,
}

impl Server {
    /// Starts serving on a free port of 127.0.0.1, below a secret of `secret`'s making.
    pub fn start(loader: Loader, library: Library, secret: &str) -> std::io::Result<Server> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let prefix = format!("/{secret}/");
        let serving = Arc::new(Serving { loader, library, program: Mutex::new(None), loaded: Mutex::new(None), draft: Mutex::new(None) });
        let below = prefix.clone();
        let kept = serving.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let serving = serving.clone();
                let below = below.clone();
                // Requests side by side: a long one does not hold the others.
                std::thread::spawn(move || {
                    let _ = serve(stream, &below, &serving);
                });
            }
        });
        Ok(Server { address: format!("http://127.0.0.1:{port}{prefix}"), serving: kept })
    }

    /// The project as the unsaved Design text makes it, which a drawing asked
    /// with `draft=1` shows; none when there is no such text.
    pub fn set_draft(&self, draft: Option<Project>) {
        *self.serving.draft.lock().unwrap_or_else(|p| p.into_inner()) = draft;
    }

    /// Where it serves, ending in `/`: `http://127.0.0.1:41234/<secret>/`.
    pub fn address(&self) -> &str {
        &self.address
    }
}

/// A request as the server reads it.
struct Request {
    method: String,
    path: String,
    query: BTreeMap<String, String>,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut words = line.split_whitespace();
    let method = words.next()?.to_string();
    let target = words.next()?.to_string();
    let mut length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0; length.min(64 * 1024 * 1024)];
    reader.read_exact(&mut body).ok()?;
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    Some(Request { method, path: decoded(path), query: query_of(query), body })
}

/// `a=1&b=two` as names and values.
pub fn query_of(query: &str) -> BTreeMap<String, String> {
    query.split('&').filter(|p| !p.is_empty()).map(|pair| pair.split_once('=').unwrap_or((pair, ""))).map(|(k, v)| (decoded(k), decoded(v))).collect()
}

/// Percent-encoding undone.
fn decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => match std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|hex| u8::from_str_radix(hex, 16).ok()).ok_or(()) {
                Ok(byte) => {
                    out.push(byte);
                    i += 3;
                }
                Err(_) => {
                    out.push(b'%');
                    i += 1;
                }
            },
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    // Any page of the IDE may ask (its origin differs per system and in development); only one that knows the secret path finds anything.
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nAccess-Control-Allow-Private-Network: true\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

/// The media type of a served file by its name.
pub fn media_type(name: &str) -> &'static str {
    match name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).as_deref() {
        Some("js") => "text/javascript",
        Some("wasm") => "application/wasm",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        _ => "application/octet-stream",
    }
}

/// A name that stays inside its folder: no `..`, no root, no backslash.
fn inside(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('/') && !name.contains('\\') && name.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

fn serve(mut stream: TcpStream, below: &str, serving: &Serving) -> std::io::Result<()> {
    let Some(request) = read_request(&mut stream) else { return Ok(()) };
    if request.method == "OPTIONS" {
        return respond(&mut stream, "204 No Content", "text/plain", b"");
    }
    let Some(path) = request.path.strip_prefix(below) else {
        return respond(&mut stream, "404 Not Found", "text/plain", b"not here");
    };
    match (request.method.as_str(), path) {
        ("GET", name) if crate::clarity_ui::BROWSER_FILES.contains(&name) => match (serving.library)() {
            Ok(folder) => match std::fs::read(folder.join(name)) {
                Ok(bytes) => respond(&mut stream, "200 OK", media_type(name), &bytes),
                Err(err) => respond(&mut stream, "404 Not Found", "text/plain", err.to_string().as_bytes()),
            },
            Err(why) => respond(&mut stream, "404 Not Found", "text/plain", why.as_bytes()),
        },
        ("GET", "files.json") => {
            let Some(shown) = Shown::of_query(&request.query) else {
                return respond(&mut stream, "400 Bad Request", "text/plain", b"files.json names a window or an app");
            };
            match (serving.loader)() {
                Ok(mut loaded) => {
                    let design = request.query.get("design").is_some_and(|d| d == "1");
                    if request.query.get("draft").is_some_and(|d| d == "1")
                        && let Some(draft) = serving.draft.lock().unwrap_or_else(|p| p.into_inner()).clone()
                    {
                        loaded.project = draft;
                    }
                    let text = files_json(&loaded.project, &loaded.title, &shown, design, &showing_of(&request.query));
                    let open: Vec<String> = request.query.get("open").map(|o| o.split(',').filter(|n| !n.is_empty()).map(str::to_string).collect()).unwrap_or_default();
                    *serving.program.lock().unwrap_or_else(|p| p.into_inner()) = Some(Arc::new(PreviewProgram::new(loaded.project.clone(), shown.main()).opening(open)));
                    *serving.loaded.lock().unwrap_or_else(|p| p.into_inner()) = Some(loaded);
                    respond(&mut stream, "200 OK", "application/json", text.as_bytes())
                }
                Err(why) => respond(&mut stream, "404 Not Found", "text/plain", why.as_bytes()),
            }
        }
        ("GET", name) if name.starts_with("resources/") => {
            let name = &name["resources/".len()..];
            let dir = serving.loaded.lock().unwrap_or_else(|p| p.into_inner()).as_ref().map(|l| l.dir.clone());
            match dir.filter(|_| inside(name)).map(|d| d.join(clarity_spec::resources::RESOURCES_DIR).join(name)).and_then(|p| std::fs::read(p).ok()) {
                Some(bytes) => respond(&mut stream, "200 OK", media_type(name), &bytes),
                None => respond(&mut stream, "404 Not Found", "text/plain", b"no such resource"),
            }
        }
        ("POST", "invoke") => {
            let asked: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
            let command = asked.get("command").and_then(Value::as_str).unwrap_or_default();
            let args = asked.get("args").cloned().unwrap_or(Value::Null);
            let program = serving.program.lock().unwrap_or_else(|p| p.into_inner()).clone();
            let answer = match program {
                Some(program) => program.answer(command, &args),
                None => json!({ "error": "The preview has not read the project yet." }),
            };
            respond(&mut stream, "200 OK", "application/json", answer.to_string().as_bytes())
        }
        _ => respond(&mut stream, "404 Not Found", "text/plain", b"not here"),
    }
}

/// The project in `dir` (its modules) with the project file `file`, as the preview shows it.
pub fn load(file: &Path, dir: &Path) -> Result<Loaded, String> {
    let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let title = clarity_project::load(file).title;
    Ok(Loaded { project, title, dir: dir.to_path_buf() })
}

/// A secret for the server's path: hard to guess for another page of this machine.
pub fn secret() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for _ in 0..2 {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default());
        hasher.write_u32(std::process::id());
        out.push_str(&format!("{:016x}", hasher.finish()));
    }
    out
}
