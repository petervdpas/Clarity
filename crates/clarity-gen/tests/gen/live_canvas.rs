//! The IDE's design canvas and preview as the IDE starts them, in a real
//! browser: the preview server (`preview::Server`, as the IDE runs it) with
//! Clarity.UI's browser files, and a page that starts a window with the
//! designer's options (`app/src/lib/clarityWindow.ts` startOptions), at a
//! normal size and at none (a canvas covered by Clarity settings).
//!
//! Ignored: it needs the browser files (target/clarity-ui-browser, built
//! through DevTools) and Chromium. Run with `cargo test -p clarity-gen
//! --test live_canvas -- --ignored --nocapture`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use clarity_gen::preview::{self, Server};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// The page: the page script from the server, a canvas of `width` by `height`, a start with the options, and what came of it written into the page.
fn page(base: &str, window: &str, design: bool, width: u32, height: u32) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"></head><body>
<canvas id="c" style="width:{width}px;height:{height}px;display:block"></canvas><pre id="out"></pre>
<script>
const out = (t) => (document.getElementById("out").textContent += t + "\n");
window.addEventListener("error", (e) => out("ERROR " + e.message));
window.addEventListener("unhandledrejection", (e) => out("REJECTED " + (e.reason && e.reason.message || e.reason)));
// As in the IDE's webview (WebKitGTK): no OffscreenCanvas, so the page script draws on the page's own thread.
HTMLCanvasElement.prototype.transferControlToOffscreen = undefined;
const s = document.createElement("script");
s.src = "{base}clarity-ui-page.js";
s.onerror = () => out("ERROR page script did not load");
s.onload = async () => {{
  try {{
    const ui = window.ClarityUiPage.start(document.getElementById("c"), {{
      files: "{base}files.json?window={window}&design={design}&revision=1", chosen: {{}}, server: "{base}invoke",
      resources: "{base}resources/", input: {input}, access: {input}, storage: "s", choices: "c" }});
    out("START CALLED on " + (ui.worker && ui.worker.constructor.name === "Worker" ? "a worker" : "the page thread"));
    await ui.started;
    out("STARTED");
    const read = await ui.step({{ do: "read" }});
    out("READ " + Object.keys(read.shown || {{}}).length + " parts shown");
  }} catch (e) {{ out("FAILED " + (e && e.message || e)); }}
}};
document.head.appendChild(s);
</script></body></html>"#,
        design = if design { 1 } else { 0 },
        input = !design,
    )
}

/// The page the IDE makes of a session: the design canvas and the preview
/// side by side on one page, the design reloaded with new files after an
/// edit, a canvas stopped and another started when the person switches,
/// and both reloaded and drawn a while.
fn session_page(base: &str, window: &str, other: &str) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"></head><body>
<div id="place"></div><pre id="out"></pre>
<script>
const out = (t) => (document.getElementById("out").textContent += t + "\n");
window.addEventListener("error", (e) => out("ERROR " + e.message));
window.addEventListener("unhandledrejection", (e) => out("REJECTED " + (e.reason && e.reason.message || e.reason)));
const options = (w, design, rev) => ({{ files: "{base}files.json?window=" + w + "&design=" + (design ? 1 : 0) + "&revision=" + rev, chosen: {{}}, server: "{base}invoke",
  resources: "{base}resources/", input: !design, access: !design, storage: "s", choices: "c" }});
