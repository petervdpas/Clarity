//! Generates the shop example as C# with Avalonia and builds it with
//! `dotnet build` (the AXAML compiler checks every binding), then runs it with
//! `--check`: the program opens its database, applies the migration steps and
//! stops before showing a window. Slow (NuGet and the compiler), so it only
//! runs when asked, one at a time:
//! `cargo test -p clarity-gen --test csharp_build -- --ignored --test-threads=1 --exact <name>`.
//!
//! The projects build on the runtime's own projects in runtimes/csharp (the
//! CLARITY_RUNTIME switch of the generated project file), so a change to the
//! runtime is tested before it is published; one test builds on the packages
//! on nuget.org, as a project outside this repository does.

use std::path::Path;
use std::process::Command;

use clarity_project::{Backend, SourceKind, Ui};

mod scratch;
use scratch::empty;

/// Which runtime a generated project builds on.
#[derive(Clone, Copy)]
enum Runtime {
    /// The runtime's projects in this repository.
    Local,
    /// The published packages.
    Packages,
}

/// `dotnet` with `args` in `out`, on `runtime`.
fn dotnet(out: &Path, args: &[&str], runtime: Runtime) -> Command {
    let mut command = Command::new("dotnet");
    command.args(args).current_dir(out);
    match runtime {
        Runtime::Local => command.env("CLARITY_RUNTIME", Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/csharp")),
        Runtime::Packages => command.env_remove("CLARITY_RUNTIME"),
    };
    command
}

/// Generates the example `name` (examples/<name>.clarity) as C# with
/// Avalonia, builds it with warnings as errors and runs `--check`.
fn builds_and_runs_in_csharp(name: &str, program: &str, translated: &str, runtime: Runtime) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join(format!("examples/{name}.clarity"));
    // As Run loads it: with the project file's language, currency and themes.
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    // Its translations, whose satellite assembly the build must make.
    assert!(!project.translations.is_empty(), "the example has translations");
    let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = clarity_gen::generate(&project, &settings).unwrap();
    assert!(files.iter().any(|f| f.path == format!("Texts.{translated}.resx")));

    let out = std::env::temp_dir().join(format!("clarity-gen-csharp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();

    let built = dotnet(&out, &["build", "--nologo", "-v", "quiet", "-warnaserror"], runtime).output().expect("the .NET SDK is installed");
    assert!(
        built.status.success(),
        "the generated project does not build; it is in {}\n{}",
        out.display(),
        String::from_utf8_lossy(&built.stdout)
    );

    let ran = dotnet(&out, &["run", "--no-build", "--", "--check"], runtime).output().unwrap();
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(ran.status.success(), "{said}{}", String::from_utf8_lossy(&ran.stderr));
    assert!(said.contains(&format!("{program}: the databases are ready.")), "{said}");
    // The database the project file names, with every step applied.
    assert!(out.join(format!("data/{name}.db")).exists());
    // The translated texts as a satellite assembly, as .NET looks for them.
    assert!(out.join(format!("bin/Debug/net10.0/{translated}/{program}.resources.dll")).exists());
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore]
fn the_shop_example_builds_and_runs_in_csharp() {
    builds_and_runs_in_csharp("shop", "Shop", "nl", Runtime::Local);
}

/// The shop on the runtime's packages from nuget.org, at the version the
/// generator names: what a project outside this repository builds on.
#[test]
#[ignore]
fn the_shop_example_builds_and_runs_on_the_published_packages() {
    builds_and_runs_in_csharp("shop", "Shop", "nl", Runtime::Packages);
}

/// The second example: master and detail, dialogs, a tree of genres, loans.
#[test]
#[ignore]
fn the_records_example_builds_and_runs_in_csharp() {
    builds_and_runs_in_csharp("records", "Records", "de", Runtime::Local);
}

/// The third example: tasks edited in their table, and the people who finish them.
#[test]
#[ignore]
fn the_todo_example_builds_and_runs_in_csharp() {
    builds_and_runs_in_csharp("todo", "Todo", "nl", Runtime::Local);
}

/// The shop on each server database: the project builds with only that
/// driver, and where a scratch database is named (the variables of
/// tests/servers.rs) the program migrates it with `--check`, twice.
#[test]
#[ignore]
fn the_shop_example_builds_and_migrates_on_every_server_database() {
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
        settings.stack.backend = Backend::Csharp;
        settings.stack.ui = Ui::Avalonia;
        settings.datasources[0].kind = kind;
        settings.datasources[0].url = "env:SHOP_DATABASE".into();
        let files = clarity_gen::generate(&project, &settings).unwrap();

        let out = std::env::temp_dir().join(format!("clarity-gen-csharp-{variable}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();
        let built = dotnet(&out, &["build", "--nologo", "-v", "quiet", "-warnaserror"], Runtime::Local).output().unwrap();
        assert!(built.status.success(), "{variable}: the generated project does not build; it is in {}\n{}", out.display(), String::from_utf8_lossy(&built.stdout));

        if let Ok(at) = std::env::var(variable) {
            empty(&at, dialect, &tables);
            for _ in 0..2 {
                let ran = dotnet(&out, &["run", "--no-build", "--", "--check"], Runtime::Local)
                    .env("SHOP_DATABASE", &at)
                    .output()
                    .unwrap();
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
