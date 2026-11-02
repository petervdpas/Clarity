//! Charts of U1c: the web stacks carry `chart` in their model for the
//! renderer, which groups the rows itself; on C# Clarity.UI draws it
//! from the window's view model.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, with charts by an enum, a number and a reference.
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
fn csharp_offers_a_chart_the_records_it_groups_by() {
    let csharp = generated(Backend::Csharp, Ui::Clarity);
    has(&csharp, "FriendChoices = Choice.Of(Db.All<Friend>()");
}

#[test]
fn the_web_stacks_carry_the_charts_in_their_model() {
    let rust = generated(Backend::Rust, Ui::Webview);
    has(&rust, "\"chart\":\"bar\"");
    has(&rust, "\"by\":\"format\"");
    has(&rust, "\"measure\":{\"kind\":\"sum\",\"field\":\"rating\"}");
}
