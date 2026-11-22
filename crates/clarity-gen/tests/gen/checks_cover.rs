//! The checks on GitHub (.github/workflows/checks.yml) leave no runtime test
//! out (docs/ledger/modularity.md, refactor phase 0): every test a runtime
//! has runs there, so a break cannot ship unnoticed.

use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn checks() -> String {
    std::fs::read_to_string(repo().join(".github/workflows/checks.yml")).unwrap()
}

/// The `//go:build` tags of the Go runtime's test files, each once.
fn go_test_tags() -> Vec<String> {
    let mut tags = Vec::new();
    let mut folders = vec![repo().join("runtimes/go")];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if path.to_string_lossy().ends_with("_test.go") {
                let text = std::fs::read_to_string(&path).unwrap();
                if let Some(line) = text.lines().find(|l| l.starts_with("//go:build ")) {
                    for tag in line["//go:build ".len()..].split(|c: char| !c.is_alphanumeric() && c != '_').filter(|t| !t.is_empty()) {
                        if !tags.contains(&tag.to_string()) {
                            tags.push(tag.to_string());
                        }
                    }
                }
            }
        }
    }
    tags.sort();
    tags
}

#[test]
fn the_checks_run_every_go_package_with_every_tag_its_tests_use() {
    let checks = checks();
    assert!(checks.contains("go test ./...\n"), "the checks run every package of the Go runtime, without tags");
    let tagged = checks.lines().find(|l| l.trim_start().starts_with("go test -tags ")).expect("the checks run the Go runtime with its tags");
    assert!(tagged.trim_end().ends_with(" ./..."), "the tagged run takes every package: {tagged}");
    assert!(!tagged.contains("-run"), "no test is filtered out by name: {tagged}");
    for tag in go_test_tags() {
        assert!(tagged.contains(&tag), "the Go tests' tag {tag} is not in the checks' tagged run: {tagged}");
    }
}

/// The C runtime's tests are listed by hand in tests.h (`X(name)`): every
/// test function of a `*_test.c` file is there, so none compiles and never runs.
#[test]
fn every_c_runtime_test_is_listed() {
    let folder = repo().join("runtimes/c/tests");
    let listed = std::fs::read_to_string(folder.join("tests.h")).unwrap();
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(&folder).unwrap().flatten() {
        let path = entry.path();
        if !path.to_string_lossy().ends_with("_test.c") {
            continue;
        }
        for line in std::fs::read_to_string(&path).unwrap().lines() {
            let Some(name) = line.strip_prefix("void ").and_then(|l| l.strip_suffix("(void) {")) else { continue };
            if !listed.contains(&format!("X({name})")) {
                missing.push(format!("{name} ({})", path.file_name().unwrap().to_string_lossy()));
            }
        }
    }
    assert!(missing.is_empty(), "C tests not in runtimes/c/tests/tests.h, so never run: {missing:#?}");
}

/// A generated file behind its source fails the checks: after the workspace's
/// tests (which write the TypeScript types) the tree must be as committed.
#[test]
fn the_checks_fail_on_files_the_tests_changed() {
    let checks = checks();
    let tests = checks.find("cargo test --workspace").expect("the workspace's tests");
    let after = checks[tests..].find("git status --porcelain").expect("the checks look at the tree after the workspace's tests");
    assert!(checks[tests + after..].contains("exit 1"), "a changed tree fails the checks");
}

