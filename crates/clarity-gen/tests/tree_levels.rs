//! A tree of several kinds of records on every stack (DESIGN 5.33): placed,
//! it is a tree of its rows arranged by key, with the handlers that load the
//! rows and run the handler of the kind of row picked.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module shop

entity Project
    name: text(80), label @projectNameLabel

entity Domain
    project: ref Project, label @domainProjectLabel
    name: text(80), label @domainNameLabel

page Main: window
    content name main at 1,1 span 12 rows 10
        view Explorer

page Explorer: view
    data picked: text
    tree name modelTree at 1,1 span 4 rows 10
        level Project show name order by name
            group @domainsNode
                level Domain under project show name
        on select Domain: domainPicked

code Explorer
    handler domainPicked(d: Domain)
        picked = d.name
";

const EN: &str = "translation en
    projectNameLabel \"Name\"
    domainProjectLabel \"Project\"
    domainNameLabel \"Name\"
    domainsNode \"Domains\"
";

#[test]
fn a_tree_of_levels_is_generated_on_every_stack() {
    let mut project = clarity_spec::load_converted([("shop.clarity", SOURCE)]);
    project.locale.language = Some("en".into());
    clarity_spec::translations::attach(&mut project, [("translations/en.clarity", EN)]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp, Backend::Csharp] {
        let mut settings = starter("Shop", "shop");
        settings.stack.backend = backend;
        settings.stack.ui = Ui::Clarity;
        let code: String = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}")).iter().map(|f| format!("{}\n", f.text)).collect();
        assert!(code.contains("<Tree name=\"modelTree\"") && code.contains("bind=\"modelTreeRows\""), "{backend:?} has no tree of rows");
        assert!(code.contains("onselect=\"modelTreePicked\""), "{backend:?}");
    }
}
