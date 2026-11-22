//! The output folder holds one program per target (manual/code.commands.md):
//! `out/desktop/` and `out/cli/`, each a project of its own. Output written
//! before, with the desktop program in `out/` itself, moves: the build
//! tools' folders and the data go to `out/desktop/`, the rest is made again.

use std::path::{Path, PathBuf};

use clarity_gen::{OutFile, Program};
use clarity_project::{Backend, Target};

fn file(path: &str, text: &str) -> OutFile {
    OutFile { path: path.to_string(), text: text.to_string(), bytes: None }
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn each_target_is_written_into_its_own_folder() {
    let out = scratch("out-layout-targets");
    let programs = [
        Program { target: Target::Desktop, files: vec![file("Cargo.toml", "desktop"), file("src/main.rs", "fn main() {}")] },
        Program { target: Target::Cli, files: vec![file("Cargo.toml", "cli")] },
    ];
    clarity_gen::write_programs(&out, &programs, Backend::Rust, &[]).unwrap();
    assert_eq!(read(&out.join("desktop/Cargo.toml")), "desktop");
    assert_eq!(read(&out.join("desktop/src/main.rs")), "fn main() {}");
    assert_eq!(read(&out.join("cli/Cargo.toml")), "cli");
    assert!(out.join(clarity_gen::OUT_MARKER).is_file());

    // A target no longer generated goes, with its build output.
    std::fs::create_dir_all(out.join("cli/target")).unwrap();
    clarity_gen::write_programs(&out, &programs[..1], Backend::Rust, &[]).unwrap();
    assert!(!out.join("cli").exists());
    assert!(out.join("desktop/Cargo.toml").is_file());
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn output_in_the_old_place_moves_to_the_desktop_folder() {
    let out = scratch("out-layout-old");
    std::fs::create_dir_all(out.join("target/debug")).unwrap();
    std::fs::create_dir_all(out.join("data")).unwrap();
    std::fs::create_dir_all(out.join("src")).unwrap();
    std::fs::write(out.join(clarity_gen::OUT_MARKER), "").unwrap();
    std::fs::write(out.join("Cargo.toml"), "old").unwrap();
    std::fs::write(out.join("src/main.rs"), "old").unwrap();
    std::fs::write(out.join("target/debug/shop"), "built").unwrap();
    std::fs::write(out.join("data/shop.db"), "rows").unwrap();
    let programs = [Program { target: Target::Desktop, files: vec![file("Cargo.toml", "new")] }];
    clarity_gen::write_programs(&out, &programs, Backend::Rust, &[]).unwrap();
    assert_eq!(read(&out.join("desktop/Cargo.toml")), "new");
    assert_eq!(read(&out.join("desktop/target/debug/shop")), "built");
    assert_eq!(read(&out.join("desktop/data/shop.db")), "rows");
    assert!(!out.join("Cargo.toml").exists());
    assert!(!out.join("src").exists());
    assert!(!out.join("target").exists());
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn a_folder_clarity_did_not_make_is_refused() {
    let out = scratch("out-layout-foreign");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("notes.txt"), "mine").unwrap();
    let programs = [Program { target: Target::Desktop, files: vec![file("Cargo.toml", "new")] }];
    assert!(clarity_gen::write_programs(&out, &programs, Backend::Rust, &[]).is_err());
    assert_eq!(read(&out.join("notes.txt")), "mine");
    std::fs::remove_dir_all(&out).unwrap();
}

/// A program served to a browser alone (the web target) is written into
/// `out/web`; with a desktop window too it is the desktop program.
#[test]
fn a_program_served_alone_is_the_web_program() {
    let project = clarity_spec::keyed::load_keyed([("music.clarity", include_str!("../fixtures/asks.clarity"))], &[], "en");
    let program_of = |backend: Backend, targets: Vec<Target>| {
        let mut settings = clarity_project::starter("A", "a");
        settings.stack.backend = backend;
        settings.stack.ui = clarity_project::usual_ui(backend);
        settings.stack.targets = targets;
        clarity_gen::programs(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}")).iter().map(|p| p.target).collect::<Vec<_>>()
    };
    assert_eq!(program_of(Backend::Rust, vec![Target::Web]), [Target::Web]);
    // Beside the desktop target, the desktop program of Rust and Go serves the web too (--serve).
    assert_eq!(program_of(Backend::Go, vec![Target::Desktop, Target::Web]), [Target::Desktop]);
    assert_eq!(program_of(Backend::Rust, vec![Target::Web, Target::Cli]), [Target::Web, Target::Cli]);
}

/// With `include migrate` ("Include data migration tools", docs/ledger/migrations.md)
/// the output folder holds Clarity's migration tool, copied from beside Clarity,
/// and the databases it migrates (`clarity-migrate.json`); generating again keeps
/// them, and without the line they go.
#[test]
fn the_output_holds_the_data_migration_tools_when_the_project_includes_them() {
    let out = scratch("out-layout-migrate");
    let clarity = scratch("out-layout-migrate-clarity");
    let tool = clarity_gen::migrate_program_name();
    std::fs::create_dir_all(&clarity).unwrap();
    std::fs::write(clarity.join(&tool), "the tool").unwrap();
    let project = clarity_spec::load_converted([("shop.clarity", "module shop\n\nentity Customer\n    name: text(80)\n")]);
    let mut settings = clarity_project::starter("Shop", "shop");
    let programs = [Program { target: Target::Desktop, files: vec![file("Cargo.toml", "desktop")] }];
    clarity_gen::write_programs(&out, &programs, Backend::Rust, &[]).unwrap();

    settings.include_migrate = true;
    let written = clarity_gen::write_migration_tools(&out, &project, &settings, &clarity).unwrap();
    assert_eq!(written, [clarity_gen::MIGRATE_FILE.to_string(), tool.clone()]);
    assert_eq!(read(&out.join(&tool)), "the tool");
    assert!(read(&out.join(clarity_gen::MIGRATE_FILE)).contains("\"source\": \"main\""), "{}", read(&out.join(clarity_gen::MIGRATE_FILE)));
    // Written again unchanged, nothing is said to change.
    assert_eq!(clarity_gen::write_migration_tools(&out, &project, &settings, &clarity).unwrap(), Vec::<String>::new());

    // Generating the programs again leaves them where they are.
    clarity_gen::write_programs(&out, &programs, Backend::Rust, &[]).unwrap();
    assert_eq!(read(&out.join(&tool)), "the tool");
    assert!(out.join(clarity_gen::MIGRATE_FILE).is_file());

    // Not beside Clarity, the tool cannot be included: Generate says so.
    let nowhere = scratch("out-layout-migrate-nowhere");
    let refused = clarity_gen::write_migration_tools(&out, &project, &settings, &nowhere).unwrap_err();
    assert_eq!(refused, format!("the project includes the data migration tools, but {tool} is not beside Clarity in {}", nowhere.display()));

    // Without the line, they go.
    settings.include_migrate = false;
    clarity_gen::write_migration_tools(&out, &project, &settings, &nowhere).unwrap();
    assert!(!out.join(&tool).exists() && !out.join(clarity_gen::MIGRATE_FILE).exists());
    std::fs::remove_dir_all(&out).unwrap();
    std::fs::remove_dir_all(&clarity).unwrap();
}
