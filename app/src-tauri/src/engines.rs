//! Engines in the designer, as in an IDE: each engine's source is opened,
//! saved and built in Clarity. A new engine gets a source with a stub for
//! each function, a function added later gets its stub, and Build (or Run,
//! for what changed) compiles it into the engines folder. The compiler's
//! messages go to the output, and as problems at their lines to the engine's
//! editor.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;

use clarity_project::engines::{self, EngineBuilt, EngineInfo, EngineSource};
use clarity_spec::code::EngineDecl;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::WatchState;
use crate::build::{BuildDone, BuildState, send_done, send_line};

/// The problems of an engine's last build, sent when it ends.
const ENGINE_BUILT: &str = "engine-built";

/// The engines folder of the open project and the declaration of `name`.
fn engine_of(state: &WatchState, name: &str) -> Result<(PathBuf, EngineDecl, clarity_spec::Project), String> {
    let dir = crate::open_dir_of(state)?;
    let project = clarity_project::load_modules(&dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let engine = project
        .files
        .iter()
        .flat_map(|f| &f.engines)
        .find(|e| e.name.name == name)
        .cloned()
        .ok_or_else(|| format!("there is no engine {name}"))?;
    Ok((dir.join(clarity_spec::engines::ENGINES_DIR), engine, project))
}

fn enums_of(project: &clarity_spec::Project) -> impl Fn(&str) -> bool + '_ {
    move |name| project.files.iter().flat_map(|f| &f.enums).any(|e| e.name.name == name)
}

fn info(folder: &Path, engine: &EngineDecl) -> EngineInfo {
    let wasm = folder.join(&engine.file);
    let source = engines::source_of(folder, &engine.file);
    let written = source.as_ref().and_then(|s| std::fs::read_to_string(&s.main).ok());
    EngineInfo {
        name: engine.name.name.clone(),
        file: engine.file.clone(),
        built: wasm.is_file(),
        needs_build: source.as_ref().is_some_and(|s| engines::needs_build(s, &wasm)),
        missing_functions: written.map(|text| engines::missing_functions(&text, engine)).unwrap_or_default(),
        missing_tools: if source.is_some() { clarity_gen::tools::engine_tools(&clarity_gen::tools::System) } else { Vec::new() },
        source,
    }
}

#[tauri::command]
pub fn engine_info(name: String, state: State<WatchState>) -> Result<EngineInfo, String> {
    let (folder, engine, _) = engine_of(&state, &name)?;
    Ok(info(&folder, &engine))
}

/// The text of the engine's source file.
#[tauri::command]
pub fn engine_source_text(name: String, state: State<WatchState>) -> Result<String, String> {
    let (folder, engine, _) = engine_of(&state, &name)?;
    let source = engines::source_of(&folder, &engine.file).ok_or_else(|| format!("{name} has no source next to its file"))?;
    std::fs::read_to_string(&source.main).map_err(|err| format!("cannot read {}: {err}", source.main.display()))
}

/// Saves the engine's source file.
#[tauri::command]
pub fn save_engine_source(name: String, text: String, state: State<WatchState>) -> Result<EngineInfo, String> {
    let (folder, engine, _) = engine_of(&state, &name)?;
    let source = engines::source_of(&folder, &engine.file).ok_or_else(|| format!("{name} has no source next to its file"))?;
    std::fs::write(&source.main, text).map_err(|err| format!("cannot write {}: {err}", source.main.display()))?;
    Ok(info(&folder, &engine))
}

/// Makes the source of a declared engine that has none: a crate with a stub per function.
#[tauri::command]
pub fn new_engine_source(name: String, state: State<WatchState>) -> Result<EngineInfo, String> {
    let (folder, engine, project) = engine_of(&state, &name)?;
    engines::scaffold(&folder, &engine, &enums_of(&project))?;
    Ok(info(&folder, &engine))
}

