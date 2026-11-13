//! Rebuild in development (Clarity settings > Rebuild, only in an IDE run
//! from its repository): a tick for each of Clarity's own pieces the IDE
//! builds from the repository's source (Clarity.UI's library and browser
//! files, the other base libraries), and Restart, which hides the IDE,
//! builds what is ticked with the splash screen showing the build line for
//! line, clears every tick and shows the IDE again, drawing with what was
//! built. Starting the IDE with pieces ticked does the same before the IDE
//! first shows: the splash turns into the build. The ticks are kept in `target/dev-rebuild.json`, so an agent that
//! changed a piece ticks it there and the person restarts. An installed
//! Clarity has no repository and so no such section.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

/// A piece of Clarity the IDE rebuilds from the repository's source.
pub struct Piece {
    /// Its tick's key, as `target/dev-rebuild.json` holds it.
    pub key: &'static str,
    pub name: &'static str,
    /// What uses it, as people read it.
    pub uses: &'static str,
    /// The command that builds it, run in the repository.
    pub command: &'static [&'static str],
}

/// The pieces, in the order they show and build: the library before the browser files.
pub const PIECES: &[Piece] = &[
    Piece { key: "clarityUi", name: "Clarity.UI's library", uses: "Go, C, C++ and Rust desktop programs", command: &["cargo", "build", "--release", "-p", "clarity-ui-ffi"] },
    Piece { key: "browser", name: "Clarity.UI's browser files", uses: "the design, the preview and web programs", command: &["bash", "runtimes/rust-ui-browser/build.sh"] },
    Piece { key: "audio", name: "Clarity.Audio's library", uses: "programs that play songs", command: &["cargo", "build", "--release", "-p", "clarity-audio-ffi"] },
    Piece { key: "video", name: "Clarity.Video's library", uses: "programs that play videos", command: &["cargo", "build", "--release", "-p", "clarity-video-ffi"] },
    Piece { key: "image", name: "Clarity.Image's library", uses: "programs with picture functions", command: &["cargo", "build", "--release", "-p", "clarity-image-ffi"] },
    Piece { key: "formats", name: "Clarity.Formats' library", uses: "programs reading data formats", command: &["cargo", "build", "--release", "-p", "clarity-formats-ffi"] },
    Piece { key: "auth", name: "Clarity.Auth's library", uses: "programs that sign in", command: &["cargo", "build", "--release", "-p", "clarity-auth-ffi"] },
    Piece { key: "appMcp", name: "Clarity.AppMcp's library", uses: "programs offering commands to AI agents", command: &["cargo", "build", "--release", "-p", "clarity-app-mcp-ffi"] },
];

/// The file that holds the ticks, in the repository's build folder.
pub fn ticks_file(repo: &Path) -> PathBuf {
    repo.join("target").join("dev-rebuild.json")
}

/// What the file holds.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Ticks {
    #[serde(default)]
    ticked: Vec<String>,
}

/// The keys ticked in `file`, of known pieces only, in the pieces' order; none when there is no file.
pub fn ticked(file: &Path) -> Vec<String> {
    let kept: Ticks = std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    PIECES.iter().filter(|p| kept.ticked.iter().any(|k| k == p.key)).map(|p| p.key.to_string()).collect()
}

/// Ticks `key` or takes its tick away; an unknown key is refused.
pub fn set_ticked(file: &Path, key: &str, on: bool) -> Result<Vec<String>, String> {
    if !PIECES.iter().any(|p| p.key == key) {
        return Err(format!("'{key}' is not a piece Clarity rebuilds"));
    }
    let mut keys = ticked(file);
    keys.retain(|k| k != key);
    if on {
        keys.push(key.to_string());
    }
    write(file, &keys)?;
    Ok(ticked(file))
}

/// Takes every tick away.
pub fn clear(file: &Path) -> Result<(), String> {
    write(file, &[])
}

fn write(file: &Path, keys: &[String]) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(&Ticks { ticked: keys.to_vec() }).map_err(|e| e.to_string())?;
    std::fs::write(file, text + "\n").map_err(|e| format!("cannot write {}: {e}", file.display()))
}

