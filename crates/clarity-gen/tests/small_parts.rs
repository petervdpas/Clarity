//! The small parts of U1b on every stack: the web stacks draw them from the
//! model; Avalonia writes a separator line, a HyperlinkButton, a badge, a
//! ProgressBar and a running bar for busy.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, whose Albums window has each small part.
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
fn the_web_stacks_carry_the_small_parts_in_their_model() {
    for backend in [Backend::Rust, Backend::Go] {
        let code = generated(backend, Ui::Web);
        for kind in ["separator", "link", "badge", "progress", "busy"] {
            has(&code, &format!("\"kind\":\"{kind}\""));
        }
        has(&code, "\"to\":\"https://lucide.dev\"");
        has(&code, "\"max\":\"10\"");
    }
}

#[test]
fn avalonia_writes_each_small_part_in_its_idiom() {
    let code = generated(Backend::Csharp, Ui::Avalonia);
    has(&code, "Classes=\"separator\"");
    has(&code, "<HyperlinkButton x:Name=\"lucideLink\"");
    has(&code, "NavigateUri=\"https://lucide.dev\"");
    has(&code, "NavigateUri=\"{Binding Album.Site, Converter={x:Static local:Shown.Address}}\"");
    has(&code, "Classes=\"badge\"");
    has(&code, "<ProgressBar x:Name=\"copiesBar\"");
    has(&code, "Maximum=\"10\" Value=\"{Binding Album.Copies}\"");
    has(&code, "<ProgressBar IsIndeterminate=\"True\" />");
    has(&code, "public static readonly IValueConverter Address");
}
