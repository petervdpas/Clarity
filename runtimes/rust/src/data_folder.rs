//! The folder a program's SQLite databases open in when the environment
//! variable CLARITY_DATA names one: each under its own file name, so a run in
//! the IDE or by hand (a live check on a copy) chooses its data without
//! editing the generated program. The same as the C# runtime's `DataFolder`.

/// The environment variable that names the folder.
pub const VARIABLE: &str = "CLARITY_DATA";

/// The file of an SQLite database at `location`, with `env` reading the
/// environment: in the folder CLARITY_DATA names, under its own file name,
/// or where the project put it when the variable is unset or empty. A
/// location from the environment (`env:`) or a secret setting (`setting:`)
/// is as it says, and so is SQLite's own `:memory:`.
pub fn sqlite(location: &str, env: impl Fn(&str) -> Option<String>) -> String {
    if location.starts_with("env:") || location.starts_with("setting:") || location == ":memory:" {
        return location.to_string();
    }
    match env(VARIABLE).filter(|folder| !folder.is_empty()) {
        Some(folder) => match std::path::Path::new(location).file_name() {
            Some(name) => std::path::Path::new(&folder).join(name).display().to_string(),
            None => location.to_string(),
        },
        None => location.to_string(),
    }
}

/// `sqlite` on the process's own environment.
pub fn sqlite_here(location: &str) -> String {
    sqlite(location, |name| std::env::var(name).ok())
}
