//! Run and Build: the open project is generated into its `out/` folder and
//! compiled with the stack's own toolchain (cargo for Rust, dotnet for C#). Run starts the
//! application; Build makes a release program. Engines whose source changed
//! are built first. The toolchain's output is sent to the frontend line by line.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
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

/// Whether a web application Run starts can listen at `address`, the
/// project's port: no other program may listen there already.
pub fn can_serve_at(address: &str) -> Result<(), String> {
    match std::net::TcpListener::bind(address) {
        Ok(_) => Ok(()),
        Err(_) => {
            let port = address.rsplit(':').next().unwrap_or(address);
            Err(format!("another program listens on port {port}; stop it, or set another port on the Project tab"))
        }
    }
}


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
    /// The heading of the manual's setup page (`clarity_gen::tools::SETUP_TOPIC`)
    /// that tells how to install the first of `missing`.
    pub guide: Option<String>,
    /// What the web target needs beyond these (DESIGN 7.6), found and missing.
    pub web_found: Vec<String>,
    pub web_missing: Vec<String>,
    /// Whether Clarity can install what the web target is missing, and whether the system asks the person's permission for it.
    pub web_installable: bool,
    pub web_elevated: bool,
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
            web_found: t.web_found,
            web_missing: t.web_missing,
            web_installable: t.web_install.is_some(),
            web_elevated: t.web_install.is_some_and(|i| i.elevated),
        })
        .collect()
}

/// Generates the project of the project file `file` into its output folder.
/// Returns the folder of the program to build, the desktop one or (with
/// `cli`) the command-line one, and how its stack builds it, as a debug or
/// (with `release`) a release program.
/// A generated program: where it is, how to build it, and what its last
/// build was made from.
pub struct Generated {
    pub out: PathBuf,
    pub toolchain: clarity_gen::Toolchain,
    pub stack: clarity_project::Stack,
    /// The mark of its generated files, which a build records.
    pub fingerprint: String,
    pub release: bool,
    /// The last build, while it was made from these same files and its
    /// program is there: Run starts it without building again.
    pub built: Option<(clarity_gen::Toolchain, bool)>,
}

pub fn generate_out(file: &Path, release: bool, cli: bool) -> Result<Generated, String> {
    let dir = clarity_project::module_dir(file);
    // Each web API's module follows its description and picks (DESIGN 5.23).
    clarity_project::refresh_apis(file)?;
    let mut project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    // Each native part's header follows its declaration, and a new part gets a starter (DESIGN 5.19).
    let written = clarity_gen::parts::write_part_folders(&dir, &project).map_err(|err| format!("cannot write the parts' folders: {err}"))?;
    if !written.is_empty() {
        project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    }
    let loaded = clarity_project::load(file);
    let shown = file.file_name().unwrap_or_default().to_string_lossy().to_string();
    let settings = match (loaded.settings, loaded.problems.first()) {
        (Some(settings), None) => settings,
        (_, Some(problem)) => return Err(format!("{shown}: {problem}")),
        (None, None) => return Err(format!("{shown} cannot be read")),
    };
    let built = clarity_project::build_settings(&dir, &settings);
    let programs = clarity_gen::programs(&project, &built).map_err(|errors| errors.join("; "))?;
    let out = clarity_project::out_dir(file, &settings);
    clarity_gen::write_programs(&out, &programs, settings.stack.backend, &settings.keeps).map_err(|err| format!("cannot write {}: {err}", out.display()))?;
    // Each program is in its own folder of the output; one without windows has only the command line.
    let cli = cli || !settings.stack.has_ui();
    if cli && !settings.stack.has(clarity_project::Target::Cli) {
        return Err(format!("{shown} has no command-line program; add the cli target to the stack"));
    }
    let target = if cli { clarity_project::Target::Cli } else { clarity_gen::windowed_target(&settings) };
    let toolchain_of = |release: bool| if cli { clarity_gen::cli_toolchain(&settings, release) } else { clarity_gen::toolchain(&settings, release) };
    let toolchain = toolchain_of(release).ok_or_else(|| format!("{shown}: Clarity cannot build this stack yet"))?;
    let out = clarity_project::target_dir(&out, target);
    let fingerprint = programs.iter().find(|p| p.target == target).map(clarity_gen::fingerprint).unwrap_or_default();
    let built = clarity_gen::last_build(&out, &fingerprint)
        .and_then(|release| toolchain_of(release).map(|t| (t, release)))
        .filter(|(t, _)| out.join(&t.program).exists());
    Ok(Generated { out, toolchain, stack: settings.stack, fingerprint, release, built })
}

