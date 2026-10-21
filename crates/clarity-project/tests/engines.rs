//! An engine's source next to its file: found, made from a template with a
//! stub for each declared function, built into the engines folder, and the
//! compiler's errors read as problems at their lines.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use clarity_project::engines::{self, Problem, Severity};

const MODULE: &str = "module game

entity Player
    name: text(40)

enum Level
    Easy
    Hard

engine Board from \"board.wasm\"
    uses random
    state per window
    on open
    function play(square: integer, level: Level) -> text
    function best(players: list<Player>, at: date) -> Player
    function score(price: money, done: bool, picture: bytes) -> decimal
    function restart() -> integer
";

fn engine() -> clarity_spec::code::EngineDecl {
    let project = clarity_spec::load_sources([("game.clarity", MODULE)]);
    project.files[0].engines[0].clone()
}

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-engines-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn is_enum(name: &str) -> bool {
    name == "Level"
}

/// Each declared function gets a stub that takes its values as the
/// declaration gives them and answers a value of its result.
#[test]
fn a_stub_takes_and_gives_the_declared_types() {
    let decl = engine();
    let stubs: Vec<String> = decl.functions.iter().map(|f| engines::stub(f, &is_enum)).collect();
    assert_eq!(
        stubs[0],
        "// play(square: integer, level: Level) -> text\nclarity_engine::function!(play, |(square, level): (i64, String)| {\n    let _ = (square, level);\n    Ok(String::new())\n});\n"
    );
    assert_eq!(
        stubs[1],
        "// best(players: list<Player>, at: date) -> Player\nclarity_engine::function!(best, |(players, at): (Vec<clarity_engine::serde_json::Value>, Option<String>)| {\n    let _ = (players, at);\n    Ok(clarity_engine::serde_json::Value::Null)\n});\n"
    );
    assert_eq!(
        stubs[2],
        "// score(price: money, done: bool, picture: bytes) -> decimal\nclarity_engine::function!(score, |(price, done, picture): (String, bool, clarity_engine::Bytes)| {\n    let _ = (price, done, picture);\n    Ok(String::from(\"0\"))\n});\n"
    );
    assert_eq!(stubs[3], "// restart() -> integer\nclarity_engine::function!(restart, |(): ()| {\n    Ok(0i64)\n});\n");
}

