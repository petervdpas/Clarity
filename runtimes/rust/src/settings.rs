//! The settings each person keeps for themselves (DESIGN 5.17): one JSON
//! file per application in the folder for application settings of the
//! person's system (conformance/settings.json, `file`). Of what was kept, a
//! start takes back only what still fits its setting (`fits`, `taken`).

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

/// A setting as a kept value is checked against it: its type (`text`,
/// `integer`, `enum`, ...), its lowest and highest number, and the values it
/// allows (an enum's, the languages, the themes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Setting {
    pub kind: &'static str,
    pub min: Option<&'static str>,
    pub max: Option<&'static str>,
    pub values: &'static [&'static str],
}

impl Setting {
    /// A setting of type `kind` with no range and no values.
    pub const fn of(kind: &'static str) -> Setting {
        Setting { kind, min: None, max: None, values: &[] }
    }
}

/// Whether a kept value fits its setting (conformance/settings.json, `fits`).
pub fn fits(setting: &Setting, value: &Value) -> bool {
    use std::str::FromStr;
    let (min, max) = (setting.min.and_then(|m| rust_decimal::Decimal::from_str(m).ok()), setting.max.and_then(|m| rust_decimal::Decimal::from_str(m).ok()));
    let within = |n: rust_decimal::Decimal| min.is_none_or(|m| n >= m) && max.is_none_or(|m| n <= m);
    let text = value.as_str();
    match setting.kind {
        "integer" | "duration" => value.as_i64().is_some_and(|n| within(n.into())),
        "decimal" | "money" => text.filter(|t| plain_number(t)).and_then(|t| rust_decimal::Decimal::from_str(t).ok()).is_some_and(within),
        "bool" => value.is_boolean(),
        "date" => text.is_some_and(|t| shaped(t, "dddd-dd-dd")),
        "datetime" => text.is_some_and(|t| shaped(t, "dddd-dd-ddTdd:dd") || shaped(t, "dddd-dd-ddTdd:dd:dd")),
        "enum" | "language" | "theme" => text.is_some_and(|t| setting.values.contains(&t)),
        "color" => text.is_some_and(|t| t.is_empty() || (t.len() == 7 && t.starts_with('#') && t[1..].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))),
        // A reference is kept as the record's id, a whole number above 0.
        "ref" => value.as_i64().is_some_and(|id| id > 0) && !value.is_f64(),
        _ => text.is_some(),
    }
}

/// `-12` or `3.50`: digits, a dot with digits after it, a minus first.
fn plain_number(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = match digits.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (digits, None),
    };
    let all = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    all(whole) && fraction.is_none_or(all)
}

/// Whether `text` has the shape of `pattern`, where `d` is any digit and
/// every other character is itself.
fn shaped(text: &str, pattern: &str) -> bool {
    text.len() == pattern.len() && text.bytes().zip(pattern.bytes()).all(|(t, p)| if p == b'd' { t.is_ascii_digit() } else { t == p })
}

/// The kept values a start takes back: only of the settings there are, and
/// only those that fit (conformance/settings.json, `kept`).
pub fn taken(settings: &[(&str, Setting)], kept: &Map<String, Value>) -> Map<String, Value> {
    settings
        .iter()
        .filter_map(|(name, setting)| kept.get(*name).filter(|value| fits(setting, value)).map(|value| (name.to_string(), value.clone())))
        .collect()
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

/// The value a file store reads for its folder setting `setting`: what the
/// running handler set (`held`), else what was kept, else nothing
/// (conformance/settings.json, `now`).
pub fn now(held: &Map<String, Value>, kept: &Map<String, Value>, setting: &str) -> String {
    held.get(setting).and_then(Value::as_str).or_else(|| kept.get(setting).and_then(Value::as_str)).unwrap_or_default().to_string()
}

/// The settings `program` kept; none when it kept none, or they cannot be read.
pub fn read(program: &str) -> Map<String, Value> {
    file(program)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Map<String, Value>>(&text).ok())
        .unwrap_or_default()
}

/// What keeping `kept` over the file's values `on_file` writes: each kept
/// value in place of the file's, the keys the program does not keep (the
/// person's agent switches) as they were (conformance/settings.json, `merge`).
pub fn merged(mut on_file: Map<String, Value>, kept: &Map<String, Value>) -> Map<String, Value> {
    for (key, value) in kept {
        on_file.insert(key.clone(), value.clone());
    }
    on_file
}

/// Keeps `values` as the settings of `program`, merged into the file's
/// (`merged`), written whole and then put in place, so a crash leaves the
/// old file or the new one.
pub fn keep(program: &str, values: &Map<String, Value>) -> Result<()> {
    let values = &merged(read(program), values);
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
