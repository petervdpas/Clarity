//! The accordion and the scroll box of U1b: an accordion's `section` blocks
//! fold so that one is open at a time, which code reads and sets as
//! `part.open` (from 1, 0 for none); a scroll box's parts may stand below
//! its rows and scroll into view.

use clarity_spec::edit::{Edit, apply};
use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, ContainerKind, Event, WidgetKind};
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

entity Customer
    name: text(80), required
    street: text
    city: text
    shipping: text
    returns: text

page Help: window
    data customer: Customer
    accordion name faq at 1,1 span 6 rows 8 on change: sectionOpened
        section \"Shipping\"
            text name shippingText at 1,1 span 6 rows 2 bind customer.shipping
        section \"Returns\"
            text name returnsText at 1,1 span 6 rows 2 bind customer.returns
    scroll name details at 7,1 span 6 rows 2
        textbox name streetBox at 1,1 span 6 bind customer.street
        textbox name cityBox at 1,5 span 6 bind customer.city
";

const CODE: &str = "
code Help
    handler sectionOpened
        if faq.open == 0
            faq.open = 2
";

fn project(page: &str, code: &str) -> Project {
    let text = format!("{page}{code}");
    load_keyed([("shop.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str, code: &str) -> Vec<String> {
    project(page, code).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn an_accordion_has_sections_and_a_scroll_box_holds_parts() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let faq = &file.pages[0].widgets[0];
    assert_eq!(faq.props.kind, WidgetKind::Accordion);
    assert_eq!(widgets::container(WidgetKind::Accordion), Some(ContainerKind::Accordion));
    assert!(widgets::has_event(WidgetKind::Accordion, Event::Change));
    let titles: Vec<Option<&str>> = faq.pages.iter().map(|p| p.title.as_deref()).collect();
    assert_eq!(titles, vec![Some("Shipping"), Some("Returns")]);
    let details = &file.pages[0].widgets[1];
    assert_eq!(details.props.kind, WidgetKind::Scroll);
    assert_eq!(widgets::container(WidgetKind::Scroll), Some(ContainerKind::Plain));
    assert_eq!(details.children.len(), 2);
    assert_eq!(widgets::widget_line(&faq.props), "accordion name faq at 1,1 span 6 rows 8 on change: sectionOpened");
    assert_eq!(widgets::widget_line(&details.props), "scroll name details at 7,1 span 6 rows 2");
}

#[test]
fn code_reads_and_sets_the_open_section_and_the_titles_are_keys() {
    let p = project(SHOP, CODE);
    let found: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(found.is_empty(), "a part below the scroll box's rows is fine: {found:#?}");
    let en = &p.translations["en"];
    assert!(en.values().any(|v| v == "Shipping") && en.values().any(|v| v == "Returns"), "{en:?}");
    let wrong = errors(SHOP, &CODE.replace("faq.open = 2", "faq.open = \"two\""));
    assert!(wrong.iter().any(|m| m.contains("cannot set integer to text")), "{wrong:#?}");
    let elsewhere = errors(SHOP, &CODE.replace("faq.open = 2", "details.open = 2"));
    assert!(!elsewhere.is_empty(), "only an accordion has an open section");
}

#[test]
fn an_accordion_needs_a_section_and_the_designer_adds_them() {
    let none = SHOP
        .replace("        section \"Shipping\"\n            text name shippingText at 1,1 span 6 rows 2 bind customer.shipping\n", "")
        .replace("        section \"Returns\"\n            text name returnsText at 1,1 span 6 rows 2 bind customer.returns\n", "");
    assert!(errors(&none, CODE).iter().any(|m| m.contains("an accordion needs at least one 'section'")), "{:#?}", errors(&none, CODE));
    let at = SHOP.find("accordion name faq").unwrap() as u32;
    let outcome = apply(SHOP, &Edit::AddTab { item: at, title: "Payment".into() }).unwrap();
    assert!(outcome.text.contains("        section \"Payment\"\n"), "{}", outcome.text);
    let page = SHOP.find("page Help").unwrap() as u32;
    let outcome = apply(SHOP, &Edit::PlaceWidget { container: page, props: widgets::new_props(WidgetKind::Accordion, 1, 12) }).unwrap();
    assert!(outcome.text.contains("        section \"Section 1\"\n        section \"Section 2\"\n"), "{}", outcome.text);
}