/// A new engine is a Rust crate in a folder named after its file, with the
/// interface, its on open and a stub per function; it builds as it is.
#[test]
fn a_new_engine_is_a_crate_with_a_stub_per_function() {
    let dir = folder("new");
    let source = engines::scaffold(&dir, &engine(), &is_enum).unwrap();
    assert_eq!(source.folder, dir.join("board"));
    assert_eq!(source.main, dir.join("board/src/lib.rs"));
    let cargo = std::fs::read_to_string(dir.join("board/Cargo.toml")).unwrap();
    assert!(cargo.contains("name = \"board\""), "{cargo}");
    assert!(cargo.contains("crate-type = [\"cdylib\"]"), "{cargo}");
    assert!(cargo.contains(&format!("clarity-engine = \"{}\"", engines::ENGINE_LIBRARY_VERSION)), "{cargo}");
    assert!(cargo.contains("[workspace]"), "{cargo}");
    assert_eq!(std::fs::read_to_string(dir.join("board/.gitignore")).unwrap(), "/target\n");
    let lib = std::fs::read_to_string(&source.main).unwrap();
    assert!(lib.contains("clarity_engine::interface!();"), "{lib}");
    assert!(lib.contains("clarity_engine::on_open!(|| {\n    Ok(())\n});"), "{lib}");
    assert!(lib.contains("clarity_engine::function!(play, "), "{lib}");
    assert!(lib.contains("clarity_engine::function!(restart, "), "{lib}");
    // Found again as the engine's source; a second scaffold does not overwrite it.
    assert_eq!(engines::source_of(&dir, "board.wasm"), Some(source));
    assert!(engines::scaffold(&dir, &engine(), &is_enum).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// An engine without a crate next to it has no source Clarity knows.
#[test]
fn an_engine_without_a_crate_has_no_source() {
    let dir = folder("none");
    assert_eq!(engines::source_of(&dir, "board.wasm"), None);
    std::fs::create_dir_all(dir.join("board")).unwrap();
    assert_eq!(engines::source_of(&dir, "board.wasm"), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

fn touch(path: &Path, at: SystemTime) {
    std::fs::File::options().write(true).open(path).unwrap().set_modified(at).unwrap();
}

/// The file needs building when there is none, or the source changed since.
#[test]
fn an_engine_needs_building_when_its_source_is_newer() {
    let dir = folder("stale");
    let source = engines::scaffold(&dir, &engine(), &is_enum).unwrap();
    let wasm = dir.join("board.wasm");
    assert!(engines::needs_build(&source, &wasm));
    let now = SystemTime::now();
    std::fs::write(&wasm, b"\0asm").unwrap();
    touch(&source.main, now - Duration::from_secs(60));
    touch(&source.folder.join("Cargo.toml"), now - Duration::from_secs(60));
    touch(&wasm, now);
    assert!(!engines::needs_build(&source, &wasm));
    touch(&source.main, now + Duration::from_secs(5));
    assert!(engines::needs_build(&source, &wasm));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Cargo builds it for WebAssembly, short messages to read; a development
/// build of Clarity points the engine library at its own copy.
#[test]
fn an_engine_builds_with_cargo_for_webassembly() {
    let dir = folder("build");
    let source = engines::scaffold(&dir, &engine(), &is_enum).unwrap();
    let build = engines::build_command(&source, None);
    assert_eq!(build.tool, "cargo");
    assert_eq!(build.args, ["build", "--release", "--target", "wasm32-unknown-unknown", "--message-format", "short"]);
    assert_eq!(build.dir, source.folder);
    assert_eq!(build.artifact, source.folder.join("target/wasm32-unknown-unknown/release/board.wasm"));
    let local = engines::build_command(&source, Some(Path::new("/repo/runtimes/rust-engine")));
    assert_eq!(&local.args[6..], ["--config", "patch.crates-io.clarity-engine.path=\"/repo/runtimes/rust-engine\""]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The compiler's short messages are problems at their file and line.
#[test]
fn compiler_messages_are_problems_at_their_lines() {
    let folder = Path::new("/p/engines/board");
    let lines = [
        "   Compiling board v0.1.0 (/p/engines/board)",
        "src/lib.rs:12:5: error[E0308]: mismatched types: expected `String`, found integer",
        "src/lib.rs:3:9: warning: unused variable: `level`",
        "error: could not compile `board` (lib) due to 1 previous error",
    ];
    let problems: Vec<Problem> = lines.iter().filter_map(|l| engines::problem_in(folder, l)).collect();
    assert_eq!(
        problems,
        [
            Problem {
                file: folder.join("src/lib.rs"),
                line: 12,
                column: 5,
                severity: Severity::Error,
                message: "mismatched types: expected `String`, found integer".into(),
            },
            Problem { file: folder.join("src/lib.rs"), line: 3, column: 9, severity: Severity::Warning, message: "unused variable: `level`".into() },
        ]
    );
}

/// A function declared after the source was made gets its stub added; the
/// rest of the source stays as written.
#[test]
fn missing_functions_get_their_stubs_added() {
    let decl = engine();
    let written = "clarity_engine::interface!();\n\nclarity_engine::function!(play, |(s, l): (i64, String)| { Ok(format!(\"{s}{l}\")) });\n";
    assert_eq!(engines::missing_functions(written, &decl), ["best", "score", "restart"]);
    let added = engines::add_stubs(written, &decl, &is_enum);
    assert!(added.starts_with(written), "{added}");
    assert!(added.contains("clarity_engine::function!(best, "), "{added}");
    assert!(engines::missing_functions(&added, &decl).is_empty());
    // Where a function is in the source, for jumping to it.
    assert_eq!(engines::function_line(&added, "play"), Some(3));
    assert_eq!(engines::function_line(&added, "nothing"), None);
}

/// A new engine builds as Clarity makes it, every stub type and all. Slow:
/// cargo builds it for WebAssembly.
#[test]
#[ignore]
fn a_new_engine_builds_as_it_is_made() {
    let dir = folder("builds");
    let source = engines::scaffold(&dir, &engine(), &is_enum).unwrap();
    let library = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/rust-engine").canonicalize().unwrap();
    let build = engines::build_command(&source, Some(&library));
    let out = std::process::Command::new(build.tool).args(&build.args).current_dir(&build.dir).output().unwrap();
    let text = String::from_utf8_lossy(&out.stderr);
    let problems: Vec<Problem> = text.lines().filter_map(|l| engines::problem_in(&build.dir, l)).collect();
    assert!(out.status.success(), "{text}");
    assert!(problems.is_empty(), "{problems:#?}");
    let module = clarity_spec::engines::read_module(&std::fs::read(&build.artifact).unwrap()).unwrap();
    for name in ["play", "best", "score", "restart", "clarity_open", "clarity_alloc", "clarity_free"] {
        assert!(module.functions.iter().any(|f| f == name), "{name} in {:?}", module.functions);
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
