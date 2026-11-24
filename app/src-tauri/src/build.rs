//! Run and Build: the open project is generated into its `out/` folder and
//! compiled with the stack's own toolchain (cargo for Rust, dotnet for C#). Run starts the
//! application; Build makes a release program. Engines whose source changed
//! are built first. The toolchain's output is sent to the frontend line by line.

use crate::fail_safe::Held;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

/// One line of toolchain output.
pub(crate) const BUILD_LINE: &str = "build-line";
/// A build starts.
pub(crate) const BUILD_STARTED: &str = "build-started";
/// The end of a build.
pub(crate) const BUILD_DONE: &str = "build-done";
/// The application Run started runs: an AppRunning.
pub(crate) const APP_RUNNING: &str = "app-running";
/// The application Run started has ended, by Stop or by itself.
pub(crate) const APP_ENDED: &str = "app-ended";

/// The application Run started, as the screen shows it: where a browser
/// opens it, when it is served to one (the web target).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AppRunning {
    pub address: Option<String>,
}

pub use clarity_build::{Generated, Source, can_serve_at};

#[derive(Default)]
pub struct BuildState {
    pub(crate) busy: AtomicBool,
    /// The application Run started, so Stop and the next Run can end it.
    running: Mutex<Option<Child>>,
    /// Where a browser opens the application Run started, when it is served to one.
    address: Mutex<Option<String>>,
    /// What the Output panel types to a command-line program Run started.
    input: Mutex<Option<ChildStdin>>,
    /// What the Output panel shows of the last build, for the agent.
    pub(crate) output: Mutex<Output>,
}

/// How many lines of a build's output are kept, as many as the Output panel keeps.
pub(crate) const OUTPUT_LINES: usize = 400;

/// The output of the last build: its lines and how it ended.
#[derive(Debug, Default)]
pub struct Output {
    started: bool,
    lines: std::collections::VecDeque<String>,
    left_out: usize,
    /// Whether it succeeded, and its message; none while it runs.
    ended: Option<(bool, String)>,
}

impl Output {
    /// A build starts: the output of the one before goes.
    pub fn begin(&mut self) {
        *self = Output { started: true, ..Output::default() };
    }

    pub fn line(&mut self, line: &str) {
        if self.lines.len() == OUTPUT_LINES {
            self.lines.pop_front();
            self.left_out += 1;
        }
        self.lines.push_back(line.to_string());
    }

    pub fn finish(&mut self, ok: bool, message: &str) {
        self.ended = Some((ok, message.to_string()));
    }

    /// What a screen that reloaded picks up: its lines, and how it ended; none before a build.
    pub fn snapshot(&self, busy: bool) -> Option<BuildSnapshot> {
        self.started.then(|| BuildSnapshot {
            busy,
            lines: self.lines.iter().cloned().collect(),
            ended: self.ended.clone().map(|(ok, message)| BuildEnded { ok, message }),
        })
    }

    /// The output as the agent reads it: how the build ended, then its lines.
    pub fn text(&self) -> String {
        if !self.started {
            return "Nothing was built or run since Clarity started.".into();
        }
        let state = match &self.ended {
            None => "The build is still running.".to_string(),
            Some((true, message)) => format!("The last build succeeded: {message}."),
            Some((false, message)) => format!("The last build failed: {message}."),
        };
        let count = self.lines.len();
        let which = match (self.left_out, count) {
            (0, 1) => "Its output (1 line)".to_string(),
            (0, n) => format!("Its output ({n} lines)"),
            (left, n) => format!("Its last {n} lines ({left} earlier ones are left out)"),
        };
        let lines: Vec<&str> = self.lines.iter().map(String::as_str).collect();
        format!("{state} {which}:\n{}", lines.join("\n"))
    }
}

/// The last build as the Output panel shows it, for a screen that reloaded
/// halfway (Vite while developing, or a crash of the page): whether it still
/// runs, its lines and how it ended.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BuildSnapshot {
    pub busy: bool,
    pub lines: Vec<String>,
    pub ended: Option<BuildEnded>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BuildEnded {
    pub ok: bool,
    pub message: String,
}

/// The last build, for a screen that reloaded; none before one.
pub fn snapshot(app: &AppHandle) -> Option<BuildSnapshot> {
    let state = app.state::<BuildState>();
    let busy = state.busy.load(Ordering::SeqCst);
    state.output.held().ok().and_then(|o| o.snapshot(busy))
}

