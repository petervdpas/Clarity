//! The engines of a project (DESIGN 5.11): the files of its `engines`
//! folder, checked against the engines its modules declare. The files are
//! read here as WebAssembly only far enough to list what they import and
//! export; every stack's host checks them again when the application starts
//! (conformance/engines.json).

use std::path::Path;

use crate::diagnostic::Diagnostic;
use crate::model::Project;

/// The folder of engines in a project's folder.
pub const ENGINES_DIR: &str = "engines";

/// The services of Clarity an engine may use (`uses clock, random, log,
/// language, progress`): functions of module `clarity` its file may import.
pub const SERVICES: [&str; 5] = ["clock", "random", "log", "language", "progress"];

/// How long one call of an engine may run when its declaration says nothing.
pub const DEFAULT_SECONDS: u32 = 5;
/// How much memory an engine may grow to when its declaration says nothing.
pub const DEFAULT_MEGABYTES: u32 = 256;
/// The longest time limit a declaration may give.
pub const MAX_SECONDS: u32 = 3600;
/// The largest memory limit a declaration may give: all a WebAssembly module can address.
pub const MAX_MEGABYTES: u32 = 4096;

/// What an engine exports besides its functions: the engine interface.
pub const INTERFACE: [&str; 2] = ["clarity_alloc", "clarity_free"];

/// What an engine beside the screen exports besides the engine interface:
/// its frame loop (DESIGN 5.12, conformance/screen-engines.json).
pub const SCREEN_INTERFACE: [&str; 2] = ["clarity_tick", "clarity_draw"];

/// An engine beside the screen as the renderer runs it (DESIGN 5.12): its
/// file, the services it uses and its limits per frame.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct ScreenEngine {
    pub name: String,
    /// The engine's WebAssembly file, in base64.
    pub wasm: String,
    pub uses: Vec<String>,
    /// `on open`: `clarity_open` runs once, before the first frame.
    pub opens: bool,
    /// How long one frame may run.
    pub seconds: u32,
    /// How much memory the engine may grow to.
    pub megabytes: u32,
}

/// The engines beside the screen of `project`, with their files; one whose
/// file is missing is left out (the checker says so).
pub fn screen_engines(project: &Project) -> Vec<ScreenEngine> {
    use base64::Engine as _;
    let files = project.engines.as_deref().unwrap_or_default();
    project
        .files
        .iter()
        .flat_map(|f| &f.engines)
        .filter(|e| e.beside)
        .filter_map(|e| {
            let file = files.iter().find(|f| f.name == e.file)?;
            Some(ScreenEngine {
                name: e.name.name.clone(),
                wasm: base64::engine::general_purpose::STANDARD.encode(&file.bytes),
                uses: e.uses.iter().map(|u| u.name.clone()).collect(),
                opens: e.opens,
                seconds: e.seconds.unwrap_or(DEFAULT_SECONDS),
                megabytes: e.megabytes.unwrap_or(DEFAULT_MEGABYTES),
            })
        })
        .collect()
}

/// A file of the engines folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// The files of the engines folder in `dir`; none when there is no folder.
pub fn engine_files(dir: &Path) -> std::io::Result<Option<Vec<EngineFile>>> {
    let folder = dir.join(ENGINES_DIR);
    if !folder.is_dir() {
        return Ok(None);
    }
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&folder)? {
        let path = entry?.path();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if path.is_file() && name.ends_with(".wasm") {
            found.push(EngineFile { bytes: std::fs::read(&path)?, name });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Some(found))
}

/// What a WebAssembly module imports (`module.name`) and exports, by name.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Module {
    pub imports: Vec<String>,
    /// Exported functions; the memory is in `memory`.
    pub functions: Vec<String>,
    pub memory: bool,
}

