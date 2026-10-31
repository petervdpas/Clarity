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

/// This machine's system, as the cases name it.
pub const OS: &str = if cfg!(target_os = "windows") {
    "windows"
} else if cfg!(target_os = "macos") {
    "macos"
} else {
    "linux"
};

/// Where `program` keeps its settings on this machine.
pub fn file(program: &str) -> Option<PathBuf> {
    file_in(OS, |name| std::env::var(name).ok(), program).map(PathBuf::from)
}

/// The folder of file store `store` when a folder setting names it, on
/// system `os`: the kept value when it is text that is not empty, else
/// `default`, else a folder named after the store; a relative folder is in
/// `settings_folder`, the folder of the settings file.
pub fn store_folder_in(os: &str, settings_folder: &str, store: &str, kept: &Value, default: &str) -> String {
    let chosen = kept.as_str().filter(|k| !k.is_empty()).unwrap_or(default);
    let chosen = if chosen.is_empty() { store } else { chosen };
    let bytes = chosen.as_bytes();
    let absolute = if os == "windows" {
        chosen.starts_with("\\\\") || chosen.starts_with("//") || (bytes.len() > 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && matches!(bytes[2], b'/' | b'\\'))
    } else {
        chosen.starts_with('/')
    };
    if absolute { chosen.to_string() } else { format!("{settings_folder}/{chosen}") }
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

/// The settings `program` kept, with its secrets (DESIGN 5.27): each name of
/// `secrets` that the system's protected store holds, beside the file's.
#[cfg(feature = "secrets")]
pub fn read_with(program: &str, secrets: &[&str]) -> Map<String, Value> {
    let mut values = read(program);
    for name in secrets {
        values.remove(*name);
        if let Some(secret) = crate::secrets::get(program, name) {
            kept_secrets().insert((program.to_string(), name.to_string()), secret.clone());
            values.insert(name.to_string(), Value::String(secret));
        }
    }
    values
}

/// Keeps `values` as the settings of `program`, those named in `secrets` in
/// the system's protected store and never in the file.
#[cfg(feature = "secrets")]
pub fn keep_with(program: &str, values: &Map<String, Value>, secrets: &[&str]) -> Result<()> {
    let mut plain = values.clone();
    for name in secrets {
        if let Some(value) = plain.remove(*name) {
            let text = value.as_str().unwrap_or_default();
            let key = (program.to_string(), name.to_string());
            // Only a secret that changed goes to the store: another setting
            // changing never needs it, nor fails where it cannot be reached.
            if kept_secrets().get(&key).map_or("", String::as_str) == text {
                continue;
            }
            crate::secrets::set(program, name, text)?;
            kept_secrets().insert(key, text.to_string());
        }
    }
    keep(program, &plain)
}

/// Each secret as the protected store holds it now, as read or kept, by
/// program and name; none is empty.
#[cfg(feature = "secrets")]
fn kept_secrets() -> std::sync::MutexGuard<'static, std::collections::HashMap<(String, String), String>> {
    static KEPT: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<(String, String), String>>> = std::sync::OnceLock::new();
    KEPT.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