const pause = (ms) => new Promise((r) => setTimeout(r, ms));
// A canvas of its own for each start, as the IDE mounts one per view; the one it replaces goes.
const canvas = (old) => {{ if (old) old.remove(); const c = document.createElement("canvas"); c.style.cssText = "width:900px;height:600px;display:block"; document.getElementById("place").appendChild(c); return c; }};
// As in the IDE's webview (WebKitGTK): no OffscreenCanvas, so the page script draws on the page's own thread.
HTMLCanvasElement.prototype.transferControlToOffscreen = undefined;
const s = document.createElement("script");
s.src = "{base}clarity-ui-page.js";
s.onload = async () => {{
  try {{
    const page = window.ClarityUiPage;
    let a = canvas(null), b = canvas(null);
    let design = page.start(a, options("{window}", true, 1));
    let shown = page.start(b, options("{window}", false, 1));
    await design.started; await shown.started; out("BOTH STARTED");
    for (let rev = 2; rev < 6; rev++) {{ const why = await design.reload(options("{window}", true, rev).files); out("RELOAD " + rev + " " + (why || "ok")); await pause(200); }}
    design.stop(); out("DESIGN STOPPED");
    a = canvas(a); design = page.start(a, options("{other}", true, 7));
    await design.started; out("OTHER STARTED");
    shown.stop(); b = canvas(b); shown = page.start(b, options("{other}", false, 8));
    await shown.started; out("OTHER SHOWN");
    await pause(1500);
    const read = await design.step({{ do: "read" }}); out("READ " + Object.keys(read.shown || {{}}).length);
    out("STARTED ALL");
  }} catch (e) {{ out("FAILED " + (e && e.message || e)); }}
}};
document.head.appendChild(s);
</script></body></html>"#
    )
}

/// The page of a swap while canvases run, as the IDE does it (DevTools'
/// Swap of Clarity.UI's browser files): the design and the preview drawn
/// with the first files; the page says SWAP, the server then serves the
/// other files at the same addresses; the page forgets Clarity.UI's scripts
/// (app/src/lib/clarityWindow.ts resetPageScript), loads the page script
/// again and starts both canvases anew, the old ones stopped.
fn swap_page(base: &str, window: &str) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"></head><body><div id="place"></div><pre id="out"></pre>
<script>
const out = (t) => (document.getElementById("out").textContent += t + "\n");
window.addEventListener("error", (e) => out("ERROR " + e.message));
window.addEventListener("unhandledrejection", (e) => out("REJECTED " + (e.reason && e.reason.message || e.reason)));
HTMLCanvasElement.prototype.transferControlToOffscreen = undefined;
const options = (design, rev) => ({{ files: "{base}files.json?window={window}&design=" + (design ? 1 : 0) + "&revision=" + rev, chosen: {{}}, server: "{base}invoke",
  resources: "{base}resources/", input: !design, access: !design, storage: "s", choices: "c" }});
const pause = (ms) => new Promise((r) => setTimeout(r, ms));
const canvas = (old) => {{ if (old) old.remove(); const c = document.createElement("canvas"); c.style.cssText = "width:900px;height:600px;display:block"; document.getElementById("place").appendChild(c); return c; }};
// The canvas a start draws on after a swap: a new element in the old one's place (ClarityWindow.svelte). On the same
// element the new library takes over the stopped one's drawing context and aborts ("cannot unwind").
const again = (old) => {{ const c = canvas(null); old.replaceWith(c); return c; }};
const load = () => new Promise((yes, no) => {{ const s = document.createElement("script"); s.src = "{base}clarity-ui-page.js"; s.onload = () => yes(window.ClarityUiPage); s.onerror = () => no(new Error("page script")); document.head.appendChild(s); }});
const forget = () => {{
  for (const name of ["ClarityUiPage", "ClarityUiPageWorker", "ClarityUi"]) {{ try {{ delete window[name]; }} catch {{ window[name] = undefined; }} if (window[name] !== undefined) window[name] = undefined; }}
  for (const sel of ['script[src$="clarity-ui-page.js"]', 'script[src$="clarity-ui-worker.js"]', 'script[src$="clarity-ui.js"]']) for (const e of document.querySelectorAll(sel)) e.remove();
}};
(async () => {{
  try {{
    let page = await load();
    let a = canvas(null), b = canvas(null);
    let design = page.start(a, options(true, 1)), shown = page.start(b, options(false, 1));
    await design.started; await shown.started; out("FIRST STARTED");
    out("SWAP"); await pause(2500);
    // As the IDE's canvas (ClarityWindow.svelte) does on a swap: each drawing stopped, then the new files started in its place.
    design.stop(); shown.stop();
    forget();
    page = await load();
    const design2 = page.start(again(a), options(true, 2));
    const shown2 = page.start(again(b), options(false, 2));
    await design2.started; await shown2.started; out("SWAPPED STARTED");
    await pause(1500);
    const read = await design2.step({{ do: "read" }}); out("READ " + Object.keys(read.shown || {{}}).length);
  }} catch (e) {{ out("FAILED " + (e && e.message || e)); }}
}})();
</script></body></html>"#
    )
}

