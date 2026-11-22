//! Every generated project explains itself in a README.md (P3): what it is,
//! the tools its stack needs and where to get them, and how to build and
//! run it without Clarity, so it can be built on a machine Clarity is not on.

use clarity_gen::generate;
use clarity_project::{Backend, Settings, SourceKind, Ui, starter};
use clarity_spec::load_converted;

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
fn a_server_database_is_named_by_its_variable() {
    let mut settings = starter("Shop", "shop");
    settings.datasources[0].kind = SourceKind::Postgres;
    settings.datasources[0].url = "env:SHOP_DATABASE".into();
    let text = readme(&settings);
    has(&text, "`SHOP_DATABASE`");
    has(&text, "PostgreSQL");
    has(&text, "postgres://user:password@host:5432/database");
}

/// Every runtime opens an SQLite file in the folder CLARITY_DATA names, so
/// every stack's README says so.
#[test]
fn every_stack_says_clarity_data_moves_its_sqlite_file() {
    for backend in [Backend::Rust, Backend::Go, Backend::Csharp, Backend::C, Backend::Cpp] {
        let mut settings = starter("Shop", "shop");
        settings.stack.backend = backend;
        settings.stack.ui = Ui::Clarity;
        has(&readme(&settings), "Set the environment variable `CLARITY_DATA` to a folder to open it there instead, under its own file name.");
    }
}

/// A file store says where its pictures are, and which variable names it.
#[test]
fn a_file_store_says_where_its_pictures_are() {
    let mut settings = starter("Shop", "shop");
    settings.filestores.push(clarity_project::FileStore { name: "photos".into(), location: "env:PHOTO_FOLDER".into() });
    settings.filestores.push(clarity_project::FileStore { name: "scans".into(), location: "scans".into() });
    let text = readme(&settings);
    has(&text, "- `photos`: a file store of pictures. Set the environment variable `PHOTO_FOLDER` to its folder.\n");
    has(&text, "- `scans`: the file store of pictures in the folder `scans`; a relative path is taken from the folder the program starts in.\n");
}

/// A built program keeps the database the project keeps beside it in the
/// person's folder for application data (conformance/data-folder.json), so
/// the README says where that is on each system and that a run in Clarity
/// works on the project's own file, never the machine's full path.
#[test]
fn a_sqlite_file_is_kept_in_the_persons_data_folder() {
    let dir = std::env::temp_dir().join("clarity-readme-shop");
    let unbuilt = starter("Shop", "shop");
    let written = unbuilt.datasources[0].url.clone();
    let settings = clarity_project::build_settings(&dir, &unbuilt);
    assert_eq!("data:shop/shop.db", settings.datasources[0].url);
    let mut project = load_converted([("shop.clarity", SHOP)]);
    let mut source = clarity_spec::SourceRef::sql(&unbuilt.datasources[0].name);
    source.address = written.clone();
    project.sources.push(source);
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let text = &files.iter().find(|f| f.path == "README.md").expect("a README.md").text;
    has(text, "kept in the person's folder for application data: `~/.local/share/shop/shop.db` on Linux");
    has(text, &format!("Run in Clarity, it is the project's `{written}`."));
    assert!(!text.contains(&dir.display().to_string()), "{text}");
}
