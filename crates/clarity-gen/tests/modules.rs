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
    button \"Loans\" name button1 at 1,1 span 2 on click: loansClicked
    button \"On loan\" name button2 at 3,1 span 2 on click -> LoanBrowse

code AlbumForm
    handler loansClicked
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
    button \"Album\" name button1 at 1,1 span 2 on click -> AlbumForm
";

const STACKS: [(Backend, Ui); 5] =
    [(Backend::Rust, Ui::Web), (Backend::Csharp, Ui::Avalonia), (Backend::Go, Ui::Web), (Backend::C, Ui::Web), (Backend::Cpp, Ui::Web)];

fn project() -> Project {
    let project = load_sources([("music.clarity", MUSIC), ("loans.clarity", LOANS)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    project
}

#[test]
fn every_stack_generates_modules_that_use_each_other() {
    let project = project();
    for (backend, ui) in STACKS {
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

/// Module names a designer may well pick, which are also the names of files
/// a stack writes for the whole project.
const COMMON_NAMES: [&str; 12] = ["records", "windows", "schema", "main", "app", "page", "program", "views", "shown", "texts", "lib", "pictures"];

#[test]
fn a_module_never_overwrites_a_file_of_the_stack() {
    let texts: Vec<(String, String)> = COMMON_NAMES
        .iter()
        .map(|name| (format!("{name}.clarity"), format!("module {name}\n\nfunction {name}_twice(n: integer) -> integer\n    return n * 2\n")))
        .collect::<Vec<_>>()
        .into_iter()
        .chain([("start.clarity".to_string(), "module start\n\napp Records\n    main Main\n\npage Main: window\n    title \"Main\"\n".to_string())])
        .collect();
    let project = load_sources(texts.iter().map(|(f, t)| (f.as_str(), t.as_str())));
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut clashes = Vec::new();
    for (backend, ui) in STACKS {
        let mut settings = starter("Records", "records");
        settings.stack.backend = backend;
        settings.stack.ui = ui;
        let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{backend:?}: {e:#?}"));
        let mut paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        paths.sort_unstable();
        let twice: Vec<_> = paths.windows(2).filter(|w| w[0].eq_ignore_ascii_case(w[1])).map(|w| w[0]).collect();
        // Go builds a file named like `windows.go` only on that system.
        if backend == Backend::Go && paths.contains(&"windows.go") {
            clashes.push("Go writes windows.go, which builds on Windows only".to_string());
        }
        if !twice.is_empty() {
            clashes.push(format!("{backend:?} writes {twice:?} twice"));
        }
    }
    assert!(clashes.is_empty(), "{clashes:#?}");
}
