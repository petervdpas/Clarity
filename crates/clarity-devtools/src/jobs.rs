//! The builds DevTools runs when Build (or a release check's Run) is
//! pressed: one at a time, as the machine can bear, the others waiting in
//! the order pressed. Each keeps its last lines and how it ended, so its tab
//! shows it while it builds and after. Each change is also written to the
//! build's own file (`record_in`, JobRecord), which an agent reads while it
//! builds (the MCP tool build_state).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The most lines a build keeps: its tab shows the last ones.
pub const LINES_KEPT: usize = 400;

/// A command to run, in a folder, with what it adds to the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runnable {
    pub program: String,
    pub args: Vec<String>,
    pub dir: PathBuf,
    pub env: Vec<(String, String)>,
}

impl Runnable {
    pub fn new(program: &str, args: &[&str], dir: &Path) -> Runnable {
        Runnable { program: program.to_string(), args: args.iter().map(|a| a.to_string()).collect(), dir: dir.to_path_buf(), env: Vec::new() }
    }

    /// The same with `name=value` in its environment.
    pub fn with_env(mut self, name: &str, value: &str) -> Runnable {
        self.env.push((name.to_string(), value.to_string()));
        self
    }

    /// The command as people read it: `cargo build --release -p clarity-ui-ffi`.
    pub fn shown(&self) -> String {
        std::iter::once(self.program.as_str()).chain(self.args.iter().map(String::as_str)).collect::<Vec<_>>().join(" ")
    }
}

/// How a build stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "why", rename_all = "camelCase")]
pub enum JobState {
    Waiting,
    Running,
    Passed,
    Failed(String),
}

/// A build: what runs, its lines so far and how it stands.
#[derive(Debug, Clone, Serialize)]
pub struct Job {
    pub key: String,
    pub title: String,
    /// The command, as people read it.
    pub command: String,
    pub state: JobState,
    pub lines: Vec<String>,
    /// Who pressed it, when an agent asked: `Claude Code`.
    pub by: Option<String>,
    /// When it was pressed, started and ended, in milliseconds since 1970.
    pub queued: Option<u64>,
    pub started: Option<u64>,
    pub ended: Option<u64>,
}

/// What a stream says: a line that ended, or the line being drawn now,
/// which the next drawing or the line's end replaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Line(String),
    Drawing(String),
}

/// Reads `from` as a terminal shows it: `\n` ends a line, `\r` (not before
/// `\n`) goes back to its start, so what follows draws over it, as cargo's
/// progress bar does. After each read, the line being drawn is told as it is.
pub fn read_pieces(mut from: impl std::io::Read, out: &mut dyn FnMut(Piece)) {
    let mut line: Vec<u8> = Vec::new();
    let mut back = false;
    let mut buffer = [0u8; 4096];
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim_end_matches('\r').to_string();
    loop {
        let n = match from.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for &byte in &buffer[..n] {
            match byte {
                b'\n' => {
                    out(Piece::Line(text(&line)));
                    line.clear();
                    back = false;
                }
                b'\r' => back = true,
                _ => {
                    if back {
                        line.clear();
                        back = false;
                    }
                    line.push(byte);
                }
            }
        }
        if !line.is_empty() && !back {
            out(Piece::Drawing(text(&line)));
        }
    }
    if !line.is_empty() {
        out(Piece::Line(text(&line)));
    }
}

/// Called with a build each time it changes: a line, its start, its end.
pub type OnChange = Arc<dyn Fn(&Job) + Send + Sync>;

#[derive(Default)]
struct Inner {
    jobs: BTreeMap<String, Job>,
    waiting: VecDeque<(String, Runnable, Option<OnChange>)>,
    running: bool,
    /// The build running now and its program's process.
    current: Option<(String, u32)>,
    /// The builds Stop was pressed for while they ran.
    stopped: BTreeSet<String>,
    /// The folder each build's state is written to as it changes.
    records: Option<PathBuf>,
}

