//! Clarity.UI in a real browser (round B): a small server serves the
//! browser files (runtimes/rust-ui-browser/build.sh writes them to
//! target/clarity-ui-browser, or CLARITY_UI_BROWSER names them), a window's
//! markup and a program answering the wire; a headless Chromium with a
//! profile of its own opens the page, the page's own pointer handlers
//! click the button, and the server's reply shows in the canvas's pixels.
//!
//! Ignored: it needs the browser files (and so the Emscripten SDK) and
//! Chromium. Run with `cargo test -p clarity-ui --test hosts browser_page
//! -- --ignored --test-threads=1`; CLARITY_BROWSER_PROFILE names the folder
//! the browser's profile and the shots go to (target/tmp otherwise).
//!
//! Besides the click: a long call (a handler that takes seconds) while the
//! page draws on, its bar and Stop; and a phone-sized page at two device
//! pixels per CSS pixel, clicked by touch.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use base64::Engine as _;
use clarity_runtime::ui::WindowState;
use serde_json::{Map, Value, json};

const MAIN: &str = r#"<Window name="Main">
  <Heading name="title" at="1,1" span="12" caption="Clarity.UI in the browser" />
  <Button name="greet" at="1,2" span="4" caption="Greet" onclick="greet" />
  <Textbox name="answer" at="1,3" span="8" bind="answer" />
</Window>
"#;

const ANSWER: &str = "Hello from the server";

/// The test's page: the page script on a canvas that fills it, and a check
/// that clicks the button through the canvas's own pointer events and posts
/// what it saw.
const PAGE: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Clarity.UI</title>
<style>html, body { margin: 0; height: 100%; overflow: hidden; } #clarity-ui { width: 100vw; height: 100vh; }</style>
</head><body><canvas id="clarity-ui"></canvas>
<script>window.addEventListener("error", (e) => fetch("/result", { method: "POST", body: JSON.stringify({ error: e.message + " at " + e.filename + ":" + e.lineno }) }));</script>
<script src="clarity-ui-page.js"></script>
<script>
(async () => {
  const report = (what) => fetch("/result", { method: "POST", body: JSON.stringify(what) });
  try {
    const ui = window.clarityUi;
    await ui.started;
    const before = await ui.shot();
    const read = await ui.step({ do: "read" });
    const [x, y, w, h] = read.shown.greet;
    const at = ui.canvas.getBoundingClientRect();
    // As a page's own script makes one: no isPrimary, no pointerType; a mouse's all the same.
    const point = { clientX: at.left + x + w / 2, clientY: at.top + y + h / 2, button: 0, bubbles: true };
    ui.canvas.dispatchEvent(new PointerEvent("pointermove", point));
    ui.canvas.dispatchEvent(new PointerEvent("pointerdown", point));
    ui.canvas.dispatchEvent(new PointerEvent("pointerup", point));
    let after = null;
    for (let i = 0; i < 50; i++) {
      after = await ui.step({ do: "read" });
      if (after.data && after.data.answer) break;
      await new Promise((r) => setTimeout(r, 100));
    }
    const shot = await ui.shot();
    const box = [ui.canvas.clientWidth, ui.canvas.clientHeight, window.innerWidth, window.innerHeight];
    const inPage = !(ui.worker instanceof Worker);
    // New files on the same canvas (the IDE's designer after a change): the window opens again, and says where its parts are.
    const reloaded = await ui.reload("clarity/files.json");
    const again = await ui.step({ do: "read" });
    const layout = await ui.layout();
    report({ read, after, before, shot, box, title: document.title, dpr: window.devicePixelRatio, kept: JSON.parse(localStorage.getItem("clarity.settings") || "null"), inPage, reloaded, again: again.window, laid: layout.parts.map((p) => p.name) });
  } catch (why) {
    report({ error: String(why && why.stack || why) });
  }
})();
</script></body></html>
"#;

fn browser_files() -> PathBuf {
    match std::env::var_os("CLARITY_UI_BROWSER") {
        Some(folder) if !folder.is_empty() => PathBuf::from(folder),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/clarity-ui-browser"),
    }
}

fn chromium() -> Option<PathBuf> {
    ["chromium", "chromium-browser", "google-chrome", "google-chrome-stable"].iter().find_map(|name| {
        std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|p| p.join(name)).find(|p| p.is_file()))
    })
}

/// What a test's server answers on the wire, by command and arguments.
type Answers = Arc<dyn Fn(&str, &Value) -> Value + Send + Sync>;

/// What the test's server serves: the page, the program's window, its answers.
struct Site {
    page: String,
    /// A page around it, which shows it in a frame (`/page`); none: it is the page.
    outer: Option<String>,
    main: &'static str,
    /// The program's model (files.json's `model`), empty for none.
    model: &'static str,
    answers: Answers,
}

/// The program on the server: it starts (no app), opens Main, and its `greet` puts the answer in the window's data.
pub(super) fn answer(command: &str, args: &Value) -> Value {
    let reply = |state: WindowState| json!({ "value": serde_json::to_value(super::core_windows_fake::reply(state)).unwrap() });
    match command {
        "started" => json!({ "value": { "main": "Main" } }),
        "start_app" => reply(WindowState::new("", Map::new())),
        "open_window" => {
            let mut data = Map::new();
            data.insert("answer".into(), json!(""));
            data.insert("greeting".into(), json!({ "id": 1, "note": "" }));
            // The answer brings the page's settings back: the page keeps them.
            let mut answered = reply(WindowState::new(args["window"].as_str().unwrap_or("Main"), data));
            answered["settings"] = json!({ "seen": "yes" });
            answered
        }
        "dispatch_event" => {
            let mut state: WindowState = serde_json::from_value(args["state"].clone()).unwrap();
            if args["handler"] == json!("greet") {
                state.data.insert("answer".into(), json!(ANSWER));
            }
            reply(state)
        }
        // The person's edit: the window as it came, as a program answers an edit without a change handler.
        "edit_event" => reply(serde_json::from_value(args["state"].clone()).unwrap()),
        other => json!({ "error": format!("The test's program does not answer {other}.") }),
    }
}

