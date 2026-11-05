//! A project with no entities stores nothing: it needs no data source, and
//! every backend generates it without one. One with entities still needs one.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module plain

page Main: window
    label \"Hello\" name hello at 1,1 span 4
";

#[test]
fn a_project_without_entities_needs_no_data_source() {
    for (backend, ui) in [(Backend::Rust, Ui::Clarity), (Backend::Go, Ui::Clarity), (Backend::C, Ui::Clarity), (Backend::Cpp, Ui::Clarity), (Backend::Csharp, Ui::Clarity)] {
        let mut settings = starter("Plain", "plain");
        settings.stack.backend = backend;
        settings.stack.ui = ui;
        settings.datasources.clear();
        let project = clarity_spec::load_in([("plain.clarity", SOURCE)], [] as [(&str, &str); 0], &clarity_project::source_refs(&settings));
        let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
        assert!(errors.is_empty(), "{errors:#?}");
        generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    }
}

#[test]
fn a_project_with_entities_still_needs_one() {
    let mut settings = starter("Plain", "plain");
    settings.datasources.clear();
    let source = format!("{SOURCE}\nentity Note\n    text: text(80)\n");
    let project = clarity_spec::load_in([("plain.clarity", source.as_str())], [] as [(&str, &str); 0], &clarity_project::source_refs(&settings));
    let errors = generate(&project, &settings).err().expect("no data source for the entities");
    assert!(errors.iter().any(|e| e.contains("data source")), "{errors:#?}");
}