/// What a build stopped by hand ended with.
const STOPPED: &str = "stopped";

/// Ends the process `pid` with everything it started (cargo's compilers and test programs).
fn end_process(pid: u32) {
    let pid = pid.to_string();
    let _ = if cfg!(windows) {
        clarity_project::background_command("taskkill").args(["/T", "/F", "/PID", &pid]).stdout(Stdio::null()).stderr(Stdio::null()).status()
    } else {
        // The build runs in a process group of its own, which a minus names.
        clarity_project::background_command("kill").args(["-TERM", "--", &format!("-{pid}")]).stdout(Stdio::null()).stderr(Stdio::null()).status()
    };
}

/// The builds, shared by whoever presses Build; cloning shares them.
#[derive(Clone, Default)]
pub struct Jobs {
    inner: Arc<(Mutex<Inner>, Condvar)>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl Jobs {
    /// Writes each build's state to its own file in `dir` from now on, as
    /// it changes. A record another Clarity left waiting or running (it
    /// ended while it built) is written as stopped: no build of it goes on.
    pub fn record_in(&self, dir: &Path) {
        for mut left in read_records(dir) {
            if matches!(left.state, RecordState::Waiting | RecordState::Running) && self.job(&left.key).is_none() {
                left.state = RecordState::Stopped;
                left.why = Some(ENDED_WITH_CLARITY.into());
                left.ended = Some(left.written);
                let _ = write_record(dir, &left);
            }
        }
        if let Ok(mut inner) = self.inner.0.lock() {
            inner.records = Some(dir.to_path_buf());
        }
    }

    /// Presses Build: `run` builds now, or after the builds before it.
    /// Refused while the same key waits or builds.
    pub fn start(&self, key: &str, title: &str, run: Runnable, on_change: Option<OnChange>) -> Result<(), String> {
        self.start_by(key, title, run, None, on_change)
    }

    /// The same, pressed for `by` (an agent).
    pub fn start_by(&self, key: &str, title: &str, run: Runnable, by: Option<String>, on_change: Option<OnChange>) -> Result<(), String> {
        let (lock, _) = &*self.inner;
        let mut inner = lock.lock().map_err(|_| "the builds' record is broken".to_string())?;
        if let Some(job) = inner.jobs.get(key)
            && matches!(job.state, JobState::Waiting | JobState::Running)
        {
            return Err(format!("{} is building already", job.title));
        }
        let job = Job { key: key.to_string(), title: title.to_string(), command: run.shown(), state: JobState::Waiting, lines: Vec::new(), by, queued: Some(now_ms()), started: None, ended: None };
        record(&inner.records, &job);
        if let Some(f) = &on_change {
            f(&job);
        }
        inner.jobs.insert(key.to_string(), job);
        inner.waiting.push_back((key.to_string(), run, on_change));
        if !inner.running {
            inner.running = true;
            let jobs = self.clone();
            std::thread::spawn(move || jobs.work());
        }
        Ok(())
    }

    /// Presses Stop for the builds `keys`: the one running ends now, the
    /// waiting ones never start; each ends failed, "stopped". How many it stopped.
    pub fn stop(&self, keys: &[String]) -> usize {
        let (lock, ended) = &*self.inner;
        let (dropped, running) = {
            let Ok(mut inner) = lock.lock() else { return 0 };
            let (dropped, kept): (Vec<_>, Vec<_>) = inner.waiting.drain(..).partition(|(key, _, _)| keys.contains(key));
            inner.waiting = kept.into();
            // Running, its process started or about to: once started, run ends it.
            let running: Vec<String> = inner.jobs.values().filter(|j| j.state == JobState::Running && keys.contains(&j.key)).map(|j| j.key.clone()).collect();
            inner.stopped.extend(running.iter().cloned());
            let pid = inner.current.as_ref().filter(|(key, _)| running.contains(key)).map(|(_, pid)| *pid);
            (dropped, (running.len(), pid))
        };
        let (running, pid) = running;
        if let Some(pid) = pid {
            end_process(pid);
        }
        for (key, _, on_change) in &dropped {
            self.change(key, on_change, |job| {
                job.state = JobState::Failed(STOPPED.into());
                job.ended = Some(now_ms());
            });
        }
        ended.notify_all();
        dropped.len() + running
    }

    /// The build kept under `key`.
    pub fn job(&self, key: &str) -> Option<Job> {
        self.inner.0.lock().ok()?.jobs.get(key).cloned()
    }

    /// Every build kept.
    pub fn all(&self) -> Vec<Job> {
        self.inner.0.lock().map(|i| i.jobs.values().cloned().collect()).unwrap_or_default()
    }

    /// Waits until the build under `key` has ended, `most` at most; the build as it is then.
    pub fn wait(&self, key: &str, most: Duration) -> Option<Job> {
        let (lock, ended) = &*self.inner;
        let until = Instant::now() + most;
        let mut inner = lock.lock().ok()?;
        loop {
            let job = inner.jobs.get(key)?.clone();
            let left = until.saturating_duration_since(Instant::now());
            if matches!(job.state, JobState::Passed | JobState::Failed(_)) || left.is_zero() {
                return Some(job);
            }
            inner = ended.wait_timeout(inner, left).ok()?.0;
        }
    }

    fn change(&self, key: &str, on_change: &Option<OnChange>, f: impl FnOnce(&mut Job)) {
        let (lock, ended) = &*self.inner;
        let job = {
            let Ok(mut inner) = lock.lock() else { return };
            let Some(job) = inner.jobs.get_mut(key) else { return };
            f(job);
            let job = job.clone();
            // Written while the builds are held, so a later change is never overwritten by an earlier one.
            record(&inner.records, &job);
            job
        };
        ended.notify_all();
        if let Some(on) = on_change {
            on(&job);
        }
    }

    /// Runs the waiting builds one after the other, until none waits.
    fn work(&self) {
        loop {
            let next = {
                let Ok(mut inner) = self.inner.0.lock() else { return };
                match inner.waiting.pop_front() {
                    Some(next) => next,
                    None => {
                        inner.running = false;
                        return;
                    }
                }
            };
            let (key, run, on_change) = next;
            self.change(&key, &on_change, |job| {
                job.state = JobState::Running;
                job.started = Some(now_ms());
            });
            let mut ended = self.run(&key, &run, &on_change);
            if let Ok(mut inner) = self.inner.0.lock() {
                inner.current = None;
                if inner.stopped.remove(&key) {
                    ended = JobState::Failed(STOPPED.into());
                }
            }
            self.change(&key, &on_change, |job| {
                job.state = ended;
                job.ended = Some(now_ms());
            });
        }
    }

    fn run(&self, key: &str, run: &Runnable, on_change: &Option<OnChange>) -> JobState {
        // Which stream's line is being drawn over now (the last line, not ended yet).
        let mut drawing: Option<usize> = None;
        let mut put = |stream: usize, piece: Piece| {
            self.change(key, on_change, |job| {
                let (text, ended) = match piece {
                    Piece::Line(text) => (text, true),
                    Piece::Drawing(text) => (text, false),
                };
                if drawing == Some(stream) {
                    job.lines.pop();
                }
                job.lines.push(text);
                drawing = (!ended).then_some(stream);
                if job.lines.len() > LINES_KEPT {
                    let over = job.lines.len() - LINES_KEPT;
                    job.lines.drain(..over);
                }
            })
        };
        // Plain: shown as people know the folder, on Windows without canonicalize's `\\?\`.
        let dir = run.dir.canonicalize().map(clarity_project::plain).unwrap_or_else(|_| run.dir.clone());
        put(usize::MAX, Piece::Line(format!("{} (in {})", run.shown(), dir.display())));
        let mut command = clarity_project::background_command(&run.program);
        command.args(&run.args).current_dir(&run.dir).envs(run.env.iter().map(|(k, v)| (k, v))).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut command, 0);
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => return JobState::Failed(format!("cannot start {}: {err}", run.program)),
        };
        let stopped_already = match self.inner.0.lock() {
            Ok(mut inner) => {
                inner.current = Some((key.to_string(), child.id()));
                inner.stopped.contains(key)
            }
            Err(_) => false,
        };
        if stopped_already {
            end_process(child.id());
        }
        // cargo reports on stderr, dotnet and go on stdout: both are the build's lines.
        let (sender, pieces) = std::sync::mpsc::channel::<(usize, Piece)>();
        let readers: Vec<_> = [child.stdout.take().map(|o| Box::new(o) as Box<dyn std::io::Read + Send>), child.stderr.take().map(|e| Box::new(e) as Box<dyn std::io::Read + Send>)]
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(stream, from)| {
                let sender = sender.clone();
                std::thread::spawn(move || read_pieces(from, &mut |piece| {
                    let _ = sender.send((stream, piece));
                }))
            })
            .collect();
        drop(sender);
        for (stream, piece) in pieces {
            put(stream, piece);
        }
        for reader in readers {
            let _ = reader.join();
        }
        match child.wait() {
            Ok(status) if status.success() => JobState::Passed,
            Ok(status) => JobState::Failed(format!("{} ended with {status}", run.shown())),
            Err(err) => JobState::Failed(format!("{} stopped: {err}", run.program)),
        }
    }
}