/// Why the generated project in `out` cannot be built on this machine: the
/// tools of its stack that are missing; none when they are all there.
pub fn cannot_build(tools: &clarity_gen::tools::StackTools, out: &Path) -> Option<String> {
    (!tools.missing.is_empty()).then(|| {
        format!(
            "the project is generated in {}; to build {} here, install {} (the manual's page Setting up the build tools says how)",
            out.display(),
            clarity_gen::tools::stack_name(tools.backend, tools.ui),
            tools.missing.join(", and ")
        )
    })
}

/// Builds a custom part or a library on its own, in its folder, its output
/// in the Output panel: what Build in the part's view does. `steps` run in
/// order, with `env`; the first to fail ends it.
pub fn start_part(app: AppHandle, name: String, folder: PathBuf, steps: Vec<Vec<String>>, env: Vec<(String, String)>) -> Result<(), String> {
    start_steps(app, folder, steps, env, format!("{name} built"), format!("{name} did not build"))
}

/// Installs what the web target of C# with Avalonia is missing (DESIGN 7.6),
/// .NET's WebAssembly tools, its output in the Output panel; the system asks
/// the person's permission first where .NET is installed for every user.
pub fn install_web_tools(app: AppHandle) -> Result<(), String> {
    let tools = clarity_gen::tools::check(clarity_project::Backend::Csharp, clarity_project::Ui::Avalonia, &clarity_gen::tools::System);
    let Some(install) = tools.web_install else {
        return Err(if tools.web_missing.is_empty() { "the WebAssembly tools are installed already".into() } else { "Clarity cannot install the WebAssembly tools here; the manual says how".into() });
    };
    let folder = std::env::temp_dir();
    let step = std::iter::once(install.program).chain(install.args).collect();
    start_steps(app, folder, vec![step], vec![], ".NET's WebAssembly tools are installed".into(), ".NET's WebAssembly tools were not installed".into())
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
    let mut child = Command::new(clarity_gen::tools::Probe::find(&clarity_gen::tools::System, tool).unwrap_or_else(|| tool.into()))
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
    let found = generate_out(&file, release, command_line.is_some()).and_then(|generated| {
        let (out, stack) = (generated.out.clone(), generated.stack.clone());
        // A program served to a browser alone runs as a web server, at the project's port.
        let serves = (command_line.is_none() && stack.has(clarity_project::Target::Web) && !stack.has(clarity_project::Target::Desktop))
            .then(|| clarity_project::load(&file).settings.map(|s| clarity_project::serve_address(&s)))
            .flatten();
        let mut tools = clarity_gen::tools::check(stack.backend, stack.ui, &clarity_gen::tools::System);
        // A program served to a browser may need more (DESIGN 7.6).
        if stack.has(clarity_project::Target::Web) {
            tools.missing.extend(tools.web_missing.clone());
        }
        // Native parts are built with CMake and a C++ compiler, whatever the stack.
        let parts = out.join(clarity_gen::parts::FOLDER).join("CMakeLists.txt").exists();
        let tools_missing = parts && ["cmake", "c++"].iter().any(|tool| clarity_gen::tools::Probe::find(&clarity_gen::tools::System, tool).is_none());
        match cannot_build(&tools, &out) {
            Some(why) => Err(why),
            None if tools_missing => Err(format!("the project is generated in {}; its custom parts need CMake and a C++ compiler", out.display())),
            // The native parts are built first, where the stack's own build does not build them.
            None => Ok((clarity_gen::parts::steps_before(&out, stack.backend), generated, serves)),
        }
    });
    let (before, generated, serves) = match found {
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
            // Run starts the last build when nothing changed since: the
            // generated files, the engines and the parts are as it had them.
            Ok(false) if run && let Some((toolchain, _)) = &generated.built => build(&app, &generated, toolchain, false, run, serves, command_line.as_deref()),
            Ok(_) => match before.iter().find_map(|step| run_step(&app, &generated.out, step).err()) {
                Some(message) => BuildDone { ok: false, message, binary: None },
                None => build(&app, &generated, &generated.toolchain, true, run, serves, command_line.as_deref()),
            },
            Err(message) => BuildDone { ok: false, message, binary: None },
        };
        app.state::<BuildState>().busy.store(false, Ordering::SeqCst);
        send_done(&app, done);
    });
    Ok(())
}

