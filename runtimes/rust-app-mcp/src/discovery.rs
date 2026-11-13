//! `running.json`: where `<app> mcp` finds the app while it runs. The
//! running desktop app writes it beside its own settings.json, so each app
//! has its own and two apps never meet; it names the app, its process, the
//! local socket or pipe it listens on and a token a forwarder must give.
//! Only the person can read it. The app removes it when it ends.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

pub const FILE: &str = "running.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Running {
    pub app: String,
    pub pid: u32,
    pub address: String,
    pub token: String,
}

impl Running {
    /// This process, listening at `address`, with a new random token.
    pub fn new(app: &str, address: &str) -> Result<Running, String> {
        let mut bytes = [0u8; 24];
        getrandom::fill(&mut bytes).map_err(|e| format!("no random token: {e}"))?;
        let token = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Ok(Running { app: app.to_string(), pid: std::process::id(), address: address.to_string(), token })
    }

    pub fn to_json(&self) -> Value {
        json!({ "app": self.app, "pid": self.pid, "address": self.address, "token": self.token })
    }

    pub fn from_json(value: &Value) -> Option<Running> {
        Some(Running {
            app: value.get("app")?.as_str()?.to_string(),
            pid: u32::try_from(value.get("pid")?.as_u64()?).ok()?,
            address: value.get("address")?.as_str()?.to_string(),
            token: value.get("token")?.as_str()?.to_string(),
        })
    }
}

/// The file in the app's settings folder.
pub fn file_in(settings_folder: &Path) -> PathBuf {
    settings_folder.join(FILE)
}

/// Writes `running` to `file`, readable by the person only.
pub fn write(file: &Path, running: &Running) -> Result<(), String> {
    use std::io::Write;
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|e| format!("cannot make {}: {e}", folder.display()))?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut out = options.open(file).map_err(|e| format!("cannot write {}: {e}", file.display()))?;
    out.write_all(running.to_json().to_string().as_bytes()).map_err(|e| format!("cannot write {}: {e}", file.display()))
}

/// What `file` says, when it is there and whole.
pub fn read(file: &Path) -> Option<Running> {
    let text = std::fs::read_to_string(file).ok()?;
    Running::from_json(&serde_json::from_str(&text).ok()?)
}

/// Removes `file` when it is this process's own, so an app ending never
/// removes the file of another that runs.
pub fn remove(file: &Path) {
    if read(file).is_some_and(|r| r.pid == std::process::id()) {
        let _ = std::fs::remove_file(file);
    }
}
