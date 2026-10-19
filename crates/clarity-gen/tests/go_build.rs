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
