//! `clarity build` and `clarity run`: what the IDE's Build and Run do, from
//! the terminal (docs/ledger/build-tools.md), on a project `clarity gen` wrote.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PROJECT: &str = "project Hello\n    stack go cli\n";
const MODULE: &str = "module hello\n\ncli Hello\n    help @helloHelp\n\ncommand greet\n    help @greetHelp\n    print @greeting\n";
const ENGLISH: &str = "translation en\n    helloHelp \"Says hello\"\n    greetHelp \"Greets you\"\n    greeting \"Hello from the terminal\"\n";

/// A project of its own in a scratch folder: a Go program with one command.
fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("clarity-build-run-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("hello/translations")).unwrap();
    std::fs::write(dir.join("hello.clarity"), PROJECT).unwrap();
    std::fs::write(dir.join("hello/hello.clarity"), MODULE).unwrap();
    std::fs::write(dir.join("hello/translations/en.clarity"), ENGLISH).unwrap();
    dir
}

fn clarity(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clarity")).args(args).current_dir(dir).output().unwrap()
}

fn said(out: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

/// Build and Run build what Generate wrote, as the IDE does: a project not
/// generated yet is refused, with what to do.
#[test]
fn build_and_run_refuse_a_project_not_generated_yet() {
    let dir = project("refused");
    for command in ["build", "run"] {
        let out = clarity(&dir, &[command, "hello.clarity"]);
        assert!(!out.status.success(), "{command}: {}", said(&out));
        assert!(said(&out).contains("is not generated yet: run clarity gen first"), "{command}: {}", said(&out));
    }
    let out = clarity(&dir, &["build", "nothing.clarity"]);
    assert!(!out.status.success() && said(&out).contains("is not a project file"), "{}", said(&out));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// After `clarity gen`, `clarity build` builds the program with the stack's
/// own tools and `clarity run` runs it in the terminal, a command-line
/// program with the words after `--`. Ignored: it needs Go and the internet
/// (the runtime module).
#[test]
#[ignore]
fn a_generated_project_builds_and_runs_from_the_terminal() {
    let dir = project("built");
    let generated = clarity(&dir, &["gen", "hello.clarity"]);
    assert!(generated.status.success(), "{}", said(&generated));
    let built = clarity(&dir, &["build", "hello.clarity"]);
    assert!(built.status.success(), "{}", said(&built));
    assert!(said(&built).contains("go build"), "{}", said(&built));
    assert!(said(&built).contains("built "), "{}", said(&built));
    let ran = clarity(&dir, &["run", "hello.clarity", "--", "greet"]);
    assert!(ran.status.success(), "{}", said(&ran));
    assert!(String::from_utf8_lossy(&ran.stdout).contains("Hello from the terminal"), "{}", said(&ran));
    std::fs::remove_dir_all(&dir).unwrap();
}
