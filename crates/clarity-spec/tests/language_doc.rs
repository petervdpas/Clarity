//! docs/LANGUAGE.md is the language reference, so its examples must be
//! valid Clarity: every snippet parses, and the complete example checks
//! clean.

use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_sources, widgets};

const DOC: &str = include_str!("../../../docs/LANGUAGE.md");

/// The fenced code blocks of the document, in order.
fn blocks() -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for line in DOC.lines() {
        if line.starts_with("```") {
            match current.take() {
                Some(block) => out.push(block),
                None => current = Some(String::new()),
            }
        } else if let Some(block) = current.as_mut() {
            block.push_str(line);
            block.push('\n');
        }
    }
    out
}

/// A snippet as a file the parser reads: declarations get a module line,
/// parts get a window around them. Other blocks (the project file, the
/// folder tree, the grammar) are not module source.
fn as_module(block: &str) -> Option<String> {
    let first = block.split_whitespace().next()?;
    match first {
        "module" => Some(block.to_string()),
        "enum" | "entity" | "page" | "app" | "theme" | "code" | "function" | "action" => {
            Some(format!("module doc\n{block}"))
        }
        word if widgets::by_keyword(word).is_some() => {
            let indented: String = block.lines().map(|l| format!("    {l}\n")).collect();
            Some(format!("module doc\npage Doc: window\n{indented}"))
        }
        _ => None,
    }
}

#[test]
fn every_snippet_parses() {
    let mut checked = 0;
    for block in blocks() {
        let Some(source) = as_module(&block) else { continue };
        let (_, diagnostics) = parse(&source, "doc.clarity");
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| &d.message)
            .collect();
        assert!(errors.is_empty(), "{errors:#?} in\n{source}");
        checked += 1;
    }
    assert!(checked >= 12, "only {checked} snippets were found");
}

#[test]
fn every_migration_snippet_parses() {
    let steps: Vec<String> = blocks().into_iter().filter(|b| b.starts_with("migration ")).collect();
    assert!(!steps.is_empty(), "the reference shows a migration step");
    for step in steps {
        let (_, diagnostics) = clarity_spec::migrate::parse(&step, "0001.clarity");
        assert!(diagnostics.is_empty(), "{diagnostics:#?} in\n{step}");
    }
}

#[test]
fn the_complete_example_checks_clean() {
    let example = blocks()
        .into_iter()
        .find(|b| b.starts_with("module customers\n"))
        .expect("section 11 has the customers module");
    let project = load_sources([("customers.clarity", example.as_str())]);
    let problems: Vec<_> = project
        .diagnostics
        .iter()
        .map(|d| (d.severity, &d.message))
        .collect();
    assert!(problems.is_empty(), "{problems:#?}");
}
