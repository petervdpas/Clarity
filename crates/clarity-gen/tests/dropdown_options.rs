//! The dropdown's options of U1c on every stack: a typed dropdown's values
//! come from the shell (`field_values`), a search dropdown's records carry
//! the fields it shows; Avalonia writes Clarity's TypedInput and LookupInput.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, with a typed and a search dropdown.
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
fn every_web_shell_gives_a_fields_values_and_the_fields_a_search_shows() {
    let rust = generated(Backend::Rust, Ui::Web);
    has(&rust, "fn field_values(");
    has(&rust, "(\"Friend\", \"city\") => db.all::<");
    has(&rust, "fields: serde_json::Map::from_iter([(\"name\".to_string(), clarity_runtime::ui::to_value(&r.name)?), (\"city\".to_string(), clarity_runtime::ui::to_value(&r.city)?)])");
    let go = generated(Backend::Go, Ui::Web);
    has(&go, "func (windows) Values(");
    has(&go, "Fields: map[string]any{\"name\": r.Name, \"city\": r.City}");
    let c = generated(Backend::C, Ui::Web);
    has(&c, "static bool values(");
    has(&c, "cJSON_AddItemToObject(fields, \"city\", clarity_json_text(r->city));");
    let cpp = generated(Backend::Cpp, Ui::Web);
    has(&cpp, "std::vector<std::string> values(");
    has(&cpp, "nlohmann::json{{\"name\", r.name}, {\"city\", r.city}}");
}

#[test]
fn avalonia_writes_a_typed_and_a_search_dropdown() {
    let csharp = generated(Backend::Csharp, Ui::Avalonia);
    has(&csharp, "<c:TypedInput x:Name=\"cityBox\"");
    has(&csharp, "Values=\"{Binding FriendCityValues}\"");
    has(&csharp, "FriendCityValues = [.. Db.All<Friend>().Select(r => r.City)];");
    has(&csharp, "<c:LookupInput x:Name=\"lentToBox\"");
    has(&csharp, "Choices=\"{Binding FriendNameCityChoices}\"");
    has(&csharp, "FriendNameCityChoices = Choice.Of(Db.All<Friend>(), r => r.Name, r => [Shown.Converter.TextOf(r.Name), Shown.Converter.TextOf(r.City)]);");
    has(&csharp, "Choices=\"{Binding FriendChoices}\"");
    has(&csharp, "lentToBox.ValueChanged += (_, _) =>");
}
