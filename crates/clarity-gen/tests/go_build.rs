//! Generates the examples as Go and checks the module as a Go developer
//! would: gofmt has nothing to change, go mod tidy has no requirement to
//! add or drop, go vet finds nothing, go build builds it, and the program runs with
//! `--check` (it opens its database, applies the migration steps and stops). Slow (the Go toolchain and the
//! module cache), so it only runs when asked:
//! `cargo test -p clarity-gen --test go_build -- --ignored --test-threads=1 --exact <name>`.
//!
//! The modules build on the runtime in runtimes/go through a go.work next to
//! them (Go's workspace, the switch a developer of the runtime uses), so a
//! change to the runtime is tested before it is published; one test builds
//! on the published runtime module, as a module outside this repository
//! does, and one records the published module's sums.

use std::path::Path;
use std::process::Command;

use clarity_project::{Backend, SourceKind, Ui};

mod scratch;
use scratch::empty;

/// Clarity's Go runtime module.
const RUNTIME: &str = "github.com/petervdpas/Clarity/runtimes/go";

/// Which runtime a generated module builds on.
#[derive(Clone, Copy, PartialEq)]
enum Runtime {
    /// The runtime in this repository, through a go.work.
    Local,
    /// The published module.
    Published,
}

fn runtime_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/go").canonicalize().unwrap()
}

/// `tool` with `args` in `dir`; it must succeed. GOWORK is left to the
/// go.work in `dir`, which only a local build has.
fn run(dir: &Path, tool: &str, args: &[&str]) -> String {
    let out = Command::new(tool).args(args).current_dir(dir).env_remove("GOWORK").output().unwrap_or_else(|e| panic!("{tool}: {e}"));
    let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "{tool} {args:?} failed in {}:\n{said}", dir.display());
    said
}

/// Writes the module into a new scratch folder `out`, on `runtime`.
fn write(out: &Path, files: &[clarity_gen::OutFile], runtime: Runtime) {
    let _ = std::fs::remove_dir_all(out);
    std::fs::create_dir_all(out).unwrap();
    clarity_gen::write_out(out, files, &["data"]).unwrap();
    if runtime == Runtime::Local {
        let go = include_str!("../../../runtimes/go/go.mod").lines().find(|l| l.starts_with("go ")).unwrap();
        // As `go work init . && go work edit -replace RUNTIME=<runtimes/go>` writes it.
        std::fs::write(out.join("go.work"), format!("{go}\n\nuse .\n\nreplace {RUNTIME} => {}\n", runtime_dir().display())).unwrap();
    }
}

/// What a Go developer would check first: gofmt and go mod tidy have nothing
/// to change in the code and the requirements, and go vet finds nothing.
/// go.sum may hold sums the build never reads (it is the runtime's, less the
/// drivers left out); go build fails on one that is missing. Before the
/// runtime's version is published, tidy reads it from this repository
/// through a replace, added for the check only.
fn checks(out: &Path, runtime: Runtime) {
    assert_eq!(run(out, "gofmt", &["-l", "."]), "", "gofmt would change these files");
    let module = std::fs::read_to_string(out.join("go.mod")).unwrap();
    if runtime == Runtime::Local {
        std::fs::write(out.join("go.mod"), format!("{module}\nreplace {RUNTIME} => {}\n", runtime_dir().display())).unwrap();
    }
    let tidy = Command::new("go").args(["mod", "tidy", "-diff"]).current_dir(out).env("GOWORK", "off").output().unwrap();
    std::fs::write(out.join("go.mod"), &module).unwrap();
    let said = String::from_utf8_lossy(&tidy.stdout);
    assert!(!said.contains("current/go.mod"), "go mod tidy would change the requirements:\n{said}{}", String::from_utf8_lossy(&tidy.stderr));
    run(out, "go", &["vet", "./..."]);
}

