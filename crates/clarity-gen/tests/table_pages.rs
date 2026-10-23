//! A table shown a page at a time (U1c): the web stacks carry `page` in
//! their model for the renderer; Avalonia writes it on Clarity's table with
//! the bar's own words.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, whose friends table shows 20 at a time.
const MUSIC: &str = include_str!("fixtures/asks.clarity");

fn generated(backend: Backend, ui: Ui) -> String {
    let project = load_keyed([("music.clarity", MUSIC)], &[], "en");
    let mut settings: Settings = starter("Music", "music");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let files: Vec<OutFile> = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| format!("// {}\n{}\n", f.path, f.text)).collect()
}

#[test]
fn avalonia_pages_its_table_with_the_bars_own_words() {
    let csharp = generated(Backend::Csharp, Ui::Avalonia);
    assert!(csharp.contains("PageSize=\"20\" FirstText=\"{x:Static local:Texts.FirstPage}\""), "no page size");
}

#[test]
fn the_web_stacks_carry_the_page_in_their_model() {
    let rust = generated(Backend::Rust, Ui::Web);
    assert!(rust.contains("\"page\":20"), "no page in the model");
}