/// Why a build a Clarity that ended left waiting or running reads as stopped.
const ENDED_WITH_CLARITY: &str = "Clarity ended before this build did";

/// How a build stands, as its record says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordState {
    Waiting,
    Running,
    Succeeded,
    Failed,
    Stopped,
}

impl RecordState {
    /// As people read it.
    pub fn text(self) -> &'static str {
        match self {
            RecordState::Waiting => "waiting",
            RecordState::Running => "running",
            RecordState::Succeeded => "succeeded",
            RecordState::Failed => "failed",
            RecordState::Stopped => "stopped",
        }
    }
}

/// A build's state as it is written to its file each time it changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRecord {
    pub key: String,
    pub title: String,
    pub command: String,
    pub state: RecordState,
    /// Why it failed.
    pub why: Option<String>,
    /// Who pressed it, when an agent asked; none for the person.
    pub by: Option<String>,
    /// When it was pressed, started and ended, in milliseconds since 1970.
    pub queued: Option<u64>,
    pub started: Option<u64>,
    pub ended: Option<u64>,
    /// Its last lines, LINES_KEPT at most.
    pub lines: Vec<String>,
    /// When this record was written.
    pub written: u64,
}

impl JobRecord {
    /// The record of `job` as it is now.
    pub fn of(job: &Job) -> JobRecord {
        let (state, why) = match &job.state {
            JobState::Waiting => (RecordState::Waiting, None),
            JobState::Running => (RecordState::Running, None),
            JobState::Passed => (RecordState::Succeeded, None),
            JobState::Failed(why) if why == STOPPED => (RecordState::Stopped, None),
            JobState::Failed(why) => (RecordState::Failed, Some(why.clone())),
        };
        let over = job.lines.len().saturating_sub(LINES_KEPT);
        JobRecord {
            key: job.key.clone(),
            title: job.title.clone(),
            command: job.command.clone(),
            state,
            why,
            by: job.by.clone(),
            queued: job.queued,
            started: job.started,
            ended: job.ended,
            lines: job.lines[over..].to_vec(),
            written: now_ms(),
        }
    }