fn respond(mut stream: TcpStream, status: &str, kind: &str, body: &[u8]) {
    let head = format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", body.len());
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

/// Serves one request: the page, the browser files, the program's files and wire, the result.
fn serve(stream: TcpStream, files: &Path, site: &Site, asked: &Mutex<Vec<String>>, posted: &Mutex<Vec<Value>>, result: &Sender<Value>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let mut length = 0;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0; length];
    let _ = reader.read_exact(&mut body);
    let path = line.split_whitespace().nth(1).unwrap_or("/").split('?').next().unwrap_or("/").to_string();
    match path.as_str() {
        "/" => respond(stream, "200 OK", "text/html; charset=utf-8", site.outer.as_ref().unwrap_or(&site.page).as_bytes()),
        "/page" => respond(stream, "200 OK", "text/html; charset=utf-8", site.page.as_bytes()),
        "/clarity/files.json" => {
            let served = json!({ "title": "Greeter", "windows": { "Main": site.main }, "apps": {}, "themes": {}, "model": site.model, "resources": [] });
            respond(stream, "200 OK", "application/json", served.to_string().as_bytes())
        }
        "/clarity/invoke" => {
            let request = posted_value(&body);
            let command = request["command"].as_str().unwrap_or_default().to_string();
            asked.lock().unwrap().push(command.clone());
            posted.lock().unwrap().push(posted_body(&request));
            let answered = (site.answers)(&command, &request["args"]);
            respond(stream, "200 OK", "application/json", answered.to_string().as_bytes())
        }
        "/result" => {
            respond(stream, "200 OK", "text/plain", b"");
            let _ = result.send(serde_json::from_slice(&body).unwrap_or(Value::Null));
        }
        _ => {
            let name = path.trim_start_matches('/');
            let kind = match name {
                "clarity-ui.wasm" => "application/wasm",
                "clarity-ui.js" | "clarity-ui-worker.js" | "clarity-ui-page.js" => "text/javascript",
                _ => return respond(stream, "404 Not Found", "text/plain", b"not here"),
            };
            match std::fs::read(files.join(name)) {
                Ok(bytes) => respond(stream, "200 OK", kind, &bytes),
                Err(_) => respond(stream, "404 Not Found", "text/plain", b"not built"),
            }
        }
    }
}

struct Browser(Child);

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn png(base64: &Value) -> skia_safe::Image {
    let bytes = base64::engine::general_purpose::STANDARD.decode(base64.as_str().expect("a shot")).unwrap();
    skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&bytes)).expect("the shot is a PNG")
}

fn pixels(image: &skia_safe::Image) -> Vec<u8> {
    let info = skia_safe::ImageInfo::new_n32_premul((image.width(), image.height()), None);
    let mut pixels = vec![0u8; (image.width() * image.height() * 4) as usize];
    assert!(image.read_pixels(&info, &mut pixels, (image.width() * 4) as usize, (0, 0), skia_safe::image::CachingHint::Allow));
    pixels
}

/// The page open in a headless Chromium with a profile of its own, away
/// from any desktop session: what the server was asked, and what the page reports.
struct Opened {
    scratch: PathBuf,
    asked: Arc<Mutex<Vec<String>>>,
    /// Every wire request as posted, without its arguments.
    posted: Arc<Mutex<Vec<Value>>>,
    results: Receiver<Value>,
    _browser: Browser,
}