/// The test sources of a reader of conformance/readers.json, as text.
fn reader_sources(reader: &str) -> Vec<String> {
    let root = repo();
    let (folders, ends): (Vec<&str>, Vec<&str>) = match reader {
        "rust" => (vec!["runtimes/rust/tests"], vec![".rs"]),
        "go" => (vec!["runtimes/go"], vec!["_test.go"]),
        "csharp" => (vec!["runtimes/csharp"], vec![".cs"]),
        "c" => (vec!["runtimes/c/tests"], vec![".c"]),
        "cpp" => (vec!["runtimes/cpp/tests"], vec![".cpp", ".hpp"]),
        "ui" => (vec!["runtimes", "crates"], vec![".rs"]),
        "web" => (vec!["runtimes/web/src", "app/src"], vec![".test.ts"]),
        other => panic!("no reader {other}"),
    };
    let mut found = Vec::new();
    let mut stack: Vec<PathBuf> = folders.iter().map(|f| root.join(f)).collect();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.is_dir() {
                if !["target", "node_modules", "bin", "obj", "out"].contains(&name.as_str()) {
                    stack.push(path);
                }
                continue;
            }
            let p = path.to_string_lossy();
            let kept = match reader {
                // C#'s tests are its *.Tests projects.
                "csharp" => p.contains(".Tests/"),
                // Clarity.UI, the shared libraries and the crates: their tests, and the shared libraries' sources.
                "ui" => (p.contains("/tests/") && !p.contains("runtimes/rust/tests")) || (p.contains("/runtimes/rust-") && p.contains("/src/")),
                _ => true,
            };
            if kept && ends.iter().any(|e| name.ends_with(e)) {
                found.push(std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    found
}

/// Every conformance file names the tests that read it (conformance/readers.json),
/// and they do: a file no one checks, a reader that stops reading, or a gap
/// closed but still listed as not yet, fails here.
#[test]
fn every_conformance_file_is_read_by_the_tests_it_names() {
    let list: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(repo().join("conformance/readers.json")).unwrap()).unwrap();
    let files = list["files"].as_object().unwrap();
    let mut on_disk: Vec<String> = std::fs::read_dir(repo().join("conformance"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".json") && n != "readers.json")
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = files.keys().cloned().collect();
    listed.sort();
    assert_eq!(listed, on_disk, "conformance/readers.json lists every conformance file and no other");
    let sources: std::collections::BTreeMap<&str, Vec<String>> = ["rust", "go", "csharp", "c", "cpp", "ui", "web"].into_iter().map(|r| (r, reader_sources(r))).collect();
    let mut wrong = Vec::new();
    for (file, entry) in files {
        let not_yet = entry["notYet"].as_object();
        for reader in entry["readers"].as_array().unwrap().iter().map(|r| r.as_str().unwrap()) {
            let reads = sources[reader].iter().any(|s| s.contains(file.as_str()));
            let waiting = not_yet.is_some_and(|n| n.contains_key(reader));
            match (reads, waiting) {
                (false, false) => wrong.push(format!("{file}: {reader}'s tests do not read it")),
                (true, true) => wrong.push(format!("{file}: {reader}'s tests read it now: take it out of notYet")),
                _ => {}
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The tests about the examples themselves, which read examples/: their builds,
/// checks and sweeps over every example, the examples' diagrams and texts, the
/// release checks' list of them, and tests that only build paths named so.
const ABOUT_THE_EXAMPLES: [&str; 24] = [
    "runtimes/rust-ui/tests/windows/every_example.rs",
    "crates/clarity-spec/tests/spec/design_edit.rs",
    "crates/clarity-gen/tests/gen/examples_build.rs",
    "crates/clarity-gen/tests/gen/ide_build/mod.rs",
    "crates/clarity-gen/tests/gen/checks_generate.rs",
    "crates/clarity-gen-rust/tests/rust/build.rs",
    "crates/clarity-gen-go/tests/go/build.rs",
    "crates/clarity-gen-cpp/tests/cpp/build.rs",
    "crates/clarity-gen-csharp/tests/csharp/build.rs",
    "crates/clarity-gen/tests/gen/live_canvas.rs",
    "crates/clarity-devtools/tests/devtools/devtools.rs",
    "crates/clarity-devtools/tests/devtools/disk.rs",
    "crates/clarity-gen-rust/tests/rust/ui_library.rs",
    "crates/clarity-gen/tests/gen/checks_cover.rs",
    "crates/clarity-spec/tests/spec/cxaml.rs",
    "crates/clarity-spec/tests/spec/keying.rs",
    "crates/clarity-spec/tests/spec/placed_places.rs",
    "crates/clarity-spec/tests/spec/highlight.rs",
    "crates/clarity-spec/tests/spec/translations.rs",
    "crates/clarity-spec/tests/spec/diagram_views.rs",
    "crates/clarity-project/tests/settings.rs",
    "app/src-tauri/src/lib.rs",
    "app/src/lib/ui/everyWindow.svelte.test.ts",
    "runtimes/rust-ui/tests/windows/windows_canvas_drag.rs",
];

/// Tests that take an example as the project they test something else on: each
/// moves to a project of its own (inline, or test-projects/), so removing or
/// changing an example breaks no test about something else. The user, 2026-11-21:
/// "I dont think its very smart to have tests depend on examples btw!"
const ON_AN_EXAMPLE_YET: [&str; 0] = [];

/// No test takes an example as its project but those about the examples, and
/// those named to move: a new test on an example fails here.
#[test]
fn only_tests_about_the_examples_read_them() {
    let root = repo();
    let mut readers = Vec::new();
    let mut folders: Vec<PathBuf> = ["crates", "runtimes", "app/src-tauri/src", "app/src"].iter().map(|f| root.join(f)).collect();
    while let Some(dir) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.is_dir() {
                if !["target", "node_modules", "bin", "obj", "out", "dist"].contains(&name.as_str()) {
                    folders.push(path);
                }
                continue;
            }
            let rel = path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
            let test_file = rel.contains("/tests/") || name.ends_with(".test.ts") || rel.starts_with("app/src-tauri/src/");
            if !test_file || !(name.ends_with(".rs") || name.ends_with(".ts")) {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            // Test code only: a source file's after its #[cfg(test)].
            let text = if rel.starts_with("app/src-tauri/src/") { text.find("#[cfg(test)]").map_or(String::new(), |at| text[at..].to_string()) } else { text };
            if text.contains("examples/") || text.contains("\"examples\"") || text.contains("/examples\"") {
                readers.push(rel);
            }
        }
    }
    let unnamed: Vec<&String> = readers.iter().filter(|r| !ABOUT_THE_EXAMPLES.contains(&r.as_str()) && !ON_AN_EXAMPLE_YET.contains(&r.as_str())).collect();
    assert!(unnamed.is_empty(), "tests that take an example as their project: give each a project of its own (inline or test-projects/): {unnamed:#?}");
    let moved: Vec<&&str> = ON_AN_EXAMPLE_YET.iter().filter(|f| !readers.iter().any(|r| r == *f)).collect();
    assert!(moved.is_empty(), "moved off the examples: take them out of ON_AN_EXAMPLE_YET: {moved:#?}");
}
