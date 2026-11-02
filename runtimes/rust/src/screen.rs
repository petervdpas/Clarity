//! Engines beside the screen (DESIGN 5.12, conformance/screen-engines.json),
//! as a canvas sees them: frames, each the input since the last and the
//! milliseconds it took, answered with the canvas's shapes and what the
//! engine told. The words are plain Rust, with no WebAssembly: a screen
//! (Clarity.UI) runs any `Frames`, and the runtime's `engine::ScreenEngine`
//! (feature `engines`) is the one that runs an engine's file.

use serde_json::Value;

/// Pointer, a key or a hear on a canvas, as its engine gets it with the next frame.
#[derive(Debug, Clone, PartialEq)]
pub enum ScreenInput {
    /// The pointer: `down`, `move` or `up`, in the canvas's units.
    Pointer { kind: String, x: i64, y: i64 },
    /// A key, `down` or `up`, by Clarity's name (`a`, `Space`, `ArrowLeft`).
    Key { kind: String, name: String },
    /// What a handler told the canvas (`sky.start(2)`): the engine's function of that name.
    Hear { name: String, values: Vec<Value> },
}

impl ScreenInput {
    /// Adds `input` to what waits for the next frame: moves in a row keep only the last.
    pub fn queue(waiting: &mut Vec<ScreenInput>, input: ScreenInput) {
        let is_move = |i: &ScreenInput| matches!(i, ScreenInput::Pointer { kind, .. } if kind == "move");
        match waiting.last_mut() {
            Some(last) if is_move(last) && is_move(&input) => *last = input,
            _ => waiting.push(input),
        }
    }
}

/// What the engine told the window: the handler wired to it runs with its values.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenTell {
    pub tell: String,
    pub values: Vec<Value>,
}

/// A frame: the canvas's shapes (conformance/drawing.json), and what the engine told while making it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Drawn {
    pub shapes: Vec<Value>,
    pub tells: Vec<ScreenTell>,
}

/// Which input an engine beside the screen takes: whether it exports
/// `clarity_pointer` and `clarity_key`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Hears {
    pub pointer: bool,
    pub keys: bool,
}

/// A canvas's frames: each the input and milliseconds since the last,
/// answered with the canvas's shapes; what went wrong when the engine
/// failed, after which every frame fails alike. Dropping it ends the engine.
pub trait Frames: Send {
    /// The pointer and keys it takes.
    fn hears(&self) -> Hears;

    /// One frame: the input since the last goes first, then `ms`.
    fn frame(&mut self, ms: f64, input: &[ScreenInput]) -> Result<Drawn, String>;
}

/// What a frame of an instance that is gone answers.
pub const GONE: &str = "The engine ended.";

/// The engines beside the screen a program has open, each by the number
/// `Program::engine_open` gives: the screen holds only the number, so each
/// call between them is plain data. A frame runs with only its own instance
/// held, so the canvases of several windows draw at once.
pub struct Instances {
    next: std::sync::atomic::AtomicU64,
    open: std::sync::Mutex<std::collections::BTreeMap<u64, std::sync::Arc<std::sync::Mutex<Box<dyn Frames>>>>>,
}

impl Default for Instances {
    fn default() -> Instances {
        Instances::new()
    }
}

impl Instances {
    /// None open yet; a static of a generated program.
    pub const fn new() -> Instances {
        Instances { next: std::sync::atomic::AtomicU64::new(1), open: std::sync::Mutex::new(std::collections::BTreeMap::new()) }
    }

    fn get(&self, id: u64) -> Option<std::sync::Arc<std::sync::Mutex<Box<dyn Frames>>>> {
        self.open.lock().unwrap_or_else(|p| p.into_inner()).get(&id).cloned()
    }

    /// Keeps `frames` open: its number.
    pub fn open(&self, frames: Box<dyn Frames>) -> u64 {
        let id = self.next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.open.lock().unwrap_or_else(|p| p.into_inner()).insert(id, std::sync::Arc::new(std::sync::Mutex::new(frames)));
        id
    }

    /// The pointer and keys instance `id` takes; none of either when it is gone.
    pub fn hears(&self, id: u64) -> Hears {
        self.get(id).map(|f| f.lock().unwrap_or_else(|p| p.into_inner()).hears()).unwrap_or_default()
    }

    /// One frame of instance `id`; one that is gone says so.
    pub fn frame(&self, id: u64, ms: f64, input: &[ScreenInput]) -> Result<Drawn, String> {
        match self.get(id) {
            Some(frames) => frames.lock().unwrap_or_else(|p| p.into_inner()).frame(ms, input),
            None => Err(GONE.to_string()),
        }
    }

    /// Ends instance `id`; it goes once a frame running now ends.
    pub fn close(&self, id: u64) {
        self.open.lock().unwrap_or_else(|p| p.into_inner()).remove(&id);
    }

    /// How many are open.
    pub fn count(&self) -> usize {
        self.open.lock().unwrap_or_else(|p| p.into_inner()).len()
    }
}