fn open(name: &str, site: Site, flags: &[&str]) -> Opened {
    let files = browser_files();
    for file in ["clarity-ui.js", "clarity-ui.wasm", "clarity-ui-worker.js", "clarity-ui-page.js"] {
        assert!(files.join(file).is_file(), "{} is not built: run runtimes/rust-ui-browser/build.sh", files.join(file).display());
    }
    let chromium = chromium().expect("no Chromium on this machine");
    let scratch = std::env::var_os("CLARITY_BROWSER_PROFILE").map(PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_TARGET_TMPDIR")).join("browser-page"));
    let profile = scratch.join(format!("profile-{name}"));
    let _ = std::fs::remove_dir_all(&profile);
    std::fs::create_dir_all(&profile).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let asked: Arc<Mutex<Vec<String>>> = Arc::default();
    let posted: Arc<Mutex<Vec<Value>>> = Arc::default();
    let (sent, results) = channel();
    let site = Arc::new(site);
    {
        let (asked, posted, files) = (asked.clone(), posted.clone(), files.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (asked, posted, files, sent, site) = (asked.clone(), posted.clone(), files.clone(), sent.clone(), site.clone());
                std::thread::spawn(move || serve(stream, &files, &site, &asked, &posted, &sent));
            }
        });
    }

    let browser = Browser(
        Command::new(chromium)
            .args(["--headless=new", "--no-first-run", "--no-default-browser-check", "--disable-extensions", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
            .args(flags)
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg(format!("http://{address}/"))
            .env("HOME", &profile)
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Chromium starts"),
    );
    Opened { scratch, asked, posted, results, _browser: browser }
}

fn posted_value(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap_or(Value::Null)
}

/// A posted request without its arguments: command, browser, settings.
fn posted_body(posted: &Value) -> Value {
    json!({ "command": posted["command"], "browser": posted["browser"], "settings": posted["settings"] })
}

fn save_shot(scratch: &Path, name: &str, shot: &Value) {
    let _ = std::fs::write(scratch.join(name), base64::engine::general_purpose::STANDARD.decode(shot.as_str().unwrap_or_default()).unwrap_or_default());
}

#[test]
#[ignore = "needs the browser files (Emscripten) and Chromium"]
fn a_click_in_the_browser_runs_the_servers_handler_and_its_reply_is_drawn() {
    let site = Site { page: PAGE.to_string(), outer: None, main: MAIN, model: "", answers: Arc::new(answer) };
    let Opened { scratch, asked, posted, results, _browser } = open("click", site, &["--window-size=900,600", "--force-device-scale-factor=1"]);
    let result = results.recv_timeout(Duration::from_secs(90)).expect("the page reported within 90 s");
    assert!(result.get("error").is_none(), "the page failed: {}", result["error"]);

    let asked = asked.lock().unwrap().clone();
    assert!(asked.contains(&"open_window".to_string()), "{asked:?}");
    assert!(asked.contains(&"dispatch_event".to_string()), "{asked:?}: {}", json!({ "read": result["read"], "after": result["after"] }));
    assert_eq!(result["title"], json!("Greeter"));
    // Every request names this tab and carries the settings the page keeps;
    // the settings an answer brought back are kept, by the page and in the next requests.
    let posted = posted.lock().unwrap().clone();
    let tab = posted[0]["browser"].as_str().expect("a request names its tab").to_string();
    assert_eq!(tab.len(), 36, "one id per tab: {tab}");
    assert!(posted.iter().all(|p| p["browser"] == json!(tab) && p["settings"].is_object()), "{posted:?}");
    let opened = posted.iter().position(|p| p["command"] == json!("open_window")).unwrap();
    assert_eq!(posted[0]["settings"], json!({}));
    assert!(posted[opened + 1..].iter().all(|p| p["settings"] == json!({ "seen": "yes" })), "{posted:?}");
    assert!(posted[opened + 1..].iter().any(|p| p["command"] == json!("dispatch_event")));
    assert_eq!(result["kept"], json!({ "seen": "yes" }));
    assert_eq!(result["after"]["data"]["answer"], json!(ANSWER), "{}", result["after"]);

    // The reply shows: the answer's box is drawn otherwise than before.
    let (before, after) = (png(&result["before"]), png(&result["shot"]));
    save_shot(&scratch, "browser-before.png", &result["before"]);
    save_shot(&scratch, "browser-after.png", &result["shot"]);
    assert_eq!((after.width(), after.height()), (before.width(), before.height()));
    let at: Vec<f64> = result["read"]["shown"]["answer"].as_array().expect("the answer's place").iter().map(|v| v.as_f64().unwrap()).collect();
    let scale = result["dpr"].as_f64().unwrap_or(1.0);
    let (left, top) = ((at[0] * scale) as i32, (at[1] * scale) as i32);
    let (right, bottom) = (((at[0] + at[2]) * scale) as i32, ((at[1] + at[3]) * scale) as i32);
    let (a, b) = (pixels(&before), pixels(&after));
    let width = before.width();
    let changed = (top..bottom).flat_map(|y| (left..right).map(move |x| ((y * width + x) * 4) as usize)).filter(|i| a[*i..*i + 4] != b[*i..*i + 4]).count();
    assert!(changed > 50, "the answer's box shows no new text: {changed} pixels changed");
    // The page draws something: not one color all over.
    assert!(b.chunks(4).any(|p| p != &b[0..4]));
}

/// Where a worker cannot draw with WebGL2 (WebKitGTK, the webview of Tauri
/// on Linux), the page draws on its own thread: the same worker script, the
/// requests synchronous there too. Chromium told to, as WebKitGTK must.
#[test]
#[ignore = "needs the browser files (Emscripten) and Chromium"]
fn on_the_page_thread_a_click_runs_the_servers_handler_too() {
    let page = PAGE.replace("<script src=\"clarity-ui-page.js\"></script>", "<script>window.clarityUiOptions = { thread: \"page\" };</script><script src=\"clarity-ui-page.js\"></script>");
    let site = Site { page, outer: None, main: MAIN, model: "", answers: Arc::new(answer) };
    let Opened { scratch, asked, results, _browser, .. } = open("page-thread", site, &["--window-size=900,600", "--force-device-scale-factor=1"]);
    let result = results.recv_timeout(Duration::from_secs(90)).expect("the page reported within 90 s");
    assert!(result.get("error").is_none(), "the page failed: {}", result["error"]);
    assert_eq!(result["inPage"], json!(true));
    assert!(asked.lock().unwrap().contains(&"dispatch_event".to_string()));
    assert_eq!(result["after"]["data"]["answer"], json!(ANSWER), "{}", result["after"]);
    save_shot(&scratch, "page-thread.png", &result["shot"]);
    // Reloaded from its files on the same canvas, it opens its window again and lays out its parts.
    assert_eq!(result["reloaded"], Value::Null);
    assert_eq!(result["again"], json!("Main"));
    assert!(result["laid"].as_array().unwrap().contains(&json!("greet")), "{}", result["laid"]);
}

/// A phone-sized page at two device pixels per CSS pixel: the canvas fills
/// the page, it draws at the device's pixels (sharp text), and a finger's
/// tap (pointer events of type touch) clicks.
#[test]
#[ignore = "needs the browser files (Emscripten) and Chromium"]
fn a_phone_sized_page_draws_sharp_fills_the_page_and_a_tap_clicks() {
    // A finger must say what it is: a touch pointer, the primary one.
    let page = PAGE.replace("button: 0, bubbles: true };", r#"button: 0, pointerId: 2, isPrimary: true, pointerType: "touch", bubbles: true };"#);
    assert_ne!(page, PAGE);
    // A headless window is at least 500 wide: the phone's page is a frame of a phone's size in it.
    let outer = r#"<!doctype html><html><head><meta charset="utf-8"></head><body style="margin:0"><iframe src="/page" width="390" height="844" style="border:0;display:block"></iframe></body></html>"#;
    let site = Site { page, outer: Some(outer.to_string()), main: MAIN, model: "", answers: Arc::new(answer) };
    let Opened { scratch, asked, results, _browser, .. } = open("phone", site, &["--window-size=900,1000", "--force-device-scale-factor=2", "--touch-events=enabled"]);
    let result = results.recv_timeout(Duration::from_secs(90)).expect("the page reported within 90 s");
    assert!(result.get("error").is_none(), "the page failed: {}", result["error"]);
    save_shot(&scratch, "browser-phone.png", &result["shot"]);
    assert_eq!(result["dpr"].as_f64(), Some(2.0));
    // The canvas fills the page.
    let b: Vec<f64> = result["box"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
    assert_eq!((b[0], b[1]), (b[2], b[3]), "the canvas is the page's size: {b:?}");
    assert_eq!((b[2], b[3]), (390.0, 844.0), "a phone's size: {b:?}");
    // Drawn at the device's pixels, not scaled up from CSS pixels.
    let shot = png(&result["shot"]);
    assert_eq!((shot.width() as f64, shot.height() as f64), (b[2] * 2.0, b[3] * 2.0));
    // The tap ran the handler, and the window fits the width.
    assert!(asked.lock().unwrap().contains(&"dispatch_event".to_string()));
    assert_eq!(result["after"]["data"]["answer"], json!(ANSWER), "{}", result["after"]);
    let at: Vec<f64> = result["read"]["shown"]["answer"].as_array().expect("the answer's place").iter().map(|v| v.as_f64().unwrap()).collect();
    assert!(at[0] + at[2] <= b[2] + 0.5, "the answer's box fits the phone's width: {at:?}");
    // Sharp text: hard edges, a dark pixel beside a light one somewhere in the title's row.
    let pixels = pixels(&shot);
    let title: Vec<f64> = result["read"]["shown"]["title"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
    let row = ((title[1] + title[3] / 2.0) * 2.0) as i32;
    let width = shot.width();
    let light = |x: i32| pixels[((row * width + x) * 4) as usize..][..3].iter().map(|c| *c as i32).sum::<i32>();
    let steepest = (((title[0] * 2.0) as i32)..(((title[0] + title[2]) * 2.0) as i32 - 1).min(width - 1)).map(|x| (light(x) - light(x + 1)).abs()).max().unwrap_or(0);
    assert!(steepest > 300, "the title's text has no hard edge: {steepest}");
}

const LONG_MAIN: &str = r#"<Window name="Main">
  <Button name="long" at="1,1" span="4" caption="Long" onclick="long" />
  <Textbox name="answer" at="1,2" span="8" bind="answer" />
</Window>
"#;

/// The page clicks Long, reads the page while the handler runs (the worker
/// is not held), finds the bar's Stop and clicks it, and waits for the answer.
const LONG_PAGE: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Clarity.UI</title>
<style>html, body { margin: 0; height: 100%; overflow: hidden; } #clarity-ui { width: 100vw; height: 100vh; }</style>
</head><body><canvas id="clarity-ui"></canvas>
<script src="clarity-ui-page.js"></script>
<script>
(async () => {
  const report = (what) => fetch("/result", { method: "POST", body: JSON.stringify(what) });
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  try {
    const ui = window.clarityUi;
    await ui.started;
    const click = (place) => {
      const [x, y, w, h] = place;
      const at = ui.canvas.getBoundingClientRect();
      const point = { clientX: at.left + x + w / 2, clientY: at.top + y + h / 2, button: 0, pointerId: 1, isPrimary: true, pointerType: "mouse", bubbles: true };
      ui.canvas.dispatchEvent(new PointerEvent("pointermove", point));
      ui.canvas.dispatchEvent(new PointerEvent("pointerdown", point));
      ui.canvas.dispatchEvent(new PointerEvent("pointerup", point));
    };
    const read = await ui.step({ do: "read" });
    const clicked = Date.now();
    click(read.shown.long);
    let stop = null, during = null, reads = 0;
    for (let i = 0; i < 100 && !stop; i++) {
      during = await ui.step({ do: "read" });
      reads++;
      stop = (during.buttons || []).find((b) => b.text === "Stop");
      if (!stop) await sleep(50);
    }
    const barAfter = Date.now() - clicked;
    await sleep(400);
    const bar = await ui.shot();
    if (stop) click(stop.at);
    let after = null;
    for (let i = 0; i < 100; i++) {
      after = await ui.step({ do: "read" });
      if (after.data && after.data.answer) break;
      await sleep(50);
    }
    report({ barAfter, reads, during, after, bar, answeredAfter: Date.now() - clicked, stop: !!stop });
  } catch (why) {
    report({ error: String(why && why.stack || why) });
  }
})();
</script></body></html>
"#;

/// The server's long handler: it says it is one of four along and runs
/// until its watch is stopped (at most `LONG`), then answers how it ended.
const LONG: Duration = Duration::from_secs(8);

#[derive(Default)]
struct Watches {
    running: Option<u64>,
    stopped: BTreeSet<u64>,
    ended: BTreeSet<u64>,
}

fn long_answers(watches: Arc<Mutex<Watches>>) -> Answers {
    Arc::new(move |command: &str, args: &Value| {
        let id = args["id"].as_u64().unwrap_or(0);
        match command {
            "watches" => json!({ "value": true }),
            "start_watch" => json!({ "value": null }),
            "watch_progress" => {
                let running = watches.lock().unwrap().running == Some(id);
                json!({ "value": if running { json!([1, 4]) } else { Value::Null } })
            }
            "stop_watch" => {
                watches.lock().unwrap().stopped.insert(id);
                json!({ "value": null })
            }
            "end_watch" => {
                let mut w = watches.lock().unwrap();
                w.ended.insert(id);
                if w.running == Some(id) {
                    w.running = None;
                }
                json!({ "value": null })
            }
            "dispatch_event" if args["handler"] == json!("long") => {
                let watch = args["watch"].as_u64().expect("a long call's request names its watch");
                watches.lock().unwrap().running = Some(watch);
                let until = Instant::now() + LONG;
                while Instant::now() < until && !watches.lock().unwrap().stopped.contains(&watch) {
                    std::thread::sleep(Duration::from_millis(20));
                }
                let mut state: WindowState = serde_json::from_value(args["state"].clone()).unwrap();
                let stopped = watches.lock().unwrap().stopped.contains(&watch);
                state.data.insert("answer".into(), json!(if stopped { "stopped" } else { "finished" }));
                json!({ "value": serde_json::to_value(super::core_windows_fake::reply(state)).unwrap() })
            }
            other => answer(other, args),
        }
    })
}

/// A long call in the browser: the handler takes seconds on the server, the
/// page draws on and reads meanwhile, the bar shows how far it is, and its
/// Stop reaches the server's watch, which ends the handler early.
#[test]
#[ignore = "needs the browser files (Emscripten) and Chromium"]
fn a_long_call_in_the_browser_shows_its_bar_and_stop_stops_it() {
    let watches: Arc<Mutex<Watches>> = Arc::default();
    let site = Site { page: LONG_PAGE.to_string(), outer: None, main: LONG_MAIN, model: "", answers: long_answers(watches.clone()) };
    let Opened { scratch, asked, results, _browser, .. } = open("long", site, &["--window-size=900,600", "--force-device-scale-factor=1"]);
    let result = results.recv_timeout(Duration::from_secs(90)).expect("the page reported within 90 s");
    assert!(result.get("error").is_none(), "the page failed: {}", result["error"]);
    save_shot(&scratch, "browser-long-bar.png", &result["bar"]);
    let asked = asked.lock().unwrap().clone();
    // The worker was not held: the bar showed and the page read while the handler ran.
    assert!(result["stop"].as_bool().unwrap(), "the bar's Stop showed: {}", result["during"]);
    assert!(result["barAfter"].as_u64().unwrap() < 3000, "the bar showed while the handler ran: {}", result["barAfter"]);
    assert!(asked.contains(&"watch_progress".to_string()), "the bar asked how far: {asked:?}");
    assert!(asked.contains(&"stop_watch".to_string()), "Stop went to the server: {asked:?}");
    // Stop ended the handler early, its answer was taken and the watch ended.
    assert_eq!(result["after"]["data"]["answer"], json!("stopped"), "{}", result["after"]);
    assert!(result["answeredAfter"].as_u64().unwrap() < LONG.as_millis() as u64, "{}", result["answeredAfter"]);
    let w = watches.lock().unwrap();
    assert_eq!(w.stopped.len(), 1);
    assert!(w.ended.is_superset(&w.stopped), "end_watch followed: {:?}", w.ended);
    // The bar is gone.
    assert!(!result["after"]["buttons"].as_array().unwrap().iter().any(|b| b["text"] == json!("Stop")), "{}", result["after"]["buttons"]);
}

/// The page's own handling of what the person does, checked in the browser
/// by what it sends the worker and what it leaves the browser to do: AltGr,
/// the Mac's Option and dead keys write text, keys the browser keeps are
/// left to it, a phone's keyboard edits, a cancelled touch clicks nothing,
/// sound plays through Web Audio, and links and shown files open only what
/// is safe.
const INPUT_PAGE: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Clarity.UI</title>
<style>html, body { margin: 0; height: 100%; overflow: hidden; } #clarity-ui { width: 100vw; height: 100vh; }</style>
</head><body><canvas id="clarity-ui"></canvas>
<script src="clarity-ui-page.js"></script>
<script>
(async () => {
  const report = (what) => fetch("/result", { method: "POST", body: JSON.stringify(what) });
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  try {
    const ui = window.clarityUi;
    await ui.started;
    const sent = [];
    const post = ui.worker.postMessage.bind(ui.worker);
    ui.worker.postMessage = (message, transfer) => { sent.push(message); post(message, transfer); };
    const opened = [];
    window.open = (url) => { opened.push(String(url).slice(0, 5)); return null; };
    const typing = document.querySelector("textarea");
    typing.focus();
    const out = {};
    const run = (name, events) => {
      const from = sent.length;
      const prevented = events.map((e) => { typing.dispatchEvent(e); return e.defaultPrevented; });
      out[name] = { prevented, sent: sent.slice(from).filter((m) => m.type === "key" || m.type === "text").map((m) => m.type === "key" ? [m.down ? "down" : "up", m.key, m.held] : ["text", m.text]) };
    };
    const key = (type, init) => new KeyboardEvent(type, Object.assign({ bubbles: true, cancelable: true }, init));
    const typed = (text) => { typing.value = text; return new InputEvent("input", { bubbles: true, inputType: "insertText", data: text }); };
    // A touch the page takes away clicks nothing.
    const read = await ui.step({ do: "read" });
    const [x, y, w, h] = read.shown.greet;
    const at = ui.canvas.getBoundingClientRect();
    const point = { clientX: at.left + x + w / 2, clientY: at.top + y + h / 2, button: 0, pointerId: 2, isPrimary: true, pointerType: "touch", bubbles: true };
    const from = sent.length;
    ui.canvas.dispatchEvent(new PointerEvent("pointerdown", point));
    ui.canvas.dispatchEvent(new PointerEvent("pointercancel", point));
    ui.canvas.dispatchEvent(new PointerEvent("pointerup", point));
    out.touchKinds = sent.slice(from).filter((m) => m.type === "pointer").map((m) => m.kind);
    await sleep(300);
    out.afterCancel = (await ui.step({ do: "read" })).data;

    run("altGraph", [key("keydown", { key: "@", code: "KeyQ", ctrlKey: true, altKey: true, modifierAltGraph: true }), typed("@")]);
    run("option", [key("keydown", { key: "å", code: "KeyA", altKey: true }), typed("å")]);
    run("altShortcut", [key("keydown", { key: "a", code: "KeyA", altKey: true })]);
    run("dead", [key("keydown", { key: "Dead", code: "Quote" }), key("keydown", { key: "é", code: "KeyE" }), typed("é")]);
    run("kept", [key("keydown", { key: "F5", code: "F5" }), key("keydown", { key: "+", code: "Equal", ctrlKey: true }), key("keydown", { key: "r", code: "KeyR", ctrlKey: true })]);
    run("shortcut", [key("keydown", { key: "s", code: "KeyS", ctrlKey: true }), key("keydown", { key: "Tab", code: "Tab" })]);
    run("phone", [key("keydown", { key: "Unidentified", keyCode: 229 }), typed("x"), new InputEvent("beforeinput", { bubbles: true, cancelable: true, inputType: "deleteContentBackward" }), new InputEvent("beforeinput", { bubbles: true, cancelable: true, inputType: "insertLineBreak" })]);

    // Sound through Web Audio: played to its end, the worker hears it.
    ui.worker.onmessage({ data: { type: "tell", tell: "play", id: 77, data: "__WAV__", volume: 1, repeat: false } });
    for (let i = 0; i < 60 && !sent.some((m) => m.type === "sound-ended" && m.id === 77); i++) await sleep(50);
    out.soundEnded = sent.some((m) => m.type === "sound-ended" && m.id === 77);

    // Links and shown files: only what is safe opens.
    const tell = (t) => ui.worker.onmessage({ data: Object.assign({ type: "tell" }, t) });
    tell({ tell: "open", address: "javascript:alert(1)" });
    tell({ tell: "open", address: "data:text/html,x" });
    tell({ tell: "open", address: "https://example.org" });
    tell({ tell: "show", name: "a.html", data: "PGI+PC9iPg==" });
    tell({ tell: "show", name: "a.pdf", data: "JVBERg==", type: "application/pdf" });
    out.opened = opened;
    report(out);
  } catch (why) {
    report({ error: String(why && why.stack || why) });
  }
})();
</script></body></html>
"#;

/// A short silent WAV (8 kHz, 16 bits, a tenth of a second), as base64.
fn silent_wav() -> String {
    let samples = 800u32;
    let data = samples * 2;
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&8000u32.to_le_bytes());
    wav.extend_from_slice(&16000u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data.to_le_bytes());
    wav.resize(wav.len() + data as usize, 0);
    base64::engine::general_purpose::STANDARD.encode(wav)
}

#[test]
#[ignore = "needs the browser files (Emscripten) and Chromium"]
fn the_pages_keys_touch_sound_and_links_in_the_browser() {
    let page = INPUT_PAGE.replace("__WAV__", &silent_wav());
    let site = Site { page, outer: None, main: MAIN, model: "", answers: Arc::new(answer) };
    let Opened { results, _browser, .. } = open("input", site, &["--window-size=900,600", "--force-device-scale-factor=1", "--autoplay-policy=no-user-gesture-required"]);
    let r = results.recv_timeout(Duration::from_secs(90)).expect("the page reported within 90 s");
    assert!(r.get("error").is_none(), "the page failed: {}", r["error"]);
    let case = |name: &str| (r[name]["prevented"].clone(), r[name]["sent"].clone());
    // AltGr and Option write their text: the key goes without Ctrl and Alt, the browser writes.
    assert_eq!(case("altGraph"), (json!([false, false]), json!([["down", "@", 0], ["text", "@"]])), "{}", r["altGraph"]);
    assert_eq!(case("option"), (json!([false, false]), json!([["down", "\u{e5}", 0], ["text", "\u{e5}"]])), "{}", r["option"]);
    // Alt with a plain letter is a shortcut.
    assert_eq!(case("altShortcut"), (json!([true]), json!([["down", "a", 4]])));
    // A dead key sends nothing and is left to the browser; its letter comes as text.
    assert_eq!(case("dead"), (json!([false, false, false]), json!([["down", "\u{e9}", 0], ["text", "\u{e9}"]])), "{}", r["dead"]);
    // Keys the browser keeps are its own.
    assert_eq!(case("kept"), (json!([false, false, false]), json!([])));
    assert_eq!(case("shortcut"), (json!([true, true]), json!([["down", "s", 2], ["down", "Tab", 0]])));
    // A phone's keyboard: text typed, deleting and a new line as keys.
    assert_eq!(
        case("phone"),
        (json!([false, false, true, true]), json!([["text", "x"], ["down", "Backspace", 0], ["up", "Backspace", 0], ["down", "Enter", 0], ["up", "Enter", 0]])),
        "{}",
        r["phone"]
    );
    // A cancelled touch: pressed, cancelled, released, and no handler ran.
    assert_eq!(r["touchKinds"], json!([1, 4, 2]));
    assert_eq!(r["afterCancel"]["answer"], json!(""), "{}", r["afterCancel"]);
    // (Enter on the focused button afterwards runs it: the keys come after the touch.)
    assert_eq!(r["soundEnded"], json!(true), "the page's sound did not end");
    // Only the web page and the PDF opened: no javascript:, data: or HTML.
    assert_eq!(r["opened"], json!(["https", "blob:"]));
}

/// A window for the mirror: a heading, a button, labelled inputs: one the
/// program writes, one of a field of the model that the person writes (an
/// edit the server is told of, `edit_event`).
pub(super) const ACCESS_MAIN: &str = r#"<Window name="Main">
  <Heading name="title" at="1,1" span="12" caption="Clarity.UI in the browser" />
  <Button name="greet" at="1,2" span="4" caption="Greet" onclick="greet" />
  <Label name="answerLabel" at="1,3" span="3" caption="Answer" for="answer" />
  <Textbox name="answer" at="4,3" span="8" bind="answer" />
  <Label name="noteLabel" at="1,4" span="3" caption="Note" for="note" />
  <Textbox name="note" at="4,4" span="8" bind="greeting.note" undoScope="note" />
</Window>
"#;

/// The model of the mirror's window: its `greeting` record, whose `note` the person writes.
pub(super) const ACCESS_MODEL: &str = r##"{
  "project": {
    "locale": { "language": "en" },
    "files": [{
      "pages": [ { "name": { "name": "Main" }, "kind": "window", "title": "Greeter",
        "data": [ { "name": { "name": "greeting" }, "of": { "kind": "record", "entity": { "name": "Greeting" } } } ] } ],
      "apps": [], "enums": [], "records": [],
      "entities": [ { "name": { "name": "Greeting" }, "fields": [ { "name": { "name": "note" }, "type": { "kind": "text" }, "label": "Note", "modifiers": {} } ] } ]
    }]
  }
}"##;

/// The page's accessible mirror (A1 phase 2) as a screen reader uses it:
/// the roles and names are there; a click on the mirrored button runs its
/// handler; text typed into the mirrored input is typed, a value set on it
/// is written; the DOM's focus follows the screen's (Tab from the keys).
const ACCESS_PAGE: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Clarity.UI</title>
<style>html, body { margin: 0; height: 100%; overflow: hidden; } #clarity-ui { width: 100vw; height: 100vh; }</style>
</head><body><canvas id="clarity-ui"></canvas>
<script>window.addEventListener("error", (e) => fetch("/result", { method: "POST", body: JSON.stringify({ error: e.message + " at " + e.filename + ":" + e.lineno }) }));</script>
<script src="clarity-ui-page.js"></script>
<script>
(async () => {
  const report = (what) => fetch("/result", { method: "POST", body: JSON.stringify(what) });
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (what, test) => {
    for (let i = 0; i < 100; i++) {
      const found = await test();
      if (found) return found;
      await sleep(50);
    }
    const active = document.activeElement;
    throw new Error("waited in vain for " + what + "; the DOM's focus on " + (active ? active.outerHTML.slice(0, 200) : "nothing") + "; seen " + JSON.stringify(window.lastSeen));
  };
  try {
    const ui = window.clarityUi;
    await ui.started;
    const mirror = ui.mirror;
    const sent = [];
    const post = ui.worker.postMessage.bind(ui.worker);
    ui.worker.postMessage = (message, transfer) => { if (message.type !== "step") sent.push(message); post(message, transfer); };
    window.sent = sent;
    const button = await until("the mirrored button", () => mirror.querySelector('[role="button"][aria-label="Greet"]'));
    const nodes = [mirror, ...mirror.querySelectorAll("[data-clarity-node]")].map((e) => ({ tag: e.tagName.toLowerCase(), role: e.getAttribute("role"), label: e.getAttribute("aria-label"), text: e.firstChild && e.firstChild.nodeType === 3 ? e.firstChild.data : null, at: [e.offsetLeft, e.offsetTop, e.offsetWidth, e.offsetHeight] }));
    const read = await ui.step({ do: "read" });

    // A screen reader's click on the mirrored button.
    button.click();
    const clicked = await until("the handler's answer", async () => { const r = await ui.step({ do: "read" }); return r.data && r.data.answer ? r : null; });

    // Its focus on the mirrored input, then text typed, then a value set.
    const input = mirror.querySelector('input[aria-label="Note"]');
    input.focus();
    const focusedInput = await until("the screen's focus on the input", async () => { const r = await ui.step({ do: "read" }); return r.focused === "note" ? r : null; });
    input.value = input.value + "!";
    input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: "!" }));
    // Typed where the screen's caret is: the screen's input holds it, and the mirror shows what the screen holds.
    const typed = await until("the typed text in the mirror", async () => {
      const node = (await ui.step({ do: "access" })).access.nodes.find((n) => n.part === "note");
      window.lastSeen = [node && node.value, input.value, window.sent.slice(-6)];
      return node && node.value && node.value.includes("!") && input.value === node.value ? node.value : null;
    });
    input.value = "Grace";
    input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertReplacementText" }));
    const set = await until("the value set", async () => {
      const node = (await ui.step({ do: "access" })).access.nodes.find((n) => n.part === "note");
      const data = (await ui.step({ do: "read" })).data;
      window.lastSeen = [node && node.value, input.value, data, window.sent.slice(-3)];
      return node && node.value === "Grace" && input.value === "Grace" && data.greeting && data.greeting.note === "Grace" ? data : null;
    });

    // The input the program filled, bound to the window's own data: set too.
    const filled = mirror.querySelector('input[aria-label="Answer"]');
    filled.focus();
    await sleep(200);
    filled.value = filled.value + "?";
    filled.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: "?" }));
    await sleep(200);
    filled.value = "Ada";
    filled.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertReplacementText" }));
    let answerSet = null;
    for (let i = 0; i < 40 && !(answerSet && answerSet.answer === "Ada"); i++) { await sleep(50); answerSet = (await ui.step({ do: "read" })).data; }
    const answerShown = filled.value;

    // The person's Tab moves the screen's focus, and the DOM's follows.
    const before = document.activeElement.getAttribute("data-clarity-node");
    const tab = (type) => document.activeElement.dispatchEvent(new KeyboardEvent(type, { key: "Tab", code: "Tab", bubbles: true, cancelable: true }));
    tab("keydown");
    tab("keyup");
    const after = await until("the DOM's focus to move", () => { const now = document.activeElement.getAttribute("data-clarity-node"); return now && now !== before ? now : null; });
    const access = (await ui.step({ do: "access" })).access;
    report({ nodes, read, typed, answerSet, answerShown, clicked: clicked.data, focusedInput: focusedInput.focused, set, inputValue: input.value, before, after, treeFocus: String(access.focus), active: document.activeElement.getAttribute("aria-label"), live: [...document.querySelectorAll("[data-clarity-live]")].map((e) => e.getAttribute("role")) });
  } catch (why) {
    report({ error: String(why && why.stack || why) });
  }
})();
</script></body></html>
"#;

