//! Logging (Clarity settings, for every project): when on, the IDE writes
//! what it does into a log file, the designer's steps and the errors of its
//! commands, so a problem can be followed afterwards. A new file each start,
//! the last few kept, in the logs folder of Clarity's own folder.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How many log files are kept: this start's and the ones before it.
const KEPT: usize = 5;

/// The log file of this start, while logging is on.
static LOG: Mutex<Option<(File, Instant)>> = Mutex::new(None);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Choice {
    #[serde(default)]
    enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct LoggingView {
    /// Whether the IDE writes a log.
    pub enabled: bool,
    /// The folder the log files are in.
    pub folder: String,
    /// The log file of this start, while logging is on.
    pub file: String,
}

fn choice_file(home: &Path) -> PathBuf {
    home.join("logging.json")
}

fn folder(home: &Path) -> PathBuf {
    home.join("logs")
}

fn log_file(home: &Path, n: usize) -> PathBuf {
    if n == 0 { folder(home).join("ide.log") } else { folder(home).join(format!("ide.{n}.log")) }
}

fn load(home: &Path) -> Choice {
    std::fs::read_to_string(choice_file(home)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// Writes the choice back by read-modify-write, keeping keys it does not know.
fn save(home: &Path, choice: &Choice) -> Result<(), String> {
    let file = choice_file(home);
    let mut value: serde_json::Value =
        std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| serde_json::json!({}));
    if !value.is_object() {
        value = serde_json::json!({});
    }
    value["enabled"] = serde_json::Value::Bool(choice.enabled);
    std::fs::create_dir_all(home).map_err(|err| format!("cannot make {}: {err}", home.display()))?;
    let text = serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?;
    std::fs::write(&file, text + "\n").map_err(|err| format!("cannot write {}: {err}", file.display()))
}

/// Moves the earlier logs one place up, the oldest beyond KEPT going, and
/// opens a new ide.log.
fn start_file(home: &Path) -> Result<File, String> {
    std::fs::create_dir_all(folder(home)).map_err(|err| format!("cannot make {}: {err}", folder(home).display()))?;
    let _ = std::fs::remove_file(log_file(home, KEPT - 1));
    for n in (0..KEPT - 1).rev() {
        let _ = std::fs::rename(log_file(home, n), log_file(home, n + 1));
    }
    let file = log_file(home, 0);
    File::create(&file).map_err(|err| format!("cannot write {}: {err}", file.display()))
}

fn start(home: &Path) -> Result<(), String> {
    let file = start_file(home)?;
    *LOG.lock().unwrap_or_else(|e| e.into_inner()) = Some((file, Instant::now()));
    line("logging started");
    Ok(())
}

fn stop() {
    line("logging stopped");
    *LOG.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn view(home: &Path) -> LoggingView {
    let enabled = load(home).enabled;
    LoggingView {
        enabled,
        folder: folder(home).display().to_string(),
        file: if enabled { log_file(home, 0).display().to_string() } else { String::new() },
    }
}

/// Writes one line into the log, with the seconds since logging started,
/// when logging is on; only the file has it, never the terminal.
pub fn line(text: &str) {
    let mut log = LOG.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((file, since)) = log.as_mut() {
        let _ = writeln!(file, "{:>9.3}s {text}", since.elapsed().as_secs_f64());
        let _ = file.flush();
    }
}

/// Starts the log of this start when logging is on; called once at startup.
pub fn begin() {
    let home = clarity_project::clarity_home();
    if load(&home).enabled {
        if let Err(err) = start(&home) {
            eprintln!("logging: {err}");
        }
    }
}

/// Logging as it is set.
#[tauri::command]
pub fn logging_view() -> LoggingView {
    view(&clarity_project::clarity_home())
}

/// Turns logging on or off, at once and for the next starts.
#[tauri::command]
pub fn set_logging(enabled: bool) -> Result<LoggingView, String> {
    let home = clarity_project::clarity_home();
    save(&home, &Choice { enabled })?;
    let running = LOG.lock().unwrap_or_else(|e| e.into_inner()).is_some();
    match (enabled, running) {
        (true, false) => start(&home)?,
        (false, true) => stop(),
        _ => {}
    }
    Ok(view(&home))
}

/// Opens the logs folder in the system's file manager.
#[tauri::command]
pub fn open_log_folder() -> Result<(), String> {
    let folder = folder(&clarity_project::clarity_home());
    std::fs::create_dir_all(&folder).map_err(|err| format!("cannot make {}: {err}", folder.display()))?;
    #[cfg(target_os = "windows")]
    let opened = std::process::Command::new("explorer").arg(&folder).spawn();
    #[cfg(target_os = "macos")]
    let opened = std::process::Command::new("open").arg(&folder).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let opened = std::process::Command::new("xdg-open").arg(&folder).spawn();
    opened.map(|_| ()).map_err(|err| format!("cannot open {}: {err}", folder.display()))
}

/// A line from the IDE's front end, such as the designer's steps.
#[tauri::command]
pub fn log_line(line: String) {
    self::line(&line);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("clarity-logging-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_choice_is_kept_with_keys_it_does_not_know() {
        let home = home("choice");
        assert!(!load(&home).enabled, "off until chosen");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(choice_file(&home), r#"{"other": 3}"#).unwrap();
        save(&home, &Choice { enabled: true }).unwrap();
        assert!(load(&home).enabled);
        let kept: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(choice_file(&home)).unwrap()).unwrap();
        assert_eq!(kept["other"], 3);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn each_start_has_a_new_file_and_the_last_five_are_kept() {
        let home = home("rotate");
        for n in 0..7 {
            let mut file = start_file(&home).unwrap();
            writeln!(file, "start {n}").unwrap();
        }
        let read = |n: usize| std::fs::read_to_string(log_file(&home, n)).unwrap();
        assert_eq!(read(0), "start 6\n");
        assert_eq!(read(4), "start 2\n");
        assert!(!log_file(&home, 5).exists());
        let _ = std::fs::remove_dir_all(&home);
    }
}
