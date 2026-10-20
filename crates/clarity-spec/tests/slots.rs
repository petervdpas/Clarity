//! Text with slots (I1b): `"Saved {customer.name}"` in what the screen
//! shows, checked like any expression.

use clarity_spec::code::{ExprKind, StmtKind, TextPart, template};
use clarity_spec::{Project, Severity, load_sources};

fn window(code: &str) -> Project {
    let text = format!(
        "module shop

entity Customer
    name: text(80)
    since: date

page Card: window
    data customer: Customer
    button \"Save\" name save at 1,1 span 2 on click: saved
    textbox name nameInput at 1,2 span 4 bind customer.name

code Card
    handler saved
{}",
        code.lines().map(|l| format!("        {l}\n")).collect::<String>()
    );
    load_sources([("shop.clarity", text.as_str())])
}

fn messages(project: &Project, severity: Severity) -> Vec<String> {
    project.diagnostics.iter().filter(|d| d.severity == severity).map(|d| d.message.clone()).collect()
}

#[test]
fn shown_text_has_slots() {
    let project = window("message \"Saved {customer.name}, a customer since {customer.since}\"\nsave.caption = \"Save {customer.name}\"\nnameInput.error = \"{customer.name} is taken\"");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let StmtKind::Message { text, .. } = &project.files[0].code[0].handlers[0].body[0].kind else { panic!() };
    let ExprKind::Text { parts } = &text.kind else { panic!("{text:?}") };
    assert_eq!(parts.len(), 4);
    assert!(matches!(&parts[0], TextPart::Plain { text } if text == "Saved "));
    assert_eq!(template(parts), "Saved {customer.name}, a customer since {customer.since}");
}

#[test]
fn doubled_braces_are_braces() {
    let project = window("message \"{{not a slot}} but {customer.name}\"");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let StmtKind::Message { text, .. } = &project.files[0].code[0].handlers[0].body[0].kind else { panic!() };
    let ExprKind::Text { parts } = &text.kind else { panic!() };
    assert!(matches!(&parts[0], TextPart::Plain { text } if text == "{not a slot} but "));
    // Written back, the braces are doubled again.
    assert_eq!(template(parts), "{{not a slot}} but {customer.name}");
    // Without a slot, it is plain text with the braces read.
    let plain = window("message \"{{x}}\"");
    let StmtKind::Message { text, .. } = &plain.files[0].code[0].handlers[0].body[0].kind else { panic!() };
    assert!(matches!(&text.kind, ExprKind::Str { value } if value == "{x}"));
}

#[test]
fn a_slot_is_a_path_to_a_value() {
    let errors = |code: &str| messages(&window(code), Severity::Error);
    assert_eq!(errors("message \"Hello {customer}\""), [
        "a slot shows a value, and {customer} is Customer; name one of its fields, like {customer.name}"
    ]);
    assert_eq!(errors("message \"{customer.age}\""), ["Customer has no field 'age'"]);
    assert_eq!(errors("message \"{customer.name + 1}\""), [
        "a slot names a value or a field, like {customer.name}, not {customer.name + 1}"
    ]);
    assert_eq!(errors("message \"{customer.name\""), ["'{' starts a slot; close it with '}', or write '{{' for a brace"]);
    assert_eq!(errors("message \"done}\""), ["'}' ends a slot; write '}}' for a brace"]);
}

#[test]
fn text_with_slots_is_only_for_showing() {
    let errors = messages(&window("let t = \"Saved {customer.name}\"\nmessage t"), Severity::Error);
    assert_eq!(errors, ["text with slots is text to show; use it in 'message', 'refuse', or a part's caption or error"]);
}

#[test]
fn joined_shown_text_suggests_slots() {
    let joined = window("message \"Saved \" + customer.name");
    assert_eq!(messages(&joined, Severity::Warning), [
        "write this as one text with slots, like \"Saved {customer.name}\", so it can be translated and its values shown in the person's format"
    ]);
    // Text that is not shown may be joined.
    let kept = window("customer.name = \"Ada \" + customer.name");
    assert!(kept.diagnostics.is_empty(), "{:#?}", kept.diagnostics);
}
