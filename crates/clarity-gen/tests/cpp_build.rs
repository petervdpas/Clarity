//! Generates the C++ examples on their own stack, and the test projects of
//! tests/fixtures and conformance as C++, and builds them as a C++ developer
//! would:
//! `cmake --workflow --preset debug` configures and builds without a
//! warning from the program's own code, and the program runs with `--check`
//! (it opens its database, applies the migration steps and stops). Slow
//! (a C++ build), so it only runs when asked:
//! `cargo test -p clarity-gen --test cpp_build -- --ignored`.

use std::path::Path;
use std::process::Command;

use clarity_project::{Backend, Ui};

mod served;

/// Where builds go: target/generated in the repository, on disk; never the
/// system's temporary folder, which may live in memory.
fn scratch() -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated");
    std::fs::create_dir_all(&dir).expect("target/generated can be made");
    dir
}

fn run(dir: &Path, tool: &str, args: &[&str]) -> String {
    let out = Command::new(tool).args(args).current_dir(dir).output().unwrap_or_else(|e| panic!("{tool}: {e}"));
    let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "{tool} {args:?} failed in {}:\n{said}", dir.display());
    said
}

fn builds_and_runs_in_cpp(name: &str, program: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join(format!("examples/{name}.clarity"));
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
    // An example builds on the stack its project file names, and no other.
    assert_eq!((settings.stack.backend, settings.stack.ui), (Backend::Cpp, Ui::Web), "{name} is not a C++ example");
    let out = scratch().join(format!("clarity-gen-cpp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    // The database and stores in the scratch folder, not in the example.
    settings.datasources[0].url = "data/app.db".into();
    for store in &mut settings.filestores {
        store.location = format!("data/{}", store.name);
    }
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    clarity_gen::write_out(&out, &files, &["build", "data"]).unwrap();

    let said = run(&out, "cmake", &["--workflow", "--preset", "debug"]);
    // The program's own code builds without a warning.
    let own: Vec<&str> = said.lines().filter(|l| l.contains("warning") && l.contains("/src/")).collect();
    assert!(own.is_empty(), "warnings:\n{}", own.join("\n"));
    // A program with native parts has them built beside it.
    if !clarity_gen::parts::native_parts(&project).is_empty() {
        assert!(out.join("build/debug/libclarity_parts.so").exists(), "the parts library is not beside {name}");
    }
    let said = run(&out, &out.join("build/debug").join(name).display().to_string(), &["--check"]);
    assert!(said.contains(&format!("{program}: the databases are ready.")), "{said}");
    // A project with records has its database; one without needs none.
    if project.stored_entities().next().is_some() {
        assert!(out.join("data/app.db").exists());
    }
    std::fs::remove_dir_all(&out).unwrap();
}

/// The game whose field of orbs a native library draws with SFML (a custom
/// part, DESIGN 5.19): the program builds with its parts library beside it.
#[test]
#[ignore]
fn the_orbs_example_builds_and_runs_in_cpp() {
    builds_and_runs_in_cpp("orbs", "Orbs");
}

/// The test engine of conformance/engines.json in a generated application:
/// every function called from Clarity code, its values crossing as JSON.
const ENGINE_SHOP: &str = include_str!("../../../conformance/engines/shop.clarity");
const TEST_ENGINE: &[u8] = include_bytes!("../../../conformance/engines/test.wasm");

/// A program beside the generated one that calls its functions.
const ENGINE_CHECK: &str = r#"#include <iostream>

#include "app.hpp"

#define EXPECT(holds, what)                                  \
    do {                                                     \
        if (!(holds)) {                                      \
            std::cerr << "failed: " << (what) << "\n";       \
            return 1;                                        \
        }                                                    \
    } while (0)

int main() {
    using namespace shop;
    try {
        engines::check();
        clarity::Db db({{"main", clarity::Dialect::Sqlite, ":memory:"}});
        EXPECT(answer(db, 2) == 42, "answer");
        EXPECT(greeting(db, "Grüße") == "Hello, Grüße!", "greeting");
        std::vector<Line> lines{{0, clarity::Decimal::parse("9.95"), 3}, {0, clarity::Decimal::parse("0.10"), 1}};
        EXPECT(sum(db, lines) == clarity::Decimal::parse("29.95"), "sum");
        EXPECT(size_class(db, 12) == Size::Large, "size");
        Order order{7, "ORD-7", clarity::Decimal::parse("100.05")};
        auto less = cheaper(db, order);
        EXPECT(less.id == 7 && less.total == clarity::Decimal::parse("90.05"), "cheaper");
        // Bytes cross to an engine and back as they are.
        EXPECT(backwards(db, clarity::Bytes{1, 2, 3}) == clarity::Bytes({3, 2, 1}), "backwards");
        EXPECT(measured(db, clarity::Bytes{1, 2, 3}) == 6, "measured");
        // An engine with state counts on from what its on open set.
        EXPECT(counted(db) == 11 && counted(db) == 12, "counted");
        // Each open window has an instance of its own of an engine with state
        // per window, which goes when the window closes (as the shell reports it).
        auto one = clarity::opened("Main", nlohmann::json::object(), nlohmann::json::object());
        auto other = clarity::opened("Main", nlohmann::json::object(), nlohmann::json::object());
        MainWindow a{}, b{};
        clarity::Ui in_a(one), in_b(other);
        a.moved(db, in_a);
        b.moved(db, in_b);
        a.moved(db, in_a);
        EXPECT(a.moves == 12 && b.moves == 11, "moves");
        clarity::window_closed(one.instance);
        a.moved(db, in_a);
        b.moved(db, in_b);
        EXPECT(a.moves == 11 && b.moves == 12, "moves after closing");
    } catch (const std::exception& e) {
        std::cerr << "failed: " << e.what() << "\n";
        return 1;
    }
    // The engine's own error reads as a refusal, in its words.
    try {
        clarity::Db db({{"main", clarity::Dialect::Sqlite, ":memory:"}});
        refused(db, "no stock");
        EXPECT(false, "refused");
    } catch (const clarity::RefusedError& e) {
        EXPECT(std::string(e.what()) == "no stock", "refused");
    }
    std::cout << "every engine function answered\n";
    return 0;
}
"#;

#[test]
#[ignore]
fn an_engine_answers_in_the_generated_application_in_cpp() {
    let mut project = clarity_spec::load_sources([("shop.clarity", ENGINE_SHOP)]);
    clarity_spec::engines::attach(&mut project, vec![clarity_spec::engines::EngineFile { name: "test.wasm".into(), bytes: TEST_ENGINE.to_vec() }]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = clarity_project::starter("Shop", "shop");
    settings.stack.backend = Backend::Cpp;
    settings.stack.ui = Ui::Web;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-cpp-engine-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["build"]).unwrap();
    // The check program: every source of the application but its main.
    std::fs::create_dir_all(out.join("check")).unwrap();
    std::fs::write(out.join("check/engine_check.cpp"), ENGINE_CHECK).unwrap();
    let lists = std::fs::read_to_string(out.join("CMakeLists.txt")).unwrap();
    std::fs::write(
        out.join("CMakeLists.txt"),
        format!("{lists}\nset(CHECK_SOURCES ${{PROGRAM_SOURCES}})\nlist(FILTER CHECK_SOURCES EXCLUDE REGEX \"src/main.cpp$\")\nadd_executable(engine_check ${{CHECK_SOURCES}} check/engine_check.cpp)\ntarget_include_directories(engine_check PRIVATE src)\ntarget_link_libraries(engine_check PRIVATE clarity clarity_shell clarity_engine)\n"),
    )
    .unwrap();
    let built = run(&out, "cmake", &["--workflow", "--preset", "debug"]);
    let own: Vec<&str> = built.lines().filter(|l| l.contains("warning") && (l.contains("/src/") || l.contains("/engine/"))).collect();
    assert!(own.is_empty(), "warnings:\n{}", own.join("\n"));
    let said = run(&out, &out.join("build/debug/engine_check").display().to_string(), &[]);
    assert!(said.contains("every engine function answered"), "{said}");
    // The program itself checks its engines at start, then opens.
    let said = run(&out, &out.join("build/debug/shop").display().to_string(), &["--check"]);
    assert!(said.contains("the databases are ready"), "{said}");
    std::fs::remove_dir_all(&out).unwrap();
}

/// The noughts example's Board engine, written in Clarity and compiled to
/// WebAssembly, played through the C++ application's engine host from a
/// program of its own.
const BOARD_CHECK: &str = r#"#include <iostream>

#include "app.hpp"

namespace noughts {
const std::string& model();
}

#define EXPECT(holds, what)                                  \
    do {                                                     \
        if (!(holds)) {                                      \
            std::cerr << "failed: " << (what) << "\n";       \
            return 1;                                        \
        }                                                    \
    } while (0)

using namespace noughts;

// The engine's functions in window `w`; what it says is a key, shown in the person's language.
static std::string play(uint64_t w, int64_t square) {
    return clarity::text_of(engines::board().call_in_as<std::string>(w, "play", nlohmann::json::array({square})));
}
static int64_t mark(uint64_t w, int64_t square) { return engines::board().call_in_as<int64_t>(w, "mark", nlohmann::json::array({square})); }
static std::string restart(uint64_t w) { return clarity::text_of(engines::board().call_in_as<std::string>(w, "restart", nlohmann::json::array())); }
static int64_t taken(uint64_t w) { return engines::board().call_in_as<int64_t>(w, "taken", nlohmann::json::array()); }

static std::string refusal(auto call) {
    try {
        call();
    } catch (const clarity::RefusedError& e) {
        return e.what();
    }
    return "";
}

int main() {
    clarity::load_translations(model());
    EXPECT(play(1, 1) == "Your move", "play");
    EXPECT(mark(1, 1) == 1, "your cross");
    // The computer took the centre.
    EXPECT(mark(1, 5) == 2, "the centre");
    EXPECT(refusal([&] { play(1, 5); }) == "That square is taken", "taken");
    EXPECT(refusal([&] { play(1, 10); }) == "There are nine squares", "nine squares");
    // Another window has a board of its own, and a closed window's board goes.
    EXPECT(mark(2, 1) == 0, "another window");
    clarity::window_closed(1);
    EXPECT(mark(1, 1) == 0, "closed");
    // The computer blocks your line.
    play(3, 1);
    play(3, 2);
    EXPECT(mark(3, 3) == 2, "blocked");
    EXPECT(restart(3) == "Your move", "restart");
    EXPECT(taken(3) == 0, "an empty board");
    std::cout << "the board played\n";
    return 0;
}
"#;

#[test]
#[ignore]
fn an_engine_written_in_clarity_runs_in_cpp() {
    board_runs_in_cpp(false);
}

/// The same Board run by the interpreter (`engines interpreted`).
#[test]
#[ignore]
fn an_interpreted_engine_runs_in_cpp() {
    board_runs_in_cpp(true);
}

fn board_runs_in_cpp(interpreted: bool) {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/board.clarity");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let mut settings = clarity_project::load(&file).settings.expect("the fixture has a project file");
    settings.stack.backend = Backend::Cpp;
    settings.stack.ui = Ui::Web;
    settings.datasources[0].url = "data/app.db".into();
    settings.engines_interpreted = interpreted;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-cpp-board-{}-{}", interpreted, std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["build"]).unwrap();
    // The check program: every source of the application but its main.
    std::fs::create_dir_all(out.join("check")).unwrap();
    std::fs::write(out.join("check/board_check.cpp"), BOARD_CHECK).unwrap();
    let lists = std::fs::read_to_string(out.join("CMakeLists.txt")).unwrap();
    std::fs::write(
        out.join("CMakeLists.txt"),
        format!("{lists}\nset(CHECK_SOURCES ${{PROGRAM_SOURCES}})\nlist(FILTER CHECK_SOURCES EXCLUDE REGEX \"src/main.cpp$\")\nadd_executable(board_check ${{CHECK_SOURCES}} check/board_check.cpp)\ntarget_include_directories(board_check PRIVATE src)\ntarget_link_libraries(board_check PRIVATE clarity clarity_shell clarity_engine)\n"),
    )
    .unwrap();
    let built = run(&out, "cmake", &["--workflow", "--preset", "debug"]);
    let own: Vec<&str> = built.lines().filter(|l| l.contains("warning") && l.contains("/src/")).collect();
    assert!(own.is_empty(), "warnings:\n{}", own.join("\n"));
    let said = run(&out, &out.join("build/debug/board_check").display().to_string(), &[]);
    assert!(said.contains("the board played"), "{said}");
    std::fs::remove_dir_all(&out).unwrap();
}

/// A window that asks each kind of question (DESIGN 5.14) builds without a
/// warning in its own code.
#[test]
#[ignore]
fn questions_and_their_answers_build_in_cpp() {
    let project = clarity_spec::keyed::load_keyed([("music.clarity", include_str!("fixtures/asks.clarity"))], &[], "en");
    let mut settings = clarity_project::starter("Music", "music");
    settings.stack.backend = Backend::Cpp;
    settings.stack.ui = Ui::Web;
    let out = scratch().join(format!("clarity-gen-cpp-asks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    clarity_gen::write_out(&out, &files, &["build", "data"]).unwrap();
    let said = run(&out, "cmake", &["--workflow", "--preset", "debug"]);
    let own: Vec<&str> = said.lines().filter(|l| l.contains("warning") && l.contains("/src/")).collect();
    assert!(own.is_empty(), "warnings:\n{}", own.join("\n"));
    std::fs::remove_dir_all(&out).unwrap();
}

/// The web target (DESIGN 7.3): a project whose only target is the web
/// builds into a web server that serves the renderer and answers its commands.
#[test]
#[ignore]
fn the_web_target_serves_in_cpp() {
    let project = clarity_spec::keyed::load_keyed([("music.clarity", include_str!("fixtures/asks.clarity"))], &[], "en");
    let mut settings = clarity_project::starter("Music", "music");
    settings.stack.backend = Backend::Cpp;
    settings.stack.ui = Ui::Web;
    settings.stack.targets = vec![clarity_project::Target::Web];
    let out = scratch().join(format!("clarity-gen-cpp-web-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    clarity_gen::write_out(&out, &files, &["build", "data"]).unwrap();
    let said = run(&out, "cmake", &["--workflow", "--preset", "debug"]);
    let own: Vec<&str> = said.lines().filter(|l| l.contains("warning") && l.contains("/src/")).collect();
    assert!(own.is_empty(), "warnings:\n{}", own.join("\n"));
    served::links_no_desktop_toolkit(&out.join("build/debug/music"));
    served::serves(&out, &out.join("build/debug/music"), &[], "127.0.0.1:18733");
    // The same project for the desktop does link it, so the check can tell.
    settings.stack.targets = vec![clarity_project::Target::Desktop];
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    clarity_gen::write_out(&out, &files, &["build", "data"]).unwrap();
    run(&out, "cmake", &["--workflow", "--preset", "debug"]);
    served::links_the_desktop_toolkit(&out.join("build/debug/music"));
    std::fs::remove_dir_all(&out).unwrap();
}
