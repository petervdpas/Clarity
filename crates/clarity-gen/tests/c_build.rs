//! Generates the examples as C and builds them as a C developer would:
//! `cmake --workflow --preset debug` configures and builds without a
//! warning from the program's own code, and the program runs with `--check`
//! (it opens its database, applies the migration steps and stops). Slow
//! (a C build), so it only runs when asked:
//! `cargo test -p clarity-gen --test c_build -- --ignored`.

use std::path::Path;
use std::process::Command;

use clarity_project::{Backend, Ui};

fn run(dir: &Path, tool: &str, args: &[&str]) -> String {
    let out = Command::new(tool).args(args).current_dir(dir).output().unwrap_or_else(|e| panic!("{tool}: {e}"));
    let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "{tool} {args:?} failed in {}:\n{said}", dir.display());
    said
}

fn builds_and_runs_in_c(name: &str, program: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join(format!("examples/{name}.clarity"));
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
    settings.stack.backend = Backend::C;
    settings.stack.ui = Ui::Web;
    let out = std::env::temp_dir().join(format!("clarity-gen-c-{name}-{}", std::process::id()));
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
    let said = run(&out, &out.join("build/debug").join(name).display().to_string(), &["--check"]);
    assert!(said.contains(&format!("{program}: the databases are ready.")), "{said}");
    assert!(out.join("data/app.db").exists());
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore]
fn the_shop_example_builds_and_runs_in_c() {
    builds_and_runs_in_c("shop", "Shop");
}

#[test]
#[ignore]
fn the_records_example_builds_and_runs_in_c() {
    builds_and_runs_in_c("records", "Records");
}

/// The third example: tasks edited in their table, and the people who finish them.
#[test]
#[ignore]
fn the_todo_example_builds_and_runs_in_c() {
    builds_and_runs_in_c("todo", "Todo");
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
    let out = std::env::temp_dir().join(format!("clarity-gen-c-engine-{}", std::process::id()));
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