/// Writes the page of each start, and opens them all in the live tests' browser (live/canvas.mjs): a line per page.
fn run_all(starts: &[(String, String, String, bool, u32, u32)], sessions: &[(String, String)]) -> Vec<String> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("live-canvas");
    std::fs::create_dir_all(&dir).unwrap();
    let pages: Vec<serde_json::Value> = starts
        .iter()
        .map(|(name, base, window, design, width, height)| {
            let file = dir.join(format!("{}.html", name.replace(' ', "-")));
            std::fs::write(&file, page(base, window, *design, *width, *height)).unwrap();
            serde_json::json!({ "name": name, "url": format!("file://{}", file.display()) })
        })
        .chain(sessions.iter().map(|(name, html)| {
            let file = dir.join(format!("{}.html", name.replace(' ', "-")));
            std::fs::write(&file, html).unwrap();
            serde_json::json!({ "name": name, "url": format!("file://{}", file.display()) })
        }))
        .collect();
    let list = dir.join("pages.json");
    std::fs::write(&list, serde_json::to_string(&pages).unwrap()).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/live/canvas.mjs");
    let out = Command::new("node").arg(&script).arg(&dir).arg(&list).output().expect("node runs");
    let said = String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect::<Vec<_>>();
    assert!(!said.is_empty(), "{}", String::from_utf8_lossy(&out.stderr));
    said
}

#[test]
#[ignore = "needs the browser files (target/clarity-ui-browser) and Chromium"]
fn the_canvas_starts_every_way_the_ide_starts_it() {
    let repo = repo();
    // The browser files tried: the cDdev's own build, or those CLARITY_UI_BROWSER names (a release's, to compare).
    let library = std::env::var_os("CLARITY_UI_BROWSER").map(PathBuf::from).unwrap_or_else(|| repo.join("target/clarity-ui-browser"));
    assert!(library.join("clarity-ui.wasm").is_file(), "build the browser files through DevTools first");
    let mut servers = Vec::new();
    let mut starts = Vec::new();
    let mut sessions = Vec::new();
    // invoices shows a report (the pdfReport part): the server draws it with the set of browser files with reports.
    for (example, windows) in [("novel", ["ChaptersPart", "Bookcase"]), ("records", ["AlbumBrowse", "ArtistBrowse"]), ("invoices", ["Orders", "OrdersView"])] {
        let dir = repo.join("examples").join(example);
        let file = repo.join("examples").join(format!("{example}.clarity"));
        let loader: preview::Loader = Arc::new(move || preview::load(&file, &dir));
        let lib = library.clone();
        let server = Server::start(loader, Arc::new(move || Ok(lib.clone())), "test").unwrap();
        for window in windows {
            for (design, width, height) in [(true, 1000, 700), (true, 0, 0), (false, 1000, 700), (false, 0, 0)] {
                starts.push((format!("{example} {window} design={design} {width}x{height}"), server.address().to_string(), window.to_string(), design, width, height));
            }
        }
        sessions.push((format!("{example} session {} then {}", windows[0], windows[1]), session_page(server.address(), windows[0], windows[1])));
        servers.push(server);
    }
    let said = run_all(&starts, &sessions);
    for line in &said {
        eprintln!("{line}");
    }
    let failed: Vec<&String> = said.iter().filter(|l| !l.contains("STARTED") || l.contains("ERROR") || l.contains("call_indirect")).collect();
    assert!(failed.is_empty(), "{failed:#?}");
}

