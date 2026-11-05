//! A reference copied from another reference keeps it as it is, set or not:
//! `copy.target = p.target` copies the empty reference too, instead of
//! loading the record it would point at (which fails when there is none).

use clarity_gen::generate;
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module m

entity Thing
    name: text(40)

entity Part
    name: text(40)
    target: ref Thing

action copy_part(p: Part)
    effects db.write
    let copy = new Part
    copy.name = p.name
    copy.target = p.target
    save copy

app M
    main Main

page Main: window
    data parts: list<Part>
    table name grid at 1,1 span 6 rows 4 bind parts columns name
";

fn generated(backend: Backend) -> String {
    let project = clarity_spec::keyed::load_keyed([("m.clarity", SOURCE)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings = starter("M", "m");
    settings.stack.backend = backend;
    settings.stack.ui = Ui::Clarity;
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

fn copy_line(backend: Backend) -> String {
    let code = generated(backend);
    code.lines().filter(|l| l.contains("opy.target") || l.contains("opy.Target") || l.contains("opy->target")).map(str::trim).collect::<Vec<_>>().join(" | ")
}

#[test]
fn every_stack_copies_a_reference_as_it_is() {
    assert_eq!(copy_line(Backend::Rust), "copy.target = p.target;");
    assert_eq!(copy_line(Backend::Cpp), "copy.target = p.target;");
    assert_eq!(copy_line(Backend::Csharp), "copy.Target = p.Target;");
    assert!(generated(Backend::Go).contains("\tcopy_.Target = p.Target\n"));
    assert!(generated(Backend::C).contains("    copy.target = p->target;\n"));
}

