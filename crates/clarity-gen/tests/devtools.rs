//! DevTools (Clarity settings, a Clarity run from its repository): the
//! builds a tab's Build button runs, one at a time; the release planning,
//! read from the repository as it is; and the agent's ask to press Build.

use std::path::{Path, PathBuf};
use std::time::Duration;

use clarity_gen::tools::devtools::jobs::{JobState, Jobs, Runnable};
use clarity_gen::tools::devtools::release::{self, Check, Planned, UNITS};
use clarity_gen::tools::devtools::swap;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::current_dir().unwrap().join("target").join("devtools-tests").join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn shell(script: &str) -> Runnable {
    if cfg!(windows) {
        Runnable::new("cmd", &["/C", script], Path::new("."))
    } else {
        Runnable::new("sh", &["-c", script], Path::new("."))
    }
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

#[test]
fn a_build_keeps_its_lines_and_how_it_ended() {
    let jobs = Jobs::default();
    jobs.start("a", "A", shell("echo one && echo two"), None).unwrap();
    let job = jobs.wait("a", Duration::from_secs(20)).unwrap();
    assert_eq!(job.state, JobState::Passed);
    assert!(job.lines.iter().any(|l| l == "one") && job.lines.iter().any(|l| l == "two"), "{:?}", job.lines);
    jobs.start("b", "B", shell("exit 3"), None).unwrap();
    assert!(matches!(jobs.wait("b", Duration::from_secs(20)).unwrap().state, JobState::Failed(_)));
}

#[test]
fn builds_run_one_at_a_time_in_the_order_pressed() {
    let jobs = Jobs::default();
    let slow = if cfg!(windows) { "ping -n 2 127.0.0.1 > NUL" } else { "sleep 1" };
    jobs.start("first", "First", shell(slow), None).unwrap();
    jobs.start("second", "Second", shell("echo second"), None).unwrap();
    assert_eq!(jobs.job("second").unwrap().state, JobState::Waiting, "the second waits while the first builds");
    assert!(jobs.start("second", "Second", shell("echo again"), None).is_err(), "a build waiting or building is not pressed twice");
    let first = jobs.wait("first", Duration::from_secs(20)).unwrap();
    let second = jobs.wait("second", Duration::from_secs(20)).unwrap();
    assert_eq!((first.state, second.state.clone()), (JobState::Passed, JobState::Passed));
    assert!(second.started.unwrap() >= first.ended.unwrap());
}

#[test]
fn every_piece_has_a_build_with_at_most_two_jobs_of_cargo() {
    let repo = repo();
    for piece in &swap::PIECES {
        let run = swap::build(&repo, piece);
        assert!(!run.shown().is_empty(), "{}", piece.key);
        if run.program == "cargo" || run.shown().contains("build.sh") {
            assert!(run.env.iter().any(|(k, v)| k == "CARGO_BUILD_JOBS" && v == "2"), "{}: the machine has 7.5 GB", piece.key);
        }
    }
    assert_eq!(swap::build(&repo, swap::piece("clarityUi").unwrap()).shown(), "cargo build --release -p clarity-ui-ffi");
    assert_eq!(swap::build(&repo, swap::piece("goModule").unwrap()).shown(), "go build ./...");
}

#[test]
fn the_signal_says_to_press_build() {
    let repo = temp("signal");
    let said = swap::behind(&repo, swap::piece("clarityUi").unwrap(), "linux").unwrap();
    assert!(said.contains("press Build") && said.contains("cargo build --release -p clarity-ui-ffi"), "{said}");
}

#[test]
fn releases_go_after_the_crates_they_name() {
    // The order of the plan is the table's; each crate's versioned clarity-* dependencies (its Cargo.toml) come first.
    let repo = repo();
    for (i, unit) in UNITS.iter().enumerate() {
        for needed in release::needs(&repo, unit) {
            let at = UNITS.iter().position(|u| u.crate_name == Some(needed.as_str()));
            assert!(at.is_some_and(|at| at < i), "{} needs {needed}, which must be released before it", unit.name);
        }
    }
}

#[test]
fn every_release_reads_its_version_from_the_repository() {
    let repo = repo();
    for unit in &UNITS {
        if unit.version.is_some() {
            let v = release::source_version(&repo, unit).unwrap_or_else(|e| panic!("{}: {e}", unit.name));
            assert!(release::parse(&v).is_some(), "{}: {v}", unit.name);
        }
    }
}

#[test]
fn a_bump_raises_the_minor_version_and_the_crates_that_name_it() {
    let dir = temp("bump");
    std::fs::create_dir_all(dir.join("runtimes/rust")).unwrap();
    std::fs::create_dir_all(dir.join("runtimes/rust-ui")).unwrap();
    std::fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = [\"runtimes/rust\", \"runtimes/rust-ui\"]\n").unwrap();
    std::fs::write(dir.join("runtimes/rust/Cargo.toml"), "[package]\nname = \"clarity-runtime\"\nversion = \"0.61.0\"\n").unwrap();
    std::fs::write(dir.join("runtimes/rust-ui/Cargo.toml"), "[package]\nname = \"clarity-ui\"\nversion = \"0.27.0\"\n\n[dependencies]\nclarity-runtime = { path = \"../rust\", version = \"0.61.0\", default-features = false }\nother = { version = \"0.61.0\" }\n").unwrap();
    let unit = release::unit("clarityRuntime").unwrap();
    assert_eq!(release::bump(&dir, unit).unwrap(), "0.62.0");
    assert_eq!(release::source_version(&dir, unit).unwrap(), "0.62.0");
    let ui = std::fs::read_to_string(dir.join("runtimes/rust-ui/Cargo.toml")).unwrap();
    assert!(ui.contains("clarity-runtime = { path = \"../rust\", version = \"0.62.0\""), "{ui}");
    assert!(ui.contains("other = { version = \"0.61.0\" }"), "another crate's version stays");
    assert!(ui.contains("version = \"0.27.0\""), "clarity-ui's own stays");
}

#[test]
fn versions_compare_as_numbers() {
    assert!(release::parse("0.10.0") > release::parse("0.9.3"));
    assert_eq!(release::next_minor("1.1.0").unwrap(), "1.2.0");
    assert_eq!(release::newest(&["runtimes/rust/v0.9.0".into(), "runtimes/rust/v0.10.0".into(), "runtimes/rust-ui/v0.99.0".into()], "runtimes/rust/v"), Some("runtimes/rust/v0.10.0".into()));
}

#[test]
fn the_plan_has_the_release_helpers_steps_in_order() {
    let planned = vec![
        Planned { key: "clarityRuntime".into(), version: "0.62.0".into(), from: Some("runtimes/rust/v0.61.0".into()), note: String::new(), assets: vec![], packages: vec![] },
        Planned { key: "csharpRuntime".into(), version: "0.99.0".into(), from: Some("runtimes/csharp/v0.98.0".into()), note: "Folders.".into(), assets: vec![], packages: vec!["Clarity.Runtime".into()] },
        Planned { key: "goRuntime".into(), version: "0.65.0".into(), from: Some("runtimes/go/v0.64.0".into()), note: String::new(), assets: vec![], packages: vec![] },
        Planned { key: "ide".into(), version: "1.2.0".into(), from: Some("v1.1.0".into()), note: String::new(), assets: vec!["Clarity-v1.2.0-SHA256SUMS.txt".into()], packages: vec![] },
    ];
    let plan = release::plan("Clarity 1.2.0", "Release 1.2.0", &planned).unwrap();
    let steps = plan["steps"].as_array().unwrap();
    assert_eq!(steps[0]["workflow"], "checks.yml", "the checks on main come first");
    assert!(steps[0].get("tag").is_none());
    assert_eq!(steps[1]["tag"], "runtimes/rust/v0.62.0");
    assert_eq!(steps[1]["check"]["crates"]["name"], "clarity-runtime");
    assert_eq!(steps[2]["next_after"], "workflow", "nuget.org lists slowly");
    assert_eq!(steps[2]["check"]["nuget"]["packages"][0], "Clarity.Runtime");
    assert_eq!(steps[2]["note"], "Folders.");
    assert_eq!(steps[3]["check"]["go_proxy"]["version"], "v0.65.0");
    assert!(steps[4]["manual"].as_str().unwrap().contains("Go sums"), "the sums follow the Go tag");
    assert_eq!(steps[4]["commit_message"], "Go runtime v0.65.0 sums");
    assert_eq!(steps[5]["tag"], "v1.2.0");
    assert_eq!(steps[5]["check"]["github_release"]["assets"][0], "Clarity-v1.2.0-SHA256SUMS.txt");
    assert_eq!(plan["commit_message"], "Release 1.2.0");
}

#[test]
fn a_release_out_of_order_or_unknown_is_refused() {
    let p = |key: &str| Planned { key: key.into(), version: "1.0.0".into(), from: None, note: String::new(), assets: vec![], packages: vec![] };
    assert!(release::plan("t", "m", &[p("ide"), p("clarityRuntime")]).is_err());
    assert!(release::plan("t", "m", &[p("nothing")]).is_err());
}

#[test]
fn assets_of_the_last_release_take_the_new_version() {
    let said = release::assets_for(&["clarity-ui-0.26.0-linux-x64.tar.gz".into(), "SHA256SUMS".into()], "0.26.0", "0.27.0");
    assert_eq!(said, vec!["clarity-ui-0.27.0-linux-x64.tar.gz".to_string(), "SHA256SUMS".to_string()]);
}

#[test]
fn every_generated_file_guard_is_a_check() {
    // A guard added to the code (CLARITY_WRITE_*) is one the release planning runs.
    let repo = repo();
    let mut found = std::collections::BTreeSet::new();
    for dir in ["crates", "runtimes"] {
        walk(&repo.join(dir), &mut |path| {
            if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(path).unwrap_or_default();
                for (i, _) in text.match_indices("CLARITY_WRITE_") {
                    let name: String = text[i..].chars().take_while(|c| c.is_ascii_uppercase() || *c == '_').collect();
                    if name.len() > "CLARITY_WRITE_".len() && !name.ends_with('_') {
                        found.insert(name);
                    }
                }
            }
        });
    }
    let guarded: std::collections::BTreeSet<String> = release::GUARDS.iter().map(|g| g.variable.to_string()).collect();
    let missing: Vec<_> = found.difference(&guarded).collect();
    assert!(missing.is_empty(), "guards the release planning does not run: {missing:?}");
}