/// The DevTools protocol of the test's Chromium, over a WebSocket of its own
/// (text frames, masked with zeros, as a client must mask).
struct Devtools {
    stream: TcpStream,
    next: u64,
}

impl Devtools {
    /// The page's session: the port Chromium wrote into the profile, its page's socket.
    fn page(profile: &Path) -> Devtools {
        let started = Instant::now();
        let port: u16 = loop {
            if let Some(port) = std::fs::read_to_string(profile.join("DevToolsActivePort")).ok().and_then(|t| t.lines().next().and_then(|l| l.trim().parse().ok())) {
                break port;
            }
            assert!(started.elapsed() < Duration::from_secs(20), "Chromium wrote no DevToolsActivePort");
            std::thread::sleep(Duration::from_millis(100));
        };
        let mut list = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(list, "GET /json/list HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n").unwrap();
        // Read by its length: the DevTools server may keep the connection open.
        list.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        let mut reader = BufReader::new(list);
        let mut length = 0;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            if header.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                length = value.trim().parse().unwrap_or(0);
            }
        }
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).unwrap();
        let targets: Value = serde_json::from_slice(&body).expect("the targets read");
        let url = targets.as_array().unwrap().iter().find(|t| t["type"] == json!("page")).and_then(|t| t["webSocketDebuggerUrl"].as_str()).expect("a page target").to_string();
        let path = url.splitn(4, '/').nth(3).map(|p| format!("/{p}")).unwrap();
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(stream, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n").unwrap();
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            head.push(byte[0]);
        }
        assert!(head.starts_with(b"HTTP/1.1 101"), "{}", String::from_utf8_lossy(&head));
        stream.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
        Devtools { stream, next: 0 }
    }

    fn frame(&mut self) -> (u8, bool, Vec<u8>) {
        let mut two = [0u8; 2];
        self.stream.read_exact(&mut two).unwrap();
        let len = match two[1] & 0x7f {
            126 => {
                let mut b = [0u8; 2];
                self.stream.read_exact(&mut b).unwrap();
                u16::from_be_bytes(b) as usize
            }
            127 => {
                let mut b = [0u8; 8];
                self.stream.read_exact(&mut b).unwrap();
                u64::from_be_bytes(b) as usize
            }
            n => n as usize,
        };
        let mut payload = vec![0u8; len];
        self.stream.read_exact(&mut payload).unwrap();
        (two[0] & 0x0f, two[0] & 0x80 != 0, payload)
    }

    /// Calls `method` and waits for its answer, past the events that come between.
    fn call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let text = json!({ "id": self.next, "method": method, "params": params }).to_string();
        let mut out = vec![0x81u8];
        match text.len() {
            n if n < 126 => out.push(0x80 | n as u8),
            n if n < 65536 => {
                out.push(0x80 | 126);
                out.extend_from_slice(&(n as u16).to_be_bytes());
            }
            n => {
                out.push(0x80 | 127);
                out.extend_from_slice(&(n as u64).to_be_bytes());
            }
        }
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(text.as_bytes());
        self.stream.write_all(&out).unwrap();
        loop {
            let mut message = Vec::new();
            loop {
                let (_, last, payload) = self.frame();
                message.extend_from_slice(&payload);
                if last {
                    break;
                }
            }
            let Ok(answer) = serde_json::from_slice::<Value>(&message) else { continue };
            if answer["id"] == json!(self.next) {
                return answer;
            }
        }
    }
}

