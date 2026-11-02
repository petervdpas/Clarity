//! A canvas an engine beside the screen draws (DESIGN 5.12,
//! conformance/screen-engines.json): one instance from when it first shows,
//! a frame at the screen's pace, the frame's work off the screen's thread
//! and its shapes shown on the next frame the screen draws. While it runs it
//! asks the screen for every frame (`Cx::animate`). When the engine stops,
//! it says why in its place. Its pointer and keys go to the engine, when it
//! takes them: it keeps the pointer while it is down (at the drawing's
//! nearest edge beside it), and takes the keyboard when pressed or tabbed
//! to. What handlers tell it (`PartHost::hears`) goes to the engine with the
//! next frame; what the engine tells runs the window's handler (`PartHost::told`).
//! A canvas that `runs` an engine is made as this part (`canvas::make`).
//!
//! The engine itself is the program's (the runtime hosts its WebAssembly).
//! The part knows its instance only by number, through the part host
//! (`PartHost::engine_start`, `engine_frame`, `engine_drawn`, `engine_stop`):
//! plain data both ways. The host decides where a frame runs
//! (`crate::engines::Engines`).

use std::rc::Rc;
use std::time::Instant;

use serde_json::Value;

use super::{paint, paper_of, point_near, point_of, units_of, whole, Shape};
use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};
use crate::picture::Picture;

/// Pointer, a key or a hear on a canvas, as its engine gets it with the next frame.
#[derive(Debug, Clone, PartialEq)]
pub enum ScreenInput {
    /// The pointer: down, move or up, in the canvas's units.
    Pointer { kind: String, x: i64, y: i64 },
    /// A key, down or up, by Clarity's name.
    Key { kind: String, name: String },
    /// What a handler told the canvas (`sky.start(2)`): the engine's function of that name.
    Hear { name: String, values: Vec<Value> },
}

