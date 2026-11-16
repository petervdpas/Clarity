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
fn a_stopped_build_ends_at_once_and_the_ones_waiting_never_start() {
    let jobs = Jobs::default();
    let long = if cfg!(windows) { "ping -n 60 127.0.0.1 > NUL" } else { "sleep 60" };
    jobs.start("long", "Long", shell(long), None).unwrap();
    jobs.start("next", "Next", shell("echo next"), None).unwrap();
    jobs.start("other", "Other", shell("echo other"), None).unwrap();
    while jobs.job("long").unwrap().state != JobState::Running {
        std::thread::sleep(Duration::from_millis(20));
    }
    let stopped = jobs.stop(&["long".to_string(), "next".to_string()]);
    assert_eq!(stopped, 2, "the one building and the one waiting");
    let long = jobs.wait("long", Duration::from_secs(10)).unwrap();
    assert_eq!(long.state, JobState::Failed("stopped".into()));
    let next = jobs.job("next").unwrap();
    assert_eq!(next.state, JobState::Failed("stopped".into()));
    assert!(next.started.is_none(), "it never started");
    assert_eq!(jobs.wait("other", Duration::from_secs(10)).unwrap().state, JobState::Passed, "a build not stopped goes on");
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
fn the_checks_in_the_checkout_run_as_on_github() {
    // The IDE's own settings (dev.sh's CLARITY_RUNTIME, a chosen library) stay out, a test target gets the features it
    // declares (GitHub tests the whole workspace, which turns them on), the library the C header's test links is built
    // first, and a path through .. is read as the folder it is.
    let repo = repo();
    let changed = vec!["runtimes/rust-auth/src/lib.rs".to_string(), "runtimes/rust-ui-ffi/src/lib.rs".to_string(), "examples/orbs/orbs.clarity".to_string()];
    let checks = release::checks(&repo.join("app").join(".."), &changed);
    let at = |key: &str| checks.iter().position(|c| c.key == key).unwrap_or_else(|| panic!("{key}"));
    let env = |c: &Check, name: &str| c.run.env.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v.clone());
    for c in checks.iter().filter(|c| c.fresh) {
        assert_eq!(c.run.dir, release::fresh_checkout_dir(&repo), "{}: the folder itself", c.key);
        assert_eq!(env(c, "CLARITY_RUNTIME").as_deref(), Some(""), "{}: the published runtimes, as on GitHub", c.key);
        assert_eq!(env(c, "CLARITY_UI_BROWSER").as_deref(), Some(""), "{}", c.key);
        assert_eq!(env(c, "CARGO_PROFILE_DEV_DEBUG").as_deref(), Some("0"), "{}: no debug info, which made each test target a program of 90 MB", c.key);
    }
    assert!(checks[at("test:clarity-auth:openid")].run.shown().ends_with("--test openid --features openid"), "{}", checks[at("test:clarity-auth:openid")].run.shown());
    assert!(!checks[at("test:clarity-auth:--lib")].run.shown().contains("--features"));
    let library = &checks[at("library:clarity-ui-ffi")];
    assert_eq!(library.run.shown(), "cargo build -p clarity-ui-ffi");
    assert!(library.fresh && at("freshCheckout") < at("library:clarity-ui-ffi") && at("library:clarity-ui-ffi") < at("test:clarity-ui-ffi:c_header"));
    assert!(!checks[at("guard:test:clarity-spec:--lib")].fresh);
    let none = release::checks(&repo, &["runtimes/rust-auth/src/lib.rs".to_string()]);
    assert!(!none.iter().any(|c| c.key == "library:clarity-ui-ffi"), "built only for the library's own tests");
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
    assert_eq!(release::checkout_state(&dir).unwrap(), release::tree_state(&repo).unwrap(), "a check in the checkout counts on the tree it was made from");
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
    let passed = CheckRecord { key: "spec:todo".into(), title: "todo: spec check".into(), passed: true, why: None, ended: 1, head: "abc".into(), tree: "t".into(), lines: vec![] };
    let failed = CheckRecord { key: "test:clarity-gen:devtools".into(), title: "clarity-gen: devtools".into(), passed: false, why: Some("exit 101".into()), ended: 2, head: "abc".into(), tree: "t".into(), lines: vec!["panicked".into()] };
    release::record_check(&dir, &passed).unwrap();
    release::record_check(&dir, &failed).unwrap();
    release::record_check(&dir, &CheckRecord { passed: true, why: None, ended: 3, lines: vec![], ..failed.clone() }).unwrap();
    let records = release::check_records(&dir);
    assert_eq!(records.len(), 2);
    assert_eq!(records["spec:todo"], passed);
    assert!(records["test:clarity-gen:devtools"].passed && records["test:clarity-gen:devtools"].ended == 3, "run again, the last run counts");
    assert!(release::checks_file(&dir).starts_with(dir.join("target")), "kept under the ignored target/");
    assert!(!release::checks_file(&dir).with_extension("json.new").exists(), "written beside it and moved over it, nothing left behind");
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

#[test]
fn a_check_counts_only_on_the_tree_it_ran_on() {
    // The tree is the commit and its uncommitted changes, each file's content too.
    let tree = release::fingerprint("abc", " M a.rs\t1f\n");
    assert_eq!(tree, release::fingerprint("abc", " M a.rs\t1f\n"), "the same tree, the same fingerprint");
    assert_ne!(tree, release::fingerprint("abd", " M a.rs\t1f\n"), "another commit");
    assert_ne!(tree, release::fingerprint("abc", " M a.rs\t2e\n"), "a file changed again");
    let record = |key: &str, passed: bool, tree: &str| CheckRecord { key: key.into(), title: format!("check {key}"), passed, why: None, ended: 1, head: "abc".into(), tree: tree.into(), lines: vec![] };
    let records: std::collections::BTreeMap<String, CheckRecord> =
        [record("a", true, &tree), record("b", false, &tree), record("c", true, "older")].into_iter().map(|r| (r.key.clone(), r)).collect();
    let checks = [("a", "check a"), ("b", "check b"), ("c", "check c"), ("d", "check d")];
    let held = release::held_back_of(checks.iter().map(|(k, t)| (k.to_string(), t.to_string(), tree.clone())), &records);
    assert_eq!(held, ["check b: failed", "check c: ran on another tree", "check d: not run"]);
    assert!(release::held_back_of([("a".to_string(), "check a".to_string(), tree.clone())].into_iter(), &records).is_empty(), "all passed on this tree: none held back");
}

#[test]
fn a_check_counts_on_the_files_it_can_read_only() {
    // A crate's test reads every file of the tree but the Rust code of the crates it does not build: a change there,
    // or another crate's version, leaves its pass standing; its own code, a manual page or an example does not.
    let repo = repo();
    let checks = release::checks(&repo, &["runtimes/rust-image/src/lib.rs".to_string(), "crates/clarity-spec/src/check.rs".to_string()]);
    let check = |key: &str| checks.iter().find(|c| c.key == key).unwrap_or_else(|| panic!("{key}")).clone();
    let image = check("test:clarity-image:--lib");
    let spec = check("test:clarity-spec:--lib");
    let state = release::tree_state(&repo).unwrap();
    let print = |state: &release::TreeState, c: &Check| state.fingerprint(c);
    let changed = |file: &str| {
        let mut s = state.clone();
        s.files.insert(file.to_string(), "changed".into());
        s
    };
    assert_eq!(print(&state, &image), print(&release::tree_state(&repo).unwrap(), &image), "the same tree, the same fingerprint");
    let spec_code = changed("crates/clarity-spec/src/check.rs");
    assert_ne!(print(&spec_code, &spec), print(&state, &spec), "its own code");
    assert_eq!(print(&spec_code, &image), print(&state, &image), "code of a crate it does not build");
    let ffi_manifest = changed("runtimes/rust-ui-ffi/Cargo.toml");
    assert_eq!(print(&ffi_manifest, &image), print(&state, &image), "another crate's manifest (a version bump)");
    let mut lock = changed("Cargo.lock");
    assert_eq!(print(&lock, &image), print(&state, &image), "Cargo.lock counts by what its own crates resolve to");
    lock.packages.iter_mut().filter(|p| p.name == "clarity-image").for_each(|p| p.version = "9.9.9".into());
    assert_ne!(print(&lock, &image), print(&state, &image), "its own crate's version");
    assert_ne!(print(&changed("manual/start.overview.md"), &image), print(&state, &image), "a file that is not Rust code: any test may read it");
    let mut other = image.clone();
    other.run = other.run.with_env("SOMETHING", "1");
    assert_ne!(print(&state, &other), print(&state, &image), "its command");
    let whole = check("cleanTree");
    assert_ne!(print(&changed("runtimes/rust-ui-ffi/src/lib.rs"), &whole), print(&state, &whole), "the tree's own checks read all of it");
}

#[test]
fn a_plan_starts_with_the_checks_on_main_and_never_overwrites_one() {
    let dir = temp("plans");
    let step = |title: &str, tag: Option<&str>, workflow: &str| match tag {
        Some(tag) => serde_json::json!({ "title": title, "tag": tag, "workflow": workflow }),
        None => serde_json::json!({ "title": title, "workflow": workflow }),
    };
    let good = serde_json::json!({ "title": "Clarity 1.4.0", "commit_message": "Release", "steps": [step("Checks on main", None, "checks.yml"), step("Clarity IDE 1.4.0", Some("v1.4.0"), "release.yml")] });
    let file = release::write_plan(&dir, "2026-11-17-1200", &good).unwrap();
    assert!(file.ends_with("releases/2026-11-17-1200.json"));
    assert!(release::write_plan(&dir, "2026-11-17-1200", &good).is_err(), "an existing plan is not overwritten");
    assert!(release::write_plan(&dir, "17-11-2026", &good).is_err(), "the stamp is YYYY-MM-DD-HHMM");
    let no_checks = serde_json::json!({ "title": "t", "commit_message": "m", "steps": [step("Clarity IDE 1.4.0", Some("v1.4.0"), "release.yml")] });
    assert!(release::write_plan(&dir, "2026-11-17-1201", &no_checks).unwrap_err().contains("checks.yml"), "the checks on main come first");
    // The fix after a failed release: only the commit message changes.
    release::set_commit_message(&dir, "2026-11-17-1200", "Fix the failed Clarity 1.4.0 release").unwrap();
    let kept: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(kept["commit_message"], "Fix the failed Clarity 1.4.0 release");
    assert_eq!(kept["steps"], good["steps"], "the steps stay");
    assert!(release::set_commit_message(&dir, "2026-11-17-9999", "x").is_err(), "no such plan");
}
