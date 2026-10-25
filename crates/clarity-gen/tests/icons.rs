//! Icons (U1b) on every stack: the web stacks' model carries the drawings
//! of the icons the project uses; Avalonia gets an Icons class with their
//! geometries, drawn on buttons, menu items and the icon part.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture: a button, a menu item and an icon part with icons.
const MUSIC: &str = include_str!("fixtures/asks.clarity");

fn files(backend: Backend, ui: Ui) -> Vec<OutFile> {
    let project = load_keyed([("music.clarity", MUSIC)], &[], "en");
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let mut settings: Settings = starter("Music", "music");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn all(files: &[OutFile]) -> String {
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

fn has(text: &str, expected: &str) {
    assert!(text.contains(expected), "missing {expected:?}");
}

#[test]
fn the_web_stacks_model_carries_only_the_icons_the_project_uses() {
    let check = clarity_spec::icons::path("check").unwrap();
    let unused = clarity_spec::icons::path("zap").unwrap();
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp] {
        // C keeps the model as pieces of 2000 bytes, which may cut a path; joined, it reads whole.
        let code = all(&files(backend, Ui::Web)).replace("\",\n    \"", "");
        assert!(code.contains(check), "{backend:?} carries the check icon");
        assert!(!code.contains(unused), "{backend:?} carries no icon it does not use");
    }
}

#[test]
fn an_application_with_icons_carries_lucides_license() {
    for (backend, ui) in [(Backend::Rust, Ui::Web), (Backend::Csharp, Ui::Avalonia)] {
        let files = files(backend, ui);
        let notices = files.iter().find(|f| f.path == "THIRD_PARTY_NOTICES.md").expect("the notices are written");
        assert!(notices.text.contains("ISC License") && notices.text.contains("Lucide"), "{}", notices.text);
    }
}

#[test]
fn avalonia_draws_its_icons_from_a_class_of_geometries() {
    let code = all(&files(Backend::Csharp, Ui::Avalonia));
    has(&code, "public static class Icons");
    has(&code, "public static Geometry Check { get; } = Geometry.Parse(\"");
    has(&code, "public static Geometry UserCheck { get; }");
    has(&code, "<Path Classes=\"icon\" Data=\"{x:Static local:Icons.Check}\" />");
    has(&code, "<MenuItem.Icon><Viewbox Width=\"16\" Height=\"16\">");
    has(&code, "<Border x:Name=\"friendsIcon\" Grid.Column=\"5\" Grid.Row=\"6\" Classes=\"icon\"");
    has(&code, "<Style Selector=\"Path.icon\">");
}