#[test]
#[ignore = "needs the browser files (target/clarity-ui-browser, and a release's in Clarity's cache) and Chromium"]
fn a_swap_of_the_browser_files_while_canvases_run_starts_them_anew() {
    let repo = repo();
    let own = repo.join("target/clarity-ui-browser");
    let release = std::env::var_os("CLARITY_UI_BROWSER").map(PathBuf::from).expect("CLARITY_UI_BROWSER names the release's browser files");
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("live-canvas");
    std::fs::create_dir_all(&dir).unwrap();
    // The page says SWAP; the node script then makes this mark, and the server serves the other files.
    let mark = dir.join("swapped");
    let _ = std::fs::remove_file(&mark);
    let mut said = Vec::new();
    // Both ways: the release's files swapped for the own build, and the own build's tab closed for the release's.
    // invoices shows a report (the pdfReport part), drawn by the set with reports.
    for (way, before, after) in [("to own", release.clone(), own.clone()), ("to release", own.clone(), release.clone())] {
        for (example, window) in [("novel", "ChaptersPart"), ("records", "AlbumBrowse"), ("invoices", "Orders")] {
            let folder = repo.join("examples").join(example);
            let file = repo.join("examples").join(format!("{example}.clarity"));
            let loader: preview::Loader = Arc::new(move || preview::load(&file, &folder));
            let (m, before, after) = (mark.clone(), before.clone(), after.clone());
            let library: preview::Library = Arc::new(move || Ok(if m.exists() { after.clone() } else { before.clone() }));
            let server = Server::start(loader, library, "test").unwrap();
            let _ = std::fs::remove_file(&mark);
            said.extend(run_all(&[], &[(format!("{example} swap {way} {window}"), swap_page(server.address(), window))]));
        }
    }
    for line in &said {
        eprintln!("{line}");
    }
    let failed: Vec<&String> = said.iter().filter(|l| !l.contains("SWAPPED STARTED") || l.contains("ERROR") || l.contains("call_indirect") || l.contains("exception")).collect();
    assert!(failed.is_empty(), "{failed:#?}");
}

