//! Engines beside the screen in the generated application (DESIGN 5.12):
//! the program runs them beside Clarity.UI, on the desktop in-process and on
//! the web in the server, which the library in the browser asks over the
//! wire (`engine_*`, DESIGN 7.9 round B). C# gives each canvas that runs one
//! an instance of its own.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Settings, Ui, starter};

fn files(backend: Backend, ui: Ui) -> Vec<OutFile> {
    let mut settings: Settings = starter("Play", "play");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    generated_with(settings)
}

/// The web program of `backend`: a server for Clarity.UI in the browser.
fn web_files(backend: Backend) -> Vec<OutFile> {
    let mut settings: Settings = starter("Play", "play");
    settings.stack.backend = backend;
    settings.stack.ui = clarity_project::usual_ui(backend);
    settings.stack.targets = vec![clarity_project::Target::Web];
    generated_with(settings)
}

fn generated_with(settings: Settings) -> Vec<OutFile> {
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

/// The library in the browser asks the server for an engine's frames
/// (`engine_*`), so every web server runs the engine; the model in
/// files.json names it, as the desktop's model does.
#[test]
fn the_web_stacks_run_the_engine_in_the_server_and_carry_it_in_the_model() {
    for (backend, engine) in [(Backend::C, "src/screen_engines.c"), (Backend::Cpp, "src/screen_engines.cpp"), (Backend::Rust, "engines/screen.wasm"), (Backend::Go, "engines/screen.wasm")] {
        let files = web_files(backend);
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&engine), "{backend:?} answers its engines in the server: {paths:?}");
        let model = &files.iter().find(|f| f.path == "browser/clarity/files.json").expect("files.json").text;
        let model: serde_json::Value = serde_json::from_str(model).unwrap();
        assert_eq!(model["model"]["screenEngines"][0]["name"], "Screen", "{backend:?} carries the engine in its model");
    }
}

#[test]
fn csharp_gives_each_canvas_an_instance_of_its_engine() {
    let files = files(Backend::Csharp, Ui::Clarity);
    let engines = file(&files, "ScreenEngines.cs");
    assert!(engines.contains("public static readonly Func<IFrames> Screen = () =>\n        ScreenEngine.Load(\"Screen\", Read(\"screen.wasm\"), [\"log\"], true, new EngineLimits(1, 256));"), "{engines}");
    assert!(!engines.contains("InBrowser"), "{engines}");
    let model = file(&files, "Windows/PlayViewModel.cs");
    assert!(model.contains("case \"finished\":\n                Handle(() => OnFinished(PartValues.Read<long>(values, 0)), [tell, values]);"), "{model}");
    assert!(model.contains("    /// <summary>What custom part sky told.</summary>\n    public void Told_Sky(string tell, System.Text.Json.JsonElement[] values)"), "{model}");
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
    let files = files(Backend::Rust, Ui::Clarity);
    let all: String = files.iter().map(|f| f.text.as_str()).collect();
    assert!(all.contains("(\"Play\", \"finished\")") && all.contains("(\"Play\", \"lapped\")"), "the tells' handlers answer");
    assert!(all.contains("ui.hear(\"sky\", \"start\", vec![to_value(&2"), "a hear goes with the reply");
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
        let files = generated(backend, clarity_project::usual_ui(backend));
        let carried = files.iter().any(|f| f.text.contains("\"screenEngines\":[{\"name\":\"Sky\"") || f.text.contains("\\\"screenEngines\\\":[{\\\"name\\\":\\\"Sky\\\""));
        assert!(carried, "{backend:?} carries the compiled engine in its model");
    }
    let files = generated(Backend::Csharp, Ui::Clarity);
    assert!(file(&files, "ScreenEngines.cs").contains("ScreenEngine.Load(\"Sky\", Read(\"sky.wasm\"), [], false,"), "C# loads the compiled engine");
    assert!(files.iter().any(|f| f.path == "Engines/sky.wasm"));
}

/// Go on Clarity.UI carries the engine's file in the program and gives each
/// canvas that runs it an instance of its own through the runtime's
/// `engine.Screens`, which package ui asks by number.
#[test]
fn go_on_clarity_ui_gives_each_canvas_an_instance_of_its_engine() {
    let files = files(Backend::Go, Ui::Clarity);
    assert!(files.iter().any(|f| f.path == "engines/screen.wasm" && f.bytes.is_some()));
    let engines = file(&files, "engines.go");
    assert!(engines.contains("//go:embed engines/screen.wasm\nvar screenScreenWasm []byte\n"), "{engines}");
    assert!(engines.contains("var screenEngines = &engine.Screens{Make: map[string]func() (*engine.ScreenEngine, error){\n\t\"Screen\": func() (*engine.ScreenEngine, error) {\n\t\treturn engine.LoadScreen(\"Screen\", screenScreenWasm, []string{\"log\"}, true, engine.Limits{Seconds: 1, Megabytes: 256})\n\t},\n}}\n"), "{engines}");
    // No engine of the backend, so nothing imports sync.
    assert!(!engines.contains("\"sync\""), "{engines}");
    assert!(file(&files, "main.go").contains("ui.Run(ui.Desktop{Options: options, Files: files, Facts: facts, Engines: screenEngines})"));
    assert!(file(&files, "go.mod").contains("github.com/tetratelabs/wazero"), "the engine group's modules");
}

