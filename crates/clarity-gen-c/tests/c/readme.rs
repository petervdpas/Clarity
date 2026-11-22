//! The README.md this stack writes for a project (P3): what it needs, how to
//! build and run it without Clarity (clarity_gen_core::readme and its own sections).

use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::load_converted;
use clarity_gen_c::program as generate;

const SHOP: &str = "module shop

entity Customer
    name: text(80)

page Card: window
    title \"Customer card\"
    data customer: Customer
";

fn readme(settings: &Settings) -> String {
    let project = load_converted([("shop.clarity", SHOP)]);
    let files = generate(&project, settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.into_iter().find(|f| f.path == "README.md").expect("a README.md").text
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin\n{text}");
}

#[test]
fn a_c_desktop_project_says_where_clarity_ui_is() {
    let backend = Backend::C;
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = Ui::Clarity;
    let text = readme(&settings);
    // The library comes from the folder CLARITY_UI names; without it CMake fetches the release's archive.
    has(&text, "CLARITY_UI");
    has(&text, &format!("`clarity-ui-{}-<system>.tar.gz`", clarity_gen_core::clarity_ui::version()));
}
