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
    has(&files(Backend::Rust, Ui::Web), "src/engines.rs", "&[\"clock\", \"random\", \"log\"], Limits { seconds: 2, megabytes: 64 }");
    has(&files(Backend::Go, Ui::Web), "engines.go", "[]string{\"clock\", \"random\", \"log\"}, engine.Limits{Seconds: 2, Megabytes: 64}");
    has(&files(Backend::Csharp, Ui::Avalonia), "Engines.cs", "[\"clock\", \"random\", \"log\"], new EngineLimits(2, 64)");
    has(&files(Backend::C, Ui::Web), "src/engines.c", "3, pricing_uses, (ClarityEngineLimits){2, 64}");
    has(&files(Backend::C, Ui::Web), "src/engines.c", "static const char* const pricing_uses[] = {\"clock\", \"random\", \"log\"};");
    has(&files(Backend::Cpp, Ui::Web), "src/engines.cpp", "{\"clock\", \"random\", \"log\"}, clarity::EngineLimits{2, 64}");
}