/// Rust on Clarity.UI carries the engine's file in the program and gives
/// each canvas that runs it an instance of its own through the runtime's
/// `ScreenEngine`, opened by `Program::engine_open` and kept by number.
#[test]
fn rust_on_clarity_ui_gives_each_canvas_an_instance_of_its_engine() {
    let files = files(Backend::Rust, Ui::Clarity);
    assert!(files.iter().any(|f| f.path == "engines/screen.wasm" && f.bytes.is_some()));
    let main = file(&files, "src/main.rs");
    assert!(
        main.contains("    fn engine_open(&self, name: &str) -> Option<std::result::Result<u64, String>> {\n        let made = match name {\n            \"Screen\" => clarity_runtime::engine::ScreenEngine::load(\"Screen\", include_bytes!(\"../engines/screen.wasm\"), &[\"log\"], true, clarity_runtime::engine::Limits { seconds: 1, megabytes: 256 }),\n            _ => return None,\n        };\n        Some(made.map(|engine| SCREEN_ENGINES.open(Box::new(engine))))\n    }\n")
            && main.contains("static SCREEN_ENGINES: clarity_runtime::screen::Instances = clarity_runtime::screen::Instances::new();\n")
            && main.contains("    fn engine_frame(&self, id: u64, ms: f64, input: Vec<clarity_runtime::screen::ScreenInput>) -> std::result::Result<clarity_runtime::screen::Drawn, String> {\n        SCREEN_ENGINES.frame(id, ms, &input)\n    }\n")
            && main.contains("    fn engine_close(&self, id: u64) {\n        SCREEN_ENGINES.close(id)\n    }\n"),
        "{main}"
    );
    // The runtime's engines run it, though the backend loads none.
    assert!(file(&files, "Cargo.toml").contains("\"engines\""));
    assert!(!files.iter().any(|f| f.path == "src/engines.rs"));
}

/// C++ on Clarity.UI carries the engine's file in the program and gives each
/// canvas that runs it an instance of its own through the runtime's
/// engine host (`clarity::ScreenInstances`), by its number.
#[test]
fn cpp_on_clarity_ui_gives_each_canvas_an_instance_of_its_engine() {
    let files = files(Backend::Cpp, Ui::Clarity);
    let engines = file(&files, "src/screen_engines.cpp");
    assert!(engines.contains("const unsigned char screen_wasm[] = {\n    0x00, 0x61, 0x73, 0x6d"), "{}", &engines[..300.min(engines.len())]);
    assert!(engines.contains("    if (name == \"Screen\") return clarity::ScreenEngine::load(\"Screen\", std::vector<uint8_t>(std::begin(screen_wasm), std::end(screen_wasm)), {\"log\"}, true, clarity::EngineLimits{1, 256});"), "{engines}");
    let main = file(&files, "src/main.cpp");
    assert!(main.contains("        clarity::ScreenInstances screen_instances(play::screen_engines::make);\n        options.screen_engines = [&screen_instances]"), "{main}");
    assert!(file(&files, "CMakeLists.txt").contains("clarity_engine"));
    assert!(files.iter().any(|f| f.path == "clarity-runtime/engine/screen.cpp"));
}

/// C on Clarity.UI carries the engine's file in the program and gives each
/// canvas that runs it an instance of its own through the runtime's
/// engine host (`clarity_screen_answer`), by its number.
#[test]
fn c_on_clarity_ui_gives_each_canvas_an_instance_of_its_engine() {
    let files = files(Backend::C, Ui::Clarity);
    let engines = file(&files, "src/screen_engines.c");
    assert!(engines.contains("static const uint8_t screen_wasm[] = {\n    0x00, 0x61, 0x73, 0x6d"), "{}", &engines[..300.min(engines.len())]);
    assert!(engines.contains("static const char* const screen_uses[] = {\"log\"};\n"), "{engines}");
    assert!(
        engines.contains("    if (strcmp(name, \"Screen\") == 0) return clarity_screen_engine_load(\"Screen\", screen_wasm, sizeof screen_wasm, 1, screen_uses, true, (ClarityEngineLimits){1, 256}, out);"),
        "{engines}"
    );
    let main = file(&files, "src/main.c");
    assert!(main.contains("#include \"screen_engines.h\"\n") && main.contains("    screen_engines_start();\n"), "{main}");
    assert!(main.contains("program_facts(), NULL, false, NULL, screen_engines_answer});"), "{main}");
    assert!(file(&files, "CMakeLists.txt").contains("clarity_engine"));
    assert!(files.iter().any(|f| f.path == "clarity-runtime/engine/screen.c"));
    assert!(files.iter().any(|f| f.path == "clarity-runtime/engine/engine.c"));
}
