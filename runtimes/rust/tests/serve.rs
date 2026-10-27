//! The web target (DESIGN 7.3): the application as a web server, which
//! serves the renderer's files to a browser and answers its commands with
//! the same functions its desktop commands call.

use clarity_runtime::serve;
use serde_json::{Value, json};

fn files(path: &str) -> Option<Vec<u8>> {
    match path {
        "index.html" => Some(b"<html><head><title>Shop</title></head><body></body></html>".to_vec()),
        "runtime.js" => Some(b"// the renderer".to_vec()),
        _ => None,
    }
}

fn invoke(command: &str, args: Value) -> Value {
    match command {
        "start" => json!({ "value": { "main": "Card", "args": args } }),
        _ => json!({ "error": format!("there is no command {command}") }),
    }
}

fn answered(request: &str) -> String {
    String::from_utf8(serve::answer(request.as_bytes(), &files, &mut invoke)).unwrap()
}

fn post(body: &str) -> String {
    format!("POST /clarity/invoke HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len())
}

#[test]
fn the_server_serves_the_page_and_answers_its_commands() {
    // The page says where it sends its commands.
    let page = answered("GET / HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(page.starts_with("HTTP/1.1 200 OK\r\n"), "{page}");
    assert!(page.contains("Content-Type: text/html; charset=utf-8\r\n"), "{page}");
    assert!(page.contains("<script>window.clarityServer = \"/clarity/invoke\";</script></head>"), "{page}");
    // The renderer's other files, by their kind.
    let script = answered("GET /runtime.js HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(script.starts_with("HTTP/1.1 200 OK\r\n") && script.contains("Content-Type: text/javascript\r\n") && script.ends_with("\r\n\r\n// the renderer"), "{script}");
    // A command and its arguments.
    let started = answered(&post(r#"{"command": "start", "args": {"a": 1}}"#));
    assert!(started.ends_with(r#"{"value":{"args":{"a":1},"main":"Card"}}"#), "{started}");
    let unknown = answered(&post(r#"{"command": "nonsense", "args": {}}"#));
    assert!(unknown.ends_with(r#"{"error":"there is no command nonsense"}"#), "{unknown}");
    let broken = answered(&post("not json"));
    assert!(broken.contains("\r\n\r\n{\"error\":"), "{broken}");
    // Nothing else is there.
    assert!(answered("GET /secret HTTP/1.1\r\nHost: x\r\n\r\n").starts_with("HTTP/1.1 404 Not Found\r\n"));
    assert!(answered("GET /../Cargo.toml HTTP/1.1\r\nHost: x\r\n\r\n").starts_with("HTTP/1.1 404 Not Found\r\n"));
}

#[test]
fn the_server_listens_where_it_is_told() {
    let told: Vec<String> = ["app", "--address", "0.0.0.0:9000"].iter().map(|s| s.to_string()).collect();
    assert_eq!(serve::address(&told), "0.0.0.0:9000");
    if std::env::var("CLARITY_ADDRESS").is_err() {
        assert_eq!(serve::address(&["app".to_string()]), serve::DEFAULT_ADDRESS);
    }
}
