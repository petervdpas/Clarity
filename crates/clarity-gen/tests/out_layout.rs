//! The output folder holds one program per target (DESIGN 5.20):
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
    let project = clarity_spec::keyed::load_keyed([("music.clarity", include_str!("fixtures/asks.clarity"))], &[], "en");
    let program_of = |targets: Vec<Target>| {
        let mut settings = clarity_project::starter("A", "a");
        settings.stack.targets = targets;
        clarity_gen::programs(&project, &settings).unwrap_or_else(|e| panic!("{e:#?}")).iter().map(|p| p.target).collect::<Vec<_>>()
    };
    assert_eq!(program_of(vec![Target::Web]), [Target::Web]);
    assert_eq!(program_of(vec![Target::Desktop, Target::Web]), [Target::Desktop]);
    assert_eq!(program_of(vec![Target::Web, Target::Cli]), [Target::Web, Target::Cli]);
}
