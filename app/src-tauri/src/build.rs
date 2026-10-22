//! Run and Build: the open project is generated into its `out/` folder and
//! compiled with the stack's own toolchain (cargo for Rust, dotnet for C#). Run starts the
//! application; Build makes a release program. Engines whose source changed
//! are built first. The toolchain's output is sent to the frontend line by line.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
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


#[derive(Default)]
pub struct BuildState {
    pub(crate) busy: AtomicBool,
    /// The application Run started, so Stop and the next Run can end it.
    running: Mutex<Option<Child>>,
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

/// A build starts: the Output panel's lines are kept from here.
pub(crate) fn begin_output(app: &AppHandle) {
    if let Ok(mut output) = app.state::<BuildState>().output.lock() {
        output.begin();
    }
    // The Output panel opens and empties, also when the agent started the build.
    let _ = app.emit(BUILD_STARTED, ());
}

/// Sends a line to the Output panel and keeps it.
pub(crate) fn send_line(app: &AppHandle, line: String) {
    if let Ok(mut output) = app.state::<BuildState>().output.lock() {
        output.line(&line);
    }
    let _ = app.emit(BUILD_LINE, line);
}

/// Ends a build: keeps how it ended and tells the screen.
pub(crate) fn send_done(app: &AppHandle, done: BuildDone) {
    if let Ok(mut output) = app.state::<BuildState>().output.lock() {
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
            missing: t.missing,
        })
        .collect()
}

/// Generates the project of the project file `file` into its output folder.
/// Returns the output folder and how its stack builds it, as a debug or
/// (with `release`) a release program.
pub fn generate_out(file: &Path, release: bool) -> Result<(PathBuf, clarity_gen::Toolchain, clarity_project::Stack), String> {
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
    let toolchain = clarity_gen::toolchain(&settings, release).ok_or_else(|| format!("{shown}: Clarity cannot build this stack yet"))?;
    let out = clarity_project::out_dir(file, &settings);
    clarity_gen::write_out(&out, &files, clarity_gen::build_output(settings.stack.backend)).map_err(|err| format!("cannot write {}: {err}", out.display()))?;
    Ok((out, toolchain, settings.stack))
}

/// Why the generated project in `out` cannot be built on this machine: the
/// tools of its stack that are missing; none when they are all there.
pub fn cannot_build(tools: &clarity_gen::tools::StackTools, out: &Path) -> Option<String> {
    (!tools.missing.is_empty()).then(|| {
        format!(
            "the project is generated in {}; to build {} here, install {}",
            out.display(),
            clarity_gen::tools::stack_name(tools.backend, tools.ui),
            tools.missing.join(", and ")
        )
    })
}

fn send_lines(app: &AppHandle, from: impl std::io::Read) {
    for line in BufReader::new(from).lines().map_while(Result::ok) {
        send_line(app, line);
    }
}

/// Generates, builds and (with `run`) starts the open project. Returns at
/// once; progress arrives as events.
pub fn start(app: AppHandle, file: PathBuf, release: bool, run: bool) -> Result<(), String> {
    let state = app.state::<BuildState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("a build is already running".to_string());
    }
    begin_output(&app);
    // Generating works without the stack's tools; building needs them.
    let found = generate_out(&file, release).and_then(|(out, toolchain, stack)| {
        let tools = clarity_gen::tools::check(stack.backend, stack.ui, &clarity_gen::tools::System);
        match cannot_build(&tools, &out) {
            Some(why) => Err(why),
            None => Ok((out, toolchain)),
        }
    });
    let (out, toolchain) = match found {
        Ok(found) => found,
        Err(err) => {
            state.busy.store(false, Ordering::SeqCst);
            if let Ok(mut output) = state.output.lock() {
                output.finish(false, &err);
            }
            return Err(err);
        }
    };
    let dir = clarity_project::module_dir(&file);
    std::thread::spawn(move || {
        // The engines whose source changed first, so the application has them as written.
        let done = match crate::engines::build_changed(&app, &dir) {
            Ok(()) => build(&app, &out, &toolchain, run),
            Err(message) => BuildDone { ok: false, message, binary: None },
        };
        app.state::<BuildState>().busy.store(false, Ordering::SeqCst);
        send_done(&app, done);
    });
    Ok(())
}

