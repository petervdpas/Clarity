//! A Rust program on Clarity.UI (`stack rust desktop`) as a person gets it:
//! resolved and built by cargo from nothing but the generated files, with no
//! lockfile and no step of their own. Slow (Skia's prebuilt library, SQLite),
//! so they run only when asked, one at a time:
//! `cargo test -p clarity-gen --test rust_clarity_build -- --ignored --test-threads=1 --exact <name>`.
//!
//! The projects build on the crates in this repository through a
//! `.cargo/config.toml` patch, as a developer of the crates builds them.

use std::path::{Path, PathBuf};
use std::process::Command;

use clarity_gen::{generate, parts};
use clarity_project::{Backend, Settings, Ui, starter};
use clarity_spec::keyed::load_keyed;
use clarity_spec::native::PartFile;

const GAME: &str = "module game

entity Player
    name: text(80), required
    best: integer

part Gauge
    crate \"parts/Gauge\"
    binds integer
    speed: integer, min 1, max 10, default 3
    title: text
    rate: decimal
    due: date
    tells reached(level: integer, by: text)
    hears restart(level: integer)
    hears say(words: text, loud: bool)
    size 8 x 6

page Game: window
    title \"Game\"
    data player: Player
    textbox name nameBox at 1,1 span 4 bind player.name
    Gauge name gauge at 1,2 span 8 rows 6 bind player.best speed 5 on reached: gaugeReached

code Game
    handler gaugeReached(level: integer, by: text)
        message \"Reached\"
";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// The generated project `name`, written into target/generated (on disk,
/// never the system's temporary folder) and patched onto this repository's crates.
fn written(name: &str) -> PathBuf {
    let mut settings: Settings = starter("Game", "game");
    settings.stack.backend = Backend::Rust;
    settings.stack.ui = Ui::Clarity;
    let mut project = load_keyed([("game.clarity", GAME)], &[], "en");
    assert!(project.diagnostics.is_empty(), "{:#?}", project.diagnostics);
    let part = parts::starter_for_ui(&project.files[0].parts[0], Ui::Clarity).into_iter().map(|(path, text)| PartFile { part: "Gauge".into(), path, bytes: text.into_bytes() }).collect();
    clarity_spec::native::attach(&mut project, part);
    let files = generate(&project, &settings).unwrap_or_else(|errors| panic!("{errors:#?}"));
    let out = root().join("target/generated").join(format!("clarity-ui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    clarity_gen::write_out(&out, &files, &["target"]).unwrap();
    std::fs::create_dir_all(out.join(".cargo")).unwrap();
    let crate_at = |folder: &str| root().join(folder).display().to_string();
    std::fs::write(
        out.join(".cargo/config.toml"),
        format!("[patch.crates-io]\nclarity-runtime = {{ path = {:?} }}\nclarity-ui = {{ path = {:?} }}\n", crate_at("runtimes/rust"), crate_at("runtimes/rust-ui")),
    )
    .unwrap();
    out
}

/// The `syn` a package of the lockfile depends on: `syn 3.0.9`, or plain
/// `syn` when the lockfile has only one.
fn syn_of(lock: &str, package: &str) -> Option<String> {
    let block = lock.split("[[package]]").find(|b| b.contains(&format!("\nname = \"{package}\"\n")))?;
    block.lines().map(str::trim).find(|l| l.starts_with("\"syn")).map(|l| l.trim_matches(|c| c == '"' || c == ',').to_string())
}

/// Skia's bindings are made by bindgen, which formats them with
/// prettyplease: both take any of two versions of syn, and the two must
/// agree, or the build stops in bindgen. A fresh resolve, as a person's first
/// `cargo build` makes it, picks one syn for both.
#[test]
#[ignore]
fn a_fresh_resolve_gives_bindgen_and_prettyplease_one_syn() {
    let out = written("resolve");
    let status = Command::new("cargo").args(["generate-lockfile", "--quiet"]).current_dir(&out).status().unwrap();
    assert!(status.success(), "cargo cannot resolve the project in {}", out.display());
    let lock = std::fs::read_to_string(out.join("Cargo.lock")).unwrap();
    let (bindgen, pretty) = (syn_of(&lock, "bindgen"), syn_of(&lock, "prettyplease"));
    assert!(bindgen.is_some(), "no bindgen in the lockfile");
    assert_eq!(bindgen, pretty, "bindgen and prettyplease take different versions of syn; the project is in {}", out.display());
    std::fs::remove_dir_all(&out).unwrap();
}

/// The program with a custom part on Clarity.UI builds from the generated
/// files alone: the part's crate on clarity-ui, its interface, the
/// registration and the program.
#[test]
#[ignore]
fn a_program_with_a_custom_part_builds_on_clarity_ui() {
    let out = written("part");
    // One target folder for every run, so Skia and SQLite are fetched and compiled once.
    let target = root().join("target/generated");
    let output = Command::new("cargo").args(["build", "--quiet"]).current_dir(&out).env("CARGO_TARGET_DIR", &target).output().unwrap();
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "the project does not build; it is in {}\n{said}", out.display());
    // The part's starter, its interface and the program build without a warning (the crates' own are theirs).
    let ours = said.lines().filter(|l| l.trim_start().starts_with("-->") && !l.contains("runtimes/")).collect::<Vec<_>>();
    assert!(ours.is_empty(), "{said}");
    let program = if cfg!(windows) { "game.exe" } else { "game" };
    assert!(target.join("debug").join(program).exists(), "no {program}");
    std::fs::remove_dir_all(&out).unwrap();
}
