//! Engines beside the screen in the generated application (DESIGN 5.12):
//! the renderer runs them, so the backend never loads one. The web stacks
//! carry the engine's file in the model the renderer reads; Avalonia
//! embeds it and gives each canvas that runs it an instance of its own.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};

fn files(backend: Backend, ui: Ui) -> Vec<OutFile> {
    let mut settings: Settings = starter("Play", "play");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    let mut project = clarity_spec::load_in([("play.clarity", include_str!("../../../conformance/engines/play.clarity"))], [], &clarity_project::source_refs(&settings));
    let bytes = include_bytes!("../../../conformance/engines/screen.wasm").to_vec();
    clarity_spec::engines::attach(&mut project, vec![clarity_spec::engines::EngineFile { name: "screen.wasm".into(), bytes }]);
    let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn file<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

#[test]
fn the_web_stacks_carry_the_engine_in_the_model_and_not_on_the_backend() {
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp] {
        let files = files(backend, Ui::Web);
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(!paths.iter().any(|p| p.contains("engines")), "{backend:?} has no backend engine: {paths:?}");
        // The model is a file beside the renderer, or text in the C and C++ sources.
        let carried = files.iter().any(|f| f.text.contains("\"screenEngines\":[{\"name\":\"Screen\"") || f.text.contains("\\\"screenEngines\\\":[{\\\"name\\\":\\\"Screen\\\""));
        assert!(carried, "{backend:?} carries the engine in its model");
    }
}

#[test]
fn avalonia_gives_each_canvas_an_instance_of_its_engine() {
    let files = files(Backend::Csharp, Ui::Avalonia);
    let engines = file(&files, "ScreenEngines.cs");
    assert!(engines.contains("public static readonly Func<IFrames> Screen = () => ScreenEngine.Load(\"Screen\", Read(\"screen.wasm\"), [\"log\"], true, new EngineLimits(1, 256));"), "{engines}");
    let view = file(&files, "Windows/PlayView.axaml");
    assert!(view.contains("Grid.RowSpan=\"10\" Runs=\"{x:Static local:ScreenEngines.Screen}\" State=\"{Binding Sky}\" UnitsWidth=\"160\" UnitsHeight=\"100\""), "{view}");
    // What the engine tells runs its handler with its values; a handler tells it through the canvas's state.
    let behind = file(&files, "Windows/PlayView.axaml.cs");
    assert!(behind.contains("sky.Told += (tell, values) => Vm.Told_Sky(tell, values);"), "{behind}");
    let model = file(&files, "Windows/PlayViewModel.cs");
    assert!(model.contains("case \"finished\":\n                Handle(() => OnFinished(DotNetPart.Read<long>(values, 0)));"), "{model}");
    assert!(model.contains("sky.Hear(\"start\", 2L);") || model.contains("Sky.Hear(\"start\", 2L);"), "{model}");
    let csproj = file(&files, "Play.csproj");
    assert!(csproj.contains("Clarity.Runtime.Engines") && csproj.contains("Engines/*.wasm"), "{csproj}");
    assert!(files.iter().any(|f| f.path == "Engines/screen.wasm"));
    // The backend loads no engine: there is none to check at the start.
    assert!(!files.iter().any(|f| f.path == "Engines.cs"));
}

/// On the web stacks a tell runs its handler through the backend's answers,
/// with the tell's values, and a hear comes back with the handler's reply.
#[test]
fn the_web_stacks_answer_tells_and_send_hears() {
    let files = files(Backend::Rust, Ui::Web);
    let all: String = files.iter().map(|f| f.text.as_str()).collect();
    assert!(all.contains("(\"Play\", \"finished\")") && all.contains("(\"Play\", \"lapped\")"), "the tells' handlers answer");
    assert!(all.contains("ui.hear(\"sky\", \"start\", vec![to_value(&2"), "a hear goes with the reply");
}

/// The log of an engine beside the screen goes into the application's log
/// through the shell; the Rust stack's shell is generated and writes the
/// line to the standard error, where the backend's engines write.
#[test]
fn the_rust_shell_logs_the_engines_lines() {
    let files = files(Backend::Rust, Ui::Web);
    // The shell's commands are in the desktop module.
    let main = file(&files, "src/desktop.rs");
    assert!(main.contains("fn log_line(line: String) {\n    eprintln!(\"{line}\");\n}"), "{main}");
    assert!(main.contains("            log_line,\n"), "{main}");
}

/// An engine beside the screen written in Clarity (DESIGN 5.12, N5b) is
/// compiled when the project is generated, and every stack carries it as it
/// carries one brought along.
#[test]
fn a_clarity_engine_beside_the_screen_is_carried_compiled() {
    let source = format!(
        "{}\npage Sky: window\n    canvas name sky at 1,1 span 12 rows 10 units 160 x 100 runs Sky\n",
        include_str!("../../../conformance/engines/sky.clarity")
    );
    let generated = |backend: Backend, ui: Ui| {
        let mut settings: clarity_project::Settings = clarity_project::starter("Sky", "sky");
        settings.stack.backend = backend;
        settings.stack.ui = ui;
        let project = clarity_spec::load_in([("sky.clarity", source.as_str())], [], &clarity_project::source_refs(&settings));
        let errors: Vec<&str> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.as_str()).collect();
        assert!(errors.is_empty(), "{errors:#?}");
        generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
    };
    for backend in [Backend::Rust, Backend::Go, Backend::C, Backend::Cpp] {
        let files = generated(backend, Ui::Web);
        let carried = files.iter().any(|f| f.text.contains("\"screenEngines\":[{\"name\":\"Sky\"") || f.text.contains("\\\"screenEngines\\\":[{\\\"name\\\":\\\"Sky\\\""));
        assert!(carried, "{backend:?} carries the compiled engine in its model");
    }
    let files = generated(Backend::Csharp, Ui::Avalonia);
    assert!(file(&files, "ScreenEngines.cs").contains("ScreenEngine.Load(\"Sky\", Read(\"sky.wasm\"), [], false,"), "Avalonia loads the compiled engine");
    assert!(files.iter().any(|f| f.path == "Engines/sky.wasm"));
}