fn build(app: &AppHandle, out: &Path, toolchain: &clarity_gen::Toolchain, run: bool) -> BuildDone {
    let failed = |message: String| BuildDone {
        ok: false,
        message,
        binary: None,
    };
    let mut command = Command::new(toolchain.tool);
    command
        .args(&toolchain.args)
        .envs(toolchain.env.iter().copied())
        .current_dir(out)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // The command first, so the output says what runs where.
    send_line(app, format!("{} {} (in {})", toolchain.tool, toolchain.args.join(" "), out.display()));
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => return failed(format!("cannot start {}: {err}; {}", toolchain.tool, toolchain.missing)),
    };
    // cargo reports on stderr, dotnet on stdout: both go to the output.
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
        Ok(status) if status.success() => {}
        Ok(_) => return failed("the build failed; see the output".to_string()),
        Err(err) => return failed(format!("{} stopped: {err}", toolchain.tool)),
    }
    let binary = out.join(&toolchain.program);
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

    /// A copy of the example, its stack line replaced by `stack`.
    fn example(name: &str, stack: &str) -> PathBuf {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let dir = std::env::temp_dir().join(format!("clarity-run-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let text = std::fs::read_to_string(examples.join("shop.clarity")).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("shop.clarity"), text.replace("stack rust web desktop", stack)).unwrap();
        // The modules, the migration steps, the translations, the engines and the resources.
        for folder in ["shop", "shop/migrations/main", "shop/translations", "shop/engines", "shop/resources"] {
            std::fs::create_dir_all(dir.join(folder)).unwrap();
            for entry in std::fs::read_dir(examples.join(folder)).unwrap() {
                let path = entry.unwrap().path();
                if path.is_file() {
                    std::fs::copy(&path, dir.join(folder).join(path.file_name().unwrap())).unwrap();
                }
            }
        }
        dir
    }

    #[test]
    fn generates_the_example_into_out() {
        let dir = example("rust", "stack rust web desktop");
        let (out, toolchain, _) = generate_out(&dir.join("shop.clarity"), true).unwrap();
        assert_eq!(out, dir.join("shop/out"));
        assert!(out.join("Cargo.toml").exists());
        assert!(out.join("src/main.rs").exists());
        assert_eq!(toolchain.tool, "cargo");
        assert_eq!(out.join(&toolchain.program), out.join(if cfg!(windows) { "target/release/shop.exe" } else { "target/release/shop" }));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Without the stack's tools the project is still generated, and Build
    /// says what to install, so it can be built here later or elsewhere.
    #[test]
    fn missing_tools_say_what_to_install_after_generating() {
        let check = clarity_gen::tools::StackTools {
            backend: clarity_project::Backend::Csharp,
            ui: clarity_project::Ui::Avalonia,
            generated: true,
            found: vec![],
            missing: vec!["the .NET 10 SDK, from https://dotnet.microsoft.com/download".into()],
        };
        assert_eq!(
            cannot_build(&check, Path::new("/p/shop/out")),
            Some(
                "the project is generated in /p/shop/out; to build C# with Avalonia here, install the .NET 10 SDK, from https://dotnet.microsoft.com/download"
                    .to_string()
            )
        );
        let ready = clarity_gen::tools::StackTools { missing: vec![], found: vec![".NET SDK 10.0.402".into()], ..check };
        assert_eq!(cannot_build(&ready, Path::new("/p/shop/out")), None);
    }

    /// What the Output panel shows is kept, so the agent can read it.
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

    #[test]
    fn a_csharp_project_is_generated_for_dotnet() {
        let dir = example("csharp", "stack csharp avalonia desktop");
        let (out, toolchain, _) = generate_out(&dir.join("shop.clarity"), false).unwrap();
        assert!(out.join("Shop.csproj").exists());
        assert!(!out.join("Cargo.toml").exists());
        assert_eq!(toolchain.tool, "dotnet");
        assert!(toolchain.program.starts_with("bin/Debug"), "{}", toolchain.program.display());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
