//! Lists of values as fields (U1c) on every stack: kept in one column as
//! JSON, read and saved with the record, used in code with `contains`, and
//! edited on Avalonia with Clarity's TagsInput.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, whose Album has lists of values.
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
fn a_list_is_one_text_column_that_starts_empty() {
    let code = generated(Backend::Rust, Ui::Web);
    has(&code, "\"tags\" TEXT NOT NULL DEFAULT '[]'");
    has(&code, "\"formats\" TEXT NOT NULL DEFAULT '[]'");
}

#[test]
fn every_stack_reads_and_saves_a_list_with_its_record() {
    let rust = generated(Backend::Rust, Ui::Web);
    has(&rust, "pub tags: Vec<String>,");
    has(&rust, "pub formats: Vec<Format>,");
    has(&rust, "pub plays: Vec<Option<NaiveDate>>,");
    has(&rust, "sql::list(&self.tags)");
    has(&rust, "tags: sql::get_list(\"tags\", &values[");
    has(&rust, ".tags.contains(&");
    has(&rust, "Cond::holds(\"tags\", sql::text(");
    has(&rust, "Cond::holds(\"formats\", sql::name(");
    let go = generated(Backend::Go, Ui::Web);
    has(&go, "clarity.ListValue(r.Tags)");
    has(&go, "clarity.ReadList[Format](read, ");
    has(&go, "clarity.ListHolds(");
    has(&go, "clarity.Holds(\"tags\", ");
    let c = generated(Backend::C, Ui::Web);
    has(&c, "ClarityTexts tags;");
    has(&c, "FormatValues formats;");
    has(&c, "clarity_texts_value(format_values_names(r->formats))");
    has(&c, "clarity_get_dates(\"plays\"");
    has(&c, "format_values_holds(");
    has(&c, "clarity_holds(\"plays\", clarity_date_value(");
    let cpp = generated(Backend::Cpp, Ui::Web);
    has(&cpp, "std::vector<std::string> tags;");
    has(&cpp, "clarity::sql::list(tags)");
    has(&cpp, "clarity::sql::get_list(\"formats\", values[");
    has(&cpp, "clarity::holds(");
    has(&cpp, "clarity::Cond::holds(\"tags\", ");
    let csharp = generated(Backend::Csharp, Ui::Avalonia);
    has(&csharp, "public List<string> Tags");
    has(&csharp, "ListFields.Kept(Tags)");
    has(&csharp, "ListFields.Read<Format>(\"formats\", values[");
    has(&csharp, "<c:TagsInput x:Name=\"tagsBox\"");
    has(&csharp, "Cond.Holds(\"formats\", ");
    has(&csharp, "Item=\"Choice\" Choices=\"{x:Static local:Choices.Format}\"");
    has(&csharp, "Item=\"Date\"");
    has(&csharp, "tagsBox.ValueChanged += (_, _) =>");
}

#[test]
fn migrations_add_a_list_as_an_empty_one() {
    let module = "module shop\n\nentity Product\n    name: text(80)\n    tags: list<text>\n\npage Card: window\n    data product: Product\n";
    let first = "migration \"First\"\n    add entity Product\n        name: text(80)\n";
    let second = "migration \"Tags\"\n    add field Product.tags: list<text>\n";
    let project = clarity_spec::load_with_migrations([("shop.clarity", module)], [("0001.clarity", first), ("0002.clarity", second)]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings: Settings = starter("Shop", "shop");
    settings.stack.backend = Backend::Rust;
    let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let schema = &files.iter().find(|f| f.path == "src/schema.rs").unwrap().text;
    has(schema, "ALTER TABLE \"product\" ADD COLUMN \"tags\" TEXT NOT NULL DEFAULT '[]';");
}
