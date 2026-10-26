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
    uses log
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
    assert_has(&messages(written, Severity::Error), "engine Screen beside the screen has no function clarity_tick(ms: integer), which its frames call");
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
    assert_eq!(screen.uses, ["log"]);
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

/// What an engine beside the screen tells its window, and what a handler
/// tells it (DESIGN 5.12): `tells` runs the handler wired on the canvas with
/// the tell's values; `sky.start(2)` reaches the engine's export `start`.
const GAME: &str = "module play

engine Screen from \"screen.wasm\"
    beside the screen
    uses log
    on open
    limit 1 second
    tells finished(time: integer)
    tells lap(count: integer)
    hears start(level: integer)

page Play: window
    data best: integer
    canvas name sky at 1,1 span 12 rows 10 units 160 x 100 runs Screen on finished: gameOver
    button \"Start\" name go at 1,11 span 2 on click: startGame

code Play
    handler gameOver(time: integer)
        best = time

    handler startGame
        sky.start(2)
";

#[test]
fn an_engine_beside_the_screen_tells_and_hears() {
    let (file, diags) = parse(GAME, "play.clarity");
    assert!(diags.is_empty(), "{diags:#?}");
    let screen = &file.engines[0];
    assert_eq!(screen.tells.iter().map(|t| t.name.name.as_str()).collect::<Vec<_>>(), ["finished", "lap"]);
    assert_eq!(screen.hears.iter().map(|h| h.name.name.as_str()).collect::<Vec<_>>(), ["start"]);
    let sky = &file.pages[0].widgets[0].props;
    assert_eq!((sky.tells[0].tell.as_str(), sky.tells[0].handler.as_str()), ("finished", "gameOver"));
    assert!(messages(GAME, Severity::Error).is_empty(), "{:#?}", messages(GAME, Severity::Error));
    assert!(messages(GAME, Severity::Warning).is_empty(), "{:#?}", messages(GAME, Severity::Warning));
    let line = clarity_spec::widgets::widget_line(sky);
    assert!(line.ends_with("runs Screen on finished: gameOver"), "{line}");
}

#[test]
fn tells_and_hears_fit_the_engine() {
    let errors = |source: String| messages(&source, Severity::Error);
    assert_has(&errors(GAME.replace("on finished: gameOver", "on finishes: gameOver")), "engine Screen tells no 'finishes'");
    assert_has(&errors(GAME.replace("gameOver(time: integer)", "gameOver(time: text)").replace("best = time", "best = 1")), "tell finished of engine Screen gives (time: integer); handler gameOver takes (time: text)");
    assert_has(&errors(GAME.replace("sky.start(2)", "sky.begin(2)")), "engine Screen hears no 'begin'");
    assert_has(&errors(GAME.replace("sky.start(2)", "sky.start(\"two\")")), "value 'level' of sky.start is integer, not text");
    assert_has(&errors(GAME.replace("sky.start(2)", "sky.start()")), "sky.start takes 1 value, not 0");
    assert_has(&errors(GAME.replace("tells lap(count: integer)", "tells lap(at: list<integer>)")), "a value of a tell or hear is text, a number, a date, a duration, an enum or bool");
    assert_has(&errors(GAME.replace("    hears start(level: integer)\n", "    hears start(level: integer)\n    hears start(level: integer)\n")), "engine Screen has two tells or hears named 'start'");
    // Its file has a function for every hear.
    assert_has(&errors(GAME.replace("hears start(level: integer)", "hears stop")), "engine Screen: 'screen.wasm' has no function stop, which its hear stop needs");
    // Only an engine beside the screen tells and hears.
    let backend = "module play\n\nengine Pricing from \"test.wasm\"\n    tells done\n    function add(a: integer, b: integer) -> integer\n";
    assert_has(&errors(backend.to_string()), "engine Pricing tells and hears only beside the screen: code calls it and has its answer");
}

/// The designer sets `runs` and wires the engine's tells through the part's
/// properties; the line it writes back reads as it was.
#[test]
fn the_designer_writes_a_canvas_that_runs_an_engine() {
    let (file, diagnostics) = parse(GAME, "play.clarity");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let sky = &file.pages[0].widgets[0].props;
    assert_eq!(
        clarity_spec::widgets::widget_line(sky),
        "canvas name sky at 1,1 span 12 rows 10 units 160 x 100 runs Screen on finished: gameOver"
    );
}

/// An engine beside the screen written in Clarity (DESIGN 5.12, N5b): its
/// frame loop is functions of the interface's names, what it draws is a
/// list of records, and `tell` puts a tell in the call's answer.
const WRITTEN: &str = "module play

record Shape
    op: text
    x: integer
    y: integer
    r: integer
    text: text
    color: text, default \"accent\"
    size: integer, default 8

engine Sky
    beside the screen
    tells finished(time: integer)
    hears start(level: integer)
    keep elapsed: integer = 0
    keep stage: integer = 1

    function clarity_tick(ms: integer)
        elapsed = elapsed + ms
        if elapsed > 10000
            tell finished(elapsed)

    function clarity_draw() -> list<Shape>
        let clear = new Shape
        clear.op = \"clear\"
        clear.color = \"background\"
        let dot = new Shape
        dot.op = \"circle\"
        dot.x = stage * 10
        dot.y = 50
        dot.r = 5
        return [clear, dot]

    function start(level: integer)
        elapsed = 0
        stage = level

page Play: window
    data best: integer
    canvas name sky at 1,1 span 12 rows 10 units 160 x 100 runs Sky on finished: gameOver

code Play
    handler gameOver(time: integer)
        best = time
";

#[test]
fn an_engine_beside_the_screen_is_written_in_clarity() {
    let project = load_sources([("play.clarity", WRITTEN)]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn a_written_engine_beside_the_screen_has_its_frame_loop() {
    let errors = |source: String| load_sources([("play.clarity", source.as_str())]).diagnostics.into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect::<Vec<_>>();
    assert_has(&errors(WRITTEN.replace("function clarity_draw()", "function draw()")), "engine Sky beside the screen has no function clarity_draw() -> list<Record>, which its frames call");
    assert_has(&errors(WRITTEN.replace("function clarity_tick(ms: integer)", "function clarity_tick(ms: text)")), "engine Sky beside the screen has no function clarity_tick(ms: integer), which its frames call");
    assert_has(&errors(WRITTEN.replace("    function start(level: integer)\n        elapsed = 0\n        stage = level\n", "")), "engine Sky hears start, so it has a function start(level: integer)");
    assert_has(&errors(WRITTEN.replace("tell finished(elapsed)", "tell finishes(elapsed)")), "engine Sky tells no 'finishes'");
    assert_has(&errors(WRITTEN.replace("tell finished(elapsed)", "tell finished(\"late\")")), "tell finished gives (time: integer), not text");
    assert_has(&errors(WRITTEN.replace("        best = time\n", "        best = time\n        tell finished(time)\n")), "'tell' is for an engine beside the screen");
    assert_has(&errors(WRITTEN.replace("    beside the screen\n", "    beside the screen\n    on open\n")), "engine Sky is written in Clarity: what it keeps starts as written, so it has no 'on open'");
}
