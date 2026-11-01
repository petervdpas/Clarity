//! The sound part in the output (DESIGN 5.22): what code tells a sound goes
//! to the renderer (web stacks) or to Clarity.Avalonia's Sound (C#), and the
//! sound files travel with the program.

use clarity_project::Backend;
use clarity_spec::resources::{self, ResourceFile};

const TABLE: &str = "module dice

page Table: window
    sound name rollSound source \"roll.wav\" volume 80
    sound name tune
    button \"Roll\" name roll at 1,1 span 2 on click: rollClicked

code Table
    handler rollClicked
        rollSound.play()
        tune.melody(\"C4 E4 G4\", 120)
        tune.tone(440, 0.5)
";

fn project() -> clarity_spec::Project {
    let mut p = clarity_spec::load_sources([("dice.clarity", TABLE)]);
    resources::attach(&mut p, vec![ResourceFile { name: "roll.wav".into(), bytes: b"RIFF0000WAVE".to_vec() }]);
    assert!(!p.has_errors(), "{:?}", p.diagnostics);
    p
}

fn settings(backend: Backend) -> clarity_project::Settings {
    let stack = match backend {
        Backend::Csharp => "csharp avalonia desktop",
        Backend::Rust => "rust web desktop",
        Backend::Go => "go web desktop",
        Backend::C => "c web desktop",
        Backend::Cpp => "cpp web desktop",
    };
    clarity_project::parse(&format!("project Dice\n    stack {stack}\n    datasource main sqlite \"data/dice.db\"\n")).unwrap()
}

fn code(backend: Backend) -> String {
    let files = clarity_gen::generate(&project(), &settings(backend)).unwrap_or_else(|e| panic!("{e:#?}"));
    files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

#[test]
fn the_web_stacks_tell_the_renderer() {
    assert!(code(Backend::Rust).contains("ui.hear(\"rollSound\", \"play\", vec![]);"));
    assert!(code(Backend::Go).contains("\"rollSound\", \"play\""));
    assert!(code(Backend::C).contains("\"rollSound\", \"play\""));
    assert!(code(Backend::Cpp).contains("\"rollSound\", \"play\""));
}

#[test]
fn avalonia_places_a_sound_and_tells_it() {
    let code = code(Backend::Csharp);
    assert!(code.contains("<c:Sound State=\"{Binding RollSound}\" Source=\"roll.wav\" Volume=\"80\" />"), "{code}");
    assert!(code.contains("RollSound.Hear(\"play\");"), "{code}");
    assert!(code.contains("Tune.Hear(\"melody\", \"C4 E4 G4\", 120L);"), "{code}");
}

#[test]
fn the_one_file_page_carries_the_sounds() {
    let page = clarity_gen::web::single_page(&project());
    assert!(page.contains("window.clarityResources = {"), "the page carries its resources");
    assert!(page.contains("\"roll.wav\":\"data:audio/wav;base64,"));
}
