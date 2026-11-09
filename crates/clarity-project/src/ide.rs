//! What the Clarity IDE tells other programs, kept in `ide.json` in
//! Clarity's own folder: the project it has open. `clarity mcp` reads it on
//! every call, so the agent works on the project open in the IDE and
//! follows it when the IDE opens another (docs/MCP-PLAN.md, "Which project").
//!
//! Beside the project it records which IDE has it open (its process id and
//! when that process started), so a project an IDE left behind when it
//! stopped without closing it is not open. A process id used again by
//! another program does not count: its start time differs.
//!
//! The IDE changes only its own keys and keeps every other one; a file it
//! cannot read is left as it is.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// The key holding the open project file's full path.
pub const OPEN_PROJECT: &str = "openProject";

/// The key holding the process id of the IDE that has the project open.
pub const IDE_PID: &str = "idePid";

/// The key holding when that process started, in seconds since 1970.
pub const IDE_STARTED: &str = "ideStarted";

/// One running IDE: its process id and when that process started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ide {
    pub pid: u32,
    /// Seconds since 1970, as the system tells it.
    pub started: u64,
}

impl Ide {
    /// This process; none when the system does not tell its start time.
    pub fn current() -> Option<Ide> {
        let pid = std::process::id();
        started(pid).map(|started| Ide { pid, started })
    }

    /// Whether this IDE still runs: its process id belongs to a process that started when it did.
    pub fn is_running(&self) -> bool {
        started(self.pid) == Some(self.started)
    }
}

/// When the process `pid` started; none when no such process runs.
fn started(pid: u32) -> Option<u64> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing());
    system.process(pid).map(|process| process.start_time())
}

/// Where the IDE keeps what it tells other programs: `ide.json` in Clarity's own folder.
pub fn file() -> PathBuf {
    crate::clarity_home().join("ide.json")
}

/// The project file the IDE has open; none when nothing is open, the
/// settings are missing or unreadable, the file they name is gone, or the
/// IDE that recorded it no longer runs (or did not record itself).
pub fn open_project() -> Option<PathBuf> {
    open_project_in(&file())
}

/// The open project as `file` records it; see [`open_project`].
pub fn open_project_in(file: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(file).ok()?;
    let settings: Value = serde_json::from_str(&text).ok()?;
    let project = PathBuf::from(settings.get(OPEN_PROJECT)?.as_str()?);
    let ide = Ide {
        pid: u32::try_from(settings.get(IDE_PID)?.as_u64()?).ok()?,
        started: settings.get(IDE_STARTED)?.as_u64()?,
    };
    (project.is_file() && ide.is_running()).then_some(project)
}

/// Records `project` as the project this IDE has open.
pub fn set_open_project(project: &Path) -> Result<(), String> {
    let ide = Ide::current().ok_or("the system does not tell when Clarity started; the agent cannot see its project")?;
    set_open_project_in(&file(), project, &ide)
}

/// Records `project` as open in `ide`, in `file`, keeping its other keys.
pub fn set_open_project_in(file: &Path, project: &Path, ide: &Ide) -> Result<(), String> {
    let mut settings = settings_in(file)?;
    settings.insert(OPEN_PROJECT.to_string(), Value::String(project.display().to_string()));
    settings.insert(IDE_PID.to_string(), Value::from(ide.pid));
    settings.insert(IDE_STARTED.to_string(), Value::from(ide.started));
    save(file, &settings)
}

/// Forgets `project` as the open one, when this IDE closes it.
pub fn close_project(project: &Path) -> Result<(), String> {
    close_project_in(&file(), project, std::process::id())
}

/// Forgets `project` as open in the IDE with process id `pid`, in `file`,
/// keeping its other keys. What another IDE recorded since, another project
/// or the same one, stays.
pub fn close_project_in(file: &Path, project: &Path, pid: u32) -> Result<(), String> {
    let mut settings = settings_in(file)?;
    let recorded = settings.get(OPEN_PROJECT).and_then(Value::as_str).map(PathBuf::from);
    let by = settings.get(IDE_PID).and_then(Value::as_u64);
    if recorded.as_deref() != Some(project) || by != Some(u64::from(pid)) {
        return Ok(());
    }
    for key in [OPEN_PROJECT, IDE_PID, IDE_STARTED] {
        settings.remove(key);
    }
    save(file, &settings)
}

/// The keys in `file`; none when it is not there. A file that is there but
/// is not a JSON object is refused, so it is never overwritten.
fn settings_in(file: &Path) -> Result<Map<String, Value>, String> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(err) => return Err(format!("cannot read {}: {err}", file.display())),
    };
    match serde_json::from_str(&text) {
        Ok(Value::Object(settings)) => Ok(settings),
        _ => Err(format!("{} is not a JSON object; it is left as it is", file.display())),
    }
}

/// Writes `settings` to `file` through a file beside it, so a reader never sees half of it.
fn save(file: &Path, settings: &Map<String, Value>) -> Result<(), String> {
    let failed = |err: std::io::Error| format!("cannot write {}: {err}", file.display());
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(failed)?;
    }
    let text = serde_json::to_string_pretty(settings).unwrap_or_default() + "\n";
    let next = file.with_extension("json.next");
    std::fs::write(&next, text).map_err(failed)?;
    std::fs::rename(&next, file).map_err(failed)
}
