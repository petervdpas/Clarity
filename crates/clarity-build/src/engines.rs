//! Engines built before the program (manual/code.engines.md): each engine
//! whose source changed since its file was built is compiled for WebAssembly
//! into the engines folder. The compiler's output is said line by line; its
//! messages, read as problems at their lines, go to `told`, which the IDE
//! shows in the engine's editor.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use clarity_project::engines::{self, EngineBuilt, EngineSource};
use clarity_spec::code::EngineDecl;

use crate::Say;

/// What an engine's build tells besides its lines: whether it built, and its problems.
pub type Told<'a> = &'a dyn Fn(EngineBuilt);

/// The engine library a development build of Clarity builds engines on:
/// its own copy, which may be newer than the published one.
fn library() -> Option<PathBuf> {
    if !cfg!(debug_assertions) {
        return None;
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtimes/rust-engine").canonicalize().ok()
}

/// Builds every engine of the project in `dir` whose file is older than its
/// source, before the application is built on them. Whether any was built.
pub fn build_changed(dir: &Path, say: Say, told: Told) -> Result<bool, String> {
    let project = clarity_project::load_modules(dir).map_err(|err| format!("cannot read {}: {err}", dir.display()))?;
    let folder = dir.join(clarity_spec::engines::ENGINES_DIR);
    let mut built = false;
    for engine in project.files.iter().flat_map(|f| &f.engines) {
        if let Some(source) = engines::source_of(&folder, &engine.file)
            && engines::needs_build(&source, &folder.join(&engine.file))
        {
            build(&folder, engine, &source, say, told)?;
            built = true;
        }
    }
    Ok(built)
}

/// Runs cargo for the engine, says its output, tells its problems, and
/// copies the file it made into the engines folder.
pub fn build(folder: &Path, engine: &EngineDecl, source: &EngineSource, say: Say, told: Told) -> Result<(), String> {
    let name = engine.name.name.clone();
    let missing = clarity_gen::tools::engine_tools(&clarity_gen::tools::System);
    if !missing.is_empty() {
        return Err(format!("to build engine {name}, install {}", missing.join(", and ")));
    }
    let command = engines::build_command(source, library().as_deref());
    say(format!("{} {} (in {})", command.tool, command.args.join(" "), command.dir.display()));
    let mut child = clarity_project::background_command(clarity_gen::tools::Probe::find(&clarity_gen::tools::System, command.tool).unwrap_or_else(|| command.tool.into()))
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
            say(line);
        }
    }
    let ok = child.wait().is_ok_and(|status| status.success());
    told(EngineBuilt { name: name.clone(), ok, problems });
    if !ok {
        return Err(format!("engine {name} did not build; see its problems"));
    }
    let wasm = folder.join(&engine.file);
    std::fs::copy(&command.artifact, &wasm).map_err(|err| format!("cannot copy {} to {}: {err}", command.artifact.display(), wasm.display()))?;
    say(format!("{} is built", engine.file));
    Ok(())
}