/// Opens each page in WebKitGTK (MiniBrowser of libwebkit2gtk-4.1, the
/// engine of the IDE's webview) on an Xvfb display of the test's own, cut
/// off from the desktop (no display, session bus or keyring of the
/// person's), and collects what the page wrote: the page posts it to a
/// collector of the test's own. The display ends with the test.
fn run_webkit(pages: &[(String, String)]) -> Vec<String> {
    use std::io::{BufRead, BufReader, Read, Write};
    eprintln!("WebKitGTK: {} pages", pages.len());
    let browser = Path::new("/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1/MiniBrowser");
    assert!(browser.is_file(), "WebKitGTK's MiniBrowser is not installed");
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("live-canvas-webkit");
    std::fs::create_dir_all(&dir).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let said = Arc::new(std::sync::Mutex::new(String::new()));
    let kept = said.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut length = 0usize;
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) > 2 {
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = v.trim().parse().unwrap_or(0);
                }
                line.clear();
            }
            let mut body = vec![0u8; length];
            let _ = reader.read_exact(&mut body);
            *kept.lock().unwrap() = String::from_utf8_lossy(&body).to_string();
            let mut stream = stream;
            let _ = stream.write_all(b"HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n");
        }
    });
    // One display of its own for every page, ended when done: never the person's, never left behind.
    let display = (99..200).find(|n| !Path::new(&format!("/tmp/.X11-unix/X{n}")).exists()).expect("a free display");
    let mut xvfb = Command::new("Xvfb").arg(format!(":{display}")).args(["-screen", "0", "1400x1000x24", "-nolisten", "tcp"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().expect("Xvfb starts");
    std::thread::sleep(std::time::Duration::from_millis(800));
    let mut out = Vec::new();
    for (name, html) in pages {
        let report = format!("<script>setInterval(() => fetch('http://127.0.0.1:{port}/result', {{ method: 'POST', body: document.getElementById('out').textContent }}).catch(() => {{}}), 400);</script></body>");
        let file = dir.join(format!("{}.html", name.replace(' ', "-")));
        std::fs::write(&file, html.replace("</body>", &report)).unwrap();
        *said.lock().unwrap() = String::new();
        let mut command = Command::new(browser);
        command.arg(format!("file://{}", file.display()));
        for name in ["DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "XDG_SESSION_ID", "XDG_SESSION_TYPE", "XDG_SESSION_DESKTOP", "XDG_CURRENT_DESKTOP", "DESKTOP_SESSION", "GNOME_KEYRING_CONTROL", "SSH_AUTH_SOCK"] {
            command.env_remove(name);
        }
        let err_file = dir.join(format!("{}.err", name.replace(' ', "-")));
        command.env("DISPLAY", format!(":{display}")).env("WEBKIT_DISABLE_COMPOSITING_MODE", "1").stdout(std::process::Stdio::null()).stderr(std::fs::File::create(&err_file).unwrap());
        let mut child = command.spawn().expect("MiniBrowser starts");
        let end = std::time::Instant::now() + std::time::Duration::from_secs(90);
        let text = loop {
            let now = said.lock().unwrap().clone();
            let done = ["READ", "FAILED", "ERROR", "REJECTED"].iter().any(|w| now.contains(w)) && (!name.contains("swap") || now.contains("READ") || now.contains("FAILED") || now.contains("ERROR"));
            if done {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                break said.lock().unwrap().clone();
            }
            if std::time::Instant::now() > end {
                break format!("TIMEOUT {now}");
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        };
        let _ = child.kill();
        let _ = child.wait();
        let err = std::fs::read_to_string(&err_file).unwrap_or_default();
        let errors: Vec<&str> = err.lines().filter(|l| l.contains("call_indirect") || l.contains("CONSOLE") || l.contains("rror")).take(5).collect();
        eprintln!("done {name}");
        out.push(format!("{name}: {}{}", text.trim().replace('\n', " | "), if errors.is_empty() { String::new() } else { format!(" || {}", errors.join(" | ")) }));
    }
    let _ = xvfb.kill();
    let _ = xvfb.wait();
    out
}

#[test]
#[ignore = "needs the browser files, WebKitGTK's MiniBrowser and Xvfb"]
fn the_canvas_starts_in_the_ides_own_engine() {
    let repo = repo();
    let library = std::env::var_os("CLARITY_UI_BROWSER").map(PathBuf::from).unwrap_or_else(|| repo.join("target/clarity-ui-browser"));
    let mut servers = Vec::new();
    let mut pages = Vec::new();
    // invoices shows a report (the pdfReport part): the server draws it with the set of browser files with reports.
    for (example, windows) in [("novel", ["ChaptersPart", "Bookcase"]), ("records", ["AlbumBrowse", "ArtistBrowse"]), ("invoices", ["Orders", "OrdersView"])] {
        let dir = repo.join("examples").join(example);
        let file = repo.join("examples").join(format!("{example}.clarity"));
        let loader: preview::Loader = Arc::new(move || preview::load(&file, &dir));
        let lib = library.clone();
        let server = Server::start(loader, Arc::new(move || Ok(lib.clone())), "test").unwrap();
        for (design, width, height) in [(true, 1000, 700), (true, 0, 0), (false, 1000, 700)] {
            pages.push((format!("{example} {} design={design} {width}x{height}", windows[0]), page(server.address(), windows[0], design, width, height)));
        }
        pages.push((format!("{example} session {} then {}", windows[0], windows[1]), session_page(server.address(), windows[0], windows[1])));
        servers.push(server);
    }
    let said = run_webkit(&pages);
    for line in &said {
        eprintln!("{line}");
    }
    let failed: Vec<&String> = said.iter().filter(|l| !l.contains("READ") || l.contains("ERROR") || l.contains("call_indirect") || l.contains("FAILED") || l.contains("TIMEOUT")).collect();
    assert!(failed.is_empty(), "{failed:#?}");
}