fn walk(dir: &Path, f: &mut dyn FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target" || n == "node_modules") {
                continue;
            }
            walk(&path, f);
        } else {
            f(&path);
        }
    }
}

#[test]
fn the_checks_cover_guards_tests_of_changed_crates_and_examples() {
    let repo = repo();
    let changed = vec!["crates/clarity-spec/src/check.rs".to_string(), "examples/todo/todo.clarity".to_string(), "app/src/App.svelte".to_string()];
    let checks: Vec<Check> = release::checks(&repo, &changed);
    let shown: Vec<String> = checks.iter().map(|c| c.run.shown()).collect();
    assert!(shown.iter().any(|s| s.contains("cargo test -p clarity-spec --test highlight")), "a guard: {shown:?}");
    assert!(shown.iter().any(|s| s == "cargo test -p clarity-spec --lib"), "the changed crate's lib tests");
    assert!(shown.iter().any(|s| s == "cargo test -p clarity-spec --test cxaml"), "each of its test targets");
    assert!(!shown.iter().any(|s| s.contains("-p clarity-db")), "an unchanged crate is not tested");
    assert!(shown.iter().any(|s| s.ends_with("spec check examples/maintenance")), "every example is spec checked");
    assert!(shown.iter().any(|s| s.contains("the_todo_example_builds_without_warnings")), "a changed example builds");
    assert!(!shown.iter().any(|s| s.contains("the_shop_example_builds")), "an unchanged one does not");
    assert!(shown.iter().any(|s| s.contains("svelte-check")), "app/ changed: svelte-check and the app's tests");
    let mut keys: Vec<&str> = checks.iter().map(|c| c.key.as_str()).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), checks.len(), "each check has its own key");
}

