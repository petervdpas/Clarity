//! An engine beside the screen as the canvas runs it (DESIGN 5.12): the
//! program gives a new instance of the runtime's engine
//! (`Program::screen_engine`), and this hands its frames to the canvas
//! part, input in and shapes and tells out.

use std::rc::Rc;
use std::sync::Arc;

use clarity_runtime::program::Program;
use clarity_runtime::screen as runtime;

use crate::parts::data::canvas::engine::{Drawn, Frames, Hears, MakeFrames, ScreenInput, ScreenTell};

/// The runtime's engine as the canvas's frames.
struct Running(Box<dyn runtime::Frames>);

impl Frames for Running {
    fn hears(&self) -> Hears {
        let hears = self.0.hears();
        Hears { pointer: hears.pointer, keys: hears.keys }
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
        let drawn = self.0.frame(ms, &input)?;
        Ok(Drawn { shapes: drawn.shapes, tells: drawn.tells.into_iter().map(|t| ScreenTell { tell: t.tell, values: t.values }).collect() })
    }
}

/// How a canvas that `runs` engine `name` gets a new instance of it from `program`; one the program has not says so.
pub fn make_frames(program: Arc<dyn Program>, name: &str) -> MakeFrames {
    let name = name.to_string();
    Rc::new(move || match program.screen_engine(&name) {
        Some(Ok(engine)) => Ok(Box::new(Running(engine)) as Box<dyn Frames>),
        Some(Err(why)) => Err(why),
        None => Err(crate::locale::fill(&crate::locale::translate("engineMissing", "The program has no engine {engine}."), "engine", &name)),
    })
}
