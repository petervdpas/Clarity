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