/// A build starts: the Output panel's lines are kept from here.
pub(crate) fn begin_output(app: &AppHandle) {
    if let Ok(mut output) = app.state::<BuildState>().output.held() {
        output.begin();
    }
    // The Output panel opens and empties, also when the agent started the build.
    let _ = app.emit(BUILD_STARTED, ());
}

/// Sends a line to the Output panel and keeps it.
pub(crate) fn send_line(app: &AppHandle, line: String) {
    if let Ok(mut output) = app.state::<BuildState>().output.held() {
        output.line(&line);
    }
    let _ = app.emit(BUILD_LINE, line);
}

/// Ends a build: keeps how it ended and tells the screen.
pub(crate) fn send_done(app: &AppHandle, done: BuildDone) {
    if let Ok(mut output) = app.state::<BuildState>().output.held() {
        output.finish(done.ok, &done.message);
    }
    let _ = app.emit(BUILD_DONE, done);
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BuildDone {
    pub ok: bool,
    pub message: String,
    /// The program that was built.
    pub binary: Option<String>,
}

/// The build tools of a stack on this machine, for New project and the
/// Project tab (P3).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct StackTools {
    pub backend: clarity_project::Backend,
    pub ui: clarity_project::Ui,
    /// Whether Clarity generates this stack already.
    pub generated: bool,
    /// Whether Build and Run can work.
    pub ready: bool,
    /// The tools found, as they name themselves.
    pub found: Vec<String>,
    /// What to install, with where to get it.
    pub missing: Vec<String>,
    /// The heading of the manual's setup page (`clarity_gen::tools::SETUP_TOPIC`)
    /// that tells how to install the first of `missing`.
    pub guide: Option<String>,
}

/// Looks for the build tools of every stack the catalog offers.
pub fn stack_tools() -> Vec<StackTools> {
    clarity_gen::tools::check_all(&clarity_gen::tools::System)
        .into_iter()
        .map(|t| StackTools {
            ready: t.ready(),
            backend: t.backend,
            ui: t.ui,
            generated: t.generated,
            found: t.found,
            guide: t.guides.first().map(|g| g.to_string()),
            missing: t.missing,
        })
        .collect()
}

/// Generates the project (the Generate button): the one way the clarity command and the agents generate too (`clarity_build::generate`).
pub fn generate_source(file: &Path) -> Result<Source, String> {
    clarity_build::generate(file, None)
}

/// Builds a custom part or a library on its own, in its folder, its output
/// in the Output panel: what Build in the part's view does. `steps` run in
/// order, with `env`; the first to fail ends it.
pub fn start_part(app: AppHandle, name: String, folder: PathBuf, steps: Vec<Vec<String>>, env: Vec<(String, String)>) -> Result<(), String> {
    start_steps(app, folder, steps, env, format!("{name} built"), format!("{name} did not build"))
}

/// Runs `steps` in `folder` one after the other, their output in the Output panel; how it ended as `ok` or `failed`.
fn start_steps(app: AppHandle, folder: PathBuf, steps: Vec<Vec<String>>, env: Vec<(String, String)>, ok: String, failed: String) -> Result<(), String> {
    let state = app.state::<BuildState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("a build is already running".to_string());
    }
    begin_output(&app);
    std::thread::spawn(move || {
        let mut done = BuildDone { ok: true, message: ok, binary: None };
        for step in &steps {
            if let Err(message) = run_tool(&app, &folder, step, &env) {
                done = BuildDone { ok: false, message: format!("{failed}: {message}"), binary: None };
                break;
            }
        }
        app.state::<BuildState>().busy.store(false, Ordering::SeqCst);
        // A tool that says nothing when it is happy (`go build`) would leave
        // the panel showing the command alone, which reads as a build that
        // never ended, so how it went is a line of the output as well.
        send_line(&app, done.message.clone());
        send_done(&app, done);
    });
    Ok(())
}

