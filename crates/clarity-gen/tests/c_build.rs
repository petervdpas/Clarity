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
