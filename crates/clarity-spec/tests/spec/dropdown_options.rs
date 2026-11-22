//! The dropdown's options of U1c: `typed`, a text any text may be typed
//! into, with the values the field already has as suggestions (the combo
//! box); and `search` with `show field, ...`, a reference picked among
//! many records, filtered as it is typed (the lookup).

use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::widgets;
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

entity Customer
    name: text(80), required
    city: text(60)
    photo: image

entity Order
    number: text(20)
    customer: ref Customer

page OrderCard: window
    data order: Order
    data customer: Customer
    dropdown name customerBox at 1,1 span 6 bind order.customer search show name, city on change: customerPicked
    dropdown name cityBox at 1,2 span 4 bind customer.city typed
    dropdown name plainBox at 7,1 span 4 bind order.customer search
";

const CODE: &str = "
code OrderCard
    handler customerPicked
        cityBox.visible = true
";

fn project(page: &str) -> Project {
    let text = format!("{page}{CODE}");
    load_keyed([("shop.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str) -> Vec<String> {
    project(page).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn the_options_read_and_write_back() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let parts = &file.pages[0].widgets;
    assert!(parts[0].props.search);
    assert_eq!(parts[0].props.show, ["name", "city"]);
    assert_eq!(widgets::widget_line(&parts[0].props), "dropdown name customerBox at 1,1 span 6 bind order.customer search show name, city on change: customerPicked");
    assert!(parts[1].props.typed);
    assert_eq!(widgets::widget_line(&parts[1].props), "dropdown name cityBox at 1,2 span 4 bind customer.city typed");
    assert_eq!(widgets::widget_line(&parts[2].props), "dropdown name plainBox at 7,1 span 4 bind order.customer search");
    let found = errors(SHOP);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_rules_of_the_options() {
    let says = |page: String, wanted: &str| {
        let found = errors(&page);
        assert!(found.iter().any(|m| m.contains(wanted)), "{wanted}: {found:#?}");
    };
    says(SHOP.replace("bind customer.city typed", "bind order.customer typed"), "'typed' takes any text, so it binds a text field");
    says(SHOP.replace("bind order.customer search show name, city", "bind customer.city search show name"), "'search' finds a record, so it binds a reference");
    says(SHOP.replace("search show name, city", "search show name, street"), "Customer has no field 'street' to show");
    says(SHOP.replace("search show name, city", "search show name, photo"), "a search shows text and values, not photo");
    says(SHOP.replace("search show name, city", "show name, city"), "'show' needs 'search'");
    says(SHOP.replace("    dropdown name cityBox at 1,2 span 4 bind customer.city typed\n", "    textbox name cityBox at 1,2 span 4 bind customer.city typed\n"), "'typed' does not apply to a textbox");
    says(SHOP.replace("bind customer.city typed", "bind customer.city typed search"), "a dropdown is 'typed' or 'search', not both");
}
