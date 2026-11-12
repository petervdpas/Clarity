//! docs/LANGUAGE.md is the language reference, so its examples must be
//! valid Clarity: every snippet parses, and the complete example (its
//! module and the `.cxaml` files of its windows) checks clean.

#[path = "doc_snippets.rs"]
mod doc_snippets;

use clarity_spec::parser::parse;
use clarity_spec::{Severity, load_converted};
use doc_snippets::as_module;

const DOC: &str = include_str!("../../../docs/LANGUAGE.md");

fn blocks() -> Vec<String> {
    doc_snippets::blocks(DOC)
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
    // Its app, windows and views are the `.cxaml` blocks of module customers,
    // each a file of its own in design/, named after what its root's class names.
    let pages: Vec<(String, String)> = blocks().into_iter().filter_map(|b| page_file(&b, "customers")).collect();
    assert_eq!(pages.len(), 5, "section 11 shows the app, two windows and their two views");
    let mut sources = vec![("customers.clarity", example.as_str())];
    sources.extend(pages.iter().map(|(path, text)| (path.as_str(), text.as_str())));
    let project = load_converted(sources);
    let problems: Vec<_> = project
        .diagnostics
        .iter()
        .map(|d| (d.severity, &d.message))
        .collect();
    assert!(problems.is_empty(), "{problems:#?}");
}

/// A `.cxaml` block whose root's class is a page or the app of `module`, as
/// the file it would be: `design/CustomerBrowse.cxaml` for `class="customers.CustomerBrowse"`.
fn page_file(block: &str, module: &str) -> Option<(String, String)> {
    let class = block.split("class=\"").nth(1)?.split('"').next()?;
    let page = class.strip_prefix(module)?.strip_prefix('.')?;
    Some((format!("design/{page}.cxaml"), block.to_string()))
}