#[test]
#[ignore = "needs the browser files (Emscripten) and Chromium"]
fn screen_readers_read_and_use_the_pages_mirror() {
    let site = Site { page: ACCESS_PAGE.to_string(), outer: None, main: ACCESS_MAIN, model: ACCESS_MODEL, answers: Arc::new(answer) };
    let Opened { scratch, results, asked, _browser, .. } = open("access", site, &["--window-size=900,600", "--force-device-scale-factor=1", "--remote-debugging-port=0"]);
    let r = results.recv_timeout(Duration::from_secs(90)).expect("the page reported within 90 s");
    assert!(r.get("error").is_none(), "the page failed: {}; the server was asked {:?}", r["error"], asked.lock().unwrap());

    // Roles and names: the window as an application, the heading, the button, the label as plain text, the input.
    let nodes = r["nodes"].as_array().unwrap();
    let has = |tag: &str, role: Value, label: Value| nodes.iter().any(|n| n["tag"] == json!(tag) && n["role"] == role && n["label"] == label);
    assert!(has("div", json!("application"), json!("Greeter")), "{nodes:?}");
    assert!(has("div", json!("heading"), json!("Clarity.UI in the browser")), "{nodes:?}");
    assert!(has("div", json!("button"), json!("Greet")), "{nodes:?}");
    assert!(has("input", Value::Null, json!("Answer")), "{nodes:?}");
    assert!(nodes.iter().any(|n| n["role"].is_null() && n["label"].is_null() && n["text"] == json!("Answer")), "the label is plain text: {nodes:?}");
    // Placed where the parts show.
    let greet = nodes.iter().find(|n| n["label"] == json!("Greet")).unwrap();
    let shown: Vec<f64> = r["read"]["shown"]["greet"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
    let at: Vec<f64> = greet["at"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
    assert!((at[0] - shown[0]).abs() <= 1.0 && (at[1] - shown[1]).abs() <= 1.0 && (at[2] - shown[2]).abs() <= 1.0, "{at:?} {shown:?}");

    // The click ran the handler; the input took typed text and a set value; the focus followed.
    assert_eq!(r["clicked"]["answer"], json!(ANSWER));
    assert_eq!(r["focusedInput"], json!("note"));
    assert!(r["typed"].as_str().is_some_and(|t| t.contains('!')), "{}", r["typed"]);
    // Written to the model's field and told to the server as the person's edit.
    assert_eq!(r["set"]["greeting"]["note"], json!("Grace"), "{}", r["set"]);
    assert!(asked.lock().unwrap().contains(&"edit_event".to_string()), "{:?}", asked.lock().unwrap());
    // The input bound to the window's own data (no field of the model): typed into, then set, it keeps the value.
    assert_eq!(r["answerSet"]["answer"], json!("Ada"), "{}", r["answerSet"]);
    assert_eq!(r["answerShown"], json!("Ada"));
    assert_eq!(r["inputValue"], json!("Grace"));
    assert_ne!(r["before"], r["after"]);
    assert_eq!(r["after"], r["treeFocus"], "the DOM's focus is on the tree's");
    assert_eq!(r["live"], json!(["status", "alert"]));

    // Chromium's own accessibility tree names them, as a screen reader gets it.
    let profile = scratch.join("profile-access");
    let mut devtools = Devtools::page(&profile);
    devtools.call("Accessibility.enable", json!({}));
    let tree = devtools.call("Accessibility.getFullAXTree", json!({}));
    let ax = tree["result"]["nodes"].as_array().unwrap_or_else(|| panic!("{tree}"));
    let named = |role: &str, name: &str| ax.iter().any(|n| n["role"]["value"] == json!(role) && n["name"]["value"] == json!(name) && n["ignored"] != json!(true));
    let seen: Vec<(Value, Value)> = ax.iter().filter(|n| n["ignored"] != json!(true)).map(|n| (n["role"]["value"].clone(), n["name"]["value"].clone())).collect();
    assert!(named("button", "Greet"), "{seen:?}");
    assert!(named("textbox", "Answer"), "{seen:?}");
    assert!(named("textbox", "Note"), "{seen:?}");
    assert!(named("heading", "Clarity.UI in the browser"), "{seen:?}");
    assert!(named("application", "Greeter"), "{seen:?}");
}

/// A window with every kind of part, for the mirror's cost per frame.
const EVERY: &str = r#"<Window name="Main">
<Heading name="title" at="1,1" span="6" caption="Every part" />
<Label name="label" at="1,2" span="2" caption="Name" for="name" />
<Textbox name="name" at="3,2" span="4" bind="name" />
<Label name="passLabel" at="7,2" span="2" caption="Password" for="pass" />
<Password name="pass" at="9,2" span="4" bind="pass" />
<Label name="ownerLabel" at="1,3" span="2" caption="Owner" />
<Dropdown name="owner" at="3,3" span="4" bind="owner" />
<Label name="countLabel" at="7,3" span="2" caption="Count" />
<Number name="count" at="9,3" span="4" bind="count" />
<Checkbox name="done" at="1,4" span="3" bind="done" caption="Done" />
<Switch name="urgent" at="4,4" span="3" bind="urgent" caption="Urgent" />
<Radio name="kind" at="7,4" span="6" bind="kind" />
<Slider name="level" at="1,5" span="6" bind="level" />
<Progress name="part" at="7,5" span="6" bind="level" />
<Button name="greet" at="1,6" span="2" caption="Greet" onclick="greet" />
<Button name="cancel" at="3,6" span="2" caption="Cancel" />
<Date name="due" at="7,6" span="6" bind="due" />
<Memo name="notes" at="1,7" span="6" rows="2" bind="notes" />
<Color name="tint" at="7,7" span="6" bind="tint" />
<Tabs name="pages" at="1,9" span="12" rows="3">
  <Tab title="One"><Label name="first" at="1,1" span="3" caption="First page" /></Tab>
  <Tab title="Two"><Label name="second" at="1,1" span="3" caption="Second page" /></Tab>
</Tabs>
</Window>
"#;

/// The mirror's cost per drawn frame on a window with every kind of part,
/// on the page's thread (where the test can time the library's frames):
/// Tab moves the focus each time, so each frame draws and the tree changes
/// (the mirror is told and moves the DOM's focus); timed with the mirror on and off.
const COST_PAGE: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Clarity.UI</title>
<style>html, body { margin: 0; height: 100%; overflow: hidden; } #clarity-ui { width: 100vw; height: 100vh; }</style>
</head><body><canvas id="clarity-ui"></canvas>
<script>
window.clarityUiOptions = { thread: "page" };
const frames = [];
const raf = window.requestAnimationFrame.bind(window);
window.requestAnimationFrame = (act) => raf((t) => { const from = performance.now(); act(t); frames.push(performance.now() - from); });
</script>
<script src="clarity-ui-page.js"></script>
<script>
(async () => {
  const report = (what) => fetch("/result", { method: "POST", body: JSON.stringify(what) });
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  try {
    const ui = window.clarityUi;
    await ui.started;
    for (let i = 0; i < 100 && !ui.mirror.querySelector('[role="button"]'); i++) await sleep(50);
    let applied = 0;
    const onmessage = ui.worker.onmessage;
    ui.worker.onmessage = (event) => { const from = performance.now(); onmessage(event); if (event.data.tell === "access") applied += performance.now() - from; };
    const run = async () => {
      frames.length = 0;
      applied = 0;
      const typing = document.querySelector("textarea[data-clarity-typing]");
      for (let i = 0; i < 30; i++) {
        for (const type of ["keydown", "keyup"]) typing.dispatchEvent(new KeyboardEvent(type, { key: "Tab", code: "Tab", bubbles: true, cancelable: true }));
        await sleep(40);
      }
      const drawn = frames.filter((f) => f > 0.5);
      return { frames: drawn.length, ms: drawn.reduce((a, b) => a + b, 0) / Math.max(drawn.length, 1), applied: applied / 30 };
    };
    await run();
    const on = await run();
    const nodes = ui.mirror.querySelectorAll("[data-clarity-node]").length;
    ui.worker.postMessage({ type: "access", on: false });
    await sleep(100);
    const off = await run();
    report({ on, off, nodes, roles: [...new Set([ui.mirror, ...ui.mirror.querySelectorAll("[role]")].map((e) => e.getAttribute("role")))], inputs: [...ui.mirror.querySelectorAll("input, textarea")].map((e) => e.tagName.toLowerCase() + ":" + e.type) });
  } catch (why) {
    report({ error: String(why && why.stack || why) });
  }
})();
</script></body></html>
"#;

#[test]
#[ignore = "needs the browser files (Emscripten) and Chromium"]
fn the_mirror_costs_little_per_frame_on_every_kind_of_part() {
    let site = Site { page: COST_PAGE.to_string(), outer: None, main: EVERY, model: "", answers: Arc::new(answer) };
    let Opened { results, _browser, .. } = open("access-cost", site, &["--window-size=1000,700", "--force-device-scale-factor=1"]);
    let r = results.recv_timeout(Duration::from_secs(120)).expect("the page reported within 120 s");
    assert!(r.get("error").is_none(), "the page failed: {}", r["error"]);
    eprintln!("mirror cost on every kind of part ({} nodes): on {} / off {}", r["nodes"], r["on"], r["off"]);
    let roles = r["roles"].as_array().unwrap();
    for role in ["application", "heading", "button", "checkbox", "switch", "radiogroup", "slider", "progressbar", "spinbutton", "combobox", "tablist", "tab"] {
        assert!(roles.contains(&json!(role)), "no {role} in {roles:?}");
    }
    let inputs = r["inputs"].as_array().unwrap();
    for input in ["input:text", "input:password", "textarea:textarea"] {
        assert!(inputs.contains(&json!(input)), "no {input} in {inputs:?}");
    }
    let (on, off) = (r["on"]["ms"].as_f64().unwrap(), r["off"]["ms"].as_f64().unwrap());
    assert!(r["on"]["frames"].as_u64().unwrap() > 5 && r["off"]["frames"].as_u64().unwrap() > 5, "{r}");
    assert!(r["on"]["applied"].as_f64().unwrap() > 0.0, "the mirror was told: {r}");
    assert!(on < off * 2.0 + 4.0, "the mirror costs too much a frame: {on:.2} ms on, {off:.2} ms off");
}
