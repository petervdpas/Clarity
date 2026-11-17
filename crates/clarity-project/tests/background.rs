//! Every program Clarity starts runs as a background process: on Windows a
//! console program started from Clarity's window opens a console window of
//! its own (a terminal flashing up for every tool Clarity asks its version)
//! unless it is started with no window. So nothing in Clarity's code calls
//! `Command::new` itself; it calls `clarity_project::background_command`.

use std::path::Path;

/// The source files of Clarity itself that start programs: the IDE and the crates.
fn sources(dir: &Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            // Tests start programs as tests do; they are no part of the running Clarity.
            if name != "tests" && name != "target" {
                sources(&path, found);
            }
        } else if name.ends_with(".rs") {
            found.push(path);
        }
    }
}

#[test]
fn every_program_clarity_starts_runs_in_the_background() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    sources(&root.join("app/src-tauri/src"), &mut files);
    sources(&root.join("crates"), &mut files);
    let mut raw = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        // A file's own unit tests (`mod tests`) are tests too.
        let code = text.split("\n#[cfg(test)]").next().unwrap_or_default();
        for (n, line) in code.lines().enumerate() {
            // tokio's Command (the agent over ACP) takes the same flag itself.
            let tokio_without_window = line.contains("tokio::process::Command::new(") && code.contains(".creation_flags(0x0800_0000)");
            if line.contains("Command::new(") && !line.trim_start().starts_with("//") && !tokio_without_window {
                raw.push(format!("{}:{}", file.strip_prefix(&root).unwrap_or(&file).display(), n + 1));
            }
        }
    }
    let helper = "crates/clarity-project/src/background.rs";
    raw.retain(|at| !at.starts_with(helper));
    assert!(raw.is_empty(), "start these with clarity_project::background_command:\n{}", raw.join("\n"));
}

#[test]
fn a_background_command_runs_the_program() {
    let program = if cfg!(windows) { "cmd" } else { "sh" };
    let args: &[&str] = if cfg!(windows) { &["/C", "echo here"] } else { &["-c", "echo here"] };
    let out = clarity_project::background_command(program).args(args).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "here");
}
