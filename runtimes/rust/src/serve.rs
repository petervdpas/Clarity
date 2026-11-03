//! The web target (DESIGN 7.3): the application as a web server. It serves
//! the browser files (Clarity.UI's page, library and the program's files)
//! and answers each command of the page's wire with the function the program
//! gives (`wire::answer` on its `Program`). Requests run side by side; the
//! database is the server's. Each browser tab is a page of its own (its
//! `"browser"` id): its person's settings and language (DESIGN 7.9 B2).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

/// Where a page the server serves sends its commands.
pub const INVOKE_PATH: &str = "/clarity/invoke";
/// The largest request body the server reads: a command with a file in it.
const LARGEST_BODY: usize = 64 * 1024 * 1024;

/// Where the server listens: `--address host:port`, else the environment's
/// CLARITY_ADDRESS, else `project`, where the project file says (its port).
pub fn address(args: &[String], project: &str) -> String {
    if let Some(at) = args.iter().position(|a| a == "--address")
        && let Some(address) = args.get(at + 1)
    {
        return address.clone();
    }
    match std::env::var("CLARITY_ADDRESS") {
        Ok(address) if !address.is_empty() => address,
        _ => project.to_string(),
    }
}

fn reply(status: &str, kind: &str, body: &[u8]) -> Vec<u8> {
    let mut out = format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(body);
    out
}

