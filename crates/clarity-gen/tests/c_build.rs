//! Generates the test projects of tests/fixtures and conformance as C and
//! builds them as a C developer would (no example is written in C, and each
//! example builds only on the stack its project file names):
//! `cmake --workflow --preset debug` configures and builds without a
//! warning from the program's own code, and the program runs with `--check`
//! (it opens its database, applies the migration steps and stops). Slow
//! (a C build), so it only runs when asked:
//! `cargo test -p clarity-gen --test c_build -- --ignored`.

use std::path::Path;
use std::process::Command;

use clarity_project::{Backend, Ui};

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

/// The test engine of conformance/engines.json in a generated application:
/// every function called from Clarity code, its values crossing as JSON.
const ENGINE_SHOP: &str = include_str!("../../../conformance/engines/shop.clarity");
const TEST_ENGINE: &[u8] = include_bytes!("../../../conformance/engines/test.wasm");

/// A program beside the generated one that calls its functions.
const ENGINE_CHECK: &str = r#"#include <stdio.h>
#include <string.h>

#include "app.h"

#define EXPECT(holds, what)                                                          \
    do {                                                                             \
        if (!(holds)) {                                                              \
            fprintf(stderr, "failed: %s (%s)\n", what, clarity_error()->message);    \
            return 1;                                                                \
        }                                                                            \
    } while (0)

int main(void) {
    EXPECT(engines_check(), "check");
    int64_t a = 0;
    EXPECT(answer(NULL, 2, &a) && a == 42, "answer");
    const char* g = NULL;
    EXPECT(greeting(NULL, "Grüße", &g) && strcmp(g, "Hello, Grüße!") == 0, "greeting");
    Line items[2] = {{.price = clarity_decimal(995, 2), .count = 3}, {.price = clarity_decimal(10, 2), .count = 1}};
    LineList lines = {items, 2};
    ClarityDecimal total;
    EXPECT(sum(NULL, lines, &total) && clarity_decimal_compare(total, clarity_decimal(2995, 2)) == 0, "sum");
    Size size;
    EXPECT(size_class(NULL, 12, &size) && size == SIZE_LARGE, "size");
    Order order = {.id = 7, .number = "ORD-7", .total = clarity_decimal(10005, 2)};
    Order less;
    EXPECT(cheaper(NULL, &order, &less) && less.id == 7 && clarity_decimal_compare(less.total, clarity_decimal(9005, 2)) == 0, "cheaper");
    // The engine's own error reads as a refusal, in its words.
    const char* r = NULL;
    EXPECT(!refused(NULL, "no stock", &r) && clarity_error()->kind == CLARITY_REFUSED && strcmp(clarity_error()->message, "no stock") == 0, "refused");
    // Bytes cross to an engine and back as they are.
    static const uint8_t forwards[] = {1, 2, 3};
    ClarityBytes back = {0};
    int64_t length = 0;
    EXPECT(backwards(NULL, (ClarityBytes){forwards, 3}, &back) && back.size == 3 && back.data[0] == 3 && back.data[2] == 1, "backwards");
    EXPECT(measured(NULL, (ClarityBytes){forwards, 3}, &length) && length == 6, "measured");
    // An engine with state counts on from what its on open set.
    int64_t first = 0, second = 0;
    EXPECT(counted(NULL, &first) && counted(NULL, &second) && first == 11 && second == 12, "counted");
    // Each open window has an instance of its own of an engine with state
    // per window, which goes when the window closes (as the shell reports it).
    ClarityWindowState* one = clarity_opened("Main", NULL, NULL);
    ClarityWindowState* other = clarity_opened("Main", NULL, NULL);
    MainWindow first_window = {0}, second_window = {0};
    ClarityUi* in_first = clarity_ui(one);
    ClarityUi* in_second = clarity_ui(other);
    EXPECT(main_window_moved(NULL, in_first, &first_window) && main_window_moved(NULL, in_second, &second_window) && main_window_moved(NULL, in_first, &first_window), "moved");
    EXPECT(first_window.moves == 12 && second_window.moves == 11, "moves");
    clarity_window_closed(one->instance);
    EXPECT(main_window_moved(NULL, in_first, &first_window) && main_window_moved(NULL, in_second, &second_window), "moved again");
    EXPECT(first_window.moves == 11 && second_window.moves == 12, "moves after closing");
    puts("every engine function answered");
    return 0;
}
"#;

