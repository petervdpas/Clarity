//! Engines beside the screen as a host runs them for its canvases (DESIGN
//! 5.12). A canvas knows its engine only by number, through the part host
//! (`PartHost::engine_start`, `engine_frame`, `engine_drawn`,
//! `engine_stop`), so every call between them is plain data. A host keeps
//! the instances here: each frame runs on a thread of the instance's own, so
//! the window keeps drawing, or at once for a test. The program's engines
//! come in by number too (`Program::engine_open`), wrapped as [`Frames`].

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_runtime::screen as runtime;

use crate::parts::data::canvas::engine::{Drawn, Frames, Hears, ScreenInput, ScreenTell};

/// Where an instance's frames run.
enum Runner {
    /// At once, the frame kept until it is asked for.
    Here { frames: Box<dyn Frames>, drawn: Option<Result<Drawn, String>> },
    /// On a thread of its own, which ends when the instance stops or fails.
    Thread { jobs: Sender<(f64, Vec<ScreenInput>)>, done: Receiver<Result<Drawn, String>>, busy: bool },
}

struct Instance {
    hears: Hears,
    runner: Runner,
}

/// The engine instances a host runs for its canvases, by number.
pub struct Engines {
    here: bool,
    next: Cell<u64>,
    running: RefCell<BTreeMap<u64, Instance>>,
}

impl Default for Engines {
    fn default() -> Engines {
        Engines::new()
    }
}

/// What a frame of an instance that is gone or whose thread ended answers.
fn gone() -> String {
    crate::locale::translate("screenEngineGone", "The engine ended.")
}

impl Engines {
    /// Instances whose frames each run on a thread of their own.
    pub fn new() -> Engines {
        Engines { here: false, next: Cell::new(1), running: RefCell::new(BTreeMap::new()) }
    }

    /// Instances whose frames run at once, on the caller's thread: for a test, which waits for nothing.
    pub fn here() -> Engines {
        Engines { here: true, ..Engines::new() }
    }

    /// Runs `frames` as a new instance: its number.
    pub fn start(&self, frames: Box<dyn Frames>) -> u64 {
        let id = self.next.get();
        self.next.set(id + 1);
        let hears = frames.hears();
        let runner = if self.here {
            Runner::Here { frames, drawn: None }
        } else {
            let (jobs, work) = channel::<(f64, Vec<ScreenInput>)>();
            let (answer, done) = channel();
            let mut frames = frames;
            std::thread::spawn(move || {
                for (ms, input) in work {
                    let drawn = frames.frame(ms, &input);
                    let failed = drawn.is_err();
                    if answer.send(drawn).is_err() || failed {
                        break;
                    }
                }
                // The engine goes with its thread.
                drop(frames);
            });
            Runner::Thread { jobs, done, busy: false }
        };
        self.running.borrow_mut().insert(id, Instance { hears, runner });
        id
    }

    /// The pointer and keys instance `id` takes; none of either once it is gone.
    pub fn hears(&self, id: u64) -> Hears {
        self.running.borrow().get(&id).map(|i| i.hears).unwrap_or_default()
    }

    /// Starts the next frame of instance `id`; what it draws comes through `drawn`.
    pub fn frame(&self, id: u64, ms: f64, input: Vec<ScreenInput>) {
        let mut running = self.running.borrow_mut();
        let Some(instance) = running.get_mut(&id) else { return };
        match &mut instance.runner {
            Runner::Here { frames, drawn } => *drawn = Some(frames.frame(ms, &input)),
            Runner::Thread { jobs, busy, .. } => {
                // A thread that ended takes no frame: busy, so `drawn` says it ended.
                *busy = true;
                let _ = jobs.send((ms, input));
            }
        }
    }

    /// The frame of instance `id` that finished since the last ask; none while
    /// it runs. One that is gone, or whose thread ended while it ran, says so.
    pub fn drawn(&self, id: u64) -> Option<Result<Drawn, String>> {
        let mut running = self.running.borrow_mut();
        let Some(instance) = running.get_mut(&id) else { return Some(Err(gone())) };
        match &mut instance.runner {
            Runner::Here { drawn, .. } => drawn.take(),
            Runner::Thread { done, busy, .. } => match done.try_recv() {
                Ok(drawn) => {
                    *busy = false;
                    Some(drawn)
                }
                Err(TryRecvError::Disconnected) if *busy => {
                    *busy = false;
                    Some(Err(gone()))
                }
                Err(_) => None,
            },
        }
    }

    /// Ends instance `id`: at once here, on its thread once its frame running now ends.
    pub fn stop(&self, id: u64) {
        let gone = self.running.borrow_mut().remove(&id);
        drop(gone);
    }

    /// How many instances run.
    pub fn count(&self) -> usize {
        self.running.borrow().len()
    }
}

/// An instance of the program's engine, by the number the program gave
/// (`Program::engine_open`), as the frames a host runs; it closes there
/// when it goes.
pub struct ProgramFrames {
    program: Arc<dyn Program>,
    id: u64,
    hears: Hears,
}

impl ProgramFrames {
    /// A new instance of the program's engine `name`: why it could not start,
    /// also when the program has no engine of that name.
    pub fn open(program: Arc<dyn Program>, name: &str) -> Result<ProgramFrames, String> {
        match program.engine_open(name) {
            Some(Ok(id)) => {
                let hears = program.engine_hears(id);
                Ok(ProgramFrames { program, id, hears: Hears { pointer: hears.pointer, keys: hears.keys } })
            }
            Some(Err(why)) => Err(why),
            None => Err(crate::locale::fill(&crate::locale::translate("engineMissing", "The program has no engine {engine}."), "engine", name)),
        }
    }
}

impl Frames for ProgramFrames {
    fn hears(&self) -> Hears {
        self.hears
    }

    fn frame(&mut self, ms: f64, input: &[ScreenInput]) -> Result<Drawn, String> {
        let input: Vec<runtime::ScreenInput> = input
            .iter()
            .map(|i| match i.clone() {
                ScreenInput::Pointer { kind, x, y } => runtime::ScreenInput::Pointer { kind, x, y },
                ScreenInput::Key { kind, name } => runtime::ScreenInput::Key { kind, name },
                ScreenInput::Hear { name, values } => runtime::ScreenInput::Hear { name, values },
            })
            .collect();
        let drawn = self.program.engine_frame(self.id, ms, input)?;
        Ok(Drawn { shapes: drawn.shapes, tells: drawn.tells.into_iter().map(|t| ScreenTell { tell: t.tell, values: t.values }).collect() })
    }
}

impl Drop for ProgramFrames {
    fn drop(&mut self) {
        self.program.engine_close(self.id);
    }
}
