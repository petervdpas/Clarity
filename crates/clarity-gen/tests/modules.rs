//! Two modules that use each other (X2): each sees the other's entities and
//! windows, an entity of one refers to an entity of the other and back, and
//! every stack generates the project. Server databases create both tables
//! before the references between them.

use clarity_gen::generate;
use clarity_project::{Backend, SourceKind, Ui, starter};
use clarity_spec::{Project, Severity, load_sources};

const MUSIC: &str = "module music

use loans

entity Album
    title: text(120), required
    last_loan: ref Loan

page AlbumForm: window
    data album: Album
    button \"Loans\" at 1,1 span 2 on click: loans_clicked
    button \"On loan\" at 3,1 span 2 on click -> LoanBrowse

code AlbumForm
    handler loans_clicked
        let lent = find l in Loan where l.album == album
        let n = count(lent)
        message \"{n} loans\"
";

const LOANS: &str = "module loans

use music

entity Loan
    album: ref Album, required
    lent_on: date

page LoanBrowse: window
    data loans: list<Loan>
    button \"Album\" at 1,1 span 2 on click -> AlbumForm
";

fn project() -> Project {
    let project = load_sources([("music.clarity", MUSIC), ("loans.clarity", LOANS)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    project
}

#[test]
fn every_stack_generates_modules_that_use_each_other() {
    let project = project();
    for (backend, ui) in [(Backend::Rust, Ui::Web), (Backend::Csharp, Ui::Avalonia), (Backend::Go, Ui::Web)] {
        let mut settings = starter("Records", "records");
        settings.stack.backend = backend;
        settings.stack.ui = ui;
        generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
    }
}

#[test]
fn tables_that_refer_to_each_other_get_their_references_afterwards() {
    let project = project();
    let mut settings = starter("Records", "records");
    settings.stack.backend = Backend::Go;
    settings.stack.ui = Ui::Web;
    settings.datasources[0].kind = SourceKind::Postgres;
    settings.datasources[0].url = "env:RECORDS_DATABASE".into();
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let schema = &files.iter().find(|f| f.path == "schema.go").unwrap().text;
    let created = schema.rfind("CREATE TABLE").unwrap();
    for reference in ["ALTER TABLE \"album\" ADD FOREIGN KEY (\"last_loan\") REFERENCES \"loan\"", "ALTER TABLE \"loan\" ADD FOREIGN KEY (\"album\") REFERENCES \"album\""] {
        let at = schema.find(reference).unwrap_or_else(|| panic!("no {reference} in\n{schema}"));
        assert!(at > created, "{reference} comes before a table is created");
    }
}
