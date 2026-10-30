//! The clipboard: `copy text` puts text, as it shows, on the system's
//! clipboard once the handler has run, for the person to paste anywhere.

use clarity_spec::code::StmtKind;
use clarity_spec::{Severity, load_sources};

const PAGE: &str = "module notes

page Output: window
    data diagram: text
    data copy: integer
    button \"Copy\" name copyButton at 1,1 span 2 on click: copyClicked

code Output
    handler copyClicked
        copy diagram
        copy = copy + 1
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("notes.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn copy_puts_text_on_the_clipboard() {
    let project = load_sources([("notes.clarity", PAGE)]);
    assert!(errors(PAGE).is_empty(), "{:#?}", errors(PAGE));
    let body = &project.files[0].code[0].handlers[0].body;
    assert!(matches!(&body[0].kind, StmtKind::Copy { .. }), "{:?}", body[0].kind);
    // A name `copy` stays a name: assigning to it is no copy.
    assert!(matches!(&body[1].kind, StmtKind::Assign { .. }), "{:?}", body[1].kind);
}

#[test]
fn only_a_handler_with_a_screen_copies() {
    let in_function = PAGE.replace("code Output\n", "function wrap(t: text) -> text\n    copy t\n    return t\n\ncode Output\n");
    let found = errors(&in_function);
    assert!(found.iter().any(|m| m.contains("copy")), "{found:#?}");
}
