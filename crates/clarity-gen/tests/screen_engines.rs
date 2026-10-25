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
    assert!(engines.contains("public static readonly Func<IFrames> Screen = () => ScreenEngine.Load(\"Screen\", Read(\"screen.wasm\"), [], true, new EngineLimits(1, 256));"), "{engines}");
    let view = file(&files, "Windows/PlayView.axaml");
    assert!(view.contains("Grid.RowSpan=\"10\" Runs=\"{x:Static local:ScreenEngines.Screen}\" UnitsWidth=\"160\" UnitsHeight=\"100\""), "{view}");
    let csproj = file(&files, "Play.csproj");
    assert!(csproj.contains("Clarity.Runtime.Engines") && csproj.contains("Engines/*.wasm"), "{csproj}");
    assert!(files.iter().any(|f| f.path == "Engines/screen.wasm"));
    // The backend loads no engine: there is none to check at the start.
    assert!(!files.iter().any(|f| f.path == "Engines.cs"));
}
