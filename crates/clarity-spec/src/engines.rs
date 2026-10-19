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

/// What an engine exports besides its functions: the engine interface.
pub const INTERFACE: [&str; 2] = ["clarity_alloc", "clarity_free"];

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
                // The first import is enough to refuse the module; the rest
                // of the section is not read.
                if leb(&mut inner)? > 0 {
                    let from = name(&mut inner)?;
                    let what = name(&mut inner)?;
                    module.imports.push(format!("{from}.{what}"));
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
        for engine in &file.engines {
            let at = |message: String| Diagnostic::error(format!("engine {}: '{}' {message}", engine.name.name, engine.file), engine.span).in_file(&file.path);
            let Some(bytes) = files.iter().find(|f| f.name == engine.file) else {
                found.push(at("is not in the engines folder".into()));
                continue;
            };
            match read_module(&bytes.bytes) {
                Err(message) => found.push(at(message)),
                Ok(module) => {
                    if let Some(import) = module.imports.first() {
                        found.push(at(format!("imports {import}; an engine imports nothing")));
                    }
                    if let Some(missing) = INTERFACE.iter().find(|f| !module.functions.iter().any(|e| e == *f)) {
                        found.push(at(format!("lacks {missing} of the engine interface")));
                    } else if !module.memory {
                        found.push(at("lacks the memory of the engine interface".into()));
                    }
                    for function in &engine.functions {
                        if !module.functions.contains(&function.name.name) {
                            found.push(at(format!("has no function {}", function.name.name)));
                        }
                    }
                }
            }
        }
    }
    project.diagnostics.extend(found);
    project.engines = Some(files);
}
