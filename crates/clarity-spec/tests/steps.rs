//! Wizards (DESIGN 5.15): the `steps` part, a container like `tabs` whose
//! `step` blocks show one at a time, with Back, Next and Finish of
//! Clarity's own; `on next` runs before moving on and `on finish` on the
//! last step; code reads and sets `wizard.step`, from 1.

use clarity_spec::edit::{Edit, apply};
use clarity_spec::keyed::load_keyed;
use clarity_spec::parser::parse;
use clarity_spec::widgets::{self, ContainerKind, Event, WidgetKind};
use clarity_spec::{Project, Severity};

const SHOP: &str = "module shop

entity Customer
    name: text(80), required

entity Order
    customer: ref Customer, required
    note: text

page NewOrder: dialog gives Order
    data order: Order
    steps name wizard at 1,1 span 12 rows 10 on next: checkStep on finish: done
        step \"Customer\"
            dropdown name customerPick at 1,1 span 6 bind order.customer
        step \"Note\"
            textbox name noteBox at 1,1 span 12 bind order.note
";

const CODE: &str = "
code NewOrder
    handler checkStep
        if wizard.step == 1 and order.customer == none
            refuse \"Pick a customer\"

    handler done
        save order
        close with order
";

fn project(page: &str, code: &str) -> Project {
    let text = format!("{page}{code}");
    load_keyed([("shop.clarity", text.as_str())], &[], "en")
}

fn errors(page: &str, code: &str) -> Vec<String> {
    project(page, code).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn steps_is_a_container_of_step_blocks_with_next_and_finish() {
    let (file, diagnostics) = parse(SHOP, "shop.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let wizard = &file.pages[0].widgets[0];
    assert_eq!(wizard.props.kind, WidgetKind::Steps);
    assert_eq!(widgets::container(WidgetKind::Steps), Some(ContainerKind::Steps));
    let titles: Vec<Option<&str>> = wizard.pages.iter().map(|p| p.title.as_deref()).collect();
    assert_eq!(titles, vec![Some("Customer"), Some("Note")]);
    assert_eq!(wizard.pages[0].children.len(), 1);
    assert!(widgets::has_event(WidgetKind::Steps, Event::Next) && widgets::has_event(WidgetKind::Steps, Event::Finish));
    assert!(!widgets::has_event(WidgetKind::Tabs, Event::Next));
    assert_eq!(widgets::widget_line(&wizard.props), "steps name wizard at 1,1 span 12 rows 10 on next: checkStep on finish: done");
}

#[test]
fn a_wizard_reads_and_sets_its_step_and_its_titles_are_keys() {
    let p = project(SHOP, CODE);
    let errors: Vec<&str> = p.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let en = &p.translations["en"];
    assert!(en.values().any(|v| v == "Customer") && en.values().any(|v| v == "Note"), "the step titles are keys: {en:?}");
    for (key, word) in [("back", "Back"), ("next", "Next"), ("finish", "Finish")] {
        assert_eq!(en.get(key).map(String::as_str), Some(word), "Clarity's word {key} for the buttons");
    }
    let skipping = errors_of("    handler checkStep\n        wizard.step = 2\n    handler done\n        close with order\n");
    assert!(skipping.is_empty(), "{skipping:#?}");
    let wrong = errors_of("    handler checkStep\n        wizard.step = \"two\"\n    handler done\n        close with order\n");
    assert!(wrong.iter().any(|m| m.contains("cannot set integer to text")), "{wrong:#?}");
}

fn errors_of(handlers: &str) -> Vec<String> {
    errors(SHOP, &format!("\ncode NewOrder\n{handlers}"))
}

#[test]
fn a_wizard_has_at_least_two_steps_and_steps_are_its_own() {
    let one = SHOP.replace("        step \"Note\"\n            textbox name noteBox at 1,1 span 12 bind order.note\n", "");
    assert!(errors(&one, CODE).iter().any(|m| m.contains("steps need at least two 'step'")), "{:#?}", errors(&one, CODE));
    let in_tabs = SHOP.replace("    steps name wizard at 1,1 span 12 rows 10 on next: checkStep on finish: done", "    tabs name wizard at 1,1 span 12 rows 10").replace("step \"", "tab \"");
    let (_, diagnostics) = parse(&in_tabs, "shop.clarity");
    assert!(diagnostics.is_empty(), "tabs keep their tab blocks: {diagnostics:?}");
}

#[test]
fn the_designer_adds_a_wizard_with_two_steps_and_adds_steps_to_it() {
    let at = |needle: &str| SHOP.find(needle).unwrap() as u32;
    let outcome = apply(SHOP, &Edit::AddTab { item: at("steps name wizard"), title: "Confirm".into() }).unwrap();
    assert!(outcome.text.contains("        step \"Confirm\"\n"), "{}", outcome.text);
    let page = SHOP.find("page NewOrder").unwrap() as u32;
    let props = widgets::new_props(WidgetKind::Steps, 1, 12);
    let outcome = apply(SHOP, &Edit::PlaceWidget { container: page, props }).unwrap();
    assert!(outcome.text.contains("    steps "), "{}", outcome.text);
    assert!(outcome.text.contains("        step \"Step 1\"\n        step \"Step 2\"\n"), "{}", outcome.text);
}