/// Runs one tool in `dir`, its output in the Output panel.
fn run_tool(app: &AppHandle, dir: &Path, step: &[String], env: &[(String, String)]) -> Result<(), String> {
    let Some((tool, args)) = step.split_first() else { return Ok(()) };
    send_line(app, format!("{tool} {} (in {})", args.join(" "), dir.display()));
    let mut child = clarity_project::background_command(clarity_gen::tools::Probe::find(&clarity_gen::tools::System, tool).unwrap_or_else(|| tool.into()))
        .args(args)
        .env("PATH", clarity_gen::tools::tool_path())
        // The C and C++ compilers chosen in the Environment settings, for CMake.
        .envs(clarity_gen::tools::tool_env())
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("cannot start {tool}: {err}; is it installed?"))?;
    let stdout = child.stdout.take().map(|stdout| {
        let app = app.clone();
        std::thread::spawn(move || send_lines(&app, stdout))
    });
    if let Some(stderr) = child.stderr.take() {
        send_lines(app, stderr);
    }
    if let Some(reader) = stdout {
        let _ = reader.join();
    }
    match child.wait() {
        Ok(status) if status.success() => Ok(()),
        _ => Err(format!("{tool} failed; see the output")),
    }
}

fn send_lines(app: &AppHandle, from: impl std::io::Read) {
    for line in BufReader::new(from).lines().map_while(Result::ok) {
        send_line(app, line);
    }
}

/// Generates, builds and (with `run`) starts the open project. With a
/// `command_line`, the command-line program runs with its words, its output
/// in the Output panel and what is typed there as its input. Returns at
/// once; progress arrives as events.
pub fn start(app: AppHandle, file: PathBuf, release: bool, run: bool, command_line: Option<String>) -> Result<(), String> {
    let state = app.state::<BuildState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("a build is already running".to_string());
    }
    begin_output(&app);
    // Generating works without the stack's tools; building needs them.
    let found = clarity_build::generate_out(&file, release, command_line.is_some(), clarity_build::GENERATE_IN_THE_IDE).and_then(|generated| {
        // A program served to a browser alone runs as a web server, at the project's port.
        let serves = clarity_build::serves(&file, &generated.stack, command_line.is_some());
        clarity_build::ready(&generated).map(|()| (generated, serves))
    });
    let (generated, serves) = match found {
        Ok(found) => found,
        Err(err) => {
            state.busy.store(false, Ordering::SeqCst);
            if let Ok(mut output) = state.output.held() {
                output.finish(false, &err);
            }
            return Err(err);
        }
    };
    let dir = clarity_project::module_dir(&file);
    std::thread::spawn(move || {
        // The engines whose source changed first, so the application has them as written.
        let done = match crate::engines::build_changed(&app, &dir) {
            // Run starts the last build when nothing changed since: the
            // generated files, the engines and the parts are as it had them.
            Ok(false) if run && let Some((toolchain, _)) = &generated.built => build(&app, &generated, toolchain, false, run, serves, command_line.as_deref()),
            Ok(_) => match clarity_build::toolchain(&generated, &|line| send_line(&app, line)) {
                Ok(toolchain) => build(&app, &generated, &toolchain, true, run, serves, command_line.as_deref()),
                Err(message) => BuildDone { ok: false, message, binary: None },
            },
            Err(message) => BuildDone { ok: false, message, binary: None },
        };
        app.state::<BuildState>().busy.store(false, Ordering::SeqCst);
        send_done(&app, done);
    });
    Ok(())
}

/// The folder of Clarity.UI's library for a build, its fetching said in the Output panel.
pub(crate) fn library_folder(app: &AppHandle) -> Result<PathBuf, String> {
    clarity_build::library_folder(&|line| send_line(app, line))
}