/// The commands that build what is ticked, in the pieces' order. A library
/// is built as a release (what Go, C, C++ and Rust programs link, and a C#
/// Release build) and as a debug build (what a C# Debug build links, as Run
/// builds it).
pub fn commands(keys: &[String]) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for piece in PIECES.iter().filter(|p| keys.iter().any(|k| k == p.key)) {
        let command: Vec<String> = piece.command.iter().map(|s| s.to_string()).collect();
        let debug = (command.first().map(String::as_str) == Some("cargo")).then(|| command.iter().filter(|w| *w != "--release").cloned().collect::<Vec<_>>());
        out.push(command);
        out.extend(debug);
    }
    out
}

/// A piece as the section shows it.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RebuildPiece {
    pub key: String,
    pub name: String,
    pub uses: String,
    pub ticked: bool,
}

/// The section's view: the pieces and where the ticks are kept.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RebuildView {
    pub pieces: Vec<RebuildPiece>,
    pub file: String,
}

fn view(repo: &Path) -> RebuildView {
    let file = ticks_file(repo);
    let keys = ticked(&file);
    let pieces = PIECES.iter().map(|p| RebuildPiece { key: p.key.into(), name: p.name.into(), uses: p.uses.into(), ticked: keys.iter().any(|k| k == p.key) }).collect();
    RebuildView { pieces, file: file.display().to_string() }
}

/// How a restart's build ended, when it did not: the IDE shows again with this.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RebuildFailed {
    pub message: String,
    /// The build's last lines, which say why.
    pub lines: Vec<String>,
}

/// The section's view; none in an installed Clarity, which shows no section.
#[tauri::command]
pub fn dev_rebuild() -> Option<RebuildView> {
    crate::repository().map(|repo| view(&repo))
}

/// Ticks a piece, or takes its tick away.
#[tauri::command]
pub fn set_dev_rebuild(key: String, ticked: bool) -> Result<RebuildView, String> {
    let repo = crate::repository().ok_or("only a Clarity run from its repository rebuilds its pieces")?;
    set_ticked(&ticks_file(&repo), &key, ticked)?;
    Ok(view(&repo))
}

/// The most build lines kept to say why a build failed.
const KEPT_LINES: usize = 40;

/// Whether a rebuild runs: the IDE's window stays hidden and the splash open until it ends.
#[derive(Default)]
pub struct Building(AtomicBool);

/// Whether a rebuild runs now.
pub fn building(app: &AppHandle) -> bool {
    app.try_state::<Building>().is_some_and(|b| b.0.load(Ordering::SeqCst))
}

