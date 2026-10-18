//! Generates the examples as Go and checks the module as a Go developer
//! would: gofmt has nothing to change, go mod tidy has no requirement to
//! add or drop, go vet finds nothing, go build builds it, and the program runs with
//! `--check` (it opens its database, applies the migration steps and stops). Slow (the Go toolchain and the
//! module cache), so it only runs when asked:
//! `cargo test -p clarity-gen --test go_build -- --ignored`.

use std::path::Path;
use std::process::Command;

use clarity_project::{Backend, SourceKind, Ui};

mod scratch;
use scratch::empty;

fn run(dir: &Path, tool: &str, args: &[&str]) -> String {
    let out = Command::new(tool).args(args).current_dir(dir).output().unwrap_or_else(|e| panic!("{tool}: {e}"));
    let said = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "{tool} {args:?} failed in {}:\n{said}", dir.display());
    said
}

/// What a Go developer would check first: gofmt and go mod tidy have nothing
/// to change in the code and the requirements, and go vet finds nothing.
/// go.sum may hold sums the build never reads (it is the runtime's, less the
/// drivers left out); go build fails on one that is missing.
fn checks(out: &Path) {
    assert_eq!(run(out, "gofmt", &["-l", "."]), "", "gofmt would change these files");
    let tidy = Command::new("go").args(["mod", "tidy", "-diff"]).current_dir(out).output().unwrap();
    let said = String::from_utf8_lossy(&tidy.stdout);
    assert!(!said.contains("current/go.mod"), "go mod tidy would change the requirements:\n{said}");
    run(out, "go", &["vet", "./..."]);
}

fn builds_and_runs_in_go(name: &str, program: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join(format!("examples/{name}.clarity"));
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
    settings.stack.backend = Backend::Go;
    settings.stack.ui = Ui::Web;
    let out = std::env::temp_dir().join(format!("clarity-gen-go-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    // The database in the scratch folder, not in the example.
    settings.datasources[0].url = "data/app.db".into();
    let files = clarity_gen::generate(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}"));
    clarity_gen::write_out(&out, &files, &["data"]).unwrap();

    checks(&out);
    run(&out, "go", &["build", "-o", name, "."]);
    let said = run(&out, &out.join(name).display().to_string(), &["--check"]);
    assert!(said.contains(&format!("{program}: the databases are ready.")), "{said}");
    assert!(out.join("data/app.db").exists());
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore]
fn the_shop_example_builds_and_runs_in_go() {
    builds_and_runs_in_go("shop", "Shop");
}

#[test]
#[ignore]
fn the_records_example_builds_and_runs_in_go() {
    builds_and_runs_in_go("records", "Records");
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
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        clarity_gen::write_out(&out, &files, &["data"]).unwrap();
        checks(&out);
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