/// The kind of a file the renderer is made of, by its extension.
fn kind_of(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// The page as the server gives it: it says where its commands go.
fn served(page: &[u8]) -> Vec<u8> {
    let page = String::from_utf8_lossy(page);
    page.replacen("</head>", &format!("<script>window.clarityServer = \"{INVOKE_PATH}\";</script></head>"), 1).into_bytes()
}

/// How long a page may go unasked before the server forgets it: a browser
/// tab closes without saying so.
pub const QUIET: Duration = Duration::from_secs(30 * 60);

/// One browser tab of a served program: its person's settings as the page
/// last sent them, the language its `set_language` chose, when it last asked.
struct Page {
    settings: Map<String, Value>,
    language: Option<String>,
    used: Instant,
}

/// The pages the server answers, by the id each page gives its requests.
pub struct Pages {
    pages: Mutex<BTreeMap<String, Page>>,
    quiet: Duration,
}

std::thread_local! {
    /// The settings of the page whose request this thread answers.
    static ASKING: RefCell<Option<Map<String, Value>>> = const { RefCell::new(None) };
}

/// `base` with the settings of the page whose request this thread answers
/// over it; `base` itself off a page (the desktop). What a window's settings
/// start from, so a handler sees the settings of the browser that asked.
pub fn page_settings_over(mut base: Map<String, Value>) -> Map<String, Value> {
    ASKING.with(|asking| {
        if let Some(settings) = asking.borrow().as_ref() {
            base.extend(settings.iter().map(|(name, value)| (name.clone(), value.clone())));
        }
    });
    base
}

/// The settings record of a window's state, as the wire has it.
fn settings_in(state: Option<&Value>) -> Option<&Map<String, Value>> {
    state?.get("data")?.get("settings")?.as_object()
}

/// The settings the answer's window has where they differ from what the
/// window had (`before`): none when the command changed none.
fn changed(before: &Map<String, Value>, answer: &Value) -> Option<Map<String, Value>> {
    let after = settings_in(answer.get("value").and_then(|v| v.get("state")))?;
    let mut now = after.clone();
    now.remove("id");
    now.iter().any(|(name, value)| before.get(name).is_some_and(|was| was != value)).then_some(now)
}

impl Pages {
    /// No pages yet; one unasked for `quiet` is forgotten.
    pub const fn new(quiet: Duration) -> Pages {
        Pages { pages: Mutex::new(BTreeMap::new()), quiet }
    }

    /// Answers `command` of page `browser` with `invoke`, as that page:
    /// with its settings (`sent`, as the page keeps them; none keeps those
    /// it had) and its language. `set_language` chooses the page's own. The
    /// answer carries the page's settings (`"settings"`) when the command
    /// changed them, for that page alone to keep.
    pub fn answer(&self, browser: &str, sent: Option<Map<String, Value>>, command: &str, args: Value, invoke: &mut dyn FnMut(&str, Value) -> Value) -> Value {
        let now = Instant::now();
        let (settings, language) = {
            let mut pages = self.pages.lock().unwrap_or_else(|e| e.into_inner());
            pages.retain(|id, page| id == browser || now.duration_since(page.used) <= self.quiet);
            let page = pages.entry(browser.to_string()).or_insert_with(|| Page { settings: Map::new(), language: None, used: now });
            page.used = now;
            if let Some(sent) = sent {
                page.settings = sent;
            }
            if command == "set_language" {
                page.language = args.get("language").and_then(Value::as_str).filter(|l| !l.is_empty()).map(str::to_string);
                return json!({ "value": null });
            }
            (page.settings.clone(), page.language.clone())
        };
        let before = settings_in(args.get("state")).cloned().unwrap_or_else(|| settings.clone());
        let was = ASKING.with(|asking| asking.replace(Some(settings)));
        let mut answer = crate::i18n::speaking(language.as_deref(), || invoke(command, args));
        ASKING.with(|asking| *asking.borrow_mut() = was);
        if let Some(now) = changed(&before, &answer) {
            let mut pages = self.pages.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(page) = pages.get_mut(browser) {
                page.settings.extend(now);
                answer["settings"] = Value::Object(page.settings.clone());
            }
        }
        answer
    }
}

/// The pages of the program's server.
static PAGES: Pages = Pages::new(QUIET);

/// The reply to one HTTP request: a file of `files` (by its path, without
/// the leading slash), the page at `/`, a command's answer at INVOKE_PATH.
pub fn answer(request: &[u8], files: &dyn Fn(&str) -> Option<Vec<u8>>, invoke: &mut dyn FnMut(&str, Value) -> Value) -> Vec<u8> {
    answer_in(&PAGES, request, files, invoke)
}

/// [`answer`] with the pages `pages`.
pub fn answer_in(pages: &Pages, request: &[u8], files: &dyn Fn(&str) -> Option<Vec<u8>>, invoke: &mut dyn FnMut(&str, Value) -> Value) -> Vec<u8> {
    let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") else {
        return reply("400 Bad Request", "text/plain", b"");
    };
    let head = String::from_utf8_lossy(&request[..end]);
    let mut first = head.lines().next().unwrap_or("").split(' ');
    let (method, path) = (first.next().unwrap_or(""), first.next().unwrap_or(""));
    let path = path.split('?').next().unwrap_or("");
    if path == INVOKE_PATH {
        if method != "POST" {
            return reply("405 Method Not Allowed", "text/plain", b"send a command with POST");
        }
        let answer = match serde_json::from_slice::<Value>(&request[end + 4..]) {
            Ok(sent) => match sent.get("command").and_then(Value::as_str) {
                Some(command) => {
                    let args = sent.get("args").filter(|a| a.is_object()).cloned().unwrap_or_else(|| json!({}));
                    // The page that asks (one id per tab) and its person's settings.
                    let browser = sent.get("browser").and_then(Value::as_str).unwrap_or("");
                    let settings = sent.get("settings").and_then(Value::as_object).cloned();
                    pages.answer(browser, settings, command, args, invoke)
                }
                None => json!({ "error": "a command is JSON with its command and its args" }),
            },
            Err(err) => json!({ "error": format!("a command is JSON with its command and its args: {err}") }),
        };
        return reply("200 OK", "application/json", answer.to_string().as_bytes());
    }
    let file = match path.trim_start_matches('/') {
        "" => "index.html",
        file => file,
    };
    // Only the renderer's own files: nothing above its folder.
    if file.split('/').any(|part| part == ".." || part.is_empty()) {
        return reply("404 Not Found", "text/plain", b"");
    }
    match files(file) {
        Some(body) if file == "index.html" => reply("200 OK", kind_of(file), &served(&body)),
        Some(body) => reply("200 OK", kind_of(file), &body),
        None => reply("404 Not Found", "text/plain", b""),
    }
}

/// Reads one request: its headers, then its body by its length.
fn read_request(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut reader = BufReader::new(stream);
    let mut request = Vec::new();
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
        request.extend_from_slice(line.as_bytes());
        if line == "\r\n" {
            break;
        }
    }
    let mut body = vec![0; length.min(LARGEST_BODY)];
    reader.read_exact(&mut body)?;
    request.extend_from_slice(&body);
    Ok(request)
}

/// Runs the application as a web server on `address` until it stops. The
/// requests run side by side, each on a thread of its own: while a long
/// handler's request is open, the page still asks how far it is, stops it,
/// and draws (round B of DESIGN 7.9).
pub fn serve(name: &str, address: &str, files: &(dyn Fn(&str) -> Option<Vec<u8>> + Sync), invoke: &(dyn Fn(&str, Value) -> Value + Sync)) -> std::io::Result<()> {
    let listener = TcpListener::bind(address)?;
    eprintln!("{name}: serving on http://{address}");
    std::thread::scope(|scope| {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            scope.spawn(move || {
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(60)));
                if let Ok(request) = read_request(&mut stream) {
                    let _ = stream.write_all(&answer(&request, files, &mut |command, args| invoke(command, args)));
                }
            });
        }
    });
    Ok(())
}
