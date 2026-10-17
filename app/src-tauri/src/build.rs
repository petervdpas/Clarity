//! Run and Build: the open project is generated into its `out/` folder and
//! compiled with the stack's own toolchain (cargo for Rust). Run starts the
//! application; Build makes a release program. The toolchain's output is
//! sent to the frontend line by line.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

/// One line of toolchain output.
const BUILD_LINE: &str = "build-line";
/// The end of a build.
const BUILD_DONE: &str = "build-done";


#[derive(Default)]
pub struct BuildState {
    busy: AtomicBool,
    /// The application Run started, so Stop and the next Run can end it.
    running: Mutex<Option<Child>>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BuildDone {
    pub ok: bool,
    pub message: String,
    /// The program that was built.
    pub binary: Option<String>,
}

/// Generates the project of the project file `file` into its output folder.
/// Returns the output folder and the name of the program cargo will build.
pub fn generate_out(file: &Path) -> Result<(PathBuf, String), String> {
    let dir = clarity_project::module_dir(file);
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let loaded = clarity_project::load(file);
    let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
    let settings = match (loaded.settings, loaded.problems.first()) {
        (Some(settings), None) => settings,
        (_, Some(problem)) => return Err(format!("{shown}: {problem}")),
        (None, None) => return Err(format!("{shown} cannot be read")),
    };
    let built = clarity_project::build_settings(&dir, &settings);
    let files = clarity_gen::generate(&project, &built).map_err(|errors| errors.join("; "))?;
    let out = clarity_project::out_dir(file, &settings);
    clarity_gen::write_out(&out, &files, clarity_gen::build_output(settings.stack.backend)).map_err(|err| format!("cannot write {}: {err}", out.display()))?;
    Ok((out, clarity_gen::binary_name(&settings)))
}

fn program(out: &Path, name: &str, release: bool) -> PathBuf {
    let profile = if release { "release" } else { "debug" };
    let file = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    out.join("target").join(profile).join(file)
}

/// Generates, builds and (with `run`) starts the open project. Returns at
/// once; progress arrives as events.
pub fn start(app: AppHandle, file: PathBuf, release: bool, run: bool) -> Result<(), String> {
    let state = app.state::<BuildState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("a build is already running".to_string());
    }
    let (out, name) = match generate_out(&file) {
        Ok(found) => found,
        Err(err) => {
            state.busy.store(false, Ordering::SeqCst);
            return Err(err);
        }
    };
    std::thread::spawn(move || {
        let done = build(&app, &out, &name, release, run);
        app.state::<BuildState>().busy.store(false, Ordering::SeqCst);
        let _ = app.emit(BUILD_DONE, done);
    });
    Ok(())
}

fn build(app: &AppHandle, out: &Path, name: &str, release: bool, run: bool) -> BuildDone {
    let failed = |message: String| BuildDone {
        ok: false,
        message,
        binary: None,
    };
    let mut command = Command::new("cargo");
    // Plain lines: no colors and no progress bars, which only make sense in a terminal.
    command
        .args(["build", "--color", "never"])
        .env("CARGO_TERM_PROGRESS_WHEN", "never")
        .current_dir(out)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if release {
        command.arg("--release");
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => return failed(format!("cannot start cargo: {err}; is Rust installed?")),
    };
    if let Some(stderr) = child.stderr.take() {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = app.emit(BUILD_LINE, line);
        }
    }
    match child.wait() {
        Ok(status) if status.success() => {}
        Ok(_) => return failed("the build failed; see the output".to_string()),
        Err(err) => return failed(format!("cargo stopped: {err}")),
    }
    let binary = program(out, name, release);
    if run {
        stop(app);
        match Command::new(&binary).current_dir(out).spawn() {
            Ok(child) => {
                if let Ok(mut running) = app.state::<BuildState>().running.lock() {
                    *running = Some(child);
                }
            }
            Err(err) => return failed(format!("cannot start {}: {err}", binary.display())),
        }
    }
    BuildDone {
        ok: true,
        message: if run { "running".to_string() } else { format!("built {}", binary.display()) },
        binary: Some(binary.display().to_string()),
    }
}

/// Ends the application Run started, if it still runs.
pub fn stop(app: &AppHandle) {
    if let Ok(mut running) = app.state::<BuildState>().running.lock()
        && let Some(mut child) = running.take()
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_the_example_into_out() {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let dir = std::env::temp_dir().join(format!("clarity-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("shop")).unwrap();
        std::fs::copy(examples.join("shop.clarity"), dir.join("shop.clarity")).unwrap();
        // The modules and the migration steps.
        for folder in ["shop", "shop/migrations/main"] {
            std::fs::create_dir_all(dir.join(folder)).unwrap();
            for entry in std::fs::read_dir(examples.join(folder)).unwrap() {
                let path = entry.unwrap().path();
                if path.is_file() {
                    std::fs::copy(&path, dir.join(folder).join(path.file_name().unwrap())).unwrap();
                }
            }
        }
        let (out, name) = generate_out(&dir.join("shop.clarity")).unwrap();
        assert_eq!(name, "shop");
        assert_eq!(out, dir.join("shop/out"));
        assert!(out.join("Cargo.toml").exists());
        assert!(out.join("src/main.rs").exists());
        assert_eq!(program(&out, &name, true), out.join("target/release/shop"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
