//! The engines of a project (manual/code.engines.md): the files of its `engines`
//! folder, checked against the engines its modules declare. The files are
//! read here as WebAssembly only far enough to list what they import and
//! export; every stack's host checks them again when the application starts
//! (conformance/engines.json).

use std::path::Path;

use crate::diagnostic::Diagnostic;
use crate::model::Project;

/// The folder of engines in a project's folder.
pub const ENGINES_DIR: &str = "engines";

/// The engine library (`clarity-engine`, runtimes/rust-engine) this Clarity
/// builds engines on, as an engine's Cargo.toml asks for it: a new engine's
/// source names it, and the checker warns at an engine whose source asks for
/// another.
pub const ENGINE_LIBRARY_VERSION: &str = "0.4";

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
/// its frame loop (manual/code.engines.md, conformance/screen-engines.json).
pub const SCREEN_INTERFACE: [&str; 2] = ["clarity_tick", "clarity_draw"];

/// An engine beside the screen as the renderer runs it (manual/code.engines.md): its
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

/// The sources of the engines folder in `dir`: the name of each folder that
/// holds a Cargo.toml (`engines/board/` for `board.wasm`) and that file.
pub fn engine_sources(dir: &Path) -> std::io::Result<Vec<(String, String)>> {
    let folder = dir.join(ENGINES_DIR);
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(&folder) else { return Ok(found) };
    for entry in entries {
        let path = entry?.path();
        let cargo = path.join("Cargo.toml");
        if path.is_dir() && cargo.is_file() {
            found.push((path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), std::fs::read_to_string(&cargo)?));
        }
    }
    found.sort();
    Ok(found)
}

/// Warns at each declared engine whose source (`sources`, as
/// [`engine_sources`] reads them) builds only on this machine: a path to a
/// folder outside the project, or another engine library than this Clarity's.
pub fn check_sources(project: &mut Project, sources: &[(String, String)]) {
    let mut found = Vec::new();
    for file in &project.files {
        for engine in file.engines.iter().filter(|e| e.body.is_none()) {
            let stem = engine.file.strip_suffix(".wasm").unwrap_or(&engine.file);
            let Some((_, cargo)) = sources.iter().find(|(name, _)| name == stem) else { continue };
            let warn = |message: String| Diagnostic::warning(format!("engine {}: its source {message}", engine.name.name), engine.span).in_file(&file.path);
            for line in cargo.lines().map(str::trim).filter(|l| !l.starts_with('#')) {
                if let Some(path) = quoted_after(line, "path")
                    && outside_the_project(&path)
                {
                    found.push(warn(format!("takes a crate from {path}, a folder outside the project, so it builds only where that folder is")));
                }
                let Some(rest) = line.strip_prefix("clarity-engine").map(str::trim_start).and_then(|r| r.strip_prefix('=')).map(str::trim) else { continue };
                let asked = if rest.starts_with('{') { quoted_after(rest, "version") } else { quoted(rest) };
                if let Some(asked) = asked
                    && major_minor(&asked) != major_minor(ENGINE_LIBRARY_VERSION)
                {
                    found.push(warn(format!("asks for clarity-engine {asked}; this Clarity builds engines on clarity-engine {ENGINE_LIBRARY_VERSION}")));
                }
            }
        }
    }
    project.diagnostics.extend(found);
}

/// The quoted text that starts `text` (`"0.4"` or `'0.4'`), without its quotes.
fn quoted(text: &str) -> Option<String> {
    let quote = text.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let rest = &text[1..];
    Some(rest[..rest.find(quote)?].to_string())
}

/// The quoted value of `key = "..."` in a TOML line or inline table.
fn quoted_after(line: &str, key: &str) -> Option<String> {
    let mut from = 0;
    while let Some(at) = line[from..].find(key).map(|i| i + from) {
        from = at + key.len();
        let before = line[..at].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            continue;
        }
        if let Some(value) = line[from..].trim_start().strip_prefix('=') {
            return quoted(value.trim_start());
        }
    }
    None
}

/// Whether a path in an engine's Cargo.toml (its folder is
/// `engines/<name>/` of the project) leaves the project: an absolute path,
/// or more steps up than the two folders it stands in.
fn outside_the_project(path: &str) -> bool {
    let absolute = path.starts_with('/') || path.starts_with('\\') || path.as_bytes().get(1) == Some(&b':');
    if absolute {
        return true;
    }
    let mut depth: i32 = 2;
    for step in path.split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".") {
        depth += if step == ".." { -1 } else { 1 };
        if depth < 0 {
            return true;
        }
    }
    false
}

/// What Cargo keeps a requirement compatible by: major and minor below 1.0,
/// the major from 1.0 (`^0.4.0` and `0.4` give `0.4`).
fn major_minor(requirement: &str) -> String {
    let version = requirement.trim_start_matches(['^', '=', '~', ' ']);
    let parts: Vec<&str> = version.split('.').collect();
    if parts.first() == Some(&"0") { parts.iter().take(2).copied().collect::<Vec<_>>().join(".") } else { parts.first().copied().unwrap_or_default().to_string() }
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
