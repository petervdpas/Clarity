//! Data events (D1f): `before save`, `after save`, `before delete` and
//! `after delete` in the code block of an entity, and `refuse`.

use clarity_spec::code::{DataEvent, FunctionKind, StmtKind};
use clarity_spec::rename::{RenameKind, rename};
use clarity_spec::{Project, Severity, load_sources};

const SALES: &str = "module sales

enum Status
    Pending
    Approved

entity Order
    number: text(12)
    status: Status
    total: money

entity Log
    text: text

action note(text: text)
    effects db.write
    let entry = find l in Log where l.text == text
    count(entry)

code Order
    before save
        if order.total < 0
            refuse \"An order cannot have a negative total\"
        order.number = \"ORD-\" + order.number

    after save
        note(\"saved \" + order.number)

    before delete
        if order.status != Pending
            refuse \"Only pending orders can be deleted, not {order.number}\"
";

fn errors(project: &Project) -> Vec<String> {
    project
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect()
}

fn with_events(events: &str) -> Project {
    let text = SALES.split("code Order").next().unwrap().to_string() + "code Order\n" + events;
    load_sources([("sales.clarity", text.as_str())])
}

#[test]
fn an_entity_has_data_events() {
    let project = load_sources([("sales.clarity", SALES)]);
    assert!(errors(&project).is_empty(), "{:#?}", project.diagnostics);
    let block = &project.files[0].code[0];
    let events: Vec<DataEvent> = block.events.iter().map(|e| e.event).collect();
    assert_eq!(events, [DataEvent::BeforeSave, DataEvent::AfterSave, DataEvent::BeforeDelete]);
    let StmtKind::If { branches, .. } = &block.events[0].body[0].kind else { panic!() };
    assert!(matches!(branches[0].body[0].kind, StmtKind::Refuse { .. }));

    // An event is an action on its record, named after the entity.
    let action = block.events[0].action("Order");
    assert_eq!(action.kind, FunctionKind::Event);
    assert_eq!(action.name.name, "before_save_order");
    assert_eq!(action.params[0].name.name, "order");
}

#[test]
fn events_are_checked_like_actions() {
    let project = with_events("    before save\n        message \"no\"\n        order.missing = 1\n");
    assert_eq!(
        errors(&project),
        ["'message' belongs in a handler, not in a data event", "Order has no field 'missing'"]
    );

    let project = with_events("    after save\n        save order\n    before delete\n        delete order\n");
    assert_eq!(
        errors(&project),
        [
            "saving order in its own 'after save' would run the event again, without end",
            "deleting order in its own 'before delete' would run the event again, without end",
        ]
    );
    // Deleting the record when it is saved is no loop.
    assert!(errors(&with_events("    after save\n        delete order\n")).is_empty());

    let project = with_events("    before save\n        return\n    before save\n        return\n    handler clicked\n        return\n");
    assert_eq!(
        errors(&project),
        [
            "the code of entity Order holds data events (before save, after save, before delete, after delete), not handlers",
            "'before save' is written twice",
        ]
    );
    let project = with_events("    before load\n        return\n");
    assert_eq!(
        errors(&project),
        ["unknown event 'before load'; the events are before save, after save, before delete and after delete"]
    );
}

#[test]
fn refuse_needs_a_value() {
    let project = with_events("    before save\n        refuse order\n");
    assert_eq!(errors(&project), ["'refuse' needs text or a value, not Order"]);
}

#[test]
fn events_belong_to_entities() {
    let text = SALES.to_string() + "\npage Orders: window\n    title \"Orders\"\n\ncode Orders\n    before save\n        return\n\ncode Nothing\n    handler x\n        return\n";
    let project = load_sources([("sales.clarity", text.as_str())]);
    assert_eq!(
        errors(&project),
        [
            "'before save' belongs in the code of an entity; Orders is a window",
            "there is no window, app or entity 'Nothing' in this file",
        ]
    );
}

#[test]
fn renaming_an_entity_renames_its_events() {
    let project = load_sources([("sales.clarity", SALES)]);
    let changed = rename(&project, |_| Some(SALES.to_string()), RenameKind::Entity, "Order", "Purchase").unwrap();
    let text = &changed[0].text;
    assert!(text.contains("code Purchase\n    before save\n        if purchase.total < 0\n"), "{text}");
    assert!(text.contains("purchase.number = \"ORD-\" + purchase.number"), "{text}");
    assert!(!text.contains("order."), "{text}");
    let renamed = load_sources([("sales.clarity", text.as_str())]);
    assert!(errors(&renamed).is_empty(), "{:#?}", renamed.diagnostics);
}
