//! Counting and joining in code: `a to b` is the list of whole numbers from
//! a to b, so `for i in 1 to n` counts, and `join(texts, separator)` puts a
//! list of texts together, as code that builds a file needs.

use clarity_spec::code::{ExprKind, StmtKind};
use clarity_spec::{Severity, load_sources};

const SOURCE: &str = "module counting

page Main: window
    data shown: text
    data numbers: list<integer>
    button \"Go\" name go at 1,1 span 2 on click: goClicked

code Main
    handler goClicked
        let names: list<text> = []
        for i in 1 to 3
            names = names + [\"row{i}\"]
        numbers = 2 to count(names) + 2
        shown = join(names, \", \")
";

fn errors(source: &str) -> Vec<String> {
    load_sources([("counting.clarity", source)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

#[test]
fn a_to_b_counts_and_join_puts_texts_together() {
    assert_eq!(errors(SOURCE), Vec::<String>::new());
    let project = load_sources([("counting.clarity", SOURCE)]);
    let handler = &project.files[0].code[0].handlers[0];
    let StmtKind::For { list, .. } = &handler.body[1].kind else { panic!("{:?}", handler.body[1].kind) };
    assert!(matches!(list.kind, ExprKind::Range { .. }), "{:?}", list.kind);
    assert_eq!(clarity_spec::code::print::expr(list), "1 to 3");
}

#[test]
fn a_range_counts_whole_numbers() {
    let text = SOURCE.replace("for i in 1 to 3", "for i in 1 to \"three\"");
    assert!(errors(&text).iter().any(|e| e == "'to' counts from one whole number to another, as in 1 to 10"), "{:?}", errors(&text));
}

#[test]
fn join_takes_texts_and_what_goes_between() {
    let numbers = SOURCE.replace("join(names, \", \")", "join(numbers, \", \")");
    assert!(errors(&numbers).iter().any(|e| e == "'join' takes a list of texts and the text between them, as in join(names, \", \")"), "{:?}", errors(&numbers));
    let one = SOURCE.replace("join(names, \", \")", "join(names)");
    assert!(errors(&one).iter().any(|e| e.starts_with("'join' takes a list of texts")), "{:?}", errors(&one));
}

/// `write ... to store "path"` keeps its `to`: what it writes is no range.
#[test]
fn write_to_is_no_range() {
    let source = "module files

action export(lines: text)
    effects files.write
    write lines to exports \"list.txt\"
";
    let mut project = load_sources([("files.clarity", source)]);
    clarity_spec::attach_stores(&mut project, vec!["exports".into()]);
    let errors: Vec<String> = project.diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect();
    assert_eq!(errors, Vec::<String>::new());
}
