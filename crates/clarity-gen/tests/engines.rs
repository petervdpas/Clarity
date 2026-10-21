//! Engines (DESIGN 5.11) in every stack's output: each engine is loaded
//! with the services it uses (`uses clock, random, log`) and the limits its
//! declaration gives (`limit 2 seconds, 64 MB`).

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui, starter};

fn files(backend: Backend, ui: Ui) -> Vec<OutFile> {
    let mut project = clarity_spec::load_sources([("shop.clarity", include_str!("../../../conformance/engines/shop.clarity"))]);
    let bytes = include_bytes!("../../../conformance/engines/test.wasm").to_vec();
    clarity_spec::engines::attach(&mut project, vec![clarity_spec::engines::EngineFile { name: "test.wasm".into(), bytes }]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = starter("Shop", "shop");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn has(files: &[OutFile], path: &str, expected: &str) {
    let text = &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path}")).text;
    assert!(text.contains(expected), "expected\n{expected}\nin {path}:\n{text}");
}

#[test]
fn every_stack_loads_an_engine_with_its_services_and_limits() {
    has(&files(Backend::Rust, Ui::Web), "src/engines.rs", "uses: &[\"clock\", \"random\", \"log\", \"language\"], limits: Limits { seconds: 2, megabytes: 64 }, state: State::None, opens: false }");
    has(&files(Backend::Go, Ui::Web), "engines.go", "Uses: []string{\"clock\", \"random\", \"log\", \"language\"}, Limits: engine.Limits{Seconds: 2, Megabytes: 64}, State: engine.NoState, Opens: false}");
    has(&files(Backend::Csharp, Ui::Avalonia), "Engines.cs", "[\"clock\", \"random\", \"log\", \"language\"], new EngineLimits(2, 64), EngineState.None, false)");
    has(&files(Backend::C, Ui::Web), "src/engines.c", "4, pricing_uses, (ClarityEngineLimits){2, 64}, CLARITY_ENGINE_NO_STATE, false}");
    has(&files(Backend::C, Ui::Web), "src/engines.c", "static const char* const pricing_uses[] = {\"clock\", \"random\", \"log\", \"language\"};");
    has(&files(Backend::Cpp, Ui::Web), "src/engines.cpp", "{\"clock\", \"random\", \"log\", \"language\"}, clarity::EngineLimits{2, 64}, clarity::EngineState::none, false}");
}

#[test]
fn every_stack_loads_an_engine_with_its_state_and_on_open() {
    has(&files(Backend::Rust, Ui::Web), "src/engines.rs", "Declaration { functions: &[\"count\"], uses: &[\"clock\", \"random\", \"log\", \"language\"], limits: Limits { seconds: 5, megabytes: 256 }, state: State::App, opens: true }");
    has(&files(Backend::Go, Ui::Web), "engines.go", "engine.Declaration{Functions: []string{\"count\"}, Uses: []string{\"clock\", \"random\", \"log\", \"language\"}, Limits: engine.Limits{Seconds: 5, Megabytes: 256}, State: engine.PerApp, Opens: true}");
    has(&files(Backend::Csharp, Ui::Avalonia), "Engines.cs", "new EngineDeclaration([\"count\"], [\"clock\", \"random\", \"log\", \"language\"], new EngineLimits(5, 256), EngineState.PerApp, true)");
    has(&files(Backend::C, Ui::Web), "src/engines.c", "(ClarityEngineDeclaration){1, counter_functions, 4, counter_uses, (ClarityEngineLimits){5, 256}, CLARITY_ENGINE_PER_APP, true}");
    has(&files(Backend::Cpp, Ui::Web), "src/engines.cpp", "{{\"count\"}, {\"clock\", \"random\", \"log\", \"language\"}, clarity::EngineLimits{5, 256}, clarity::EngineState::per_app, true}");
}

/// An engine with state per window is called in the handler's own window,
/// and drops a window's instance when the window closes.
#[test]
fn rust_calls_an_engine_with_state_per_window_in_its_window() {
    let files = files(Backend::Rust, Ui::Web);
    has(&files, "src/modules/shop.rs", ".call_in(ui.instance(), \"count\", vec![])");
    has(&files, "src/engines.rs", "pub fn close_window(instance: u64)");
    has(&files, "src/engines.rs", "if let Some(Ok(engine)) = BOARD.get() {\n        engine.close_window(instance);\n    }");
    has(&files, "src/main.rs", "engines::close_window(instance);");
}

#[test]
fn go_calls_an_engine_with_state_per_window_in_its_window() {
    let files = files(Backend::Go, Ui::Web);
    has(&files, "engines.go", "func boardCount(window uint64) (int64, error) {");
    has(&files, "engines.go", "return engine.As[int64](e.CallIn(window, \"count\"))");
    has(&files, "shop_module.go", "boardCount(ui.Instance())");
}

#[test]
fn csharp_calls_an_engine_with_state_per_window_in_its_window() {
    let files = files(Backend::Csharp, Ui::Avalonia);
    let view_model = &files.iter().find(|f| f.path == "Windows/MainViewModel.cs").expect("the window's ViewModel").text;
    assert!(view_model.contains("Engines.Board.CallIn<long>(Instance, \"count\")"), "{view_model}");
}

#[test]
fn c_and_cpp_call_an_engine_with_state_per_window_in_its_window() {
    let c = files(Backend::C, Ui::Web);
    has(&c, "src/engines.c", "bool board_count(uint64_t window, int64_t* result) {");
    has(&c, "src/engines.c", "if (!clarity_engine_call_in(board_engine, window, \"count\", args, &value)) return false;");
    has(&c, "src/modules/shop.c", "board_count(clarity_ui_instance(ui), &");
    let cpp = files(Backend::Cpp, Ui::Web);
    has(&cpp, "src/modules/shop.cpp", "engines::board().call_in_as<int64_t>(ui.instance(), \"count\", nlohmann::json::array())");
}
