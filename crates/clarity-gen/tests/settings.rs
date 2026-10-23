//! Settings (DESIGN 5.17) on every stack: a record type beside the
//! entities, never in a database; every window has `settings`. The web
//! stacks' shells read and keep them (the Rust commands are generated);
//! Avalonia shares one `Settings.Current`, started with the application.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, which has settings.
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

fn lacks(text: &str, unexpected: &str) {
    assert!(!text.contains(unexpected), "has {unexpected:?}");
}

#[test]
fn the_rust_app_reads_and_keeps_the_settings_and_never_stores_them() {
    let code = generated(Backend::Rust, Ui::Web);
    has(&code, "pub struct Settings {");
    has(&code, "pub settings: Settings,");
    has(&code, "fn read_settings() -> Map<String, Value>");
    has(&code, "clarity_runtime::settings::keep(PROGRAM, &settings)");
    has(&code, "            read_settings,\n            keep_settings,");
    lacks(&code, "impl Record for Settings");
    lacks(&code, "CREATE TABLE settings");
    lacks(&code, "\"Settings\" => db");
    has(&code, "\"settings\":true");
}

#[test]
fn the_other_web_stacks_keep_settings_out_of_the_database() {
    for backend in [Backend::Go, Backend::C, Backend::Cpp] {
        let code = generated(backend, Ui::Web);
        lacks(&code, "CREATE TABLE settings");
        // The model the renderer draws from marks them, written as a string of the language.
        assert!(code.contains("settings\\\":true") || code.contains("\"settings\":true"), "{backend:?} carries the settings in its model");
    }
}

#[test]
fn avalonia_shares_one_settings_started_with_the_application() {
    let code = generated(Backend::Csharp, Ui::Avalonia);
    has(&code, "public sealed partial class Settings : ObservableObject\n");
    lacks(&code, "IRecord<Settings>");
    has(&code, "public static void Start()");
    has(&code, "KeptSettings.Taken(Described(), KeptSettings.Read(ProgramName))");
    has(&code, "[\"pageSize\"] = new(\"integer\", \"10\", \"200\", null),");
    has(&code, "[\"language\"] = new(\"language\", null, null, Locale.Offered),");
    has(&code, "public Settings Settings => Settings.Current;");
    has(&code, "            Themes.Start(AppThemes.Offered);\n            Settings.Start();\n");
    has(&code, "Choices=\"{x:Static c:Choice.LanguagesOffered}\"");
}
