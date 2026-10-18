//! Values code works with (X2): `none`, the empty value of a date, a date
//! and time or a reference field; `today()` and `now()`; and the `checked`
//! state of an unbound checkbox or switch.

use clarity_spec::{Severity, load_sources};

const SHOP: &str = "module shop

entity Customer
    name: text(80)
    credit_limit: money
    since: date

entity Order
    number: text(12)
    customer: ref Customer
    shipped: datetime
    paid: bool

page Orders: window
    data orders: list<Order>
    data order: Order
    data customer: Customer
    button \"Check\" at 1,1 span 2 on click: check_clicked
    checkbox \"Shipped only\" name shipped_only at 3,1 span 3
    switch \"Bound\" name bound at 6,1 span 3 bind order.paid

code Orders
    handler check_clicked
";

fn errors(body: &str) -> Vec<String> {
    let body: String = body.lines().map(|l| format!("        {l}\n")).collect();
    let source = format!("{SHOP}{body}");
    load_sources([("shop.clarity", source.as_str())])
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

#[test]
fn dates_and_references_can_be_none() {
    let body = "if customer.since == none
    customer.since = none
if order.customer != none and order.shipped == none
    order.customer = none
orders = find o in Order where o.customer == none or o.shipped != none order by o.number";
    assert_eq!(errors(body), Vec::<String>::new());
}

#[test]
fn other_values_are_never_none() {
    assert_eq!(errors("if customer.name == none\n    return"), ["text is never none; empty text is \"\""]);
    assert_eq!(errors("customer.credit_limit = none"), ["money is never none; only a date, a date and time or a reference can be"]);
    assert_eq!(errors("let x = none"), ["'none' needs a date, a date and time or a reference to be the empty value of"]);
}

/// `today()` and `now()`: the date and the date and time where the
/// application runs.
#[test]
fn today_and_now_are_a_date_and_a_date_and_time() {
    assert_eq!(errors("customer.since = today()\norder.shipped = now()\nif customer.since < today()\n    return"), Vec::<String>::new());
    assert_eq!(errors("customer.since = now()"), ["cannot set date to datetime"]);
    assert_eq!(errors("let d = today(1)"), ["'today' takes no arguments"]);
}

/// An unbound checkbox or switch holds its own state, as an unbound
/// textbox holds its text.
#[test]
fn an_unbound_checkbox_is_checked_or_not() {
    assert_eq!(errors("if shipped_only.checked\n    shipped_only.checked = false"), Vec::<String>::new());
    assert_eq!(errors("let x = bound.checked"), ["switch 'bound' is bound to order.paid; use order.paid"]);
}

const MUSIC: &str = "module music

entity Track
    title: text(80)
    length: duration, min 0

function album_length(tracks: list<Track>) -> duration
    let total: duration = 0
    for track in tracks
        total = total + track.length
    return total

function long_tracks() -> list<Track>
    return find t in Track where t.length > 600 order by t.length desc

action double(track: Track)
    effects db.write
    track.length = track.length * 2
    save track
";

/// A duration: whole seconds, which add up, scale, compare and accept a
/// number of seconds; shown as 3:45 on every stack.
#[test]
fn a_duration_is_seconds_that_add_up() {
    let project = load_sources([("music.clarity", MUSIC)]);
    let problems: Vec<String> = project.diagnostics.iter().map(|d| d.message.clone()).collect();
    assert!(problems.is_empty(), "{problems:#?}");
    let wrong = format!("{MUSIC}\nfunction named(track: Track) -> duration\n    return track.title\n");
    let project = load_sources([("music.clarity", wrong.as_str())]);
    let problems: Vec<String> = project.diagnostics.iter().map(|d| d.message.clone()).collect();
    assert_eq!(problems, ["named must return duration, not text"]);
}
