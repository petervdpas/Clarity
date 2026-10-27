//! Compiles an engine written in Clarity to WebAssembly (DESIGN 5.12, N5b).
//!
//! The file it gives speaks the engine interface (conformance/engines.json)
//! as an engine built from Rust does, so every host runs it as any engine.
//! It is the support module Clarity carries (`support.wasm`, built from
//! `support/`) with the engine's functions added: per function an inner one
//! that works on wasm's values, and an export of the function's name that
//! reads the arguments, calls it and hands over the answer. The checker
//! gives the type of every expression, so the compiler decides nothing about
//! types itself.

mod emit;
mod interpreted;
mod lower;
mod merge;

use clarity_spec::Project;

/// The support module, with the engine interface and the `rt_*` functions
/// the compiled code calls.
const SUPPORT: &[u8] = include_bytes!("../support.wasm");

/// The support module built with the interpreter, which runs an engine's IR.
const INTERPRETER: &[u8] = include_bytes!("../interpreter.wasm");

/// How an engine written in Clarity runs: its code as wasm, or its IR run
/// by the interpreter. Either way it is a file every host runs alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Compiled,
    Interpreted,
}

/// The engine `name` of the project, written in Clarity, as a WebAssembly
/// file; or why it cannot be compiled yet.
pub fn compile(project: &Project, name: &str) -> Result<Vec<u8>, Vec<String>> {
    build(project, name, Mode::Compiled)
}

/// The engine `name` as a WebAssembly file, compiled or for the interpreter.
pub fn build(project: &Project, name: &str, mode: Mode) -> Result<Vec<u8>, Vec<String>> {
    let Some((file, engine)) = project.files.iter().find_map(|f| f.engines.iter().find(|e| e.name.name == name).map(|e| (f, e))) else {
        return Err(vec![format!("there is no engine {name}")]);
    };
    let Some(body) = &engine.body else {
        return Err(vec![format!("engine {name} is brought along in '{}', not written in Clarity", engine.file)]);
    };
    let types = clarity_spec::code::check::types(&project.files);
    let types = types.get(&file.path).cloned().unwrap_or_default();
    let records: Vec<&clarity_spec::model::Entity> = project.files.iter().flat_map(|f| &f.records).collect();
    let lowered = lower::engine(&types, &records, engine, body)?;
    let base = match mode {
        Mode::Compiled => SUPPORT,
        Mode::Interpreted => INTERPRETER,
    };
    let mut support = merge::Support::read(base).map_err(|e| vec![format!("the support module cannot be read: {e}")])?;
    support.importing(lowered.services.len() as u32);
    let code = match mode {
        Mode::Compiled => emit::engine(&support, &lowered),
        Mode::Interpreted => interpreted::engine(&support, &lowered).map_err(|e| vec![e])?,
    };
    merge::write(base, &support, code).map_err(|e| vec![format!("the engine cannot be written: {e}")])
}
