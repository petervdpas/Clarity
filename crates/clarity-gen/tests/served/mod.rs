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

/// Starts `program` in `dir` with `args` and the address, checks what it
/// serves, and stops it, whatever the checks found.
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
    let started = request(address, "POST", "/clarity/invoke", r#"{"command": "start", "args": {}}"#).unwrap_or_default();
    let unknown = request(address, "POST", "/clarity/invoke", r#"{"command": "nonsense", "args": {}}"#).unwrap_or_default();
    let _ = server.kill();
    let _ = server.wait();
    let page = page.expect("the server answers");
    let ok = |reply: &str| reply.starts_with("HTTP/1.0 200") || reply.starts_with("HTTP/1.1 200");
    assert!(ok(&page) && page.contains(r#"window.clarityServer = "/clarity/invoke";"#), "{}", &page[..page.len().min(600)]);
    assert!(ok(&started) && started.contains(r#"{"value":{"#) && started.contains(r#""main":"#), "{}", &started[..started.len().min(600)]);
    assert!(unknown.contains(r#"{"error":"there is no command nonsense"}"#), "{unknown}");
}
