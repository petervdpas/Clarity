//! Generates the shop example as C# with Avalonia and builds it with
//! `dotnet build` (the AXAML compiler checks every binding), then runs it with
//! `--check`: the program opens its database, applies the migration steps and
//! stops before showing a window. Slow (NuGet and the compiler), so it only
//! runs when asked: `cargo test -p clarity-gen --test csharp_build -- --ignored`.

use std::path::Path;
use std::process::Command;

use clarity_project::{Backend, SourceKind, Ui};

/// Generates the example `name` (examples/<name>.clarity) as C# with
/// Avalonia, builds it with warnings as errors and runs `--check`.
fn builds_and_runs_in_csharp(name: &str, program: &str, translated: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = root.join(format!("examples/{name}.clarity"));
    // As Run loads it: with the project file's language, currency and themes.
    let project = clarity_project::load_modules(&clarity_project::module_dir(&file)).unwrap();
    assert!(!project.themes.is_empty() && !project.translations.is_empty(), "the example has themes and translations");
    let mut settings = clarity_project::load(&file).settings.expect("the example has a project file");
    settings.stack.backend = Backend::Csharp;
    settings.stack.ui = Ui::Avalonia;
    let files = clarity_gen::generate(&project, &settings).unwrap();
    assert!(files.iter().any(|f| f.path == format!("Texts.{translated}.resx")));

    let out = std::env::temp_dir().join(format!("clarity-gen-csharp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["bin", "obj"]).unwrap();

    let built = Command::new("dotnet")
        .args(["build", "--nologo", "-v", "quiet", "-warnaserror"])
        .current_dir(&out)
        .output()
        .expect("the .NET SDK is installed");
    assert!(
        built.status.success(),
        "the generated project does not build; it is in {}\n{}",
        out.display(),
        String::from_utf8_lossy(&built.stdout)
    );

    let ran = Command::new("dotnet").args(["run", "--no-build", "--", "--check"]).current_dir(&out).output().unwrap();
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
    builds_and_runs_in_csharp("shop", "Shop", "nl");
}

/// The second example: master and detail, dialogs, a tree of genres, loans.
#[test]
#[ignore]
fn the_records_example_builds_and_runs_in_csharp() {
    builds_and_runs_in_csharp("records", "Records", "de");
}

/// Empties a scratch database: drops every table of `tables` and the
/// migration record, as often as references between them ask.
fn empty(at: &str, dialect: clarity_runtime::Dialect, tables: &[String]) {
    let db = clarity_runtime::Db::connect(&[("main", dialect, at)]).unwrap();
    // The tables of the server scenario (tests/servers.rs) may be there too.
    let mut left: Vec<String> = tables.iter().cloned().chain(["purchase", "order", "clarity_schema"].map(String::from)).collect();
    left.sort_unstable();
    left.dedup();
    for _ in 0..=left.len() {
        left.retain(|t| db.batch("main", &format!("DROP TABLE IF EXISTS {}", dialect.quote(t))).is_err());
    }
    assert!(left.is_empty(), "cannot drop {left:?}");
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
        let built = Command::new("dotnet").args(["build", "--nologo", "-v", "quiet", "-warnaserror"]).current_dir(&out).output().unwrap();
        assert!(built.status.success(), "{variable}: the generated project does not build; it is in {}\n{}", out.display(), String::from_utf8_lossy(&built.stdout));

        if let Ok(at) = std::env::var(variable) {
            empty(&at, dialect, &tables);
            for _ in 0..2 {
                let ran = Command::new("dotnet")
                    .args(["run", "--no-build", "--", "--check"])
                    .current_dir(&out)
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
