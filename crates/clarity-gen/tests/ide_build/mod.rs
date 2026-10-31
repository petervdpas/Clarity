//! Builds a project as the IDE builds it, and says every warning there was:
//! the programs generated as Build and Run generate them, each built with the
//! same `toolchain` command, in debug (Run) and release (Build), on the
//! runtimes in this repository. A warning of the generated code or of
//! Clarity's runtimes counts; only code fetched from elsewhere (CMake's
//! `_deps`) is left out. Used by everything_build.rs and examples_build.rs.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use clarity_project::Backend;

pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

pub fn copy(from: &Path, to: &Path, leave_out: &[&str]) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap();
        if leave_out.iter().any(|l| name == *l) {
            continue;
        }
        let target = to.join(name);
        if path.is_dir() {
            copy(&path, &target, &[]);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

/// The example `name` (examples/<name>.clarity), copied into
/// target/generated without its output and data, its parts' files written
/// as the IDE writes them: its project file, the loaded project and settings.
pub fn example(name: &str) -> (PathBuf, clarity_spec::Project, clarity_project::Settings) {
    let root = repo().join("target/generated").join(format!("example-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    copy(&repo().join("examples").join(name), &root.join(name), &["out", "data"]);
    let file = root.join(format!("{name}.clarity"));
    std::fs::copy(repo().join("examples").join(format!("{name}.clarity")), &file).unwrap();
    let settings = clarity_project::load(&file).settings.expect("the example has a project file");
    let dir = clarity_project::module_dir(&file);
    let project = clarity_project::load_modules(&dir).unwrap();
    clarity_gen::parts::write_part_folders(&dir, &project).unwrap();
    (file, clarity_project::load_modules(&dir).unwrap(), settings)
}

/// The configurations a stack is built in: debug as Run builds and release
/// as Build does. Rust's release build is left out: its warnings are its
/// debug build's, and it takes long.
fn configurations(backend: Backend) -> &'static [bool] {
    match backend {
        Backend::Rust => &[false],
        _ => &[false, true],
    }
}

/// Points the program in `dir` at the runtime in this repository, as a
/// developer of the runtime does.
fn local_runtime(dir: &Path, backend: Backend, command: &mut Command) {
    match backend {
        Backend::Csharp => {
            command.env("CLARITY_RUNTIME", repo().join("runtimes/csharp"));
        }
        Backend::Rust => {
            std::fs::create_dir_all(dir.join(".cargo")).unwrap();
            let runtime = repo().join("runtimes/rust");
            std::fs::write(dir.join(".cargo/config.toml"), format!("[patch.crates-io]\nclarity-runtime = {{ path = {:?} }}\n", runtime.display().to_string())).unwrap();
            // One target folder for every run, so the dependencies compile once.
            command.env("CARGO_TARGET_DIR", repo().join("target/generated/ide-build-target"));
        }
        Backend::Go => {
            let go = include_str!("../../../../runtimes/go/go.mod").lines().find(|l| l.starts_with("go ")).unwrap();
            std::fs::write(dir.join("go.work"), format!("{go}\n\nuse .\n\nreplace github.com/petervdpas/Clarity/runtimes/go => {}\n", repo().join("runtimes/go").display())).unwrap();
        }
        // C and C++ carry their runtime in the project.
        Backend::C | Backend::Cpp => {}
    }
}

/// The warnings among what a build said.
pub fn warnings(said: &str) -> Vec<String> {
    let mut found: Vec<String> = said
        .lines()
        .filter(|l| {
            let lower = l.to_lowercase();
            (lower.contains("warning:") || lower.contains(": warning ") || l.contains("CMake Warning")) && !l.contains("/_deps/") && !l.contains("generated 0 warnings")
        })
        .map(|l| l.trim().to_string())
        .collect();
    found.sort();
    found.dedup();
    found
}

/// Builds the program of the project of `file` for `target` as Run does (a
/// debug build, on the runtimes in this repository), and gives the folder it
/// runs in and the program itself.
pub fn build_for_run(file: &Path, project: &clarity_spec::Project, settings: &clarity_project::Settings, target: clarity_project::Target) -> (PathBuf, PathBuf) {
    let dir = clarity_project::module_dir(file);
    let built = clarity_project::build_settings(&dir, settings);
    let programs = clarity_gen::programs(project, &built).unwrap_or_else(|e| panic!("{}: {e:?}", file.display()));
    let out = clarity_project::out_dir(file, settings);
    clarity_gen::write_programs(&out, &programs, settings.stack.backend, &settings.keeps).unwrap();
    let program = programs.iter().find(|p| p.target == target).unwrap_or_else(|| panic!("{} has no {target:?} program", file.display()));
    let folder = clarity_project::target_dir(&out, program.target);
    for step in clarity_gen::parts::steps_before(&folder, settings.stack.backend) {
        let (tool, args) = step.split_first().unwrap();
        let ran = Command::new(tool).args(args).current_dir(&folder).output().unwrap();
        assert!(ran.status.success(), "{tool} {args:?}:\n{}", String::from_utf8_lossy(&ran.stderr));
    }
    let toolchain = clarity_gen::toolchain(&built, false).expect("Clarity builds this stack");
    let mut command = Command::new(toolchain.tool);
    command.args(&toolchain.args).envs(toolchain.env.iter().copied()).current_dir(&folder);
    local_runtime(&folder, settings.stack.backend, &mut command);
    let ran = command.output().unwrap_or_else(|e| panic!("{}: {e}", toolchain.tool));
    assert!(ran.status.success(), "{} {}:\n{}{}", toolchain.tool, toolchain.args.join(" "), String::from_utf8_lossy(&ran.stdout), String::from_utf8_lossy(&ran.stderr));
    // Rust builds into the shared target folder.
    let program = if settings.stack.backend == Backend::Rust {
        repo().join("target/generated/ide-build-target/debug").join(toolchain.program.file_name().unwrap())
    } else {
        folder.join(&toolchain.program)
    };
    assert!(program.exists(), "no program at {}", program.display());
    (folder, program)
}

/// Builds the project of `file` as the IDE does, every program of its stack
/// in each configuration, and fails with every warning there was. Removes
/// the build when it is clean.
pub fn builds_without_warnings(file: &Path, project: &clarity_spec::Project, settings: &clarity_project::Settings) {
    let dir = clarity_project::module_dir(file);
    let built = clarity_project::build_settings(&dir, settings);
    let programs = clarity_gen::programs(project, &built).unwrap_or_else(|e| panic!("{}: {e:?}", file.display()));
    let out = clarity_project::out_dir(file, settings);
    clarity_gen::write_programs(&out, &programs, settings.stack.backend, &settings.keeps).unwrap();
    let mut all = Vec::new();
    for program in &programs {
        let target = clarity_project::target_dir(&out, program.target);
        // Native parts are built first, where the stack's own build does not build them.
        for step in clarity_gen::parts::steps_before(&target, settings.stack.backend) {
            let (tool, args) = step.split_first().unwrap();
            let ran = Command::new(tool).args(args).current_dir(&target).output().unwrap();
            let said = format!("{}{}", String::from_utf8_lossy(&ran.stdout), String::from_utf8_lossy(&ran.stderr));
            assert!(ran.status.success(), "{tool} {args:?} in {}:\n{said}", target.display());
            all.extend(warnings(&said).into_iter().map(|w| format!("{tool}: {w}")));
        }
        for &release in configurations(settings.stack.backend) {
            let toolchain = if program.target == clarity_project::Target::Cli { clarity_gen::cli_toolchain(&built, release) } else { clarity_gen::toolchain(&built, release) }.expect("Clarity builds this stack");
            let mut command = Command::new(toolchain.tool);
            command.args(&toolchain.args).envs(toolchain.env.iter().copied()).current_dir(&target);
            local_runtime(&target, settings.stack.backend, &mut command);
            let ran = command.output().unwrap_or_else(|e| panic!("{}: {e}", toolchain.tool));
            let said = format!("{}{}", String::from_utf8_lossy(&ran.stdout), String::from_utf8_lossy(&ran.stderr));
            let how = format!("{} {} in {}", toolchain.tool, toolchain.args.join(" "), target.display());
            assert!(ran.status.success(), "{how} failed:\n{said}");
            all.extend(warnings(&said).into_iter().map(|w| format!("{how}: {w}")));
            // Go's own warnings come from vet.
            if settings.stack.backend == Backend::Go && !release {
                let vet = Command::new("go").args(["vet", "./..."]).current_dir(&target).output().unwrap();
                assert!(vet.status.success(), "go vet in {}:\n{}", target.display(), String::from_utf8_lossy(&vet.stderr));
            }
        }
    }
    assert!(all.is_empty(), "{} warnings:\n{}", all.len(), all.join("\n"));
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}
