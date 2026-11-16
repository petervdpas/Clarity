//! DevTools (Clarity settings, a Clarity run from its repository): the
//! builds a tab's Build button runs, one at a time; the release planning,
//! read from the repository as it is; and the agent's ask to press Build.

use std::path::{Path, PathBuf};
use std::time::Duration;

use clarity_gen::tools::devtools::jobs::{JobState, Jobs, Runnable};
use clarity_gen::tools::devtools::release::{self, Check, CheckRecord, UNITS};
use clarity_gen::tools::devtools::swap;

fn temp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("devtools-tests").join(format!("{name}-{}", std::process::id()));
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
fn versions_compare_as_numbers() {
    assert!(release::parse("0.10.0") > release::parse("0.9.3"));
    assert_eq!(release::next_minor("1.1.0").unwrap(), "1.2.0");
    assert_eq!(release::newest(&["runtimes/rust/v0.9.0".into(), "runtimes/rust/v0.10.0".into(), "runtimes/rust-ui/v0.99.0".into()], "runtimes/rust/v"), Some("runtimes/rust/v0.10.0".into()));
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
    assert_eq!(pending[0].action, dev_asks::Action::Build, "an ask of before had no action: a build");
    assert!(dev_asks::take(&dir, &id), "the IDE takes it once");
    assert!(dev_asks::pending(&dir).is_empty() && dev_asks::taken(&dir, &id));
    assert!(dev_asks::answer_of(&dir, &id).is_none());
    dev_asks::answer(&dir, &id, &dev_asks::Answer { ok: true, text: "built".into() }).unwrap();
    assert_eq!(dev_asks::answer_of(&dir, &id).unwrap().text, "built");
}

