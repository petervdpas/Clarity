//! The manual (manual) teaches by example, so its examples must be
//! valid Clarity: every module snippet of every page parses, and every
//! migration step reads as a step.

#[path = "doc_snippets.rs"]
mod doc_snippets;

use clarity_spec::Severity;
use clarity_spec::load_converted;
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

/// The complete example (start.complete-example) is a whole project: its
/// module and the `.cxaml` files of its app, windows and views check clean
/// together.
#[test]
fn the_complete_example_checks_clean() {
    let text = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manual/start.complete-example.md")).unwrap();
    let blocks = doc_snippets::blocks(&text);
    let example = blocks.iter().find(|b| b.starts_with("module customers\n")).expect("the page has the customers module");
    // Its app, windows and views, each a file of its own in design/, named after what its root's class names.
    let pages: Vec<(String, String)> = blocks.iter().filter_map(|b| page_file(b, "customers")).collect();
    assert_eq!(pages.len(), 5, "the page shows the app, two windows and their two views");
    let mut sources = vec![("customers.clarity", example.as_str())];
    sources.extend(pages.iter().map(|(path, text)| (path.as_str(), text.as_str())));
    let project = load_converted(sources);
    let problems: Vec<_> = project.diagnostics.iter().map(|d| (d.severity, &d.message)).collect();
    assert!(problems.is_empty(), "{problems:#?}");
}

/// A `.cxaml` block whose root's class is a page or the app of `module`, as
/// the file it would be: `design/CustomerBrowse.cxaml` for `class="customers.CustomerBrowse"`.
fn page_file(block: &str, module: &str) -> Option<(String, String)> {
    let class = block.split("class=\"").nth(1)?.split('"').next()?;
    let page = class.strip_prefix(module)?.strip_prefix('.')?;
    Some((format!("design/{page}.cxaml"), block.to_string()))
}
