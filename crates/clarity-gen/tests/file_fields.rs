//! The file field of U1c on every stack: its bytes in its column and its
//! name in the column beside it, read and saved with the record; the web
//! shells fetch, open and save a copy of it; Avalonia writes Clarity's
//! FileInput.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, whose Album has a file.
const MUSIC: &str = include_str!("fixtures/asks.clarity");

fn generated(backend: Backend, ui: Ui) -> String {
    let project = load_keyed([("music.clarity", MUSIC)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings: Settings = starter("Music", "music");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "missing {expected:?}");
}

#[test]
fn the_table_has_a_column_for_the_files_name() {
    let code = generated(Backend::Rust, Ui::Web);
    has(&code, "\"booklet\" BLOB");
    // SQLite keeps text of any length as TEXT; the server databases as VARCHAR(255).
    has(&code, "\"booklet\" BLOB,\n    \"booklet_name\" TEXT NOT NULL DEFAULT ''");
}

#[test]
fn every_stack_reads_and_saves_a_file_with_its_name() {
    let rust = generated(Backend::Rust, Ui::Web);
    has(&rust, "pub booklet: Option<RecordFile>,");
    has(&rust, "sql::get_file(\"booklet\", &values[");
    has(&rust, "PictureColumn { column: \"booklet\", store: None, name: Some(\"booklet_name\") }");
    has(&rust, "fn file_data(");
    has(&generated(Backend::Go, Ui::Web), "clarity.FileContent(r.Booklet), clarity.FileNameValue(r.Booklet)");
    has(&generated(Backend::C, Ui::Web), "clarity_get_file(\"booklet\"");
    has(&generated(Backend::Cpp, Ui::Web), "clarity::get_file(\"booklet\"");
    let csharp = generated(Backend::Csharp, Ui::Avalonia);
    has(&csharp, "public RecordFile? Booklet");
    has(&csharp, "new(\"booklet\", null, \"booklet_name\")");
    has(&csharp, "<c:FileInput x:Name=\"bookletBox\"");
    has(&csharp, "bookletBox.CopySaved += _ => Vm.Say(Level.Info, Clarity.Runtime.Text.Of(\"fileCopySaved\"));");
}

#[test]
fn migrations_add_rename_and_remove_a_files_name_with_it() {
    let module = "module shop\n\nentity Product\n    name: text(80)\n    sheet: file\n\npage Card: window\n    data product: Product\n";
    let first = "migration \"First\"\n    add entity Product\n        name: text(80)\n        manual: file\n";
    let second = "migration \"Files\"\n    rename field Product.manual to guide\n    add field Product.sheet: file\n";
    let third = "migration \"Tidy\"\n    remove field Product.guide\n";
    let project = clarity_spec::load_with_migrations([("shop.clarity", module)], [("0001.clarity", first), ("0002.clarity", second), ("0003.clarity", third)]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Rust;
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let schema = &files.iter().find(|f| f.path == "src/schema.rs").unwrap().text;
    has(schema, "\"manual\" BLOB,\n    \"manual_name\" TEXT NOT NULL DEFAULT ''");
    has(schema, "ALTER TABLE \"product\" RENAME COLUMN \"manual\" TO \"guide\";");
    has(schema, "ALTER TABLE \"product\" RENAME COLUMN \"manual_name\" TO \"guide_name\";");
    has(schema, "ALTER TABLE \"product\" ADD COLUMN \"sheet\" BLOB;");
    has(schema, "ALTER TABLE \"product\" ADD COLUMN \"sheet_name\" TEXT NOT NULL DEFAULT '';");
    // SQLite drops a column by building the table again, with the files' names copied over.
    has(schema, "INSERT INTO \"product__new\" (\"id\", \"name\", \"sheet\", \"sheet_name\") SELECT \"id\", COALESCE(\"name\", ''), \"sheet\", \"sheet_name\" FROM \"product\";");
}
