//! The outside agents' activity (docs/ACP-PLAN.md, "Agent activity"): one
//! line per tool call `clarity mcp` answers, kept short, which the IDE follows.

use std::path::{Path, PathBuf};

use clarity_project::activity::{self, Call};

fn log(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("activity-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("activity.jsonl")
}

fn call(tool: &str, path: Option<&str>, ok: bool) -> Call {
    Call { client: "codex".into(), tool: tool.into(), path: path.map(str::to_string), ok, said: if ok { "120 characters".into() } else { "no such file".into() } }
}

#[test]
fn calls_are_numbered_and_read_back_after_a_number() {
    let file = log("numbered");
    assert!(activity::after(&file, 0).is_empty(), "no log yet");
    activity::record_in(&file, &call("read_file", Some("todo.clarity"), true)).unwrap();
    activity::record_in(&file, &call("write_file", Some("design/Main.cxaml"), false)).unwrap();
    activity::record_in(&file, &call("check", None, true)).unwrap();

    let all = activity::after(&file, 0);
    assert_eq!(all.iter().map(|a| a.seq).collect::<Vec<_>>(), vec![1, 2, 3]);
    assert_eq!(all[0].client, "codex");
    assert_eq!(all[0].tool, "read_file");
    assert_eq!(all[0].path.as_deref(), Some("todo.clarity"));
    assert!(all[0].ok);
    assert!(!all[1].ok);
    assert_eq!(all[1].said, "no such file");
    assert_eq!(all[2].path, None);
    assert!(all[0].time > 0);

    let later = activity::after(&file, 2);
    assert_eq!(later.len(), 1);
    assert_eq!(later[0].tool, "check");
}

#[test]
fn a_line_still_being_written_is_not_read() {
    let file = log("half");
    activity::record_in(&file, &call("check", None, true)).unwrap();
    let mut text = std::fs::read_to_string(&file).unwrap();
    text.push_str("{\"seq\":2,\"tim");
    std::fs::write(&file, text).unwrap();
    assert_eq!(activity::after(&file, 0).len(), 1);
}

#[test]
fn the_log_keeps_only_the_last_calls() {
    let file = log("short");
    let many = 2 * activity::KEPT + 1;
    for _ in 0..many {
        activity::record_in(&file, &call("check", None, true)).unwrap();
    }
    let kept = activity::after(&file, 0);
    assert!(kept.len() <= 2 * activity::KEPT && kept.len() >= activity::KEPT, "{}", kept.len());
    assert_eq!(kept.last().unwrap().seq, many as u64, "numbers go on after trimming");
    assert!(kept.windows(2).all(|w| w[1].seq == w[0].seq + 1));
}

#[test]
fn the_result_line_is_one_short_line() {
    let file = log("said");
    let long = Call { said: format!("first line {}\nsecond line", "x".repeat(500)), ..call("check", None, false) };
    activity::record_in(&file, &long).unwrap();
    let said = &activity::after(&file, 0)[0].said;
    assert!(!said.contains('\n'));
    assert!(said.chars().count() <= activity::SAID_LIMIT + 1, "{}", said.chars().count());
}

#[test]
fn the_log_is_in_claritys_own_folder() {
    assert_eq!(activity::file(), clarity_project::clarity_home().join("activity.jsonl"));
}

/// Two agents at once (one in Clarity's panel, one outside) never share a
/// number or lose a line, through cuts too.
#[test]
fn calls_at_the_same_moment_each_get_their_own_number() {
    let file = log("together");
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let file = file.clone();
            std::thread::spawn(move || {
                for _ in 0..150 {
                    activity::record_in(&file, &call("check", None, true)).unwrap();
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let kept = activity::after(&file, 0);
    assert_eq!(kept.last().unwrap().seq, 1200);
    assert!(kept.windows(2).all(|w| w[1].seq == w[0].seq + 1), "no number twice, none missing");
}

/// A program that hangs while it holds the log does not hold up the tools:
/// the call is left out of the log after a short wait.
#[test]
fn a_log_held_too_long_is_given_up_on() {
    let file = log("held");
    let lock = std::fs::File::create(activity::lock_file(&file)).unwrap();
    lock.lock().unwrap();
    let started = std::time::Instant::now();
    assert!(activity::record_in(&file, &call("check", None, true)).is_err());
    assert!(started.elapsed() < activity::LOCK_WAIT + std::time::Duration::from_secs(1));
    lock.unlock().unwrap();
    activity::record_in(&file, &call("check", None, true)).unwrap();
}
