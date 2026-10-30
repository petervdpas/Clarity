//! Cascading deletes on every stack: deleting a project deletes its tasks
//! first, through the record's cascade that the runtime's delete runs.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module planning

entity Project
    name: text(80), required

entity Task
    project: ref Project, required, cascade
    title: text(80)

page Main: window
    button \"Go\" name go at 1,1 span 2 on click: goClicked

code Main
    handler goClicked
        for p in find p in Project where p.name == \"Old\"
            delete p
";

fn generated(backend: Backend, ui: Ui) -> String {
    let project = clarity_spec::keyed::load_keyed([("planning.clarity", SOURCE)], &[], "en");
    let mut settings = starter("Planning", "planning");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "expected\n{expected}\nin the generated program");
}

#[test]
fn every_stack_deletes_what_cascades_first() {
    let rust = generated(Backend::Rust, Ui::Web);
    has(&rust, "const CASCADES: bool = true;");
    has(&rust, "Cond::compare(\"project\", clarity_runtime::query::Op::Eq, SqlValue::Integer(id))");
    let go = generated(Backend::Go, Ui::Web);
    has(&go, "func (r *Project) Cascade(db *clarity.Db) error {");
    has(&go, "clarity.Compare(\"project\", clarity.Eq, r.ID)");
    let csharp = generated(Backend::Csharp, Ui::Avalonia);
    has(&csharp, "public static bool HasCascades => true;");
    has(&csharp, "db.Find<Task>(new Query().Filter(Cond.Compare(\"project\", Op.Eq, Id)))");
    let c = generated(Backend::C, Ui::Web);
    has(&c, ".cascade = ");
    has(&c, "clarity_compare(\"project\", CLARITY_EQ, clarity_integer_value(id))");
    let cpp = generated(Backend::Cpp, Ui::Web);
    has(&cpp, "void cascade(clarity::Db& db) const;");
    has(&cpp, "db.find<Task>(clarity::Query().filter(clarity::Cond::compare(\"project\", clarity::Op::Eq, clarity::sql::integer(id))))");
}
