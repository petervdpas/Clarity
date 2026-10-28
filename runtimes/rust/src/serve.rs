//! The web target (DESIGN 7.3): the application as a web server. It serves
//! the renderer's files to a browser, telling the page where to send its
//! commands, and answers each command with the function the program gives,
//! which calls what its desktop commands call. One request at a time; the
//! database is the server's.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

use serde_json::{Value, json};

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

/// The reply to one HTTP request: a file of `files` (by its path, without
/// the leading slash), the page at `/`, a command's answer at INVOKE_PATH.
pub fn answer(request: &[u8], files: &dyn Fn(&str) -> Option<Vec<u8>>, invoke: &mut dyn FnMut(&str, Value) -> Value) -> Vec<u8> {
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
                    invoke(command, args)
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

/// Runs the application as a web server on `address` until it stops.
pub fn serve(name: &str, address: &str, files: &dyn Fn(&str) -> Option<Vec<u8>>, invoke: &mut dyn FnMut(&str, Value) -> Value) -> std::io::Result<()> {
    let listener = TcpListener::bind(address)?;
    eprintln!("{name}: serving on http://{address}");
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        if let Ok(request) = read_request(&mut stream) {
            let _ = stream.write_all(&answer(&request, files, invoke));
        }
    }
    Ok(())
}
