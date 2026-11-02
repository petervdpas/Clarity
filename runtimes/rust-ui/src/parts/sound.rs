//! The sound part (DESIGN 5.22): heard, not seen, with no place in the
//! window. Code tells it what to play (`chime.play()`), as it tells a custom
//! part, through the reply's hears: play its source (a sound of the
//! program's resources), stop, a tone, a melody, a volume. Everything plays
//! through the output the host sets (`set_output`; rodio on the desktop),
//! nothing until then. It plays only while its window is on the screen:
//! what it plays stops as its window leaves (closed, or under a window
//! opened in place), and a sound told to play while it is off the screen is
//! not heard.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::markup::PartSpec;
use crate::melodies::{self, Note};
use crate::painter::Painter;
use crate::parts::{Part, PartBase, PartHost};

/// Something that plays; stopping it ends it.
pub trait Playing {
    fn stop(&mut self);
}

/// Where sounds play: the computer's speakers on the desktop.
pub trait SoundOutput {
    /// Plays `file` (WAV, or what the output reads) at `volume` (0 to 1), over and over when `repeat`.
    fn play(&self, file: Vec<u8>, volume: f32, repeat: bool) -> Box<dyn Playing>;
}

/// No output: what a program plays before its host sets one, and in tests.
pub struct SilentOutput;

struct Nothing;

impl Playing for Nothing {
    fn stop(&mut self) {}
}

impl SoundOutput for SilentOutput {
    fn play(&self, _file: Vec<u8>, _volume: f32, _repeat: bool) -> Box<dyn Playing> {
        Box::new(Nothing)
    }
}

thread_local! {
    static OUTPUT: RefCell<Option<Rc<dyn SoundOutput>>> = const { RefCell::new(None) };
}

/// Where every sound plays from now on: the host's output.
pub fn set_output(output: Rc<dyn SoundOutput>) {
    OUTPUT.with(|o| *o.borrow_mut() = Some(output));
}

/// Where sounds play: the output the host set, else none.
pub fn output() -> Rc<dyn SoundOutput> {
    OUTPUT.with(|o| o.borrow().clone()).unwrap_or_else(|| Rc::new(SilentOutput))
}

pub struct SoundPart {
    base: PartBase,
    playing: Vec<Box<dyn Playing>>,
    /// How loud it plays, 0 to 100: its `volume`, until code sets another.
    pub volume: i64,
    /// Each thing code told it, as it heard it.
    pub heard: Vec<String>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let volume = spec.number("volume").map(|v| v as i64).unwrap_or(100).clamp(0, 100);
    Box::new(SoundPart { base: PartBase::new(spec, host), playing: Vec::new(), volume, heard: Vec::new() })
}

/// A told value as a number, however code gave it.
fn number(given: &[Value], index: usize) -> f64 {
    match given.get(index) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

impl SoundPart {
    /// A sound of the program's resources, which `play` plays.
    pub fn source(&self) -> Option<&str> {
        self.base.spec.text("source").filter(|s| !s.is_empty())
    }

    /// Whether `play` plays its source over and over until `stop`.
    pub fn repeats(&self) -> bool {
        self.base.spec.flag("repeat")
    }

    /// How many sounds it plays now.
    pub fn playing(&self) -> usize {
        self.playing.len()
    }

    /// What code told it since it last heard: each in order.
    fn hear_all(&mut self) {
        let Some(name) = self.base.name().map(str::to_string) else { return };
        for hear in self.base.host.hears(&name) {
            self.hear(&hear.hear, &hear.values);
        }
    }

    fn hear(&mut self, hear: &str, given: &[Value]) {
        self.heard.push(hear.to_string());
        match hear {
            "play" => {
                if let Some(file) = self.source().map(str::to_string).and_then(|s| self.base.host.resource(&s)) {
                    let repeat = self.repeats();
                    self.start(file, repeat);
                }
            }
            "stop" => self.stop(),
            "tone" => {
                let note = Note { hertz: Some(number(given, 0)), seconds: number(given, 1) };
                self.start(melodies::wav(&[note]), false);
            }
            "melody" => {
                let notes = given.first().map(|n| n.as_str().map(str::to_string).unwrap_or_else(|| n.to_string())).unwrap_or_default();
                let (plays, _) = melodies::read(&notes, number(given, 1) as i64);
                self.start(melodies::wav(&plays), false);
            }
            "volume" => self.volume = (number(given, 0) as i64).clamp(0, 100),
            _ => {}
        }
    }

    fn start(&mut self, file: Vec<u8>, repeat: bool) {
        if self.base.node.attached() {
            self.playing.push(output().play(file, self.volume as f32 / 100.0, repeat));
        }
    }

    /// Stops everything it plays.
    pub fn stop(&mut self) {
        for mut playing in self.playing.drain(..) {
            playing.stop();
        }
    }
}

impl Element for SoundPart {
    fn node(&self) -> &crate::element::Node {
        &self.base.node
    }
    fn node_mut(&mut self) -> &mut crate::element::Node {
        &mut self.base.node
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn as_part(&self) -> Option<&dyn Part> {
        Some(self)
    }
    fn as_part_mut(&mut self) -> Option<&mut dyn Part> {
        Some(self)
    }

    /// Never seen: it takes no pointer, no focus and draws nothing.
    fn is_shown(&self) -> bool {
        false
    }

    /// Laid out with its window, it takes no room; what code told it is heard then, after every reply.
    fn arrange(&mut self, area: Rect, _cx: &mut Cx) {
        self.base.node.bounds = Rect::new(area.x, area.y, 0.0, 0.0);
        self.hear_all();
    }

    fn draw(&self, _p: &Painter) {}

    fn on_attached(&mut self, _cx: &mut Cx) {
        self.hear_all();
    }

    /// Its window left the screen: what it plays stops.
    fn on_detached(&mut self, _cx: &mut Cx) {
        self.stop();
    }
}

impl Part for SoundPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
