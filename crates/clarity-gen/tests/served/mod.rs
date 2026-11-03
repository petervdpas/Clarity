//! The web target (DESIGN 7.3) as each stack's build tests check it: the
//! built program, started as a web server, serves the renderer's page,
//! telling it where its commands go, and answers a command.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// One HTTP/1.0 request to `address`; the whole reply, or None when nothing listens.
fn request(address: &str, method: &str, path: &str, body: &str) -> Option<String> {
    let mut stream = TcpStream::connect(address).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok()?;
    write!(stream, "{method} {path} HTTP/1.0\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).ok()?;
    let mut reply = Vec::new();
    stream.read_to_end(&mut reply).ok()?;
    Some(String::from_utf8_lossy(&reply).into_owned())
}

/// A POST of `body` to `path` at `address`: the reply's body, once the server answers.
#[allow(dead_code)]
pub fn post(address: &str, path: &str, body: &str) -> String {
    for _ in 0..100 {
        if let Some(reply) = request(address, "POST", path, body) {
            return reply.split_once("\r\n\r\n").map(|(_, b)| b.to_string()).unwrap_or_default();
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("nothing answers at {address}")
}

/// Starts `program` in `dir` with `args` and the address, checks what it
/// serves (round B of DESIGN 7.9: Clarity.UI's page and browser files,
/// files.json and the wire the library asks), and stops it, whatever the
/// checks found.
#[allow(dead_code)]
pub fn serves(dir: &Path, program: &Path, args: &[&str], address: &str) {
    let mut server = Command::new(program).args(args).args(["--address", address]).current_dir(dir).spawn().unwrap_or_else(|e| panic!("{}: {e}", program.display()));
    let mut page = None;
    for _ in 0..100 {
        page = request(address, "GET", "/", "");
        if page.is_some() || server.try_wait().ok().flatten().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let wasm = request(address, "GET", "/clarity-ui.wasm", "").unwrap_or_default();
    let script = request(address, "GET", "/clarity-ui-page.js", "").unwrap_or_default();
    let files = request(address, "GET", "/clarity/files.json", "").unwrap_or_default();
    let started = request(address, "POST", "/clarity/invoke", r#"{"command": "started", "args": {}}"#).unwrap_or_default();
    let facts = request(address, "POST", "/clarity/invoke", r#"{"command": "facts", "args": {}}"#).unwrap_or_default();
    let unknown = request(address, "POST", "/clarity/invoke", r#"{"command": "nonsense", "args": {}}"#).unwrap_or_default();
    let _ = server.kill();
    let _ = server.wait();
    let page = page.expect("the server answers");
    let ok = |reply: &str| reply.starts_with("HTTP/1.0 200") || reply.starts_with("HTTP/1.1 200");
    let short = |reply: &str| reply[..reply.len().min(600)].to_string();
    assert!(ok(&page) && page.contains(r#"<canvas id="clarity-ui"></canvas>"#) && page.contains(r#"<script src="clarity-ui-page.js"></script>"#), "{}", short(&page));
    assert!(ok(&wasm) && wasm.to_ascii_lowercase().contains("content-type: application/wasm") && wasm.contains("\0asm"), "{}", short(&wasm));
    assert!(ok(&script) && script.to_ascii_lowercase().contains("content-type: text/javascript"), "{}", short(&script));
    let body = |reply: &str| reply.split_once("\r\n\r\n").map(|(_, b)| b.to_string()).unwrap_or_default();
    let files: serde_json::Value = serde_json::from_str(&body(&files)).unwrap_or_else(|e| panic!("files.json: {e}\n{}", short(&files)));
    assert!(files["windows"].as_object().is_some_and(|w| !w.is_empty()) && files["model"].is_object() && files["title"].is_string(), "{files}");
    let started: serde_json::Value = serde_json::from_str(&body(&started)).unwrap_or_else(|e| panic!("started: {e}\n{}", short(&started)));
    assert!(started["value"]["main"].is_string(), "{started}");
    let facts: serde_json::Value = serde_json::from_str(&body(&facts)).unwrap_or_else(|e| panic!("facts: {e}\n{}", short(&facts)));
    assert!(facts["value"].is_object(), "{facts}");
    assert!(unknown.contains(r#"{"error":"there is no command nonsense"}"#), "{unknown}");
}

/// A program served to a browser alone links nothing of the desktop
/// toolkit: on Linux, neither Clarity.UI's library nor WebKitGTK or GTK (as ldd lists them).
#[allow(dead_code)]
pub fn links_no_desktop_toolkit(program: &Path) {
    if let Some(linked) = toolkit(program) {
        assert!(!linked, "{} links the desktop toolkit", program.display());
    }
}

/// A program with a desktop window links its toolkit: the check above can tell.
#[allow(dead_code)]
pub fn links_the_desktop_toolkit(program: &Path) {
    if let Some(linked) = toolkit(program) {
        assert!(linked, "{} does not link the desktop toolkit", program.display());
    }
}

/// Whether `program` links Clarity.UI's library (or a webview's WebKitGTK or
/// GTK), as ldd lists them; none off Linux.
fn toolkit(program: &Path) -> Option<bool> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    // Without cargo's LD_LIBRARY_PATH, which would point ldd at target/debug.
    let out = Command::new("ldd").arg(program).env_remove("LD_LIBRARY_PATH").output().unwrap_or_else(|e| panic!("ldd: {e}"));
    let linked = String::from_utf8_lossy(&out.stdout);
    Some(linked.contains("libclarity_ui") || linked.contains("libwebkit") || linked.contains("libgtk"))
}