/// Shows `window` a while after a rebuild has ended (none running: after a
/// while), if its page has not shown it by then, so a page that fails to
/// load still leaves a window to see.
pub fn show_later(app: AppHandle, window: tauri::WebviewWindow) {
    std::thread::spawn(move || {
        while building(&app) {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        std::thread::sleep(std::time::Duration::from_secs(4));
        let _ = window.show();
    });
}

/// At the IDE's start, before the splash opens: when pieces are ticked, it
/// opens as the build and the IDE shows when the build has ended. False
/// when nothing is ticked: the splash is the usual one.
pub fn at_start(app: &AppHandle, title: &str, version: &str) -> bool {
    let Some(repo) = crate::repository() else { return false };
    let file = ticks_file(&repo);
    let steps = commands(&ticked(&file));
    if steps.is_empty() {
        return false;
    }
    app.state::<Building>().0.store(true, Ordering::SeqCst);
    app.state::<crate::build::BuildState>().busy.store(true, Ordering::SeqCst);
    crate::splash::open_building(app, title, version);
    let app = app.clone();
    std::thread::spawn(move || {
        let ended = build(&app, &repo, &steps);
        finish(&app, &repo, &file, ended);
    });
    true
}

/// Hides the IDE and builds what is ticked, the splash screen showing each
/// line. When all built: every tick goes, Clarity.UI's library is taken
/// again, and the IDE loads again and shows, drawing with what was built.
/// When one failed: the ticks stay and the IDE shows again with why.
#[tauri::command]
pub fn dev_restart(app: AppHandle) -> Result<(), String> {
    let repo = crate::repository().ok_or("only a Clarity run from its repository rebuilds its pieces")?;
    let file = ticks_file(&repo);
    let steps = commands(&ticked(&file));
    if steps.is_empty() {
        return Err("Tick what to rebuild first.".to_string());
    }
    let busy = &app.state::<crate::build::BuildState>().busy;
    if busy.swap(true, Ordering::SeqCst) {
        return Err("a build is already running; restart when it has ended".to_string());
    }
    app.state::<Building>().0.store(true, Ordering::SeqCst);
    let main = app.get_webview_window("main");
    let title = main.as_ref().and_then(|w| w.title().ok()).unwrap_or_else(|| "Clarity IDE".to_string());
    let version = title.strip_prefix(app.package_info().name.as_str()).unwrap_or(&title).trim().to_string();
    crate::splash::open_building(&app, &title, &version);
    if let Some(main) = &main {
        let _ = main.hide();
    }
    std::thread::spawn(move || {
        let ended = build(&app, &repo, &steps);
        finish(&app, &repo, &file, ended);
    });
    Ok(())
}

/// A rebuild has ended. Built: the ticks go, the library is taken again and
/// the IDE's page loads again, which shows it and ends the splash (main.ts).
/// Failed: the ticks stay, the IDE shows and says why.
fn finish(app: &AppHandle, repo: &Path, file: &Path, ended: Result<(), RebuildFailed>) {
    app.state::<crate::build::BuildState>().busy.store(false, Ordering::SeqCst);
    let main = app.get_webview_window("main");
    match ended {
        Ok(()) => {
            let _ = clear(file);
            if let Some(folder) = clarity_gen::clarity_ui::repository_library(repo, std::env::consts::OS) {
                clarity_gen::clarity_ui::set_bundled(folder);
            }
            crate::splash::status(app, "Starting the IDE");
            app.state::<Building>().0.store(false, Ordering::SeqCst);
            // Every script of Clarity.UI loads new with the page. The window shows first (the splash
            // stays on top until the page has drawn): a page reloaded in a hidden window keeps a stale,
            // smaller viewport in the webview, and the IDE then stops short of its window's bottom.
            if let Some(main) = main {
                let _ = main.show();
                let _ = main.eval("location.reload()");
                show_later(app.clone(), main);
            }
        }
        Err(failed) => {
            app.state::<Building>().0.store(false, Ordering::SeqCst);
            crate::splash::done(app);
            if let Some(main) = main {
                let _ = main.show();
            }
            let _ = app.emit("dev-rebuild-failed", failed);
        }
    }
}

/// The IDE's page has drawn itself: its window shows and the splash goes,
/// unless a rebuild runs, which shows them when it ends.
#[tauri::command]
pub fn main_drawn(app: AppHandle) {
    if building(&app) {
        return;
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
    }
    crate::splash::done(&app);
}

/// Runs each command in `repo`, each line it writes on the splash screen.
fn build(app: &AppHandle, repo: &Path, steps: &[Vec<String>]) -> Result<(), RebuildFailed> {
    let mut kept: Vec<String> = Vec::new();
    for step in steps {
        let shown = step.join(" ");
        crate::splash::line(app, &shown);
        let ran = run(repo, step, |line| {
            crate::splash::line(app, line);
            kept.push(line.to_string());
            if kept.len() > KEPT_LINES {
                kept.remove(0);
            }
        });
        if let Err(message) = ran {
            return Err(RebuildFailed { message: format!("{shown}: {message}"), lines: kept });
        }
    }
    Ok(())
}

/// Runs one command, its output and errors line for line to `each`.
fn run(repo: &Path, step: &[String], mut each: impl FnMut(&str)) -> Result<(), String> {
    let (program, args) = step.split_first().ok_or("an empty command")?;
    let mut command = Command::new(program);
    // As from a terminal: what `cargo run` and `tauri dev` gave the IDE is not the build's.
    for (name, _) in std::env::vars_os() {
        if name.to_str().is_some_and(left_out) {
            command.env_remove(&name);
        }
    }
    let mut child = command
        .args(args)
        .current_dir(repo)
        .env("CARGO_BUILD_JOBS", "2")
        .env("CARGO_TERM_COLOR", "never")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run {program}: {e}"))?;
    let (send, lines) = mpsc::channel::<String>();
    let readers: Vec<_> = [child.stdout.take().map(|o| Box::new(o) as Box<dyn Read + Send>), child.stderr.take().map(|e| Box::new(e) as Box<dyn Read + Send>)]
        .into_iter()
        .flatten()
        .map(|from| {
            let send = send.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(from).lines().map_while(Result::ok) {
                    let _ = send.send(line);
                }
            })
        })
        .collect();
    drop(send);
    for line in lines {
        each(&line);
    }
    for reader in readers {
        let _ = reader.join();
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err(format!("ended with {status}")) }
}

