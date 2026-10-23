//! Charts of U1c: the web stacks carry `chart` in their model for the
//! renderer, which groups the rows itself; Avalonia writes Clarity's
//! ChartView with what to group by and what to count.

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
fn avalonia_writes_a_chart_view_that_groups_its_rows() {
    let csharp = generated(Backend::Csharp, Ui::Avalonia);
    has(&csharp, "<c:ChartView x:Name=\"formatChart\"");
    has(&csharp, "Shape=\"Bar\" By=\"Format\" ByKind=\"Enum\"");
    has(&csharp, "Number=\"Count\"");
    has(&csharp, "Shape=\"Line\" By=\"Copies\" ByKind=\"Number\"");
    has(&csharp, "Number=\"Sum\" Of=\"Rating\"");
    has(&csharp, "ByChoices=\"{Binding FriendChoices}\"");
    has(&csharp, "Number=\"Average\" Of=\"Copies\"");
    has(&csharp, "FriendChoices = Choice.Of(Db.All<Friend>()");
}

#[test]
fn the_web_stacks_carry_the_charts_in_their_model() {
    let rust = generated(Backend::Rust, Ui::Web);
    has(&rust, "\"chart\":\"bar\"");
    has(&rust, "\"by\":\"format\"");
    has(&rust, "\"measure\":{\"kind\":\"sum\",\"field\":\"rating\"}");
}
