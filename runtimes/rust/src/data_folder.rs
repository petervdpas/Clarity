//! Where a program's SQLite databases open (conformance/data-folder.json): a
//! database the project keeps beside it is written `data:<program>/<file>`,
//! and opens in the folder CLARITY_DATA names (Clarity's Run names the
//! project's data folder), else in the person's folder for application data
//! under the program's name. CLARITY_DATA moves any SQLite file there under
//! its own file name, so a run in the IDE or by hand (a live check on a copy)
//! chooses its data without editing the generated program. The same in every
//! runtime.

/// The environment variable that names the folder.
pub const VARIABLE: &str = "CLARITY_DATA";

/// The file of an SQLite database at `location`, with `env` reading the
/// environment: in the folder CLARITY_DATA names, under its own file name,
/// or where the project put it when the variable is unset or empty. A
/// location from the environment (`env:`) or a secret setting (`setting:`)
/// is as it says, and so is SQLite's own `:memory:`.
pub fn sqlite(location: &str, env: impl Fn(&str) -> Option<String>) -> String {
    sqlite_in(std::env::consts::OS, location, env)
}

/// `sqlite` on system `os` (`windows`, `macos` or another).
pub fn sqlite_in(os: &str, location: &str, env: impl Fn(&str) -> Option<String>) -> String {
    if location.starts_with("env:") || location.starts_with("setting:") || location == ":memory:" {
        return location.to_string();
    }
    let set = |name: &str| env(name).filter(|v| !v.is_empty());
    let kept = location.strip_prefix("data:");
    let name = kept.map_or(location, |kept| kept.rsplit_once('/').map_or(kept, |(_, file)| file));
    if let Some(folder) = set(VARIABLE)
        && let Some(file) = std::path::Path::new(name).file_name()
    {
        return std::path::Path::new(&folder).join(file).display().to_string();
    }
    let Some(kept) = kept else { return location.to_string() };
    // The person's folder for application data, under the program's name.
    let folder = match os {
        "windows" => set("APPDATA"),
        "macos" => set("HOME").map(|home| format!("{home}/Library/Application Support")),
        _ => set("XDG_DATA_HOME").or_else(|| set("HOME").map(|home| format!("{home}/.local/share"))),
    };
    match folder {
        Some(folder) => format!("{folder}/{kept}"),
        None => name.to_string(),
    }
}

/// `sqlite` on the process's own environment.
pub fn sqlite_here(location: &str) -> String {
    sqlite(location, |name| std::env::var(name).ok())
}
