// Run builds only when something changed (every stack): a build records
// what it was made from beside the program, and Run starts the program as
// built while the generated files are still the same.

use clarity_gen::{OutFile, Program};
use clarity_project::Target;
use std::path::{Path, PathBuf};

fn file(path: &str, text: &str) -> OutFile {
    OutFile { path: path.into(), text: text.into(), bytes: None }
}

fn program(files: Vec<OutFile>) -> Program {
    Program { target: Target::Desktop, files }
}

/// Where builds go: target/generated in the repository, on disk.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_same_files_are_the_same_build_and_any_change_is_another() {
    let one = program(vec![file("src/main.rs", "fn main() {}"), file("Cargo.toml", "[package]")]);
    let same = program(vec![file("src/main.rs", "fn main() {}"), file("Cargo.toml", "[package]")]);
    assert_eq!(clarity_gen::fingerprint(&one), clarity_gen::fingerprint(&same));
    let changed = program(vec![file("src/main.rs", "fn main() { }"), file("Cargo.toml", "[package]")]);
    assert_ne!(clarity_gen::fingerprint(&one), clarity_gen::fingerprint(&changed));
    let more = program(vec![file("src/main.rs", "fn main() {}"), file("Cargo.toml", "[package]"), file("README.md", "")]);
    assert_ne!(clarity_gen::fingerprint(&one), clarity_gen::fingerprint(&more));
    // A picture is its bytes.
    let picture = |bytes: &'static [u8]| program(vec![OutFile { path: "logo.png".into(), text: String::new(), bytes: Some(bytes.into()) }]);
    assert_ne!(clarity_gen::fingerprint(&picture(&[1, 2])), clarity_gen::fingerprint(&picture(&[1, 3])));
}

#[test]
fn a_build_is_known_until_the_generated_files_change() {
    let dir = scratch("clarity-gen-built");
    let files = vec![file("src/main.rs", "fn main() {}")];
    clarity_gen::write_out(&dir, &files, &["target"]).unwrap();
    let made = clarity_gen::fingerprint(&program(files.clone()));
    assert_eq!(clarity_gen::last_build(&dir, &made), None, "nothing built yet");
    // A release build: Run then starts the release program.
    clarity_gen::record_build(&dir, &made, true).unwrap();
    assert_eq!(clarity_gen::last_build(&dir, &made), Some(true));
    // Generating the same files again keeps what the build recorded.
    clarity_gen::write_out(&dir, &files, &["target"]).unwrap();
    assert_eq!(clarity_gen::last_build(&dir, &made), Some(true));
    // A debug build after it is what Run starts.
    clarity_gen::record_build(&dir, &made, false).unwrap();
    assert_eq!(clarity_gen::last_build(&dir, &made), Some(false));
    // Other files: the build is of something else.
    let other = vec![file("src/main.rs", "fn main() { println!(); }")];
    clarity_gen::write_out(&dir, &other, &["target"]).unwrap();
    assert_eq!(clarity_gen::last_build(&dir, &clarity_gen::fingerprint(&program(other))), None);
    std::fs::remove_dir_all(&dir).unwrap();
}
