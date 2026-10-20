//! An engine's source (DESIGN 5.11): a Rust crate next to its file in the
//! engines folder, `engines/board/` for `board.wasm`. Clarity makes one from
//! a template with a stub for each declared function, tells when the file is
//! older than its source, builds it with cargo for WebAssembly and reads the
//! compiler's messages as problems at their lines. An engine written in
//! another language is a file only; Clarity checks it and leaves it be.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use clarity_spec::code::{EngineDecl, EngineFunction, EngineState, TypeRef, TypeRefKind};
use serde::Serialize;
use ts_rs::TS;

/// The version of the `clarity-engine` library a new engine depends on.
pub const ENGINE_LIBRARY_VERSION: &str = "0.1";

/// The target an engine is built for: WebAssembly without an operating system.
pub const TARGET: &str = "wasm32-unknown-unknown";

/// The source of an engine, as Clarity found or made it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct EngineSource {
    /// The crate's folder: `engines/board`.
    #[ts(type = "string")]
    pub folder: PathBuf,
    /// The file the functions are in: `engines/board/src/lib.rs`.
    #[ts(type = "string")]
    pub main: PathBuf,
    /// The crate's name, which its built file is named by.
    pub crate_name: String,
}

/// The source of the engine whose file is `file` in `engines`; none when
/// there is no crate next to it.
pub fn source_of(engines: &Path, file: &str) -> Option<EngineSource> {
    let stem = file.strip_suffix(".wasm")?;
    let folder = engines.join(stem);
    let cargo = std::fs::read_to_string(folder.join("Cargo.toml")).ok()?;
    let crate_name = package_name(&cargo).unwrap_or_else(|| stem.to_string());
    Some(EngineSource { main: folder.join("src").join("lib.rs"), folder, crate_name })
}