/// Adds a stub for each function the declaration names and the source lacks.
#[tauri::command]
pub fn add_engine_stubs(name: String, state: State<WatchState>) -> Result<EngineInfo, String> {
    let (folder, engine, project) = engine_of(&state, &name)?;
    let source = engines::source_of(&folder, &engine.file).ok_or_else(|| format!("{name} has no source next to its file"))?;
    let text = std::fs::read_to_string(&source.main).map_err(|err| format!("cannot read {}: {err}", source.main.display()))?;
    let added = engines::add_stubs(&text, &engine, &enums_of(&project));
    std::fs::write(&source.main, added).map_err(|err| format!("cannot write {}: {err}", source.main.display()))?;
    Ok(info(&folder, &engine))
}

/// Builds the engine into the engines folder. Returns at once; the output
/// arrives as build lines, the end as a build's end and the engine's problems.
#[tauri::command]
pub fn build_engine(name: String, app: AppHandle, state: State<WatchState>) -> Result<(), String> {
    let (folder, engine, _) = engine_of(&state, &name)?;
    let source = engines::source_of(&folder, &engine.file).ok_or_else(|| format!("{name} has no source next to its file"))?;
    let builds = app.state::<BuildState>();
    if builds.busy.swap(true, Ordering::SeqCst) {
        return Err("a build is already running".to_string());
    }
    crate::build::begin_output(&app);
    std::thread::spawn(move || {
        let done = match build(&app, &folder, &engine, &source) {
            Ok(()) => BuildDone { ok: true, message: format!("built {}", engine.file), binary: None },
            Err(message) => BuildDone { ok: false, message, binary: None },
        };
        app.state::<BuildState>().busy.store(false, Ordering::SeqCst);
        send_done(&app, done);
    });
    Ok(())
}

/// The engine library a development build of Clarity builds engines on:
/// its own copy, which may be newer than the published one.
fn library() -> Option<PathBuf> {
    if !cfg!(debug_assertions) {
        return None;
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/rust-engine").canonicalize().ok()
}

/// Builds every engine of the project in `dir` whose file is older than its
/// source, before the application is built on them.
pub fn build_changed(app: &AppHandle, dir: &Path) -> Result<(), String> {
    let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let folder = dir.join(clarity_spec::engines::ENGINES_DIR);
    for engine in project.files.iter().flat_map(|f| &f.engines) {
        if let Some(source) = engines::source_of(&folder, &engine.file)
            && engines::needs_build(&source, &folder.join(&engine.file))
        {
            build(app, &folder, engine, &source)?;
        }
    }
    Ok(())
}

/// Runs cargo for the engine, sends its output, tells its problems, and
/// copies the file it made into the engines folder.
fn build(app: &AppHandle, folder: &Path, engine: &EngineDecl, source: &EngineSource) -> Result<(), String> {
    let name = engine.name.name.clone();
    let missing = clarity_gen::tools::engine_tools(&clarity_gen::tools::System);
    if !missing.is_empty() {
        return Err(format!("to build engine {name}, install {}", missing.join(", and ")));
    }
    let command = engines::build_command(source, library().as_deref());
    send_line(app, format!("{} {} (in {})", command.tool, command.args.join(" "), command.dir.display()));
    let mut child = Command::new(clarity_gen::tools::Probe::find(&clarity_gen::tools::System, command.tool).unwrap_or_else(|| command.tool.into()))
        .args(&command.args)
        .env("PATH", clarity_gen::tools::tool_path())
        // The C and C++ compilers chosen in the Environment settings, for CMake.
        .envs(clarity_gen::tools::tool_env())
        .current_dir(&command.dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("cannot start cargo: {err}"))?;
    let mut problems = Vec::new();
    if let Some(stderr) = child.stderr.take() {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            problems.extend(engines::problem_in(&command.dir, &line));
            send_line(app, line);
        }
    }
    let ok = child.wait().is_ok_and(|status| status.success());
    let _ = app.emit(ENGINE_BUILT, EngineBuilt { name: name.clone(), ok, problems });
    if !ok {
        return Err(format!("engine {name} did not build; see its problems"));
    }
    let wasm = folder.join(&engine.file);
    std::fs::copy(&command.artifact, &wasm).map_err(|err| format!("cannot copy {} to {}: {err}", command.artifact.display(), wasm.display()))?;
    send_line(app, format!("{} is built", engine.file));
    Ok(())
}
