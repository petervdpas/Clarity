//! Engines written in Clarity (the Board of tests/fixtures/board.clarity)
//! become WebAssembly files when the project is generated (DESIGN 5.12,
//! N5b): every stack carries `board.wasm` and calls it as any engine file,
//! and no stack writes engine code of its own.

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

fn all(files: &[OutFile]) -> String {
    files.iter().map(|f| f.text.as_str()).collect::<Vec<_>>().join("\n")
}

/// The compiled engine is among the files, as WebAssembly: a file of its
/// own, or bytes in the program's source on C and C++.
fn carries_the_board(files: &[OutFile]) {
    if let Some(wasm) = files.iter().find(|f| f.path.ends_with("board.wasm")) {
        assert_eq!(&wasm.bytes.as_deref().expect("the engine file is bytes")[..4], b"\0asm");
        return;
    }
    let source = files.iter().find(|f| f.path == "src/engines.c" || f.path == "src/engines.cpp").expect("board.wasm, or the engines' source");
    assert!(source.text.contains("/// Engine Board, from board.wasm.") && source.text.contains("0x00, 0x61, 0x73, 0x6d"), "{}", source.path);
}

/// No stack writes the engine as code of its own any more.
fn writes_no_engine_code(files: &[OutFile]) {
    let all = all(files);
    for written in ["BoardKept", "boardKept", "BoardEngine", "board_play_with", "boardPlayWith", "PlayWith("] {
        assert!(!all.contains(written), "{written} is still written");
    }
}

#[test]
fn every_stack_carries_a_clarity_engine_as_a_file() {
    for (backend, ui) in [(Backend::Rust, Ui::Web), (Backend::Go, Ui::Web), (Backend::C, Ui::Web), (Backend::Cpp, Ui::Web), (Backend::Csharp, Ui::Avalonia)] {
        let files = files(backend, ui);
        carries_the_board(&files);
        writes_no_engine_code(&files);
        // Called through the engine host, as a brought-along engine is, in the handler's window.
        let all = all(&files);
        assert!(all.contains("\"play\""), "{backend:?}: the call of play goes to the engine by name");
    }
}

/// The keys the engine's code gives are its texts, translated by the host.
#[test]
fn a_clarity_engines_keys_are_its_says() {
    let files = files(Backend::Rust, Ui::Web);
    let all = all(&files);
    assert!(all.contains("thatSquareIsTaken"), "the keys reach the translations");
}

/// With `engines interpreted` the engine's file is the interpreter with the
/// engine's IR inside; without it, compiled code, with no IR.
#[test]
fn the_project_chooses_compiled_or_interpreted_engines() {
    let board = |interpreted: bool| {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let file = root.join("crates/clarity-gen/tests/fixtures/board.clarity");
        let project = clarity_spec::load_dir(&clarity_project::module_dir(&file)).unwrap();
        let mut settings = clarity_project::load(&file).settings.expect("the fixture has a project file");
        settings.stack.backend = Backend::Go;
        settings.stack.ui = Ui::Web;
        settings.engines_interpreted = interpreted;
        let files = generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
        files.into_iter().find(|f| f.path.ends_with("board.wasm")).and_then(|f| f.bytes).expect("board.wasm").into_owned()
    };
    let carries_ir = |bytes: &[u8]| {
        let needle = b"\"name\":\"play\"";
        bytes.windows(needle.len()).any(|w| w == needle)
    };
    assert!(carries_ir(&board(true)), "the interpreted engine carries its IR");
    assert!(!carries_ir(&board(false)), "the compiled engine carries code, not IR");
}