#[test]
fn the_repository_says_how_every_release_stands() {
    let repo = repo();
    // A checkout without tags (the checks' shallow one) has no release: each row's tag is the newest of its own the checkout has.
    let tags = release::tags(&repo).unwrap();
    let rows = release::standing(&repo).unwrap();
    assert_eq!(rows.len(), UNITS.len());
    for (row, unit) in rows.iter().zip(UNITS.iter()) {
        assert!(release::parse(&row.next).is_some(), "{}: {}", row.name, row.next);
        assert_eq!(row.tag, release::newest(&tags, unit.tag), "{}", row.name);
    }
    let ide = rows.iter().find(|r| r.key == "ide").unwrap();
    assert!(!ide.bumpable && ide.source.is_none(), "the IDE's version is its tag alone");
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

#[test]
fn the_checks_run_in_a_fresh_checkout_between_the_guards_and_a_clean_tree() {
    // As on GitHub: the tests, spec checks and example builds run in a fresh checkout (no tags, no history, nothing git ignores);
    // the guards write in the tree first, and the tree is checked clean last.
    let repo = repo();
    let fresh = release::fresh_checkout_dir(&repo);
    let checks = release::checks(&repo, &["crates/clarity-spec/src/check.rs".to_string(), "examples/todo/todo.clarity".to_string()]);
    let at = |key: &str| checks.iter().position(|c| c.key == key).unwrap_or_else(|| panic!("{key}"));
    assert!(checks.iter().filter(|c| c.key.starts_with("guard:")).all(|c| c.run.dir == repo), "the guards write in the tree");
    assert!(checks.iter().filter(|c| c.key.starts_with("guard:")).all(|c| at(&c.key) < at("freshCheckout")));
    for key in ["test:clarity-spec:--lib", "spec:todo", "build:todo"] {
        let check = &checks[at(key)];
        assert_eq!(check.run.dir, fresh, "{key} runs in the fresh checkout");
        assert!(check.run.env.iter().any(|(n, v)| n == "CARGO_TARGET_DIR" && Path::new(v) == release::fresh_target_dir(&repo)), "{key} builds in a folder of its own");
        assert_ne!(release::fresh_target_dir(&repo), repo.join("target"), "never the tree's: cargo would take one's builds for the other's");
        assert!(at("freshCheckout") < at(key));
    }
    assert_eq!(checks.last().unwrap().key, "cleanTree", "the tree is checked last");
}

#[test]
fn a_fresh_checkout_has_the_uncommitted_files_and_no_tags() {
    let repo = repo();
    let dir = temp("fresh").join("checkout");
    release::fresh_checkout(&repo, &dir).unwrap();
    assert!(release::tags(&dir).unwrap().is_empty(), "no tags, as the checks' checkout");
    let file = "crates/clarity-gen/tests/devtools.rs";
    assert_eq!(std::fs::read(dir.join(file)).unwrap(), std::fs::read(repo.join(file)).unwrap(), "the tree as it is, committed or not");
    assert!(!dir.join("target").exists(), "nothing git ignores");
    assert!(release::clean_tree(&repo, &dir).is_ok(), "the tree is as the checkout found it");
}

#[test]
fn a_file_the_checks_leave_in_the_tree_is_named() {
    let before = "?? a.txt\t1\n M b.rs\t2\n";
    assert!(release::left_behind(before, before).is_empty());
    assert_eq!(release::left_behind(before, "?? a.txt\t1\n M b.rs\t3\n?? stray/x.json\t4\n"), vec!["b.rs", "stray/x.json"], "a new file and one changed again");
}

#[test]
fn a_releases_changes_are_sorted_by_kind() {
    // What the agent reads to write a plan: a manifest or a test alone is no reason to release.
    let files: Vec<String> = [
        "runtimes/rust-formats/Cargo.toml",
        "runtimes/rust-formats/src/csv.rs",
        "runtimes/rust-formats/tests/csv.rs",
        "runtimes/rust-formats/README.md",
        "runtimes/csharp/Clarity.Runtime.Tests/Rows.cs",
        "runtimes/go/clarity/rows_test.go",
        "runtimes/csharp/Clarity.Runtime/Clarity.Runtime.csproj",
        "editors/vscode/package.json",
        "app/src/lib/devtools.test.ts",
        "runtimes/rust-formats/LICENSE",
    ]
    .map(String::from)
    .to_vec();
    let c = release::changes(&files);
    assert_eq!(c.code, ["runtimes/rust-formats/src/csv.rs"]);
    assert_eq!(c.manifest, ["runtimes/rust-formats/Cargo.toml", "runtimes/csharp/Clarity.Runtime/Clarity.Runtime.csproj", "editors/vscode/package.json"]);
    assert_eq!(c.tests, ["runtimes/rust-formats/tests/csv.rs", "runtimes/csharp/Clarity.Runtime.Tests/Rows.cs", "runtimes/go/clarity/rows_test.go", "app/src/lib/devtools.test.ts"]);
    assert_eq!(c.docs, ["runtimes/rust-formats/README.md", "runtimes/rust-formats/LICENSE"]);
}

#[test]
fn a_check_that_ended_is_kept_for_the_agent() {
    // The IDE runs the checks; the agent reads how each ended (MCP release_state), the last run of each.
    let dir = temp("records");
    assert!(release::check_records(&dir).is_empty(), "nothing run yet");
    let passed = CheckRecord { key: "spec:todo".into(), title: "todo: spec check".into(), passed: true, why: None, ended: 1, head: "abc".into(), lines: vec![] };
    let failed = CheckRecord { key: "test:clarity-gen:devtools".into(), title: "clarity-gen: devtools".into(), passed: false, why: Some("exit 101".into()), ended: 2, head: "abc".into(), lines: vec!["panicked".into()] };
    release::record_check(&dir, &passed).unwrap();
    release::record_check(&dir, &failed).unwrap();
    release::record_check(&dir, &CheckRecord { passed: true, why: None, ended: 3, lines: vec![], ..failed.clone() }).unwrap();
    let records = release::check_records(&dir);
    assert_eq!(records.len(), 2);
    assert_eq!(records["spec:todo"], passed);
    assert!(records["test:clarity-gen:devtools"].passed && records["test:clarity-gen:devtools"].ended == 3, "run again, the last run counts");
    assert!(release::checks_file(&dir).starts_with(dir.join("target")), "kept under the ignored target/");
}

#[test]
fn the_chain_of_events_follows_the_release_helpers_order() {
    // What a release starts, in order: the checks on main, then each tag, its workflow and where it shows, the Go sums after the Go tag.
    let picked = [(release::unit("clarityUi").unwrap(), "0.28.0".to_string()), (release::unit("clarityRuntime").unwrap(), "0.62.0".to_string()), (release::unit("goRuntime").unwrap(), "0.65.0".to_string()), (release::unit("ide").unwrap(), "1.3.0".to_string())];
    let needs = |u: &release::Unit| if u.key == "clarityUi" { vec!["clarity-runtime".to_string()] } else { vec![] };
    let events = release::chain_of(&picked, needs);
    let titles: Vec<&str> = events.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, ["Checks on main", "clarity-runtime 0.62.0", "clarity-ui 0.28.0", "Go runtime 0.65.0", "Go sums", "Clarity IDE 1.3.0"], "the table's order, not the order given");
    assert_eq!(events[0].workflow.as_deref(), Some("checks.yml"));
    assert_eq!(events[1].tag.as_deref(), Some("runtimes/rust/v0.62.0"));
    assert_eq!(events[1].shows.as_deref(), Some("crates.io: clarity-runtime"));
    assert!(events[2].waits_for.contains(&"clarity-runtime 0.62.0".to_string()), "a crate waits for the crate it names: {:?}", events[2].waits_for);
    assert!(events[4].tag.is_none() && events[4].waits_for == ["Go runtime 0.65.0"], "the sums follow the Go tag");
    assert_eq!(events[5].tag.as_deref(), Some("v1.3.0"));
    assert!(events.iter().skip(1).all(|e| e.waits_for.iter().any(|w| w == &events[0].title) || !e.waits_for.is_empty()), "every step waits for something before it");
}

#[test]
fn an_agents_ask_to_remove_a_folder_says_so() {
    use clarity_project::dev_asks::{self, Action};
    let dir = temp("remove-asks");
    let id = dev_asks::ask_to(&dir, Action::Remove, "debug", "Claude Code").unwrap();
    let pending = dev_asks::pending(&dir);
    assert_eq!((pending[0].id.as_str(), pending[0].action, pending[0].key.as_str()), (id.as_str(), Action::Remove, "debug"));
}
