//! The manual (manual) teaches by example, so its examples must be
//! valid Clarity: every module snippet of every page parses, and every
//! migration step reads as a step.

#[path = "doc_snippets.rs"]
mod doc_snippets;

use clarity_spec::Severity;
use clarity_spec::parser::parse;

/// Every page of the manual, by its file name, with its text.
fn pages() -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manual");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), std::fs::read_to_string(&p).unwrap()))
        .collect();
    out.sort();
    out
}

#[test]
fn every_snippet_of_the_manual_parses() {
    let mut checked = 0;
    let mut wrong = Vec::new();
    for (page, text) in pages() {
        for block in doc_snippets::blocks(&text) {
            if block.starts_with("migration ") {
                let (_, diagnostics) = clarity_spec::migrate::parse(&block, "0001.clarity");
                if !diagnostics.is_empty() {
                    wrong.push(format!("{page}: {diagnostics:?} in\n{block}"));
                }
                checked += 1;
                continue;
            }
            let Some(source) = doc_snippets::as_module(&block) else { continue };
            let (_, diagnostics) = parse(&source, "doc.clarity");
            let errors: Vec<&String> = diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| &d.message).collect();
            if !errors.is_empty() {
                wrong.push(format!("{page}: {errors:?} in\n{source}"));
            }
            checked += 1;
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n\n"));
    assert!(checked >= 30, "only {checked} snippets were found");
}
