//! HTTP calls from code (DESIGN 5.23, HC1): `http get`, `post`, `put` and
//! `delete` with headers, giving a reply with `status`, `ok` and `text`; in
//! handlers, commands and actions that declare `effects net`, never in a
//! function, an engine or a data event; a key written in the code is a
//! warning.

use clarity_spec::code::{ExprKind, HttpMethod, StmtKind};
use clarity_spec::parser::parse;
use clarity_spec::{Project, Severity, load_sources};

const API: &str = "module api

settings
    apiToken: text

entity Order
    summary: text(200)

action send_order(order: Order, token: text)
    effects net
    let reply = http post \"https://api.example.com/orders\" with order.summary header \"Authorization\": \"Bearer {token}\" header \"Accept\": \"text/plain\"
    if not reply.ok
        refuse \"not sent: {reply.status}\"

page Status: window
    data shown: text
    data order: Order
    button \"Refresh\" name refresh at 1,1 span 2 on click: refreshClicked

code Status
    handler refreshClicked
        let reply = http get \"https://status.example.com/api/now\"
        shown = reply.text
        let gone = http delete \"https://api.example.com/orders/1\"
        let put = http put \"https://api.example.com/orders/1\" with \"done\"
        message \"{gone.status} {put.ok}\"
        send_order(order, settings.apiToken)
";

fn load(source: &str) -> Project {
    load_sources([("api.clarity", source)])
}

fn said(project: &Project, severity: Severity) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == severity).map(|d| d.message.clone()).collect()
}

fn errors(source: &str) -> Vec<String> {
    said(&load(source), Severity::Error)
}

fn with(old: &str, new: &str) -> String {
    assert!(API.contains(old), "{old:?}");
    API.replacen(old, new, 1)
}

#[test]
fn an_api_client_checks() {
    let project = load(API);
    assert_eq!(said(&project, Severity::Error), Vec::<String>::new());
    assert_eq!(said(&project, Severity::Warning).iter().filter(|w| w.contains("key")).count(), 0, "{:?}", said(&project, Severity::Warning));
}

/// A stack brings its HTTP client along only for a project whose code calls out.
#[test]
fn a_project_knows_whether_it_calls_out() {
    assert!(load(API).calls_out());
    assert!(!load("module plain\n\nfunction twice(n: integer) -> integer\n    return n * 2\n").calls_out());
}

#[test]
fn parses_the_call_its_body_and_headers() {
    let (file, diagnostics) = parse(API, "api.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let StmtKind::Let { value, .. } = &file.functions[0].body[0].kind else { panic!() };
    let ExprKind::Http { method, url, body, headers, .. } = &value.kind else { panic!("{value:#?}") };
    assert_eq!(*method, HttpMethod::Post);
    assert!(matches!(&url.kind, ExprKind::Str { value } if value == "https://api.example.com/orders"));
    assert!(matches!(body.as_deref().map(|b| &b.kind), Some(ExprKind::Member { .. })));
    let names: Vec<&str> = headers.iter().map(|h| h.name.name.as_str()).collect();
    assert_eq!(names, ["Authorization", "Accept"]);
    assert!(matches!(&headers[0].value.kind, ExprKind::Text { .. }));
}

#[test]
fn an_action_that_calls_out_declares_net() {
    let errs = errors(&with("    effects net\n", ""));
    assert_eq!(errs, ["action 'send_order' calls out, which needs 'effects net'"], "{errs:?}");
}

#[test]
fn a_function_cannot_call_out() {
    let source = format!("{API}\nfunction ping() -> integer\n    let reply = http get \"https://example.com\"\n    return reply.status\n");
    let errs = errors(&source);
    assert_eq!(errs, ["function 'ping' cannot call out; an action can, with 'effects net'"], "{errs:?}");
}

#[test]
fn a_data_event_cannot_call_out() {
    let source = format!("{API}\ncode Order\n    after save\n        let reply = http post \"https://example.com/hook\" with order.summary\n");
    let errs = errors(&source);
    assert!(errs.iter().any(|e| e.contains("a data event cannot call out")), "{errs:?}");
}

#[test]
fn post_put_and_patch_send_a_body_get_and_delete_do_not() {
    let errs = errors(&with("http put \"https://api.example.com/orders/1\" with \"done\"", "http put \"https://api.example.com/orders/1\""));
    assert!(errs.iter().any(|e| e.contains("http put sends a body: write 'with' and the text")), "{errs:?}");
    // Patch changes part of a record: it sends what changes.
    let errs = errors(&with("http put \"https://api.example.com/orders/1\" with \"done\"", "http patch \"https://api.example.com/orders/1\" with \"done\""));
    assert_eq!(errs, Vec::<String>::new());
    let errs = errors(&with("http put \"https://api.example.com/orders/1\" with \"done\"", "http patch \"https://api.example.com/orders/1\""));
    assert!(errs.iter().any(|e| e.contains("http patch sends a body: write 'with' and the text")), "{errs:?}");
    let errs = errors(&with("http get \"https://status.example.com/api/now\"", "http get \"https://status.example.com/api/now\" with \"x\""));
    assert!(errs.iter().any(|e| e.contains("http get sends no body; post, put and patch do")), "{errs:?}");
    let errs = errors(&with("http delete \"https://api.example.com/orders/1\"", "http fetch \"https://api.example.com/orders/1\""));
    assert!(errs.iter().any(|e| e.contains("get, post, put, patch or delete")), "{errs:?}");
}

#[test]
fn the_url_body_and_headers_are_text() {
    let errs = errors(&with("http get \"https://status.example.com/api/now\"", "http get 42"));
    assert!(errs.iter().any(|e| e.contains("the url of an http call is text, not integer")), "{errs:?}");
    let errs = errors(&with("with \"done\"", "with 7"));
    assert!(errs.iter().any(|e| e.contains("the body of an http call is text, not integer")), "{errs:?}");
    let errs = errors(&with("header \"Accept\": \"text/plain\"", "header \"Accept\": 3"));
    assert!(errs.iter().any(|e| e.contains("the header 'Accept' is text, not integer")), "{errs:?}");
}

#[test]
fn a_reply_has_status_ok_and_text() {
    let errs = errors(&with("shown = reply.text", "shown = reply.body"));
    assert_eq!(errs, ["an http reply has status, ok and text, not 'body'"], "{errs:?}");
    let errs = errors(&with("shown = reply.text", "reply.status = 3"));
    assert!(errs.iter().any(|e| e.contains("a reply is read, not set")), "{errs:?}");
    // A reply is what it says, not a value the screen shows as it is.
    let errs = errors(&with("shown = reply.text", "shown = reply"));
    assert!(!errs.is_empty(), "a reply is no text");
}

#[test]
fn a_key_in_the_code_is_a_warning() {
    let project = load(&with("\"Bearer {token}\"", "\"Bearer abc123\""));
    let warnings = said(&project, Severity::Warning);
    assert!(
        warnings.iter().any(|w| w == "the header 'Authorization' holds a key written in the code; keep it in a setting and put it in with a slot"),
        "{warnings:?}"
    );
    let project = load(&with("header \"Accept\": \"text/plain\"", "header \"X-Api-Key\": \"abc123\""));
    assert!(said(&project, Severity::Warning).iter().any(|w| w.contains("'X-Api-Key' holds a key")), "{:?}", said(&project, Severity::Warning));
}
