//! The builds DevTools runs when Build (or a release check's Run) is
//! pressed: one at a time, as the machine can bear, the others waiting in
//! the order pressed. Each keeps its last lines and how it ended, so its tab
//! shows it while it builds and after.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

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
    /// When it started and ended, in milliseconds since 1970.
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
        let job = Job { key: key.to_string(), title: title.to_string(), command: run.shown(), state: JobState::Waiting, lines: Vec::new(), by, started: None, ended: None };
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
            job.clone()
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
            let ended = self.run(&key, &run, &on_change);
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
        let dir = run.dir.canonicalize().unwrap_or_else(|_| run.dir.clone());
        put(usize::MAX, Piece::Line(format!("{} (in {})", run.shown(), dir.display())));
        let mut command = Command::new(&run.program);
        command.args(&run.args).current_dir(&run.dir).envs(run.env.iter().map(|(k, v)| (k, v))).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => return JobState::Failed(format!("cannot start {}: {err}", run.program)),
        };
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
