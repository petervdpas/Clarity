//! A setting that refers to a record: kept as its id, read back as one, and
//! a dropdown bound to it offers the entity's records.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};

const SOURCE: &str = "module planning

entity Project
    name: text(80), required

settings
    openProject: ref Project, label \"Open project\"

page Main: window
    dropdown name openBox at 1,1 span 4 bind settings.openProject
    button \"Go\" name go at 1,2 span 2 on click: goClicked

code Main
    handler goClicked
        if settings.openProject != none
            let open = settings.openProject
            message open.name
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
fn avalonia_reads_the_id_and_offers_the_records() {
    let csharp = generated(Backend::Csharp, Ui::Avalonia);
    has(&csharp, "public long? OpenProject");
    has(&csharp, "settings.OpenProject = _openProjectKept.GetValue<long>();");
    has(&csharp, "ProjectChoices = Choice.Of(Db.All<Project>()");
}

#[test]
fn every_stack_generates_it() {
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp] {
        generated(backend, Ui::Web);
    }
}
