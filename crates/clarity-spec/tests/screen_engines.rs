//! Engines beside the screen (DESIGN 5.12, N5): an engine declared `beside
//! the screen` runs where its canvas is drawn (`canvas ... runs Screen`), one
//! instance per canvas, its frames in the renderer
//! (conformance/screen-engines.json). The backend never runs it.

use clarity_spec::engines::{self, EngineFile};
use clarity_spec::parser::parse;
use clarity_spec::{Preview, Project, Severity, load_sources};

const SCREEN_ENGINE: &[u8] = include_bytes!("../../../conformance/engines/screen.wasm");
const TEST_ENGINE: &[u8] = include_bytes!("../../../conformance/engines/test.wasm");

const PLAY: &str = "module play

engine Screen from \"screen.wasm\"
    beside the screen
    on open
    limit 1 second

page Play: window
    canvas name sky at 1,1 span 12 rows 10 units 160 x 100 runs Screen
";

fn files() -> Vec<EngineFile> {
    vec![EngineFile { name: "screen.wasm".into(), bytes: SCREEN_ENGINE.to_vec() }, EngineFile { name: "test.wasm".into(), bytes: TEST_ENGINE.to_vec() }]
}

fn project(source: &str) -> Project {
    let mut project = load_sources([("play.clarity", source)]);
    engines::attach(&mut project, files());
    project
}

fn messages(source: &str, severity: Severity) -> Vec<String> {
    project(source).diagnostics.into_iter().filter(|d| d.severity == severity).map(|d| d.message).collect()
}

fn assert_has(messages: &[String], needle: &str) {
    assert!(messages.iter().any(|m| m.contains(needle)), "missing {needle:?} in {messages:#?}");
}

#[test]
fn an_engine_beside_the_screen_runs_a_canvas() {
    let (file, diags) = parse(PLAY, "play.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let screen = &file.engines[0];
    assert!(screen.beside);
    assert!(screen.opens);
    assert_eq!(file.pages[0].widgets[0].props.runs.as_deref(), Some("Screen"));
    assert!(messages(PLAY, Severity::Error).is_empty(), "{:#?}", messages(PLAY, Severity::Error));
    // A canvas an engine draws needs no draw handler.
    assert!(messages(PLAY, Severity::Warning).is_empty(), "{:#?}", messages(PLAY, Severity::Warning));
    // The line is written back as it was read.
    let line = clarity_spec::widgets::widget_line(&file.pages[0].widgets[0].props);
    assert!(line.ends_with("units 160 x 100 runs Screen"), "{line}");
}

#[test]
fn a_canvas_runs_an_engine_beside_the_screen() {
    let errors = |source: &str| messages(source, Severity::Error);
    assert_has(&errors(&PLAY.replace("runs Screen", "runs Nowhere")), "canvas sky runs Nowhere, which is no engine");
    let backend = format!("{PLAY}\nengine Pricing from \"test.wasm\"\n    function add(a: integer, b: integer) -> integer\n");
    assert_has(&errors(&backend.replace("runs Screen", "runs Pricing")), "canvas sky runs Pricing, which runs on the backend; a canvas runs an engine declared 'beside the screen'");
    let drawn = format!("{PLAY}    on draw: drawSky\n\ncode Play\n    handler drawSky\n        sky.clear(\"background\")\n").replace("runs Screen\n    on draw", "runs Screen on draw");
    assert_has(&errors(&drawn), "canvas sky is drawn by engine Screen, so it has no draw handler");
    assert_has(&errors(&PLAY.replace("canvas name sky", "textbox name sky").replace(" units 160 x 100", "")), "'runs' does not apply to a textbox");
}

#[test]
fn an_engine_beside_the_screen_is_not_called_from_code() {
    let errors = |lines: &str| messages(&PLAY.replace("    limit 1 second\n", &format!("    limit 1 second\n{lines}")), Severity::Error);
    assert_has(&errors("    function add(a: integer, b: integer) -> integer\n"), "engine Screen runs beside the screen, so code does not call it; it has no functions");
    assert_has(&errors("    state per window\n"), "engine Screen runs beside the screen, one instance per canvas, so it has no state line");
    assert_has(&errors("    beside the screen\n"), "an engine has one 'beside the screen' line");
    let written = "module play\n\nengine Screen\n    beside the screen\n";
    assert_has(&messages(written, Severity::Error), "engine Screen beside the screen is brought along for now: give it its file, as in 'engine Screen from \"screen.wasm\"'");
}

#[test]
fn an_engine_beside_the_screen_exports_its_frame_loop() {
    let lacking = PLAY.replace("screen.wasm", "test.wasm").replace("    on open\n", "");
    assert_has(&messages(&lacking, Severity::Error), "engine Screen: 'test.wasm' lacks clarity_tick, which an engine beside the screen needs");
    // An engine beside the screen no canvas runs is a warning.
    let idle = PLAY.replace(" runs Screen", "");
    assert_has(&messages(&idle, Severity::Warning), "engine Screen runs beside the screen, but no canvas runs it");
}

/// The renderer gets the file of every engine beside the screen with the
/// model, since it runs them itself; the backend's engines stay on the backend.
#[test]
fn the_model_carries_the_engines_beside_the_screen() {
    let source = format!("{PLAY}\nengine Pricing from \"test.wasm\"\n    function add(a: integer, b: integer) -> integer\n");
    let preview = Preview::new(project(&source));
    assert_eq!(preview.screen_engines.len(), 1);
    let screen = &preview.screen_engines[0];
    assert_eq!((screen.name.as_str(), screen.opens, screen.seconds, screen.megabytes), ("Screen", true, 1, engines::DEFAULT_MEGABYTES));
    assert!(screen.uses.is_empty());
    use base64::Engine as _;
    assert_eq!(base64::engine::general_purpose::STANDARD.decode(&screen.wasm).unwrap(), SCREEN_ENGINE);
}

/// The canvas says why its engine stopped in Clarity's words, which a
/// project with an engine beside the screen translates.
#[test]
fn why_an_engine_stopped_is_a_clarity_word() {
    assert_eq!(clarity_spec::translations::SCREEN_ENGINE_KEYS, [("screenEngineFailed", "The engine stopped: {problem}")]);
    let keys = |source: &str| clarity_spec::translations::keys(&project(source)).into_iter().map(|k| k.key).collect::<Vec<_>>();
    assert!(keys(PLAY).contains(&"screenEngineFailed".to_string()));
    assert!(!keys("module play\n").contains(&"screenEngineFailed".to_string()));
}