#[test]
fn an_agents_ask_to_build_is_answered_through_the_file() {
    use clarity_project::dev_asks;
    let dir = temp("asks");
    let id = dev_asks::ask(&dir, "clarityUi", "Claude Code").unwrap();
    let pending = dev_asks::pending(&dir);
    assert_eq!(pending.len(), 1);
    assert_eq!((pending[0].key.as_str(), pending[0].by.as_str()), ("clarityUi", "Claude Code"));
    assert!(dev_asks::take(&dir, &id), "the IDE takes it once");
    assert!(dev_asks::pending(&dir).is_empty() && dev_asks::taken(&dir, &id));
    assert!(dev_asks::answer_of(&dir, &id).is_none());
    dev_asks::answer(&dir, &id, &dev_asks::Answer { ok: true, text: "built".into() }).unwrap();
    assert_eq!(dev_asks::answer_of(&dir, &id).unwrap().text, "built");
}

#[test]
fn the_repository_says_how_every_release_stands() {
    let repo = repo();
    let rows = release::standing(&repo).unwrap();
    assert_eq!(rows.len(), UNITS.len());
    for row in &rows {
        assert!(release::parse(&row.next).is_some(), "{}: {}", row.name, row.next);
        assert!(row.tag.is_some(), "{} has been released", row.name);
    }
    let ide = rows.iter().find(|r| r.key == "ide").unwrap();
    assert!(!ide.bumpable && ide.source.is_none(), "the IDE's version is its tag alone");
}

#[test]
fn the_nuget_packages_are_the_runtimes_packable_projects() {
    let packages = release::nuget_packages(&repo());
    assert!(packages.contains(&"Clarity.Runtime".to_string()) && packages.contains(&"Clarity.Ui".to_string()), "{packages:?}");
    assert!(!packages.iter().any(|p| p.contains("Tests")), "{packages:?}");
}

#[test]
fn a_line_drawn_over_with_a_carriage_return_is_one_line_as_in_a_terminal() {
    // cargo draws its progress bar over itself with \r: the build shows the last drawing, not every one after the other.
    let jobs = Jobs::default();
    let script = if cfg!(windows) { "echo one" } else { "printf 'first\\n[=>  ] 1/3\\r[==> ] 2/3\\r[===>] 3/3\\r   Compiling x\\nlast\\r\\n'" };
    jobs.start("bar", "Bar", shell(script), None).unwrap();
    let job = jobs.wait("bar", Duration::from_secs(20)).unwrap();
    if !cfg!(windows) {
        assert_eq!(job.lines[1..], ["first".to_string(), "   Compiling x".to_string(), "last".to_string()], "{:?}", job.lines);
    }
}

#[test]
fn a_build_names_its_folder_plainly() {
    let jobs = Jobs::default();
    let dir = repo().join("app").join("..");
    jobs.start("dir", "Dir", Runnable::new("git", &["--version"], &dir), None).unwrap();
    let job = jobs.wait("dir", Duration::from_secs(20)).unwrap();
    assert!(!job.lines[0].contains(".."), "{}", job.lines[0]);
}