    /// The record as people read it at `now`: its state, since when, how
    /// long, who started it, what it runs and its last `last` lines.
    pub fn told(&self, now: u64, last: usize) -> String {
        let since = |from: Option<u64>, to: u64| from.map(|f| duration_text(to.saturating_sub(f)));
        let state = self.state.text();
        let head = match self.state {
            RecordState::Waiting => match self.queued {
                Some(q) => format!("waiting for the builds before it since {}, {} so far", moment_text(q), duration_text(now.saturating_sub(q))),
                None => "waiting for the builds before it".to_string(),
            },
            RecordState::Running => match self.started {
                Some(s) => format!("running since {}, {} so far", moment_text(s), duration_text(now.saturating_sub(s))),
                None => "running".to_string(),
            },
            _ => {
                let mut text = state.to_string();
                if let Some(e) = self.ended {
                    text.push_str(&format!(" at {}", moment_text(e)));
                    if let Some(took) = since(self.started, e) {
                        text.push_str(&format!(" after {took}"));
                    }
                }
                if let Some(why) = &self.why {
                    text.push_str(&format!(": {why}"));
                }
                text
            }
        };
        let by = self.by.as_deref().unwrap_or("the person");
        let mut told = format!("{} ({}): {head}; started by {by}.\nRuns: {}", self.title, self.key, self.command);
        let shown = last.min(self.lines.len());
        if shown > 0 {
            told.push_str(&format!("\nLast {shown} of {} lines:\n{}", self.lines.len(), self.lines[self.lines.len() - shown..].join("\n")));
        }
        told
    }
}