/// Reads the import and export sections of a WebAssembly module.
pub fn read_module(bytes: &[u8]) -> Result<Module, String> {
    let not = || "is no WebAssembly module".to_string();
    if bytes.len() < 8 || &bytes[..4] != b"\0asm" {
        return Err(not());
    }
    let mut at = 8;
    let mut module = Module::default();
    let leb = |at: &mut usize| -> Result<u32, String> {
        let (mut value, mut shift) = (0u32, 0);
        loop {
            let byte = *bytes.get(*at).ok_or_else(not)?;
            *at += 1;
            value |= u32::from(byte & 0x7f).checked_shl(shift).ok_or_else(not)?;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
    };
    let name = |at: &mut usize| -> Result<String, String> {
        let len = leb(at)? as usize;
        let text = bytes.get(*at..*at + len).ok_or_else(not)?;
        *at += len;
        String::from_utf8(text.to_vec()).map_err(|_| not())
    };
    while at < bytes.len() {
        let section = bytes[at];
        at += 1;
        let size = leb(&mut at)? as usize;
        let end = at.checked_add(size).filter(|e| *e <= bytes.len()).ok_or_else(not)?;
        let mut inner = at;
        match section {
            2 => {
                for _ in 0..leb(&mut inner)? {
                    let from = name(&mut inner)?;
                    let what = name(&mut inner)?;
                    module.imports.push(format!("{from}.{what}"));
                    // What it imports: a function's type, a table, a memory or a global.
                    let byte = |at: &mut usize| -> Result<u8, String> {
                        let b = *bytes.get(*at).ok_or_else(not)?;
                        *at += 1;
                        Ok(b)
                    };
                    match byte(&mut inner)? {
                        0 => {
                            leb(&mut inner)?;
                        }
                        kind @ (1 | 2) => {
                            if kind == 1 {
                                byte(&mut inner)?;
                            }
                            let flags = byte(&mut inner)?;
                            leb(&mut inner)?;
                            if flags & 1 != 0 {
                                leb(&mut inner)?;
                            }
                        }
                        _ => inner += 2,
                    }
                }
            }
            7 => {
                for _ in 0..leb(&mut inner)? {
                    let export = name(&mut inner)?;
                    let kind = *bytes.get(inner).ok_or_else(not)?;
                    inner += 1;
                    leb(&mut inner)?;
                    match kind {
                        0 => module.functions.push(export),
                        2 if export == "memory" => module.memory = true,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        at = end;
    }
    Ok(module)
}

/// Gives the project the files of its engines folder, and checks every
/// engine its modules declare against its file.
pub fn attach(project: &mut Project, files: Vec<EngineFile>) {
    let mut found = Vec::new();
    for file in &project.files {
        // An engine written in Clarity has no file.
        for engine in file.engines.iter().filter(|e| e.body.is_none()) {
            let at = |message: String| Diagnostic::error(format!("engine {}: '{}' {message}", engine.name.name, engine.file), engine.span).in_file(&file.path);
            let Some(bytes) = files.iter().find(|f| f.name == engine.file) else {
                found.push(at("is not in the engines folder".into()));
                continue;
            };
            match read_module(&bytes.bytes) {
                Err(message) => found.push(at(message)),
                Ok(module) => {
                    let used = |import: &str| import.strip_prefix("clarity.").is_some_and(|service| engine.uses.iter().any(|u| u.name == service));
                    if let Some(import) = module.imports.iter().find(|i| !used(i)) {
                        found.push(at(format!("imports {import}; an engine imports only the services it uses")));
                    }
                    if let Some(missing) = INTERFACE.iter().find(|f| !module.functions.iter().any(|e| e == *f)) {
                        found.push(at(format!("lacks {missing} of the engine interface")));
                    } else if !module.memory {
                        found.push(at("lacks the memory of the engine interface".into()));
                    }
                    for needed in SCREEN_INTERFACE.iter().filter(|_| engine.beside) {
                        if !module.functions.iter().any(|f| f == needed) {
                            found.push(at(format!("lacks {needed}, which an engine beside the screen needs")));
                        }
                    }
                    if engine.opens && !module.functions.iter().any(|f| f == "clarity_open") {
                        found.push(at("lacks clarity_open, which its on open needs".into()));
                    }
                    for function in &engine.functions {
                        if !module.functions.contains(&function.name.name) {
                            found.push(at(format!("has no function {}", function.name.name)));
                        }
                    }
                    for hear in engine.hears.iter().filter(|_| engine.beside) {
                        if !module.functions.contains(&hear.name.name) {
                            found.push(at(format!("has no function {0}, which its hear {0} needs", hear.name.name)));
                        }
                    }
                }
            }
        }
    }
    project.diagnostics.extend(found);
    project.engines = Some(files);
}