/// The `name` of a Cargo.toml's `[package]`.
fn package_name(cargo: &str) -> Option<String> {
    let mut in_package = false;
    for line in cargo.lines().map(str::trim) {
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package && let Some(value) = line.strip_prefix("name").map(str::trim_start).and_then(|l| l.strip_prefix('=')) {
            return Some(value.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// Whether the engine's file must be built: there is none, or its source
/// changed after it was.
pub fn needs_build(source: &EngineSource, wasm: &Path) -> bool {
    let Ok(built) = std::fs::metadata(wasm).and_then(|m| m.modified()) else { return true };
    let mut newest = modified(&source.folder.join("Cargo.toml"));
    newest_in(&source.folder.join("src"), &mut newest);
    newest.is_some_and(|changed| changed > built)
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn newest_in(dir: &Path, newest: &mut Option<SystemTime>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for path in entries.filter_map(Result::ok).map(|e| e.path()) {
        if path.is_dir() {
            newest_in(&path, newest);
        } else if let Some(at) = modified(&path)
            && newest.is_none_or(|n| at > n)
        {
            *newest = Some(at);
        }
    }
}

/// Makes the source of `engine` in `engines`: a crate with the engine
/// interface, its on open and a stub per function, which builds as it is.
/// Refuses when the folder is there already.
pub fn scaffold(engines: &Path, engine: &EngineDecl, is_enum: &dyn Fn(&str) -> bool) -> Result<EngineSource, String> {
    let stem = engine.file.strip_suffix(".wasm").ok_or_else(|| format!("'{}' is no .wasm file", engine.file))?;
    let folder = engines.join(stem);
    if folder.exists() {
        return Err(format!("{} is there already", folder.display()));
    }
    let write = |path: PathBuf, text: String| -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("cannot make {}: {e}", parent.display()))?;
        }
        std::fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
    };
    write(folder.join("Cargo.toml"), cargo_toml(engine, stem))?;
    write(folder.join(".gitignore"), "/target\n".to_string())?;
    write(folder.join("src").join("lib.rs"), lib_rs(engine, is_enum))?;
    Ok(EngineSource { main: folder.join("src").join("lib.rs"), folder, crate_name: stem.to_string() })
}

fn cargo_toml(engine: &EngineDecl, stem: &str) -> String {
    format!(
        "# The {name} engine, built by Clarity into ../{file}, or with\n\
         #   cargo build --release --target {TARGET}\n\
         [package]\n\
         name = \"{stem}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\
         \n\
         [lib]\n\
         crate-type = [\"cdylib\"]\n\
         \n\
         [dependencies]\n\
         clarity-engine = \"{ENGINE_LIBRARY_VERSION}\"\n\
         \n\
         [profile.release]\n\
         opt-level = \"s\"\n\
         lto = true\n\
         strip = true\n\
         \n\
         # Not part of another workspace: it builds for WebAssembly on its own.\n\
         [workspace]\n",
        name = engine.name.name,
        file = engine.file,
    )
}

fn lib_rs(engine: &EngineDecl, is_enum: &dyn Fn(&str) -> bool) -> String {
    let kept = match engine.state {
        EngineState::None => "Every call runs on a fresh instance, so nothing is kept between calls.",
        EngineState::App => "One instance serves the whole application: what it keeps in statics stays from call to call.",
        EngineState::Window => "Each open window has an instance of its own: what it keeps in statics belongs to that window.",
    };
    let mut out = format!(
        "//! The {name} engine. Clarity wrote a stub for each function its declaration\n\
         //! names; write what each one computes. {kept}\n\
         \n\
         clarity_engine::interface!();\n",
        name = engine.name.name
    );
    if engine.opens {
        out.push_str("\n// What a new instance starts with.\nclarity_engine::on_open!(|| {\n    Ok(())\n});\n");
    }
    for function in &engine.functions {
        out.push('\n');
        out.push_str(&stub(function, is_enum));
    }
    out
}

/// A function of the engine as Clarity writes it to start: it takes the
/// declared values and answers an empty value of its result.
pub fn stub(function: &EngineFunction, is_enum: &dyn Fn(&str) -> bool) -> String {
    let names: Vec<&str> = function.params.iter().map(|p| p.name.name.as_str()).collect();
    let types: Vec<String> = function.params.iter().map(|p| rust_type(&p.ty, is_enum)).collect();
    let (names, types) = match names.len() {
        1 => (format!("({},)", names[0]), format!("({},)", types[0])),
        _ => (format!("({})", names.join(", ")), format!("({})", types.join(", "))),
    };
    let empty = function.returns.as_ref().map_or("()".to_string(), |t| empty_value(t, is_enum));
    // Until the body is written its values are unused, which is no mistake.
    let unused = match function.params.len() {
        0 => String::new(),
        1 => format!("    let _ = {};\n", function.params[0].name.name),
        _ => format!("    let _ = {names};\n"),
    };
    format!(
        "// {}\nclarity_engine::function!({}, |{names}: {types}| {{\n{unused}    Ok({empty})\n}});\n",
        signature(function),
        function.name.name
    )
}

/// `play(square: integer) -> text`, as the declaration writes it.
fn signature(function: &EngineFunction) -> String {
    let params: Vec<String> = function.params.iter().map(|p| format!("{}: {}", p.name.name, type_text(&p.ty))).collect();
    let returns = function.returns.as_ref().map(|t| format!(" -> {}", type_text(t))).unwrap_or_default();
    format!("{}({}){returns}", function.name.name, params.join(", "))
}

fn type_text(ty: &TypeRef) -> String {
    match &ty.kind {
        TypeRefKind::Text => "text".into(),
        TypeRefKind::Integer => "integer".into(),
        TypeRefKind::Decimal => "decimal".into(),
        TypeRefKind::Money => "money".into(),
        TypeRefKind::Bool => "bool".into(),
        TypeRefKind::Date => "date".into(),
        TypeRefKind::DateTime => "datetime".into(),
        TypeRefKind::Duration => "duration".into(),
        TypeRefKind::File => "file".into(),
        TypeRefKind::Bytes => "bytes".into(),
        TypeRefKind::Named { name } => name.clone(),
        TypeRefKind::List { item } => format!("list<{item}>"),
    }
}

const VALUE: &str = "clarity_engine::serde_json::Value";

/// The Rust type a value of `ty` arrives as, from its JSON (docs/manual/code.engines.md).
fn rust_type(ty: &TypeRef, is_enum: &dyn Fn(&str) -> bool) -> String {
    match &ty.kind {
        TypeRefKind::Text | TypeRefKind::Decimal | TypeRefKind::Money => "String".into(),
        TypeRefKind::Integer | TypeRefKind::Duration => "i64".into(),
        TypeRefKind::Bool => "bool".into(),
        TypeRefKind::Date | TypeRefKind::DateTime => "Option<String>".into(),
        TypeRefKind::Bytes => "clarity_engine::Bytes".into(),
        TypeRefKind::Named { name } if is_enum(name) => "String".into(),
        TypeRefKind::Named { .. } | TypeRefKind::File => VALUE.into(),
        TypeRefKind::List { .. } => format!("Vec<{VALUE}>"),
    }
}

/// An empty value of `ty`, for a stub to answer.
fn empty_value(ty: &TypeRef, is_enum: &dyn Fn(&str) -> bool) -> String {
    match &ty.kind {
        TypeRefKind::Text => "String::new()".into(),
        TypeRefKind::Decimal | TypeRefKind::Money => "String::from(\"0\")".into(),
        TypeRefKind::Integer | TypeRefKind::Duration => "0i64".into(),
        TypeRefKind::Bool => "false".into(),
        TypeRefKind::Date | TypeRefKind::DateTime => "None::<String>".into(),
        TypeRefKind::Bytes => "clarity_engine::Bytes(Vec::new())".into(),
        TypeRefKind::Named { name } if is_enum(name) => "String::new()".into(),
        TypeRefKind::Named { .. } | TypeRefKind::File => format!("{VALUE}::Null"),
        TypeRefKind::List { .. } => format!("Vec::<{VALUE}>::new()"),
    }
}

/// The functions `engine` declares that `source` does not write.
pub fn missing_functions(source: &str, engine: &EngineDecl) -> Vec<String> {
    let written = written_functions(source);
    engine.functions.iter().map(|f| f.name.name.clone()).filter(|name| !written.iter().any(|(w, _)| w == name)).collect()
}

/// `source` with a stub after it for each function `engine` declares and it lacks.
pub fn add_stubs(source: &str, engine: &EngineDecl, is_enum: &dyn Fn(&str) -> bool) -> String {
    let missing = missing_functions(source, engine);
    let mut out = source.to_string();
    for function in engine.functions.iter().filter(|f| missing.contains(&f.name.name)) {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(&stub(function, is_enum));
    }
    out
}

/// The line (from 1) where `source` writes the function `name`.
pub fn function_line(source: &str, name: &str) -> Option<u32> {
    written_functions(source).into_iter().find(|(n, _)| n == name).map(|(_, line)| line)
}

/// Each `function!(name, ...)` in `source`, with its line.
fn written_functions(source: &str) -> Vec<(String, u32)> {
    let mut found = Vec::new();
    for (at, line) in source.lines().enumerate() {
        let mut rest = line;
        while let Some(start) = rest.find("function!(") {
            rest = &rest[start + "function!(".len()..];
            let name: String = rest.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                found.push((name, at as u32 + 1));
            }
        }
    }
    found
}

/// How to build an engine: the command, where it runs, and the file it makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCommand {
    pub tool: &'static str,
    pub args: Vec<String>,
    pub dir: PathBuf,
    pub artifact: PathBuf,
}