/// A moment in milliseconds since 1970, in UTC: `2026-11-05 20:59:05 UTC`.
pub fn moment_text(ms: u64) -> String {
    let secs = ms / 1000;
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    // Days since 1970 to a civil date (Howard Hinnant's days_from_civil, inverted).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC", rest / 3600, rest / 60 % 60, rest % 60)
}

/// A length of time in milliseconds as people read it: `1 h 2 min 5 s`.
pub fn duration_text(ms: u64) -> String {
    let secs = ms / 1000;
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    match (h, m) {
        (0, 0) => format!("{s} s"),
        (0, _) => format!("{m} min {s} s"),
        _ => format!("{h} h {m} min {s} s"),
    }
}

/// The file of the build `key` in `dir`: its key with what a file name
/// cannot hold on every system (`:`) as `.`: `piece.clarityUi.debug.json`.
pub fn record_file(dir: &Path, key: &str) -> PathBuf {
    let name: String = key.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '.' }).collect();
    dir.join(format!("{name}.json"))
}

/// Writes `record` to its file in `dir`: aside first, then renamed, so it is never read half written.
pub fn write_record(dir: &Path, record: &JobRecord) -> Result<(), String> {
    static ASIDE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    let text = serde_json::to_string(record).map_err(|e| e.to_string())?;
    let n = ASIDE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let aside = dir.join(format!(".writing-{}-{n}", std::process::id()));
    std::fs::write(&aside, text).map_err(|e| format!("cannot write {}: {e}", aside.display()))?;
    std::fs::rename(&aside, record_file(dir, &record.key)).map_err(|e| {
        let _ = std::fs::remove_file(&aside);
        format!("cannot write the record of {}: {e}", record.key)
    })
}

/// The record of the build `key` in `dir`, when there is one.
pub fn read_record(dir: &Path, key: &str) -> Option<JobRecord> {
    serde_json::from_str(&std::fs::read_to_string(record_file(dir, key)).ok()?).ok()
}

/// Every build's record in `dir`, by key.
pub fn read_records(dir: &Path) -> Vec<JobRecord> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut records: Vec<JobRecord> = entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .filter_map(|e| serde_json::from_str(&std::fs::read_to_string(e.path()).ok()?).ok())
        .collect();
    records.sort_by(|a, b| a.key.cmp(&b.key));
    records
}

/// Writes `job`'s record when its builds write them; a failure to write never stops a build.
fn record(dir: &Option<PathBuf>, job: &Job) {
    if let Some(dir) = dir
        && let Err(why) = write_record(dir, &JobRecord::of(job))
    {
        eprintln!("clarity: {why}");
    }
}