/// Runs one step before the build, such as building the native parts with
/// CMake, its output in the Output panel.
fn run_step(app: &AppHandle, out: &Path, step: &[String]) -> Result<(), String> {
    let Some((tool, args)) = step.split_first() else { return Ok(()) };
    send_line(app, format!("{tool} {} (in {})", args.join(" "), out.display()));
    let mut child = Command::new(clarity_gen::tools::Probe::find(&clarity_gen::tools::System, tool).unwrap_or_else(|| tool.into()))
        .args(args)
        .env("PATH", clarity_gen::tools::tool_path())
        // The C and C++ compilers chosen in the Environment settings, for CMake.
        .envs(clarity_gen::tools::tool_env())
        .current_dir(out)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("cannot start {tool}: {err}; is CMake installed?"))?;
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
        _ => Err("the custom parts did not build; see the output".to_string()),
    }
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
    if building {
        if let Err(done) = build_with(app, out, toolchain) {
            return done;
        }
        let _ = clarity_gen::record_build(out, &generated.fingerprint, generated.release);
    }
    let binary = out.join(&toolchain.program);
    if run && let Some(line) = command_line {
        return run_command_line(app, out, &binary, line);
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
        let mut command = Command::new(&binary);
        command.current_dir(out);
        if let Some(address) = &address {
            command.args(["--address", address]);
        }
        match command.spawn() {
            Ok(child) => started(app, child, address.map(|a| format!("http://{a}"))),
            Err(err) => return failed(format!("cannot start {}: {err}", binary.display())),
        }
    }
    BuildDone {
        ok: true,
        message: if run { "running".to_string() } else { format!("built {}", binary.display()) },
        binary: Some(binary.display().to_string()),
    }
}

/// Runs the stack's build tool in `out`, its output in the Output panel.
fn build_with(app: &AppHandle, out: &Path, toolchain: &clarity_gen::Toolchain) -> Result<(), BuildDone> {
    let failed = |message: String| BuildDone { ok: false, message, binary: None };
    let mut command = Command::new(clarity_gen::tools::Probe::find(&clarity_gen::tools::System, toolchain.tool).unwrap_or_else(|| toolchain.tool.into()));
    command
        .args(&toolchain.args)
        // The tool starts others (go runs gcc and pkg-config): they look where Clarity looks.
        .env("PATH", clarity_gen::tools::tool_path())
        // The C and C++ compilers chosen in the Environment settings, for CMake.
        .envs(clarity_gen::tools::tool_env())
        .envs(toolchain.env.iter().copied())
        .current_dir(out)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // The command first, so the output says what runs where.
    send_line(app, format!("{} {} (in {})", toolchain.tool, toolchain.args.join(" "), out.display()));
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => return Err(failed(format!("cannot start {}: {err}; {}", toolchain.tool, toolchain.missing))),
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
        Ok(_) => return Err(failed("the build failed; see the output".to_string())),
        Err(err) => return Err(failed(format!("{} stopped: {err}", toolchain.tool))),
    }
    Ok(())
}

/// Runs the command-line program with the words of `line`, in its folder:
/// what it writes goes to the Output panel line by line, and what the panel
/// sends with `type_line` is its input, as a terminal's (`CLARITY_TERMINAL`).
/// Returns when it ends, with its exit code.
fn run_command_line(app: &AppHandle, out: &Path, binary: &Path, line: &str) -> BuildDone {
    stop(app);
    let words = clarity_gen::command_words(line);
    send_line(app, format!("{} {}", binary.file_name().unwrap_or_default().to_string_lossy(), line.trim()));
    let mut command = Command::new(binary);
    command.args(&words).current_dir(out).env("CLARITY_TERMINAL", "1").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => return BuildDone { ok: false, message: format!("cannot start {}: {err}", binary.display()), binary: None },
    };
    if let Ok(mut input) = app.state::<BuildState>().input.lock() {
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
    if let Ok(mut running) = app.state::<BuildState>().running.lock() {
        *running = Some(child);
    }
    for reader in readers {
        let _ = reader.join();
    }
    let status = app.state::<BuildState>().running.lock().ok().and_then(|mut r| r.take()).and_then(|mut c| c.wait().ok());
    if let Ok(mut input) = app.state::<BuildState>().input.lock() {
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
    let mut input = state.input.lock().map_err(|_| "the input is poisoned")?;
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
    if let Ok(mut running) = state.running.lock() {
        *running = Some(child);
    }
    if let Ok(mut kept) = state.address.lock() {
        kept.clone_from(&address);
    }
    let _ = app.emit(APP_RUNNING, AppRunning { address });
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(400));
            let state = app.state::<BuildState>();
            let Ok(mut running) = state.running.lock() else { return };
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
    if let Ok(mut address) = app.state::<BuildState>().address.lock() {
        *address = None;
    }
    let _ = app.emit(APP_ENDED, ());
}

