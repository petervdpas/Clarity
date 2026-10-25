//! Secret settings (DESIGN 5.27) on every stack: kept in the system's
//! protected store per program, never in the settings file. A web API signs
//! with one and a database connects with one, each named in the project file
//! as `setting:name` and in the program as `setting:<program>/name`.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, SourceKind, Ui, starter};

const SHOP: &str = "module shop

settings
    shopDb: secret

entity Customer
    name: text(80)

page Main: window
    password name connection at 1,1 span 6 bind settings.shopDb
";

fn generated(backend: Backend, ui: Ui, connection: &str) -> String {
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    // A SQLite file is no secret; a server database's connection may be.
    if connection != "data/shop.db" {
        settings.datasources[0].kind = SourceKind::Postgres;
        settings.datasources[0].url = connection.into();
    }
    let project = clarity_spec::load_in([("shop.clarity", SHOP)], [("0001.clarity", "migration \"First\"\n    add entity Customer\n        name: text(80)\n")], &clarity_project::source_refs(&settings));
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

const STACKS: [(Backend, Ui); 5] = [(Backend::Rust, Ui::Web), (Backend::Go, Ui::Web), (Backend::Csharp, Ui::Avalonia), (Backend::C, Ui::Web), (Backend::Cpp, Ui::Web)];

/// C and C++ store records in SQLite only, for now, so they have no
/// connection to keep secret yet.
#[test]
fn a_database_connects_with_the_secret_setting_of_its_program() {
    for (backend, ui) in &STACKS[..3] {
        let (backend, ui) = (*backend, *ui);
        let code = generated(backend, ui, "setting:shopDb");
        assert!(code.contains("/shopDb"), "{backend:?} names the setting of its program");
        assert!(!code.contains("\"setting:shopDb\""), "{backend:?} keeps the project file's name");
    }
}

#[test]
fn the_secrets_are_named_to_the_settings() {
    let rust = generated(Backend::Rust, Ui::Web, "data/shop.db");
    assert!(rust.contains("const SECRETS: &[&str] = &[\"shopDb\"];"));
    assert!(rust.contains("clarity_runtime::settings::read_with(PROGRAM, SECRETS)"));
    let go = generated(Backend::Go, Ui::Web, "data/shop.db");
    assert!(go.contains("clarity.Secrets = secrets.System{}"));
    let csharp = generated(Backend::Csharp, Ui::Avalonia, "data/shop.db");
    assert!(csharp.contains("KeptSettings.Read(ProgramName, Secrets)"));
    for backend in [Backend::C, Backend::Cpp] {
        let code = generated(backend, Ui::Web, "data/shop.db");
        assert!(code.contains("secrets_use();"), "{backend:?} turns the store on");
        assert!(code.contains("clarity_secrets"), "{backend:?} links the store");
    }
}
