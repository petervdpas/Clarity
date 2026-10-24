//! Engines written in Clarity (the Board of tests/fixtures/board.clarity, the
//! stack's output: a store of what each instance keeps, and per function an
//! entry point that works on it, called from handlers as any engine's.

use clarity_gen::{OutFile, generate};
use clarity_project::{Backend, Ui};

fn files(backend: Backend, ui: Ui) -> Vec<OutFile> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join("crates/clarity-gen/tests/fixtures/board.clarity");
    let project = clarity_spec::load_dir(&clarity_project::module_dir(&file)).unwrap();
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = clarity_project::load(&file).settings.expect("the fixture has a project file");
    settings.stack.backend = backend;
    settings.stack.ui = ui;
    generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"))
}

fn text<'a>(files: &'a [OutFile], path: &str) -> &'a str {
    &files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("no {path} in {:?}", files.iter().map(|f| &f.path).collect::<Vec<_>>())).text
}

fn has(files: &[OutFile], path: &str, expected: &str) {
    let text = text(files, path);
    assert!(text.contains(expected), "expected\n{expected}\nin {path}:\n{text}");
}

#[test]
fn rust_runs_an_engine_written_in_clarity() {
    let files = files(Backend::Rust, Ui::Web);
    if std::env::var("CLARITY_DUMP").is_ok() {
        println!("{}", text(&files, "src/modules/noughts.rs"));
    }
    // No WebAssembly to load: the engine is code of the module.
    assert!(!files.iter().any(|f| f.path == "src/engines.rs" || f.path.ends_with(".wasm")));
    has(&files, "src/modules/noughts.rs", "pub struct BoardKept {\n    pub squares: Vec<i64>,\n    pub lines: Vec<i64>,\n}");
    has(&files, "src/modules/noughts.rs", "pub fn board_play(db: &Db, instance: u64, square: i64) -> Result<String> {");
    has(&files, "src/modules/noughts.rs", "pub fn board_play_with(db: &Db, kept: &mut BoardKept, square: i64) -> Result<String> {");
    has(&files, "src/modules/noughts.rs", "clarity_runtime::random(");
    has(&files, "src/modules/noughts.rs", "crate::modules::noughts::board_play(db, ui.instance(), square)?");
    has(&files, "src/main.rs", "modules::noughts::board_close(instance);");
}

#[test]
fn csharp_runs_an_engine_written_in_clarity() {
    let files = files(Backend::Csharp, Ui::Avalonia);
    if std::env::var("CLARITY_DUMP").is_ok() {
        println!("{}", text(&files, "Modules/Noughts.cs"));
    }
    assert!(!files.iter().any(|f| f.path == "Engines.cs" || f.path.ends_with(".wasm")));
    has(&files, "Modules/Noughts.cs", "public static class BoardEngine\n{");
    has(&files, "Modules/Noughts.cs", "public static string Play(Db db, long instance, long square)");
    has(&files, "Modules/Noughts.cs", "public static string PlayWith(Db db, Kept kept, long square)");
    has(&files, "Modules/Noughts.cs", "static BoardEngine() => OpenWindows.Closed += Close;");
    has(&files, "Modules/Noughts.cs", "Splitmix.Random(");
}

#[test]
fn go_runs_an_engine_written_in_clarity() {
    let files = files(Backend::Go, Ui::Web);
    if std::env::var("CLARITY_DUMP").is_ok() {
        println!("{}", text(&files, "noughts_module.go"));
    }
    assert!(!files.iter().any(|f| f.path == "engines.go" || f.path.ends_with(".wasm")));
    has(&files, "noughts_module.go", "type boardKept struct {\n\tsquares []int64\n\tlines   []int64\n}");
    has(&files, "noughts_module.go", "func boardPlay(db *clarity.Db, instance uint64, square int64) (string, error) {");
    has(&files, "noughts_module.go", "func boardPlayWith(db *clarity.Db, kept *boardKept, square int64) (string, error) {");
    has(&files, "noughts_module.go", "clarity.OnWindowClosed(boardClose)");
    has(&files, "noughts_module.go", "boardPlay(db, ui.Instance(), square)");
}

#[test]
fn cpp_runs_an_engine_written_in_clarity() {
    let files = files(Backend::Cpp, Ui::Web);
    if std::env::var("CLARITY_DUMP").is_ok() {
        println!("{}", text(&files, "src/modules/noughts.cpp"));
    }
    assert!(!files.iter().any(|f| f.path == "src/engines.cpp" || f.path.ends_with(".wasm")));
    has(&files, "src/app.hpp", "std::string board_play(clarity::Db& db, uint64_t instance, int64_t square);");
    has(&files, "src/app.hpp", "void board_close(uint64_t instance);");
    has(&files, "src/modules/noughts.cpp", "struct BoardKept {\n    std::vector<int64_t> squares;\n    std::vector<int64_t> lines;\n};");
    has(&files, "src/modules/noughts.cpp", "clarity::on_window_closed(");
    has(&files, "src/modules/noughts.cpp", "board_play(db, ui.instance(), square)");
    has(&files, "src/modules/noughts.cpp", "clarity::random(");
}

#[test]
fn c_runs_an_engine_written_in_clarity() {
    let files = files(Backend::C, Ui::Web);
    if std::env::var("CLARITY_DUMP").is_ok() {
        println!("{}", text(&files, "src/modules/noughts.c"));
    }
    assert!(!files.iter().any(|f| f.path == "src/engines.c" || f.path.ends_with(".wasm")));
    has(&files, "src/app.h", "bool board_play(ClarityDb* db, uint64_t instance, int64_t square, const char** result);");
    has(&files, "src/app.h", "void board_close(uint64_t instance);");
    has(&files, "src/modules/noughts.c", "typedef struct {\n    ClarityIntegers squares;\n    ClarityIntegers lines;\n} BoardKept;");
    has(&files, "src/modules/noughts.c", "clarity_on_window_closed(board_closed, NULL)");
    has(&files, "src/modules/noughts.c", "board_play(db, clarity_ui_instance(ui), square, &");
    has(&files, "src/modules/noughts.c", "clarity_random(");
}