/// The application Run started, while it runs.
pub fn running(app: &AppHandle) -> Option<AppRunning> {
    let state = app.state::<BuildState>();
    let mut running = state.running.lock().ok()?;
    let child = running.as_mut()?;
    if !matches!(child.try_wait(), Ok(None)) {
        return None;
    }
    Some(AppRunning { address: state.address.lock().ok()?.clone() })
}

/// Ends the application Run started, if it still runs.
pub fn stop(app: &AppHandle) {
    let child = app.state::<BuildState>().running.lock().ok().and_then(|mut r| r.take());
    if let Some(mut child) = child {
        let _ = child.kill();
        let _ = child.wait();
        ended(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A copy of the example `name`, on the stack its project file names:
    /// an example is generated for its own stack only.
    fn example(name: &str) -> PathBuf {
        fn copy(from: &Path, to: &Path) {
            std::fs::create_dir_all(to).unwrap();
            for entry in std::fs::read_dir(from).unwrap() {
                let path = entry.unwrap().path();
                let file = path.file_name().unwrap();
                if path.is_dir() {
                    // What a run left behind is not the example's.
                    if file != "out" && file != "data" {
                        copy(&path, &to.join(file));
                    }
                } else {
                    std::fs::copy(&path, to.join(file)).unwrap();
                }
            }
        }
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let dir = std::env::temp_dir().join(format!("clarity-run-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let project = format!("{name}.clarity");
        std::fs::copy(examples.join(&project), dir.join(&project)).unwrap();
        copy(&examples.join(name), &dir.join(name));
        dir
    }

    #[test]
    fn generates_the_example_into_out() {
        let dir = example("shop");
        let Generated { out, toolchain, .. } = generate_out(&dir.join("shop.clarity"), true, false).unwrap();
        // The desktop program, in its own folder of the output (DESIGN 5.20).
        assert_eq!(out, dir.join("shop/out/desktop"));
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
            guides: vec!["net-on-linux"],
            web_found: vec![],
            web_missing: vec![],
            web_install: None,
        };
        assert_eq!(
            cannot_build(&check, Path::new("/p/shop/out")),
            Some(
                "the project is generated in /p/shop/out; to build C# with Avalonia here, install the .NET 10 SDK, from https://dotnet.microsoft.com/download (the manual's page Setting up the build tools says how)"
                    .to_string()
            )
        );
        let ready = clarity_gen::tools::StackTools { missing: vec![], guides: vec![], found: vec![".NET SDK 10.0.402".into()], ..check };
        assert_eq!(cannot_build(&ready, Path::new("/p/shop/out")), None);
    }

    /// What the Output panel shows is kept, so the agent can read it.
    #[test]
    fn a_web_application_is_served_where_nothing_else_listens() {
        let held = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = held.local_addr().unwrap().to_string();
        let port = held.local_addr().unwrap().port();
        assert_eq!(can_serve_at(&address), Err(format!("another program listens on port {port}; stop it, or set another port on the Project tab")));
        drop(held);
        assert_eq!(can_serve_at(&address), Ok(()));
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

    #[test]
    fn a_csharp_project_is_generated_for_dotnet() {
        let dir = example("todo");
        let Generated { out, toolchain, .. } = generate_out(&dir.join("todo.clarity"), false, false).unwrap();
        assert!(out.join("Todo.csproj").exists());
        assert!(!out.join("Cargo.toml").exists());
        assert_eq!(toolchain.tool, "dotnet");
        assert!(toolchain.program.starts_with("bin/Debug"), "{}", toolchain.program.display());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
