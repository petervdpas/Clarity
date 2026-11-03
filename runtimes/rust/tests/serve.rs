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
    assert_eq!(serve::address(&told, "127.0.0.1:9090"), "0.0.0.0:9000");
    if std::env::var("CLARITY_ADDRESS").is_err() {
        assert_eq!(serve::address(&["app".to_string()], "127.0.0.1:9090"), "127.0.0.1:9090");
    }
}

/// Requests run side by side (round B of DESIGN 7.9): while a long
/// handler's request is open, another request (how far it is, Stop) is
/// answered, and the handler's request ends once that one let it.
#[test]
fn a_request_is_answered_while_a_long_one_is_open() {
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicBool, Ordering};
    static STOPPED: AtomicBool = AtomicBool::new(false);
    let invoke = |command: &str, _args: Value| -> Value {
        match command {
            "long" => {
                for _ in 0..1000 {
                    if STOPPED.load(Ordering::SeqCst) {
                        return json!({ "value": "stopped" });
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                json!({ "value": "never stopped" })
            }
            "stop_watch" => {
                STOPPED.store(true, Ordering::SeqCst);
                json!({ "value": null })
            }
            _ => json!({ "error": "no" }),
        }
    };
    let address = {
        let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        free.local_addr().unwrap().to_string()
    };
    {
        let address = address.clone();
        std::thread::spawn(move || serve::serve("test", &address, &files, &invoke));
    }
    let ask = |body: &str| -> std::thread::JoinHandle<String> {
        let (address, request) = (address.clone(), post(body));
        std::thread::spawn(move || {
            for _ in 0..100 {
                if let Ok(mut stream) = std::net::TcpStream::connect(&address) {
                    stream.write_all(request.as_bytes()).unwrap();
                    let mut reply = String::new();
                    stream.read_to_string(&mut reply).unwrap();
                    return reply;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            panic!("nothing listens at {address}")
        })
    };
    let long = ask(r#"{"command": "long", "args": {"watch": 3}}"#);
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(ask(r#"{"command": "stop_watch", "args": {"id": 3}}"#).join().unwrap().ends_with(r#"{"value":null}"#));
    assert!(long.join().unwrap().ends_with(r#"{"value":"stopped"}"#));
}

/// Each browser apart (DESIGN 7.9 B2): a request names its page and brings
/// the person's settings as the page keeps them. A handler sees that page's
/// settings and language, a page that sends none keeps what it had, settings
/// a command changed go back to that page alone, and a page unasked for
/// longer than the quiet time is forgotten.
#[test]
fn each_browser_has_its_own_settings_and_language() {
    use clarity_runtime::{i18n, serve::Pages};
    i18n::load(r#"{"project":{"locale":{"language":"en"},"translations":{"en":{"hi":"Hello"},"nl":{"hi":"Hallo"}}}}"#);
    // A window whose settings start from the page's, as the generated kept_settings does.
    let mut invoke = |command: &str, args: Value| -> Value {
        let mut settings = serve::page_settings_over(serde_json::from_value(json!({ "id": 0, "showDone": true, "theme": "" })).unwrap());
        match command {
            "open_window" => json!({ "value": { "state": { "window": "Card", "data": { "settings": settings, "said": i18n::text("hi") } } } }),
            "dispatch_event" => {
                // The handler flips showDone on the window's settings.
                settings = args["state"]["data"]["settings"].as_object().cloned().unwrap_or_default();
                let flipped = !settings["showDone"].as_bool().unwrap_or(true);
                settings.insert("showDone".into(), Value::Bool(flipped));
                json!({ "value": { "state": { "window": "Card", "data": { "settings": settings, "said": i18n::text("hi") } }, "quit": true } })
            }
            _ => json!({ "value": null }),
        }
    };
    let pages = Pages::new(std::time::Duration::from_millis(300));
    let mut ask = |body: Value| -> Value {
        let reply = String::from_utf8(serve::answer_in(&pages, post(&body.to_string()).as_bytes(), &files, &mut invoke)).unwrap();
        serde_json::from_str(reply.split_once("\r\n\r\n").unwrap().1).unwrap()
    };
    // Page a keeps showDone off and speaks Dutch; page b sends nothing.
    assert_eq!(ask(json!({ "command": "set_language", "args": { "language": "nl" }, "browser": "a", "settings": { "showDone": false } })), json!({ "value": null }));
    let a = ask(json!({ "command": "open_window", "args": { "window": "Card" }, "browser": "a", "settings": { "showDone": false } }));
    assert_eq!(a["value"]["state"]["data"]["settings"]["showDone"], json!(false));
    assert_eq!(a["value"]["state"]["data"]["said"], json!("Hallo"));
    assert!(a.get("settings").is_none(), "opening changed no setting: {a}");
    let b = ask(json!({ "command": "open_window", "args": { "window": "Card" }, "browser": "b" }));
    assert_eq!(b["value"]["state"]["data"]["settings"]["showDone"], json!(true));
    assert_eq!(b["value"]["state"]["data"]["said"], json!("Hello"));
    // a's handler changes its settings: they come back to a alone, quit with them.
    let state = a["value"]["state"].clone();
    let flipped = ask(json!({ "command": "dispatch_event", "args": { "state": state, "handler": "flip" }, "browser": "a" }));
    assert_eq!(flipped["settings"], json!({ "showDone": true, "theme": "" }));
    assert_eq!(flipped["value"]["quit"], json!(true));
    assert_eq!(flipped["value"]["state"]["data"]["said"], json!("Hallo"));
    let b = ask(json!({ "command": "open_window", "args": { "window": "Card" }, "browser": "b" }));
    assert!(b.get("settings").is_none() && b["value"]["state"]["data"]["said"] == json!("Hello"), "{b}");
    // a sends no settings: it keeps those it had, the changed ones.
    let a = ask(json!({ "command": "open_window", "args": { "window": "Card" }, "browser": "a" }));
    assert_eq!(a["value"]["state"]["data"]["settings"]["showDone"], json!(true));
    // Unasked for longer than the quiet time, a page is forgotten: its language too.
    std::thread::sleep(std::time::Duration::from_millis(400));
    ask(json!({ "command": "open_window", "args": { "window": "Card" }, "browser": "b" }));
    let a = ask(json!({ "command": "open_window", "args": { "window": "Card" }, "browser": "a" }));
    assert_eq!(a["value"]["state"]["data"]["said"], json!("Hello"));
}