/// Builds the generated program with `toolchain` (unless `building` is
/// false: it is built already), records what it was made from, and with
/// `run` starts it.
fn build(app: &AppHandle, generated: &Generated, toolchain: &clarity_gen::Toolchain, building: bool, run: bool, serves: Option<String>, command_line: Option<&str>) -> BuildDone {
    let out = generated.out.as_path();
    let failed = |message: String| BuildDone {
        ok: false,
        message,
        binary: None,
    };
    if building && let Err(message) = clarity_build::compile(generated, toolchain, &|line| send_line(app, line)) {
        return failed(message);
    }
    let binary = out.join(&toolchain.program);
    // The program does not migrate (docs/ledger/migrations.md): Run migrates its databases first, with clarity-migrate.
    if run && let Err(message) = clarity_build::migrate(generated, &|line| send_line(app, line)) {
        return failed(message);
    }
    if run && let Some(line) = command_line {
        return run_command_line(app, out, &binary, line, generated.data.as_deref());
    }
    if run {
        stop(app);
        // A web application listens at the project's port, which no other program may hold.
        if let Some(address) = &serves
            && let Err(why) = can_serve_at(address)
        {
            return failed(why);
        }
        let address = serves;
        // What starts, and where: after a build, or the last one when nothing changed.
        send_line(app, format!("{} (in {})", binary.display(), out.display()));
        let mut command = clarity_project::background_command(&binary);
        command.current_dir(out);
        // The program works on the project's data, not the person's (conformance/data-folder.json).
        if let Some(data) = &generated.data {
            command.env("CLARITY_DATA", data);
        }
        if let Some(address) = &address {
            command.args(["--address", address]);
        }
        match command.spawn() {
            Ok(child) => {
                // Remembered while it runs, so a Clarity that ends without stopping it ends it when it starts again.
                crate::left_running::remember(&clarity_project::clarity_home(), child.id(), &binary);
                started(app, child, address.map(|a| format!("http://{a}")))
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

/// Runs the command-line program with the words of `line`, in its folder:
/// what it writes goes to the Output panel line by line, and what the panel
/// sends with `type_line` is its input, as a terminal's (`CLARITY_TERMINAL`).
/// Returns when it ends, with its exit code.
fn run_command_line(app: &AppHandle, out: &Path, binary: &Path, line: &str, data: Option<&Path>) -> BuildDone {
    stop(app);
    let words = clarity_gen::command_words(line);
    send_line(app, format!("{} {}", binary.file_name().unwrap_or_default().to_string_lossy(), line.trim()));
    let mut command = clarity_project::background_command(binary);
    command.args(&words).current_dir(out).env("CLARITY_TERMINAL", "1").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(data) = data {
        command.env("CLARITY_DATA", data);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => return BuildDone { ok: false, message: format!("cannot start {}: {err}", binary.display()), binary: None },
    };
    if let Ok(mut input) = app.state::<BuildState>().input.held() {
        *input = child.stdin.take();
    }
    let readers: Vec<_> = [child.stdout.take().map(|o| Box::new(o) as Box<dyn std::io::Read + Send>), child.stderr.take().map(|e| Box::new(e) as Box<dyn std::io::Read + Send>)]
        .into_iter()
        .flatten()
        .map(|from| {
            let app = app.clone();
            std::thread::spawn(move || send_words(&app, from))
        })
        .collect();
    if let Ok(mut running) = app.state::<BuildState>().running.held() {
        *running = Some(child);
    }
    for reader in readers {
        let _ = reader.join();
    }
    let status = app.state::<BuildState>().running.held().ok().and_then(|mut r| r.take()).and_then(|mut c| c.wait().ok());
    if let Ok(mut input) = app.state::<BuildState>().input.held() {
        *input = None;
    }
    let code = status.and_then(|s| s.code());
    BuildDone {
        ok: code == Some(0),
        message: code.map_or("stopped".to_string(), |c| format!("ended with exit code {c}")),
        binary: Some(binary.display().to_string()),
    }
}

/// Sends what a program writes to the Output panel: each line, and a
/// question that waits on the same line for its answer.
fn send_words(app: &AppHandle, mut from: impl std::io::Read) {
    let mut pending = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = match from.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        pending.extend_from_slice(&chunk[..read]);
        while let Some(end) = pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = pending.drain(..=end).collect();
            send_line(app, String::from_utf8_lossy(&line[..line.len() - 1]).trim_end_matches('\r').to_string());
        }
        // A prompt has no line end: it shows while the program waits.
        if !pending.is_empty() && read < chunk.len() {
            send_line(app, String::from_utf8_lossy(&pending).to_string());
            pending.clear();
        }
    }
    if !pending.is_empty() {
        send_line(app, String::from_utf8_lossy(&pending).to_string());
    }
}

/// Types `line` to the command-line program Run started, as its input.
pub fn type_line(app: &AppHandle, line: &str) -> Result<(), String> {
    let state = app.state::<BuildState>();
    let mut input = state.input.held().map_err(|_| "the input is poisoned")?;
    let Some(stdin) = input.as_mut() else { return Err("no command-line program is running".to_string()) };
    writeln!(stdin, "{line}").and_then(|()| stdin.flush()).map_err(|err| format!("cannot type to the program: {err}"))?;
    drop(input);
    send_line(app, format!("> {line}"));
    Ok(())
}

/// The application Run started runs: it is kept for Stop, the screen is
/// told, and when it ends by itself the screen is told that too.
fn started(app: &AppHandle, child: Child, address: Option<String>) {
    let state = app.state::<BuildState>();
    let id = child.id();
    if let Ok(mut running) = state.running.held() {
        *running = Some(child);
    }
    if let Ok(mut kept) = state.address.held() {
        kept.clone_from(&address);
    }
    let _ = app.emit(APP_RUNNING, AppRunning { address });
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(400));
            let state = app.state::<BuildState>();
            let Ok(mut running) = state.running.held() else { return };
            match running.as_mut() {
                // Stop or a new Run ended it, and said so.
                Some(child) if child.id() != id => return,
                None => return,
                Some(child) => {
                    if !matches!(child.try_wait(), Ok(None)) {
                        running.take();
                        drop(running);
                        ended(&app);
                        return;
                    }
                }
            }
        }
    });
}

