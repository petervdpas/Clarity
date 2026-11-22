//! clarity-cdev's bump_versions: each release in the chain gets its next
//! version in source, every Cargo.toml naming its crate follows (the carried
//! crates' `=` pin too), the Go runtime is left for its sums step, and a
//! second bump changes nothing.

use clarity_devtools::bump::bump_rows;
use clarity_devtools::release::{self, Standing};

fn row(key: &str, name: &str, source: &str, next: &str) -> Standing {
    let code = vec![format!("{name} changed")];
    Standing { key: key.into(), name: name.into(), tag: Some("t".into()), released: Some(source.into()), source: Some(source.into()), changed: code.clone(), changes: release::changes(&code), next: next.into(), bumped: false, bumpable: true }
}

#[test]
fn a_bump_sets_each_release_and_what_names_it_and_leaves_go_for_its_sums() {
    let repo = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("bump-repo");
    let _ = std::fs::remove_dir_all(&repo);
    let write = |file: &str, text: &str| {
        let at = repo.join(file);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, text).unwrap();
    };
    write("runtimes/rust-formats/Cargo.toml", "[package]\nname = \"clarity-formats\"\nversion = \"0.3.0\"\n");
    write("runtimes/rust/Cargo.toml", "[package]\nname = \"clarity-runtime\"\nversion = \"0.62.0\"\n\n[dependencies]\nclarity-formats = { path = \"../rust-formats\", version = \"0.3.0\", optional = true }\nother = \"0.3.0\"\n");
    write("runtimes/carried/Cargo.toml", "[dependencies]\nclarity-runtime = { version = \"=0.62.0\", default-features = false }\n");
    write("runtimes/csharp/Directory.Build.props", "<Project><PropertyGroup><Version>0.101.0</Version></PropertyGroup></Project>\n");
    write("runtimes/go/clarity/version.go", "package clarity\n\nconst Version = \"v0.65.0\"\n");
    assert!(std::process::Command::new("git").args(["init", "-q"]).current_dir(&repo).status().unwrap().success());
    let rows = [
        row("clarityFormats", "clarity-formats", "0.3.0", "0.4.0"),
        row("clarityRuntime", "clarity-runtime", "0.62.0", "0.63.0"),
        row("csharpRuntime", "C# runtime", "0.101.0", "0.102.0"),
        row("goRuntime", "Go runtime", "0.65.0", "0.66.0"),
    ];
    let done = bump_rows(&repo, &rows).unwrap();
    let read = |file: &str| std::fs::read_to_string(repo.join(file)).unwrap();
    assert!(read("runtimes/rust-formats/Cargo.toml").contains("version = \"0.4.0\""));
    let runtime = read("runtimes/rust/Cargo.toml");
    assert!(runtime.contains("\nversion = \"0.63.0\"") && runtime.contains("clarity-formats = { path = \"../rust-formats\", version = \"0.4.0\"") && runtime.contains("other = \"0.3.0\""), "{runtime}");
    assert!(read("runtimes/carried/Cargo.toml").contains("\"=0.63.0\""), "the carried pin follows");
    assert!(read("runtimes/csharp/Directory.Build.props").contains("<Version>0.102.0</Version>"));
    assert!(read("runtimes/go/clarity/version.go").contains("v0.65.0"), "Go moves in its sums step");
    assert_eq!(done.iter().filter(|b| b.what == "clarity-formats required").count(), 1, "{done:#?}");
    // As release_state then reads it, each at its version: nothing more to bump.
    let after: Vec<Standing> = rows.iter().map(|r| Standing { source: Some(r.next.clone()), ..r.clone() }).collect();
    assert!(bump_rows(&repo, &after).unwrap().is_empty());
    std::fs::remove_dir_all(&repo).unwrap();
}
