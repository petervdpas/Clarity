//! The settings each person keeps for themselves (DESIGN 5.17): one JSON
//! file per application in the folder for application settings of the
//! person's system (conformance/settings.json, `file`). The screen decides
//! which kept values still fit; this module only reads and keeps the file.

use std::path::PathBuf;

use serde_json::{Map, Value};

use crate::{Error, Result};

/// Where `program` keeps its settings on system `os` (`windows`, `macos`,
/// or another), with `env` giving the environment; None when it names no
/// folder for application settings.
pub fn file_in(os: &str, env: impl Fn(&str) -> Option<String>, program: &str) -> Option<String> {
    let set = |name: &str| env(name).filter(|v| !v.is_empty());
    let folder = match os {
        "windows" => set("APPDATA")?,
        "macos" => format!("{}/Library/Application Support", set("HOME")?),
        _ => set("XDG_CONFIG_HOME").or_else(|| set("HOME").map(|home| format!("{home}/.config")))?,
    };
    Some(format!("{folder}/{program}/settings.json"))
}

/// Where `program` keeps its settings on this machine.
pub fn file(program: &str) -> Option<PathBuf> {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    file_in(os, |name| std::env::var(name).ok(), program).map(PathBuf::from)
}

/// The settings `program` kept; none when it kept none, or they cannot be read.
pub fn read(program: &str) -> Map<String, Value> {
    file(program)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Map<String, Value>>(&text).ok())
        .unwrap_or_default()
}

/// Keeps `values` as the settings of `program`, written whole and then put
/// in place, so a crash leaves the old file or the new one.
pub fn keep(program: &str, values: &Map<String, Value>) -> Result<()> {
    let path = file(program).ok_or_else(|| Error::Invalid("this system names no folder for application settings".into()))?;
    let failed = |err: std::io::Error| Error::Invalid(format!("cannot keep the settings in {}: {err}", path.display()));
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(failed)?;
    }
    let text = serde_json::to_string_pretty(values).map_err(|err| Error::Invalid(err.to_string()))?;
    let written = path.with_extension("json.new");
    std::fs::write(&written, text).map_err(failed)?;
    std::fs::rename(&written, &path).map_err(failed)
}