/// The application Run started has ended: the screen is told.
fn ended(app: &AppHandle) {
    crate::left_running::forget(&clarity_project::clarity_home());
    if let Ok(mut address) = app.state::<BuildState>().address.held() {
        *address = None;
    }
    let _ = app.emit(APP_ENDED, ());
}

/// The application Run started, while it runs.
pub fn running(app: &AppHandle) -> Option<AppRunning> {
    let state = app.state::<BuildState>();
    let mut running = state.running.held().ok()?;
    let child = running.as_mut()?;
    if !matches!(child.try_wait(), Ok(None)) {
        return None;
    }
    Some(AppRunning { address: state.address.held().ok()?.clone() })
}

/// Ends the application Run started, if it still runs.
pub fn stop(app: &AppHandle) {
    let child = app.state::<BuildState>().running.held().ok().and_then(|mut r| r.take());
    if let Some(mut child) = child {
        let _ = child.kill();
        let _ = child.wait();
        ended(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DevTools' tab commands run on the async runtime's threads: a download
    /// there fails as a download, never with a panic of the blocking client.
    #[test]
    fn a_download_on_the_async_runtime_does_not_panic() {
        tauri::async_runtime::block_on(async {
            assert!(clarity_build::download("http://127.0.0.1:9/library.tar.gz").is_err());
            assert!(!clarity_build::answers("http://127.0.0.1:9/"));
        });
    }

    #[test]
    fn a_screen_that_reloaded_halfway_picks_the_build_up_again() {
        let mut output = Output::default();
        assert_eq!(output.snapshot(false), None, "nothing to pick up before a build");
        output.begin();
        output.line("Compiling clarity-ui v0.5.0");
        assert_eq!(output.snapshot(true), Some(BuildSnapshot { busy: true, lines: vec!["Compiling clarity-ui v0.5.0".into()], ended: None }));
        output.finish(true, "built");
        assert_eq!(output.snapshot(false).unwrap().ended, Some(BuildEnded { ok: true, message: "built".into() }));
    }

    #[test]
    fn the_output_is_kept_for_the_agent() {
        let mut output = Output::default();
        assert_eq!(output.text(), "Nothing was built or run since Clarity started.");

        output.begin();
        output.line("dotnet build (in /p/out)");
        output.line("Noughts.cs(58,14): error CS0103: The name 'Lists' does not exist");
        assert_eq!(output.text(), "The build is still running. Its output (2 lines):\ndotnet build (in /p/out)\nNoughts.cs(58,14): error CS0103: The name 'Lists' does not exist");
        output.finish(false, "the build failed; see the output");
        assert!(output.text().starts_with("The last build failed: the build failed; see the output. Its output (2 lines):\ndotnet build"), "{}", output.text());

        // A new build starts a new output; a long one keeps its last lines, as the panel does.
        output.begin();
        for i in 1..=OUTPUT_LINES + 5 {
            output.line(&format!("line {i}"));
        }
        output.finish(true, "running");
        let text = output.text();
        assert!(text.starts_with(&format!("The last build succeeded: running. Its last {OUTPUT_LINES} lines (5 earlier ones are left out):\nline 6\n")), "{}", &text[..120]);
        assert!(text.ends_with(&format!("line {}", OUTPUT_LINES + 5)));
        assert!(!text.contains("dotnet build"));
    }
}
