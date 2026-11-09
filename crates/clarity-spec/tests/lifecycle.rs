//! Window and app events (E1): `on open`, `on close` and `on return` on a
//! window, `on start` on the app, and the `reload` statement.

use clarity_spec::code::StmtKind;
use clarity_spec::edit::Edit;
use clarity_spec::model::Handler;
use clarity_spec::parser::parse;
use clarity_spec::widgets::Event;
use clarity_spec::{Severity, load_converted};

const SHOP: &str = "module shop

entity Customer
    name: text(80)

app Shop
    main Customers
    on start: appStarted

page Customers: window
    title \"Customers\"
    data customers: list<Customer>
    on open: windowOpened
    on return: windowReturned
    table name grid at 1,1 span 12 rows 6 bind customers columns name

page Card: window
    data customer: Customer
    on close: windowClosed
    button \"Close\" name closeButton at 1,1 span 2 on click: closeClicked

code Shop
    handler appStarted
        message \"Welcome\"

code Customers
    handler windowOpened
        message \"Opened\"
    handler windowReturned
        reload customers

code Card
    handler windowClosed
        save customer
    handler closeClicked
        reload customer
        close
";

fn problems(source: &str) -> Vec<(Severity, String)> {
    load_converted([("shop.clarity", source)])
        .diagnostics
        .into_iter()
        .map(|d| (d.severity, d.message))
        .collect()
}

/// The errors of `source`. A module with an error in its window or app
/// stays as written (`converted` moves only what reads clean), so the note
/// that its pages belong in design/ is left out: these errors are its own.
fn errors(source: &str) -> Vec<String> {
    problems(source)
        .into_iter()
        .filter(|(s, m)| *s == Severity::Error && !m.contains(clarity_spec::cxaml::PAGE_LEFT_IN_MODULE) && !m.contains(clarity_spec::cxaml::APP_LEFT_IN_MODULE))
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
    assert_eq!(file.apps[0].handlers, [handler(Event::Start, "appStarted")]);
    assert_eq!(
        file.pages[0].handlers,
        [handler(Event::Open, "windowOpened"), handler(Event::Return, "windowReturned")]
    );
    assert_eq!(file.pages[1].handlers, [handler(Event::Close, "windowClosed")]);
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
        errors(&SHOP.replace("    on start: appStarted", "    on open: appStarted")),
        ["the app has the event start, not open"]
    );
    assert_eq!(
        errors(&SHOP.replace("    on close: windowClosed", "    on start: windowClosed")),
        ["a window has the events open, close and return, not start"]
    );
    assert_eq!(
        errors(&SHOP.replace("    on return: windowReturned", "    on open: windowReturned")),
        ["Customers already has 'on open'"]
    );
}

#[test]
fn window_events_count_as_wired() {
    let unwired = SHOP.replace("    on return: windowReturned\n", "");
    let warnings: Vec<String> = problems(&unwired)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Warning)
        .map(|(_, m)| m)
        .collect();
    assert_eq!(warnings, ["handler 'windowReturned' is not wired to any event"]);
    let missing = SHOP.replace("    handler appStarted\n        message \"Welcome\"\n", "");
    let warnings: Vec<String> = problems(&missing)
        .into_iter()
        .filter(|(s, _)| *s == Severity::Warning)
        .map(|(_, m)| m)
        .collect();
    assert_eq!(warnings, ["handler 'appStarted' is wired but not written yet"]);
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

/// The `.cxaml` the page or app `name` of the shop becomes.
fn design(name: &str) -> String {
    let file = format!("design/{name}.cxaml");
    clarity_spec::converted([("shop/shop.clarity", SHOP)]).into_iter().find(|(p, _)| p.ends_with(&file)).map(|(_, t)| t).unwrap_or_else(|| panic!("no {file}"))
}

fn offset(text: &str, needle: &str) -> u32 {
    text.find(needle).unwrap() as u32
}

fn apply(markup: &str, edit: &Edit) -> clarity_spec::edit::EditResult<clarity_spec::edit::EditOutcome> {
    clarity_spec::cxaml::apply(None, markup, edit)
}

#[test]
fn the_designer_wires_and_unwires_window_events() {
    let card = design("Card");
    let page = offset(&card, "<Window");
    let wired = apply(&card, &Edit::SetOwnerEvent { owner: page, event: Event::Return, handler: Some("windowReturned".into()) }).unwrap().text;
    assert!(wired.contains("<Window class=\"shop.Card\" onclose=\"windowClosed\" onreturn=\"windowReturned\">\n"), "{wired}");

    let renamed = apply(&wired, &Edit::SetOwnerEvent { owner: page, event: Event::Close, handler: Some("before_closing".into()) }).unwrap().text;
    assert!(renamed.contains(" onclose=\"before_closing\""), "{renamed}");

    let removed = apply(&renamed, &Edit::SetOwnerEvent { owner: page, event: Event::Close, handler: None }).unwrap().text;
    assert!(!removed.contains("onclose"), "{removed}");

    let shop = design("Shop");
    let app = offset(&shop, "<App");
    let refused = apply(&shop, &Edit::SetOwnerEvent { owner: app, event: Event::Open, handler: Some("x".into()) });
    assert!(refused.unwrap_err().to_string().contains("the app has the event start"));
}
