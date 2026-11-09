//! A query can compare a record's id (its key): `find p in Project where
//! p.id != current.id` lists the others. Every backend writes it on the
//! key column.

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module board

entity Project
    name: text(80)

page Main: window
    data current: Project
    data others: list<Project>
    button \"Others\" name othersButton at 1,1 span 2 on click: othersClicked

code Main
    handler othersClicked
        others = find p in Project where p.id != current.id and p.id in [1, 2] order by p.name
";

const FIRST: &str = "migration \"First\"\n    add entity Project\n        name: text(80)\n";

#[test]
fn every_backend_queries_on_a_records_id() {
    for (backend, ui) in [(Backend::Rust, Ui::Clarity), (Backend::Go, Ui::Clarity), (Backend::C, Ui::Clarity), (Backend::Cpp, Ui::Clarity), (Backend::Csharp, Ui::Clarity)] {
        let mut settings = starter("Board", "board");
        settings.stack.backend = backend;
        settings.stack.ui = ui;
        let project = clarity_spec::load_in(clarity_spec::converted([("board.clarity", SOURCE)]).iter().map(|(p, t)| (p.as_str(), t.as_str())), [("0001.clarity", FIRST)], &clarity_project::source_refs(&settings));
        let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
        assert!(errors.is_empty(), "{errors:#?}");
        let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
        let code: String = files.iter().map(|f| f.text.as_str()).collect();
        assert!(code.contains("\"id\""), "{backend:?} compares the key column");
    }
}