#[test]
#[ignore]
fn an_engine_answers_in_the_generated_application_in_c() {
    let mut project = clarity_spec::load_sources([("shop.clarity", ENGINE_SHOP)]);
    clarity_spec::engines::attach(&mut project, vec![clarity_spec::engines::EngineFile { name: "test.wasm".into(), bytes: TEST_ENGINE.to_vec() }]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = clarity_project::starter("Shop", "shop");
    settings.stack.backend = clarity_project::Backend::C;
    settings.stack.ui = clarity_project::Ui::Web;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-c-engine-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["build"]).unwrap();
    // The check program: every source of the application but its main.
    std::fs::create_dir_all(out.join("check")).unwrap();
    std::fs::write(out.join("check/engine_check.c"), ENGINE_CHECK).unwrap();
    let lists = std::fs::read_to_string(out.join("CMakeLists.txt")).unwrap();
    std::fs::write(
        out.join("CMakeLists.txt"),
        format!("{lists}\nset(CHECK_SOURCES ${{PROGRAM_SOURCES}})\nlist(FILTER CHECK_SOURCES EXCLUDE REGEX \"src/main.c$\")\nadd_executable(engine_check ${{CHECK_SOURCES}} check/engine_check.c)\ntarget_include_directories(engine_check PRIVATE src)\ntarget_link_libraries(engine_check PRIVATE clarity clarity_shell clarity_engine)\n"),
    )
    .unwrap();
    let built = run(&out, "cmake", &["--workflow", "--preset", "debug"]);
    let own: Vec<&str> = built.lines().filter(|l| l.contains("warning") && (l.contains("/src/") || l.contains("/engine/"))).collect();
    assert!(own.is_empty(), "warnings:\n{}", own.join("\n"));
    let said = run(&out, &out.join("build/debug/engine_check").display().to_string(), &[]);
    assert!(said.contains("every engine function answered"), "{said}");
    std::fs::remove_dir_all(&out).unwrap();
}

/// The noughts example's Board engine, written in Clarity, as the C it
/// becomes, played from a program of its own; each call is a command of its
/// own, as the shell runs them, so what the board keeps outlives the
/// scratch arena.
const BOARD_CHECK: &str = r#"#include <stdio.h>
#include <string.h>

#include "app.h"
#include "page.h"

#define EXPECT(holds, what)                                                          \
    do {                                                                             \
        if (!(holds)) {                                                              \
            fprintf(stderr, "failed: %s (%s)\n", what, clarity_error()->message);    \
            return 1;                                                                \
        }                                                                            \
        clarity_scratch_reset();                                                     \
    } while (0)

static int64_t mark(uint64_t window, int64_t square) {
    int64_t got = -1;
    if (!board_mark(NULL, window, square, &got)) return -1;
    return got;
}

int main(void) {
    clarity_start();
    clarity_load_translations(page_model());
    const char* said = NULL;
    EXPECT(board_play(NULL, 1, 1, &said) && strcmp(said, "Your move") == 0, "play");
    EXPECT(mark(1, 1) == 1, "your cross");
    // The computer took the centre.
    EXPECT(mark(1, 5) == 2, "the centre");
    EXPECT(!board_play(NULL, 1, 5, &said) && strcmp(clarity_error()->message, "That square is taken") == 0, "taken");
    EXPECT(!board_play(NULL, 1, 10, &said) && strcmp(clarity_error()->message, "There are nine squares") == 0, "nine squares");
    // Another window has a board of its own, and a closed window's board goes.
    EXPECT(mark(2, 1) == 0, "another window");
    clarity_window_closed(1);
    EXPECT(mark(1, 1) == 0, "closed");
    // The computer blocks your line.
    EXPECT(board_play(NULL, 3, 1, &said), "first cross");
    EXPECT(board_play(NULL, 3, 2, &said), "second cross");
    EXPECT(mark(3, 3) == 2, "blocked");
    EXPECT(board_restart(NULL, 3, &said) && strcmp(said, "Your move") == 0, "restart");
    int64_t taken = -1;
    EXPECT(board_taken(NULL, 3, &taken) && taken == 0, "an empty board");
    printf("the board played\n");
    return 0;
}
"#;

#[test]
#[ignore]
fn an_engine_written_in_clarity_runs_in_c() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/board.clarity");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let mut settings = clarity_project::load(&file).settings.expect("the fixture has a project file");
    settings.stack.backend = Backend::C;
    settings.stack.ui = Ui::Web;
    settings.datasources[0].url = "data/app.db".into();
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = scratch().join(format!("clarity-gen-c-board-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["build"]).unwrap();
    std::fs::create_dir_all(out.join("check")).unwrap();
    std::fs::write(out.join("check/board_check.c"), BOARD_CHECK).unwrap();
    let lists = std::fs::read_to_string(out.join("CMakeLists.txt")).unwrap();
    std::fs::write(
        out.join("CMakeLists.txt"),
        format!("{lists}\nset(CHECK_SOURCES ${{PROGRAM_SOURCES}})\nlist(FILTER CHECK_SOURCES EXCLUDE REGEX \"src/main.c$\")\nadd_executable(board_check ${{CHECK_SOURCES}} check/board_check.c)\ntarget_include_directories(board_check PRIVATE src)\ntarget_link_libraries(board_check PRIVATE clarity clarity_shell)\n"),
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
fn questions_and_their_answers_build_in_c() {
    let project = clarity_spec::keyed::load_keyed([("music.clarity", include_str!("fixtures/asks.clarity"))], &[], "en");
    let mut settings = clarity_project::starter("Music", "music");
    settings.stack.backend = Backend::C;
    settings.stack.ui = Ui::Web;
    let out = scratch().join(format!("clarity-gen-c-asks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    clarity_gen::write_out(&out, &files, &["build", "data"]).unwrap();
    let said = run(&out, "cmake", &["--workflow", "--preset", "debug"]);
    let own: Vec<&str> = said.lines().filter(|l| l.contains("warning") && l.contains("/src/")).collect();
    assert!(own.is_empty(), "warnings:\n{}", own.join("\n"));
    std::fs::remove_dir_all(&out).unwrap();
}
