//! The slider and the spin box of U1c on every stack: the web stacks draw
//! them from the model; Avalonia writes Clarity's SliderInput and a
//! NumberInput with a step, each with its range: as written, else the
//! field's own.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;

/// The build tests' fixture, whose Albums window has a slider and a spin box.
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
fn the_web_stacks_carry_the_slider_and_the_spin_box_in_their_model() {
    for backend in [Backend::Rust, Backend::Go] {
        let code = generated(backend, Ui::Web);
        has(&code, "\"kind\":\"slider\"");
        has(&code, "\"kind\":\"spinbox\"");
        has(&code, "\"step\":\"0.5\"");
    }
}

#[test]
fn avalonia_writes_the_slider_and_the_spin_box_with_their_range() {
    let code = generated(Backend::Csharp, Ui::Avalonia);
    has(&code, "<c:SliderInput x:Name=\"ratingSlider\"");
    has(&code, "Min=\"0\" Max=\"5\" Step=\"0.5\" Whole=\"False\" Value=\"{Binding Album.Rating}\"");
    has(&code, "ratingSlider.ValueChanged += (_, _) => Vm.FavouriteChangedCommand.Execute(null);");
    has(&code, "<c:NumberInput x:Name=\"copiesSpin\"");
    has(&code, "Min=\"0\" Max=\"99\" Step=\"1\"");
}