/// The variables `cargo run` and `tauri dev` give the IDE, which a build of
/// Clarity's pieces does not take: they would make cargo build again what
/// is built. CARGO_HOME and CARGO_TARGET_DIR are the person's own and stay.
pub fn left_out(name: &str) -> bool {
    let cargo = name.starts_with("CARGO") && name != "CARGO_HOME" && name != "CARGO_TARGET_DIR";
    cargo || name.starts_with("__CARGO") || name.starts_with("TAURI_") || name == "RUST_RECURSION_COUNT"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("clarity-dev-rebuild-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_build_takes_none_of_what_cargo_run_and_tauri_dev_gave_the_ide() {
        for name in ["CARGO", "CARGO_MANIFEST_DIR", "CARGO_PKG_VERSION", "CARGO_PRIMARY_PACKAGE", "__CARGO_FIX_PLZ", "TAURI_ENV_PLATFORM", "RUST_RECURSION_COUNT"] {
            assert!(left_out(name), "{name}");
        }
        for name in ["CARGO_HOME", "CARGO_TARGET_DIR", "PATH", "HOME", "RUSTUP_HOME"] {
            assert!(!left_out(name), "{name}");
        }
    }

    #[test]
    fn nothing_is_ticked_without_the_file() {
        assert!(ticked(&folder("none").join("dev-rebuild.json")).is_empty());
    }

    #[test]
    fn ticks_are_kept_in_the_pieces_order_and_cleared() {
        let file = folder("kept").join("target").join("dev-rebuild.json");
        set_ticked(&file, "browser", true).unwrap();
        let keys = set_ticked(&file, "clarityUi", true).unwrap();
        assert_eq!(keys, ["clarityUi", "browser"], "the library builds before the browser files");
        assert_eq!(
            commands(&keys),
            [vec!["cargo", "build", "--release", "-p", "clarity-ui-ffi"], vec!["cargo", "build", "-p", "clarity-ui-ffi"], vec!["bash", "runtimes/rust-ui-browser/build.sh"]],
            "a library as a release and as a debug build (a C# Debug build links that), then the browser files"
        );
        assert_eq!(set_ticked(&file, "browser", false).unwrap(), ["clarityUi"]);
        clear(&file).unwrap();
        assert!(ticked(&file).is_empty());
    }

    #[test]
    fn an_unknown_piece_is_refused_and_one_written_by_hand_is_left_out() {
        let dir = folder("unknown");
        let file = dir.join("dev-rebuild.json");
        assert!(set_ticked(&file, "everything", true).is_err());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, r#"{"ticked": ["video", "nonsense"]}"#).unwrap();
        assert_eq!(ticked(&file), ["video"]);
    }

    #[test]
    fn every_base_library_has_a_piece() {
        for library in clarity_gen::BASE_LIBRARIES {
            assert!(PIECES.iter().any(|p| p.command.contains(&library)), "{library} has no tick");
        }
    }

    /// A library added later (a `runtimes/*-ffi` crate) gets its tick, or this fails.
    #[test]
    fn every_library_crate_of_the_repository_has_a_piece() {
        let runtimes = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes");
        let mut seen = 0;
        for entry in std::fs::read_dir(&runtimes).unwrap().flatten() {
            let manifest = entry.path().join("Cargo.toml");
            if !entry.file_name().to_string_lossy().ends_with("-ffi") || !manifest.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&manifest).unwrap();
            let name = text.lines().find_map(|l| l.strip_prefix("name = \"")).and_then(|r| r.strip_suffix('"')).expect("the crate's name");
            assert!(PIECES.iter().any(|p| p.command.contains(&name)), "{name} has no tick in Clarity settings > Rebuild");
            seen += 1;
        }
        assert!(seen >= 7, "the library crates are found ({seen})");
    }

    #[cfg(unix)]
    #[test]
    fn a_command_s_lines_come_out_and_its_failure_says_so() {
        let mut seen = Vec::new();
        run(&std::env::temp_dir(), &["sh".into(), "-c".into(), "echo one; echo two >&2".into()], |l| seen.push(l.to_string())).unwrap();
        seen.sort();
        assert_eq!(seen, ["one", "two"]);
        assert!(run(&std::env::temp_dir(), &["sh".into(), "-c".into(), "exit 3".into()], |_| {}).is_err());
    }
}
