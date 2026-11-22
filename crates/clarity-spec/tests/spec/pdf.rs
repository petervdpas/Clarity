//! `pdf of text stylesheet css theme true` (docs/ledger/reports-pdf.md, step 4):
//! a Markdown text made into a PDF's bytes, typed by the checker.

use clarity_spec::{Severity, load_converted};

const DESK: &str = "module desk

entity Note
    title: text, label @noteTitle
    body: text, label @noteBody

page Desk: window
    data note: Note
    data css: text

code Desk
    handler printClicked
        let file = pdf of note.body
        let styled = pdf of \"# {note.title}\" stylesheet css
        let themed = pdf of note.body stylesheet css theme true
        let size = file.size + styled.size + themed.size
        write themed to exports \"note.pdf\"

function printed(text: text) -> bytes
    return pdf of text theme false
";

fn errors(text: &str) -> Vec<String> {
    load_converted([("desk.clarity", text)]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect()
}

fn says(text: &str, wanted: &str) {
    let e = errors(text);
    assert!(e.iter().any(|m| m.contains(wanted)), "wanted {wanted:?} in {e:#?}");
}

#[test]
fn pdf_of_gives_bytes_and_takes_its_options() {
    let e = errors(DESK);
    // The file store is the project file's; only the code is looked at here.
    assert!(e.iter().all(|m| m.contains("exports")), "{e:#?}");
}

#[test]
fn what_pdf_of_takes_is_checked() {
    says(&DESK.replace("pdf of note.body\n", "pdf of note\n"), "'pdf of' takes a Markdown text, not Note");
    says(&DESK.replace("stylesheet css\n", "stylesheet 12\n"), "the option 'stylesheet' of pdf is text, not integer");
    says(&DESK.replace("theme true", "theme \"yes\""), "the option 'theme' of pdf is bool, not text");
    says(&DESK.replace("theme true", "theme true theme false"), "the option 'theme' is given twice");
    says(&DESK.replace("let size = file.size", "let size: text = file\n        let n = file.size"), "bytes");
}

#[test]
fn an_engine_makes_no_pdf() {
    let engine = "module eng\n\nengine Printer\n    function made(text: text) -> integer\n        return (pdf of text).size\n";
    says(engine, "'pdf of' runs in the program, not in an engine");
}

