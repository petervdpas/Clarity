//! The running app and `<app> mcp`: the app listens on loopback with a
//! token in its running.json; a connection that gives the token is served,
//! one that does not is closed; a second copy of the app does not listen;
//! the file goes when the app stops listening.

use clarity_app_mcp::relay;
use clarity_app_mcp::{Lines, discovery};

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-relay-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Answers each line with the line, said back.
fn echo(lines: &mut dyn Lines, _close: relay::Closer) {
    while let Some(line) = lines.read() {
        lines.write(&format!("heard {line}"));
    }
}

#[test]
fn a_connection_with_the_token_is_served() {
    let dir = folder("served");
    let file = discovery::file_in(&dir);
    let listening = relay::listen("shop", &file, echo).unwrap();
    assert_eq!(discovery::read(&file).unwrap().address, listening.address().to_string());
    let mut lines = relay::connect(&file).expect("the app answers");
    lines.write("ping");
    assert_eq!(lines.read().as_deref(), Some("heard ping"));
    drop(listening);
    assert!(!file.exists(), "the file goes with the app");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_connection_without_the_token_is_closed() {
    let dir = folder("token");
    let file = discovery::file_in(&dir);
    let listening = relay::listen("shop", &file, echo).unwrap();
    let mut wrong = discovery::read(&file).unwrap();
    wrong.token = "0".repeat(48);
    discovery::write(&file, &wrong).unwrap();
    let mut lines = relay::connect(&file).expect("something answers");
    lines.write("ping");
    assert_eq!(lines.read(), None, "closed, nothing said");
    drop(listening);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_second_copy_of_the_app_does_not_listen() {
    let dir = folder("twice");
    let file = discovery::file_in(&dir);
    let first = relay::listen("shop", &file, echo).unwrap();
    let second = relay::listen("shop", &file, echo);
    assert!(second.is_err_and(|e| e.contains("running already")));
    assert!(relay::connect(&file).is_some(), "the first still answers");
    drop(first);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn no_app_means_no_connection() {
    let dir = folder("none");
    let file = discovery::file_in(&dir);
    assert!(relay::connect(&file).is_none());
    // A file left by an app that ended without removing it.
    let left = discovery::Running { address: "127.0.0.1:9".into(), ..discovery::Running::new("shop", "x").unwrap() };
    discovery::write(&file, &left).unwrap();
    assert!(relay::connect(&file).is_none(), "nothing answers there");
    assert!(relay::listen("shop", &file, echo).is_ok(), "a new copy listens over the left file");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_app_closes_a_connection_and_its_session_ends() {
    let dir = folder("closed");
    let file = discovery::file_in(&dir);
    let closers: std::sync::Arc<std::sync::Mutex<Vec<relay::Closer>>> = Default::default();
    let kept = closers.clone();
    let listening = relay::listen("shop", &file, move |lines, close| {
        kept.lock().unwrap().push(close);
        echo(lines, std::sync::Arc::new(|| {}));
    })
    .unwrap();
    let mut lines = relay::connect(&file).unwrap();
    lines.write("ping");
    assert_eq!(lines.read().as_deref(), Some("heard ping"));
    (closers.lock().unwrap()[0])();
    assert_eq!(lines.read(), None, "the app closed it");
    drop(listening);
    let _ = std::fs::remove_dir_all(dir);
}

/// An app serves at most so many agents at once (the project file's `agents
/// at most`): one more is answered with the refusal on every request, and
/// is served once one of the others left.
#[test]
fn an_app_serves_at_most_so_many_agents_at_once() {
    let dir = folder("most");
    let file = discovery::file_in(&dir);
    let turned_away = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let told = turned_away.clone();
    let listening = relay::listen_limited("shop", &file, 1, "Shop serves 1 agent at once already.", move |agent| told.lock().unwrap().push(agent), echo).unwrap();
    let mut first = relay::connect(&file).expect("the app answers");
    first.write("ping");
    assert_eq!(first.read().as_deref(), Some("heard ping"));
    let mut second = relay::connect(&file).expect("the app answers");
    second.write(r#"{"jsonrpc":"2.0","id":7,"method":"initialize","params":{"clientInfo":{"name":"Codex"}}}"#);
    let refused: serde_json::Value = serde_json::from_str(&second.read().unwrap()).unwrap();
    assert_eq!(refused, serde_json::json!({ "jsonrpc": "2.0", "id": 7, "error": { "code": -32000, "message": "Shop serves 1 agent at once already." } }));
    // The open app hears who was turned away, once, to say so in its window.
    second.write(r#"{"jsonrpc":"2.0","id":8,"method":"tools/list"}"#);
    let _ = second.read();
    assert_eq!(*turned_away.lock().unwrap(), vec![Some("Codex".to_string())]);
    drop(second);
    drop(first);
    // The first left: a new one is served.
    let mut third = None;
    for _ in 0..50 {
        let mut lines = relay::connect(&file).expect("the app answers");
        lines.write("ping");
        if lines.read().as_deref() == Some("heard ping") {
            third = Some(lines);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(third.is_some(), "a place came free");
    drop(listening);
    let _ = std::fs::remove_dir_all(dir);
}