fn builds_and_runs_in_go(name: &str, program: &str, runtime: Runtime) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join(format!("examples/{name}.clarity"));
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
    settings.stack.backend = Backend::Go;
    settings.stack.ui = Ui::Web;
    let out = std::env::temp_dir().join(format!("clarity-gen-go-{name}-{}", std::process::id()));
    // The database in the scratch folder, not in the example.
    settings.datasources[0].url = "data/app.db".into();
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    write(&out, &files, runtime);

    checks(&out, runtime);
    run(&out, "go", &["build", "-o", name, "."]);
    let said = run(&out, &out.join(name).display().to_string(), &["--check"]);
    assert!(said.contains(&format!("{program}: the databases are ready.")), "{said}");
    assert!(out.join("data/app.db").exists());
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore]
fn the_shop_example_builds_and_runs_in_go() {
    builds_and_runs_in_go("shop", "Shop", Runtime::Local);
}

/// The shop on the published runtime module, at the version the generator
/// requires, with its recorded sums: what a module outside this repository
/// builds on.
#[test]
#[ignore]
fn the_shop_example_builds_and_runs_on_the_published_runtime() {
    builds_and_runs_in_go("shop", "Shop", Runtime::Published);
}

/// The sums of the published runtime version, as the Go checksum database
/// knows them, are the ones the generator writes into go.sum. With
/// CLARITY_RECORD_GO_SUMS=1 they are recorded in
/// crates/clarity-gen/src/go/runtime.sum (once, after tagging a version).
#[test]
#[ignore]
fn the_published_runtime_sums_are_recorded() {
    let version = clarity_gen::go_runtime_version();
    let scratch = std::env::temp_dir().join(format!("clarity-gen-go-sums-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    let out = Command::new("go")
        .args(["mod", "download", "-json", &format!("{RUNTIME}@{version}")])
        .current_dir(&scratch)
        .env("GOWORK", "off")
        .output()
        .unwrap();
    std::fs::remove_dir_all(&scratch).unwrap();
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(out.status.success(), "{RUNTIME}@{version} is not published yet:\n{said}{}", String::from_utf8_lossy(&out.stderr));
    let field = |name: &str| {
        let at = said.find(&format!("\"{name}\": \"")).unwrap_or_else(|| panic!("no {name} in {said}")) + name.len() + 5;
        said[at..at + said[at..].find('"').unwrap()].to_string()
    };
    let lines = format!("{RUNTIME} {version} {}\n{RUNTIME} {version}/go.mod {}\n", field("Sum"), field("GoModSum"));
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/go/runtime.sum");
    let recorded = std::fs::read_to_string(&path).unwrap();
    if std::env::var("CLARITY_RECORD_GO_SUMS").is_ok() && !recorded.contains(&lines) {
        std::fs::write(&path, format!("{recorded}{lines}")).unwrap();
        return;
    }
    assert!(recorded.contains(&lines), "{} lacks\n{lines}record them with CLARITY_RECORD_GO_SUMS=1", path.display());
}

#[test]
#[ignore]
fn the_records_example_builds_and_runs_in_go() {
    builds_and_runs_in_go("records", "Records", Runtime::Local);
}

/// The third example: tasks edited in their table, and the people who finish them.
#[test]
#[ignore]
fn the_todo_example_builds_and_runs_in_go() {
    builds_and_runs_in_go("todo", "Todo", Runtime::Local);
}

/// The shop on each server database: the module has only that driver and
/// its modules, and where a scratch database is named (the variables of
/// tests/servers.rs) the program migrates it with `--check`, twice.
#[test]
#[ignore]
fn the_shop_example_builds_and_migrates_on_every_server_database_in_go() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join("examples/shop.clarity");
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let tables: Vec<String> = project.entities().map(|e| e.table_name()).collect();
    for (kind, dialect, variable) in [
        (SourceKind::Postgres, clarity_runtime::Dialect::Postgres, "CLARITY_TEST_POSTGRES"),
        (SourceKind::Mysql, clarity_runtime::Dialect::Mysql, "CLARITY_TEST_MYSQL"),
        (SourceKind::Mssql, clarity_runtime::Dialect::Mssql, "CLARITY_TEST_MSSQL"),
    ] {
        let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
        settings.stack.backend = Backend::Go;
        settings.stack.ui = Ui::Web;
        settings.datasources[0].kind = kind;
        settings.datasources[0].url = "env:SHOP_DATABASE".into();
        let files = clarity_gen::generate(&project, &settings).unwrap();

        let out = std::env::temp_dir().join(format!("clarity-gen-go-{variable}-{}", std::process::id()));
        write(&out, &files, Runtime::Local);
        checks(&out, Runtime::Local);
        run(&out, "go", &["build", "-o", "shop", "."]);

        if let Ok(at) = std::env::var(variable) {
            empty(&at, dialect, &tables);
            for _ in 0..2 {
                let ran = Command::new(out.join("shop")).arg("--check").current_dir(&out).env("SHOP_DATABASE", &at).output().unwrap();
                let said = String::from_utf8_lossy(&ran.stdout);
                assert!(ran.status.success(), "{variable}: {said}{}", String::from_utf8_lossy(&ran.stderr));
                assert!(said.contains("Shop: the databases are ready."), "{said}");
            }
            let db = clarity_runtime::Db::connect(&[("main", dialect, &at)]).unwrap();
            let steps = project.migrations.as_ref().map_or(0, |m| m.get("main").map_or(0, Vec::len));
            assert_eq!(db.schema_version().unwrap(), steps, "{variable}: every step once");
            empty(&at, dialect, &tables);
        } else {
            eprintln!("{variable} is not set; built only");
        }
        std::fs::remove_dir_all(&out).unwrap();
    }
}

/// The test engine of conformance/engines.json in a generated application:
/// every function called from Clarity code, its values crossing as JSON.
const ENGINE_SHOP: &str = include_str!("../../../conformance/engines/shop.clarity");
const TEST_ENGINE: &[u8] = include_bytes!("../../../conformance/engines/test.wasm");

const ENGINE_TEST: &str = r#"package main

import (
	"context"
	"encoding/json"
	"testing"

	"github.com/petervdpas/Clarity/runtimes/go/clarity"
	"github.com/petervdpas/Clarity/runtimes/go/shell"
	"github.com/shopspring/decimal"
)

// Each open window has an instance of its own of an engine with state per
// window, through the shell as the screen calls it, and the window's
// instance goes when it closes.
func TestEachOpenWindowHasItsOwnEngine(t *testing.T) {
	db, err := clarity.Memory()
	if err != nil {
		t.Fatal(err)
	}
	s := shell.New(shell.Options{Name: "shop", Main: "Main", Model: model, Db: db, Windows: windows{}})
	invoke := func(command string, args any) clarity.Reply {
		t.Helper()
		text, _ := json.Marshal(args)
		answer := s.Invoke(context.Background(), command, text)
		if answer.Error != nil {
			t.Fatalf("%s: %v", command, answer.Error)
		}
		return answer.Value.(clarity.Reply)
	}
	moves := func(r clarity.Reply) float64 { return r.State.Data["moves"].(float64) }
	a := invoke("open_window", map[string]any{"window": "Main", "with": map[string]any{}})
	b := invoke("open_window", map[string]any{"window": "Main", "with": map[string]any{}})
	a = invoke("dispatch_event", map[string]any{"state": a.State, "handler": "moved"})
	b = invoke("dispatch_event", map[string]any{"state": b.State, "handler": "moved"})
	a = invoke("dispatch_event", map[string]any{"state": a.State, "handler": "moved"})
	if moves(a) != 12 || moves(b) != 11 {
		t.Fatalf("moves %v and %v", moves(a), moves(b))
	}
	invoke("close_window", map[string]any{"state": a.State})
	a = invoke("dispatch_event", map[string]any{"state": a.State, "handler": "moved"})
	b = invoke("dispatch_event", map[string]any{"state": b.State, "handler": "moved"})
	if moves(a) != 11 || moves(b) != 12 {
		t.Fatalf("after closing, moves %v and %v", moves(a), moves(b))
	}
}

func TestEveryEngineFunctionAnswersFromClarityCode(t *testing.T) {
	if err := checkEngines(); err != nil {
		t.Fatal(err)
	}
	if got, err := Answer(nil, 2); err != nil || got != 42 {
		t.Fatal(got, err)
	}
	if got, err := Greeting(nil, "Grüße"); err != nil || got != "Hello, Grüße!" {
		t.Fatal(got, err)
	}
	lines := []Line{{Price: decimal.RequireFromString("9.95"), Count: 3}, {Price: decimal.RequireFromString("0.10"), Count: 1}}
	if got, err := Sum(nil, lines); err != nil || !got.Equal(decimal.RequireFromString("29.95")) {
		t.Fatal(got, err)
	}
	if got, err := SizeClass(nil, 12); err != nil || got != SizeLarge {
		t.Fatal(got, err)
	}
	order := &Order{ID: 7, Number: "ORD-7", Total: decimal.RequireFromString("100.05")}
	if got, err := Cheaper(nil, order); err != nil || got.ID != 7 || !got.Total.Equal(decimal.RequireFromString("90.05")) {
		t.Fatal(got, err)
	}
	// The engine's own error reads as a refusal, in its words.
	if _, err := Refused(nil, "no stock"); err == nil || err.Error() != "no stock" {
		t.Fatal(err)
	}
	// Bytes cross to an engine and back as they are.
	if back, err := Backwards(nil, []byte{1, 2, 3}); err != nil || string(back) != string([]byte{3, 2, 1}) {
		t.Fatal(back, err)
	}
	if n, err := Measured(nil, []byte{1, 2, 3}); err != nil || n != 6 {
		t.Fatal(n, err)
	}
	// An engine with state counts on from what its on open set.
	first, err1 := Counted(nil)
	second, err2 := Counted(nil)
	if err1 != nil || err2 != nil || first != 11 || second != 12 {
		t.Fatalf("counted %d, %d: %v %v", first, second, err1, err2)
	}
}
"#;

#[test]
#[ignore]
fn an_engine_answers_in_the_generated_application_in_go() {
    let mut project = clarity_spec::load_sources([("shop.clarity", ENGINE_SHOP)]);
    clarity_spec::engines::attach(&mut project, vec![clarity_spec::engines::EngineFile { name: "test.wasm".into(), bytes: TEST_ENGINE.to_vec() }]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = clarity_project::starter("Shop", "shop");
    settings.stack.backend = clarity_project::Backend::Go;
    settings.stack.ui = clarity_project::Ui::Web;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = std::env::temp_dir().join(format!("clarity-gen-go-engine-{}", std::process::id()));
    write(&out, &files, Runtime::Local);
    checks(&out, Runtime::Local);
    std::fs::write(out.join("engine_test.go"), ENGINE_TEST).unwrap();
    run(&out, "go", &["test", "./..."]);
    std::fs::remove_dir_all(&out).unwrap();
}

/// Functions named like a type, an enum value, a constructor or a window
/// build beside them, and are called by their own names.
const NAME_CLASH_TEST: &str = r#"package main

import "testing"

func TestEveryFunctionKeepsItsOwnName(t *testing.T) {
	if s, err := Size_(nil, 12); err != nil || s != SizeLarge {
		t.Fatalf("Size_(12) = %v, %v", s, err)
	}
	if s, err := SizeSmall_(nil); err != nil || s != SizeSmall {
		t.Fatalf("SizeSmall_() = %v, %v", s, err)
	}
	if n, err := Sources_(nil); err != nil || n != 1 {
		t.Fatalf("Sources_() = %v, %v", n, err)
	}
}
"#;

#[test]
#[ignore]
fn functions_named_like_types_build_in_go() {
    let project = clarity_spec::load_sources([("shop.clarity", include_str!("fixtures/name_clash.clarity"))]);
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let mut settings = clarity_project::starter("Shop", "shop");
    settings.stack.backend = clarity_project::Backend::Go;
    settings.stack.ui = clarity_project::Ui::Web;
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    let out = std::env::temp_dir().join(format!("clarity-gen-go-names-{}", std::process::id()));
    write(&out, &files, Runtime::Local);
    checks(&out, Runtime::Local);
    std::fs::write(out.join("names_test.go"), NAME_CLASH_TEST).unwrap();
    run(&out, "go", &["test", "./..."]);
    std::fs::remove_dir_all(&out).unwrap();
}
