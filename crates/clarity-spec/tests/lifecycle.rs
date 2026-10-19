//! Window and app events (E1): `on open`, `on close` and `on return` on a
//! window, `on start` on the app, and the `reload` statement.

use clarity_spec::code::StmtKind;
use clarity_spec::edit::{Edit, apply};
use clarity_spec::model::Handler;
use clarity_spec::parser::parse;
use clarity_spec::widgets::Event;
use clarity_spec::{Severity, load_sources};

const SHOP: &str = "module shop

entity Customer
    name: text(80)

app Shop
    main Customers
    on start: app_started

page Customers: window
    title \"Customers\"
    data customers: list<Customer>
    on open: window_opened
    on return: window_returned
    table name grid at 1,1 span 12 rows 6 bind customers columns name

page Card: window
    data customer: Customer
    on close: window_closed
    button \"Close\" name close_button at 1,1 span 2 on click: close_clicked

code Shop
    handler app_started
        message \"Welcome\"

code Customers
    handler window_opened
        message \"Opened\"
    handler window_returned
        reload customers

code Card
    handler window_closed
        save customer
    handler close_clicked
        reload customer
        close
";

fn problems(source: &str) -> Vec<(Severity, String)> {
    load_sources([("shop.clarity", source)])
        .diagnostics
        .into_iter()
        .map(|d| (d.severity, d.message))
        .collect()
}

fn errors(source: &str) -> Vec<String> {
    problems(source)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Error)
        .map(|(_, m)| m)
        .collect()
}

#[test]
fn parses_window_and_app_events() {
    let (file, diags) = parse(SHOP, "shop.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let handler = |event, handler: &str| Handler {
        event,
        handler: handler.into(),
        args: Vec::new(),
    };
    assert_eq!(file.apps[0].handlers, [handler(Event::Start, "app_started")]);
    assert_eq!(
        file.pages[0].handlers,
        [handler(Event::Open, "window_opened"), handler(Event::Return, "window_returned")]
    );
    assert_eq!(file.pages[1].handlers, [handler(Event::Close, "window_closed")]);
    let StmtKind::Reload { data } = &file.code[1].handlers[1].body[0].kind else {
        panic!("expected reload")
    };
    assert_eq!(data.name, "customers");
}

#[test]
fn the_example_checks_clean() {
    assert_eq!(problems(SHOP), vec![]);
}

#[test]
fn each_owner_has_its_own_events() {
    assert_eq!(
        errors(&SHOP.replace("    on start: app_started", "    on open: app_started")),
        ["the app has the event start, not open"]
    );
    assert_eq!(
        errors(&SHOP.replace("    on close: window_closed", "    on start: window_closed")),
        ["a window has the events open, close and return, not start"]
    );
    assert_eq!(
        errors(&SHOP.replace("    on return: window_returned", "    on open: window_returned")),
        ["Customers already has 'on open'"]
    );
}

#[test]
fn window_events_count_as_wired() {
    let unwired = SHOP.replace("    on return: window_returned\n", "");
    let warnings: Vec<String> = problems(&unwired)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Warning)
        .map(|(_, m)| m)
        .collect();
    assert_eq!(warnings, ["handler 'window_returned' is not wired to any event"]);
    let missing = SHOP.replace("    handler app_started\n        message \"Welcome\"\n", "");
    let warnings: Vec<String> = problems(&missing)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Warning)
        .map(|(_, m)| m)
        .collect();
    assert_eq!(warnings, ["handler 'app_started' is wired but not written yet"]);
}

#[test]
fn reload_takes_the_windows_data() {
    assert_eq!(
        errors(&SHOP.replace("        reload customers", "        reload clients")),
        ["Customers has no data 'clients'"]
    );
    let in_function = format!("{SHOP}\nfunction f() -> integer\n    reload customers\n    return 1\n");
    assert_eq!(errors(&in_function), ["'reload' belongs in a handler, not in a function"]);
}

fn offset(text: &str, needle: &str) -> u32 {
    text.find(needle).unwrap() as u32
}

#[test]
fn the_designer_wires_and_unwires_window_events() {
    let page = offset(SHOP, "page Card");
    let wired = apply(
        SHOP,
        &Edit::SetOwnerEvent {
            owner: page,
            event: Event::Return,
            handler: Some("window_returned".into()),
        },
    )
    .unwrap()
    .text;
    assert!(
        wired.contains("    on close: window_closed\n    on return: window_returned\n    button \"Close\""),
        "{wired}"
    );

    let renamed = apply(
        &wired,
        &Edit::SetOwnerEvent {
            owner: page,
            event: Event::Close,
            handler: Some("before_closing".into()),
        },
    )
    .unwrap()
    .text;
    assert!(renamed.contains("    on close: before_closing\n"), "{renamed}");

    let removed = apply(
        &renamed,
        &Edit::SetOwnerEvent {
            owner: page,
            event: Event::Close,
            handler: None,
        },
    )
    .unwrap()
    .text;
    assert!(!removed.contains("on close"), "{removed}");

    let app = offset(SHOP, "app Shop");
    let refused = apply(
        SHOP,
        &Edit::SetOwnerEvent {
            owner: app,
            event: Event::Open,
            handler: Some("x".into()),
        },
    );
    assert!(refused.unwrap_err().to_string().contains("the app has the event start"));
}
