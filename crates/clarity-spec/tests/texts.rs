//! The text functions of code (conformance/texts.json): typed by the checker,
//! a project's own function of the same name taking its place.

use clarity_spec::{Severity, load_sources};

const WRITER: &str = "module writer

page Desk: window
    data text: text
    data count: integer
    data starts: bool
    data shown: text

code Desk
    handler countClicked
        let all = words(text)
        count = count(all)
        let parts = split(text, \",\")
        shown = join(parts, \"; \")
        shown = trim(lower(upper(shown)))
        shown = replace(shown, \"a\", \"b\")
        starts = startsWith(shown, \"b\") or endsWith(shown, \".\")
        shown = slice(shown, 2, 3)
        count = indexOf(shown, \"b\") + count(words(\"one two\"))
";

fn errors(text: &str) -> Vec<String> {
    load_sources([("writer.clarity", text)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn text_functions_are_typed() {
    assert!(errors(WRITER).is_empty(), "{:#?}", errors(WRITER));
    let e = errors(&WRITER.replace("shown = slice(shown, 2, 3)", "shown = slice(shown, \"2\", 3)"));
    assert!(e.iter().any(|m| m.contains("'slice' takes a text, a whole number, a whole number")), "{e:?}");
    let e = errors(&WRITER.replace("count = indexOf(shown, \"b\") + count(words(\"one two\"))", "count = zip(all, all)"));
    assert!(!e.is_empty(), "zip gives bytes, no integer: {e:?}");
    let e = errors(&WRITER.replace("count = count(all)", "count = trim(text)"));
    assert!(!e.is_empty(), "a text is no integer: {e:?}");
}