/// The cargo build of `source` for WebAssembly, with short messages to read
/// as problems. `library`, a development build of Clarity, points the
/// engine library at that copy instead of the published one.
pub fn build_command(source: &EngineSource, library: Option<&Path>) -> BuildCommand {
    let mut args: Vec<String> = ["build", "--release", "--target", TARGET, "--message-format", "short"].iter().map(|s| s.to_string()).collect();
    if let Some(library) = library {
        args.push("--config".into());
        args.push(format!("patch.crates-io.clarity-engine.path=\"{}\"", library.display()));
    }
    let artifact = source.folder.join("target").join(TARGET).join("release").join(format!("{}.wasm", source.crate_name.replace('-', "_")));
    BuildCommand { tool: "cargo", args, dir: source.folder.clone(), artifact }
}

/// How bad a compiler message is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export, rename = "EngineProblemSeverity")]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
}

/// A compiler message at a place in an engine's source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, rename = "EngineProblem")]
pub struct Problem {
    #[ts(type = "string")]
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub severity: Severity,
    pub message: String,
}

/// A problem from one line of cargo's short messages
/// (`src/lib.rs:12:5: error[E0308]: ...`); none for other lines.
pub fn problem_in(folder: &Path, line: &str) -> Option<Problem> {
    let (place, rest) = line.split_once(": ")?;
    let mut parts = place.rsplitn(3, ':');
    let column = parts.next()?.parse().ok()?;
    let at = parts.next()?.parse().ok()?;
    let file = parts.next()?;
    let (severity, rest) = if let Some(rest) = rest.strip_prefix("error") {
        (Severity::Error, rest)
    } else if let Some(rest) = rest.strip_prefix("warning") {
        (Severity::Warning, rest)
    } else {
        return None;
    };
    let message = rest.split_once(": ").map(|(_, m)| m)?;
    Some(Problem { file: folder.join(file), line: at, column, severity, message: message.to_string() })
}

/// An engine as its editor shows it: its file and its source.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub name: String,
    /// The file in the engines folder: `board.wasm`.
    pub file: String,
    /// Whether that file is there.
    pub built: bool,
    /// Its Rust source, when it has one next to it.
    pub source: Option<EngineSource>,
    /// Whether the file is older than its source, or missing.
    pub needs_build: bool,
    /// Functions the declaration names and the source does not write.
    pub missing_functions: Vec<String>,
    /// What to install to build it here; empty when nothing is missing.
    pub missing_tools: Vec<String>,
}

/// The end of an engine's build: whether it worked, and the compiler's
/// messages at their lines.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct EngineBuilt {
    pub name: String,
    pub ok: bool,
    pub problems: Vec<Problem>,
}