impl ScreenInput {
    /// Adds `input` to what waits for the next frame: moves in a row keep only the last.
    pub fn queue(waiting: &mut Vec<ScreenInput>, input: ScreenInput) {
        let is_move = |i: &ScreenInput| matches!(i, ScreenInput::Pointer { kind, .. } if kind == "move");
        if waiting.last().is_some_and(is_move) && is_move(&input) {
            *waiting.last_mut().expect("a last input") = input;
        } else {
            waiting.push(input);
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

/// Which input an engine beside the screen takes: it exports clarity_pointer, clarity_key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Hears {
    pub pointer: bool,
    pub keys: bool,
}

/// An engine's frames as a host runs them (`crate::engines::Engines`):
/// each the input and milliseconds since the last, answered with the
/// canvas's shapes; what went wrong when the engine failed. Dropping it ends
/// the engine.
pub trait Frames: Send {
    /// The pointer and keys it takes.
    fn hears(&self) -> Hears;

    /// One frame. Pointer and keys it does not take are left out of `input`.
    fn frame(&mut self, ms: f64, input: &[ScreenInput]) -> Result<Drawn, String>;
}

/// Clarity's name of a key as an engine gets it (conformance/screen-engines.json, keys); none for one it does not get.
pub fn key_name(key: Key) -> Option<String> {
    if let Some(i) = Key::LETTERS.iter().position(|k| *k == key) {
        return Some(((b'a' + i as u8) as char).to_string());
    }
    if let Some(i) = Key::DIGITS.iter().position(|k| *k == key) {
        return Some(((b'0' + i as u8) as char).to_string());
    }
    if let Some(i) = Key::FUNCTIONS.iter().position(|k| *k == key) {
        return Some(format!("F{}", i + 1));
    }
    let named = match key {
        Key::Space => "Space",
        Key::Enter => "Enter",
        Key::Escape => "Escape",
        Key::Backspace => "Backspace",
        Key::Delete => "Delete",
        Key::Left => "ArrowLeft",
        Key::Right => "ArrowRight",
        Key::Up => "ArrowUp",
        Key::Down => "ArrowDown",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::Shift => "Shift",
        Key::Control => "Control",
        Key::Alt => "Alt",
        _ => return None,
    };
    Some(named.to_string())
}

/// The keys that are down, so each goes down once however long it repeats.
#[derive(Debug, Default)]
pub struct HeldKeys {
    held: Vec<String>,
}

impl HeldKeys {
    /// Whether `name` went down now, rather than repeating.
    pub fn down(&mut self, name: &str) -> bool {
        if self.held.iter().any(|h| h == name) {
            return false;
        }
        self.held.push(name.to_string());
        true
    }

    /// Whether `name` was down.
    pub fn up(&mut self, name: &str) -> bool {
        match self.held.iter().position(|h| h == name) {
            Some(at) => {
                self.held.remove(at);
                true
            }
            None => false,
        }
    }

    /// Lets go of every key: the canvas lost the keyboard.
    pub fn all(&mut self) -> Vec<String> {
        std::mem::take(&mut self.held)
    }
}

/// The frame loop of one engine instance, which the host runs by its
/// number: one frame at a time, a frame that comes while the last is busy is
/// skipped, its time not lost. Pointer, keys and hears wait for the next
/// frame and go with it.
struct FrameLoop {
    id: u64,
    hears: Hears,
    busy: bool,
    last: Option<Instant>,
    waiting: Vec<ScreenInput>,
}

impl FrameLoop {
    fn new(id: u64, hears: Hears) -> FrameLoop {
        FrameLoop { id, hears, busy: false, last: None, waiting: Vec::new() }
    }

    fn input(&mut self, input: ScreenInput) {
        let takes = match &input {
            ScreenInput::Pointer { .. } => self.hears.pointer,
            ScreenInput::Key { .. } => self.hears.keys,
            ScreenInput::Hear { .. } => true,
        };
        if takes {
            ScreenInput::queue(&mut self.waiting, input);
        }
    }

    /// A frame of the screen at `now`: what the last frame drew, when it is
    /// done; the next one starts when none is busy. A host that runs a frame
    /// at once has it drawn in the same turn.
    fn tick(&mut self, host: &dyn PartHost, now: Instant) -> Option<Result<Drawn, String>> {
        let mut answer = None;
        if self.busy {
            match host.engine_drawn(self.id) {
                Some(drawn) => {
                    self.busy = false;
                    let failed = drawn.is_err();
                    answer = Some(drawn);
                    if failed {
                        return answer;
                    }
                }
                None => return None,
            }
        }
        // Whole milliseconds: a screen's clock has fractions, which wait for the next frame.
        let ms = match self.last {
            Some(before) => (now.saturating_duration_since(before).as_secs_f64() * 1000.0).floor(),
            None => 0.0,
        };
        // The screen lays out more often than it draws a frame: no time gone, no frame.
        if self.last.is_some() && ms <= 0.0 {
            return answer;
        }
        self.last = Some(match self.last {
            Some(was) => was + std::time::Duration::from_secs_f64(ms / 1000.0),
            None => now,
        });
        let input = std::mem::take(&mut self.waiting);
        host.engine_frame(self.id, ms, input);
        self.busy = true;
        if answer.is_none() {
            if let Some(drawn) = host.engine_drawn(self.id) {
                self.busy = false;
                answer = Some(drawn);
            }
        }
        answer
    }
}

pub struct EngineCanvasPart {
    base: PartBase,
    started: bool,
    frames: Option<FrameLoop>,
    drawing: Vec<Shape>,
    stopped: Option<String>,
    pointer_down: bool,
    held: HeldKeys,
    /// The pictures of the resources folder its image shapes draw, loaded once.
    pictures: std::cell::RefCell<std::collections::HashMap<String, Option<Picture>>>,
}

impl EngineCanvasPart {
    /// A canvas that runs the engine its `runs` names: a new instance from the host each time it joins a screen.
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> EngineCanvasPart {
        EngineCanvasPart { base: PartBase::new(spec, host), started: false, frames: None, drawing: Vec::new(), stopped: None, pointer_down: false, held: HeldKeys::default(), pictures: Default::default() }
    }

    /// The shapes of its last frame, in its units.
    pub fn drawing(&self) -> &[Shape] {
        &self.drawing
    }

    /// Why its engine stopped, which it shows in place of its drawing; none while it runs.
    pub fn stopped(&self) -> Option<&str> {
        self.stopped.as_deref()
    }

    /// Why its engine stopped, in Clarity's words in the person's language; none while it runs.
    pub fn stopped_text(&self) -> Option<String> {
        self.stopped.as_ref().map(|why| locale::fill(&locale::translate("screenEngineFailed", "The engine stopped: {problem}"), "problem", why))
    }

    /// A picture of the resources folder by its name, loaded once.
    fn picture(&self, name: &str) -> Option<Picture> {
        if let Some(kept) = self.pictures.borrow().get(name) {
            return kept.clone();
        }
        let loaded = self.base.host.resource(name).and_then(|bytes| Picture::read(name, &bytes));
        self.pictures.borrow_mut().insert(name.to_string(), loaded.clone());
        loaded
    }

    fn hears(&self) -> Hears {
        self.frames.as_ref().map(|f| f.hears).unwrap_or_default()
    }

    fn pointing(&self) -> bool {
        self.hears().pointer && self.stopped.is_none()
    }

    /// Starts its engine; one that cannot start says why.
    fn start(&mut self, cx: &mut Cx) {
        self.started = true;
        let runs = self.base.spec.text("runs").unwrap_or_default().to_string();
        match self.base.host.engine_start(&runs) {
            None => self.stopped = Some(locale::translate("screenEngineMissing", "The window has no engine of this name.")),
            Some(Ok(id)) => self.frames = Some(FrameLoop::new(id, self.base.host.engine_hears(id))),
            Some(Err(why)) => self.stop(why, cx),
        }
    }

    /// Lets go of its engine instance: the host ends it.
    fn let_go(&mut self) {
        if let Some(frames) = self.frames.take() {
            self.base.host.engine_stop(frames.id);
        }
    }

    fn stop(&mut self, why: String, cx: &mut Cx) {
        self.stopped = Some(why);
        self.let_go();
        cx.animate(self.base.node.id(), false);
        cx.invalidate();
    }

    /// Ends its engine: the canvas left its screen. One that joins a screen again starts a new engine.
    pub fn end(&mut self, cx: &mut Cx) {
        self.let_go();
        self.held.all();
        self.pointer_down = false;
        cx.animate(self.base.node.id(), false);
    }

    /// What a frame drew: its shapes show, what it told runs the window's handler; a shape that is none ends the engine.
    fn settle(&mut self, drawn: Result<Drawn, String>, cx: &mut Cx) {
        let drawn = drawn.and_then(|d| {
            let shapes = d.shapes.iter().map(Shape::of).collect::<Result<Vec<Shape>, String>>()?;
            Ok((shapes, d.tells))
        });
        match drawn {
            Ok((shapes, tells)) => {
                self.drawing = shapes;
                if let Some(name) = self.base.name() {
                    for tell in tells {
                        self.base.host.told(name, &tell.tell, &tell.values);
                    }
                }
                cx.invalidate();
            }
            Err(why) => self.stop(why, cx),
        }
    }

    fn engine_point(&self, x: f64, y: f64, near: bool) -> Option<(i64, i64)> {
        let (uw, uh) = units_of(&self.base.spec);
        let b = self.base.node.bounds;
        if near {
            point_near(uw, uh, b.width, b.height, x, y)
        } else {
            point_of(uw, uh, b.width, b.height, x, y)
        }
    }

    fn pointer(&mut self, kind: &str, at: (i64, i64)) {
        if let Some(frames) = self.frames.as_mut() {
            frames.input(ScreenInput::Pointer { kind: kind.to_string(), x: at.0, y: at.1 });
        }
    }

    fn key(&mut self, kind: &str, name: String) {
        if let Some(frames) = self.frames.as_mut() {
            frames.input(ScreenInput::Key { kind: kind.to_string(), name });
        }
    }
}

impl Element for EngineCanvasPart {
    part_element!();

    fn focusable(&self) -> bool {
        self.hears().keys && self.stopped.is_none()
    }

    /// Laid out to be drawn: its engine starts with the first frame, and each frame the screen draws is one of the engine's.
    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        if !self.started {
            self.start(cx);
        }
        if self.stopped.is_some() {
            return;
        }
        // What handlers told it goes with the next frame; what they told before it showed comes first.
        if let (Some(name), Some(frames)) = (self.base.name(), self.frames.as_mut()) {
            for hear in self.base.host.hears(name) {
                frames.input(ScreenInput::Hear { name: hear.hear, values: hear.values });
            }
        }
        let now = cx.now();
        let host = self.base.host.clone();
        if let Some(drawn) = self.frames.as_mut().and_then(|f| f.tick(host.as_ref(), now)) {
            self.settle(drawn, cx);
        }
        if self.stopped.is_none() && self.frames.is_some() {
            cx.animate(self.base.node.id(), true);
        }
    }

    /// It joined a screen: its engine starts with the next frame drawn, a new instance each time.
    fn on_attached(&mut self, cx: &mut Cx) {
        let _ = cx;
        self.started = false;
        self.stopped = None;
    }

    /// It left its screen (its window closed or went under another): its engine ends.
    fn on_detached(&mut self, cx: &mut Cx) {
        self.end(cx);
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        let _ = cx;
        if !self.pointing() || !self.base.enabled() {
            return;
        }
        let Some(point) = self.engine_point(at.x, at.y, false) else { return };
        self.pointer_down = true;
        self.pointer("down", point);
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        let _ = cx;
        if !self.pointing() {
            return;
        }
        if let Some(point) = self.engine_point(at.x, at.y, self.pointer_down) {
            self.pointer("move", point);
        }
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        let _ = cx;
        if !self.pointer_down {
            return;
        }
        self.pointer_down = false;
        if !self.pointing() {
            return;
        }
        if let Some(point) = self.engine_point(at.x, at.y, true) {
            self.pointer("up", point);
        }
    }

    /// A key goes down once, however long it repeats; a shortcut with Control, Alt or the command key is the window's, but those keys themselves are held keys.
    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        let _ = cx;
        if !self.hears().keys || self.stopped.is_some() || self.frames.is_none() {
            return false;
        }
        let Some(name) = key_name(key.key) else { return false };
        let modifier = matches!(key.key, Key::Shift | Key::Control | Key::Alt);
        if !modifier && (key.modifiers.has(Modifiers::CONTROL) || key.modifiers.has(Modifiers::ALT) || key.modifiers.has(Modifiers::META)) {
            return false;
        }
        if self.held.down(&name) {
            self.key("down", name);
        }
        true
    }

    /// A key let go goes up, when it went down here.
    fn on_key_up(&mut self, key: KeyInput, cx: &mut Cx) {
        let _ = cx;
        if let Some(name) = key_name(key.key) {
            if self.held.up(&name) {
                self.key("up", name);
            }
        }
    }

    /// Losing the focus lets go of every key still down.
    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        let _ = cx;
        if focused {
            return;
        }
        for name in self.held.all() {
            self.key("up", name);
        }
    }

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        let _clip = p.clip(bounds);
        if let Some(said) = self.stopped_text() {
            p.text(&said, bounds.inset(8.0), p.look.color("muted"), p.look.size("text"), "text", false, TextAlign::Center);
            return;
        }
        let units = units_of(&self.base.spec);
        let v = whole(units.0, units.1, bounds.width, bounds.height);
        paint(p, bounds, &self.drawing, v, units, &paper_of(&self.base), false, &|name| self.picture(name), None);
    }
}

/// A canvas that goes ends its engine, also when it never left its screen.
impl Drop for EngineCanvasPart {
    fn drop(&mut self) {
        self.let_go();
    }
}

impl Part for EngineCanvasPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    /// A canvas grows into the height its window has left, on every stack.
    fn grows(&self) -> bool {
        true
    }
}
