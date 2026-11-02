//! Custom part Dice: dice on a felt table, drawn in 3D with Bevy, as a
//! control of Clarity.UI. Told to roll, the dice tumble, bounce and land,
//! then the part tells what they show. Keep the methods of `DicePart`
//! (src/clarity.rs), which follow the part's declaration.
//!
//! Bevy renders headless into a picture of its own (src/scene.rs); while the
//! table moves, the part wakes about 60 times a second, runs one Bevy update
//! and puts the picture read back in its place. The person rolls with a click,
//! Space or Enter, turns the table by dragging or with Left and Right, and
//! comes nearer with the wheel or + and -.

mod clarity;
mod scene;
pub use clarity::*;

use std::time::Duration;

use clarity_ui::Key;
use clarity_ui::locale;
use clarity_ui::TextAlign;

use scene::Scene;

/// How often the part updates while the table moves: about 60 times a second.
const FRAME: Duration = Duration::from_millis(16);
/// How often it looks whether its place changed size while the table rests.
const IDLE: Duration = Duration::from_millis(250);
/// How far a drag turns the table, in radians per pixel.
const TURN_PER_PIXEL: f32 = 0.01;
/// How far a press may move and still be a click, in pixels.
const CLICK_SLOP: f64 = 4.0;

#[derive(Default)]
pub struct Dice {
    /// Bevy's table, made at the first wake-up, once the part's size is known.
    scene: Option<Scene>,
    /// Why Bevy could not start, said in the part's place.
    failed: Option<String>,
    /// The count and roll asked for before the table was made.
    count: Option<i64>,
    roll_asked: bool,
    /// Whether a wake-up is asked for.
    waking: bool,
    /// Where a press went down and how far it moved since, while held.
    pressed: Option<(f64, f64)>,
    dragged: f64,
}

impl Dice {
    /// Asks for a wake-up now, unless one is coming.
    fn kick(&mut self, cx: &mut CustomCx) {
        if !self.waking {
            self.waking = true;
            let now = cx.now();
            cx.wake(now);
        }
    }

    /// Makes the table at the part's size, the first time; says why when it cannot.
    fn ensure_scene(&mut self, cx: &mut CustomCx) {
        if self.scene.is_some() || self.failed.is_some() {
            return;
        }
        let (width, height) = pixels_of(cx);
        match Scene::new(width, height) {
            Ok(mut scene) => {
                if let Some(count) = self.count {
                    scene.set_count(count);
                }
                if std::mem::take(&mut self.roll_asked) {
                    scene.roll();
                }
                self.scene = Some(scene);
            }
            Err(why) => {
                self.failed = Some(why);
                cx.invalidate();
            }
        }
    }

    fn roll(&mut self, cx: &mut CustomCx) {
        match self.scene.as_mut() {
            Some(scene) => scene.roll(),
            None => self.roll_asked = true,
        }
        self.kick(cx);
    }
}

/// The part's place in whole pixels.
fn pixels_of(cx: &CustomCx) -> (u32, u32) {
    let bounds = cx.bounds();
    (bounds.width.round().max(1.0) as u32, bounds.height.round().max(1.0) as u32)
}

impl DicePart for Dice {
    fn draw(&self, painter: &Painter, area: Rect) {
        if let Some(why) = &self.failed {
            let said = locale::translate("customPartFailed", "this part could not draw");
            painter.stroke(area, painter.look.color("error"), 1.0, painter.look.radius("input"));
            painter.text(&format!("{said}: {why}"), area.inset(6.0), painter.look.color("error"), painter.look.size("small"), "text", false, TextAlign::Left);
            return;
        }
        match self.scene.as_ref().and_then(Scene::pixels) {
            Some((rgba, width, height, row_bytes)) => painter.pixels(rgba, width, height, row_bytes, area),
            // Until the first picture comes back, the felt's place stays the window's.
            None => painter.stroke(area, painter.look.color("border"), 1.0, painter.look.radius("input")),
        }
    }

    fn show_count(&mut self, count: i64, cx: &mut CustomCx) {
        self.count = Some(count);
        if let Some(scene) = self.scene.as_mut() {
            scene.set_count(count);
        }
        self.kick(cx);
    }

    fn on_roll(&mut self, cx: &mut CustomCx) {
        self.roll(cx);
    }

    fn start(&mut self, cx: &mut CustomCx) {
        // The window opened: the first wake-up makes the table.
        self.kick(cx);
    }

    fn focusable(&self) -> bool {
        true
    }

    fn pointer_pressed(&mut self, at: PointerInput, cx: &mut CustomCx) {
        self.pressed = Some((at.x, at.y));
        self.dragged = 0.0;
        cx.focus();
    }

    fn pointer_moved(&mut self, at: PointerInput, cx: &mut CustomCx) {
        let Some((x, y)) = self.pressed else { return };
        let dx = at.x - x;
        self.dragged += dx.abs() + (at.y - y).abs();
        self.pressed = Some((at.x, at.y));
        if let Some(scene) = self.scene.as_mut() {
            scene.turn(-dx as f32 * TURN_PER_PIXEL);
            self.kick(cx);
        }
    }

    fn pointer_released(&mut self, _at: PointerInput, _cx: &mut CustomCx) {
        self.pressed = None;
    }

    fn click(&mut self, cx: &mut CustomCx) {
        // A drag turned the table; only a click in place rolls.
        if self.dragged <= CLICK_SLOP {
            self.roll(cx);
        }
    }

    fn wheel(&mut self, _dx: f64, dy: f64, _held: Modifiers, cx: &mut CustomCx) -> bool {
        let Some(scene) = self.scene.as_mut() else { return false };
        // The wheel turned down (dy above 0) goes farther, up comes nearer.
        scene.zoom(if dy > 0.0 { 1.0 / 0.9 } else { 0.9 });
        self.kick(cx);
        true
    }

    fn key(&mut self, key: KeyInput, cx: &mut CustomCx) -> bool {
        if !key.modifiers.is_empty() {
            return false;
        }
        match key.key {
            Key::Space | Key::Enter => self.roll(cx),
            Key::Left | Key::Right | Key::Plus | Key::Equal | Key::Minus => {
                let Some(scene) = self.scene.as_mut() else { return false };
                match key.key {
                    Key::Left => scene.turn(0.15),
                    Key::Right => scene.turn(-0.15),
                    Key::Minus => scene.zoom(1.0 / 0.9),
                    _ => scene.zoom(0.9),
                }
                self.kick(cx);
            }
            _ => return false,
        }
        true
    }

    fn focus_changed(&mut self, _focused: bool, cx: &mut CustomCx) {
        cx.invalidate();
    }

    fn wake(&mut self, cx: &mut CustomCx) {
        self.waking = false;
        self.ensure_scene(cx);
        let Some(scene) = self.scene.as_mut() else { return };
        let size = pixels_of(cx);
        if scene.size() != size {
            scene.resize(size.0, size.1);
        }
        // At rest, with nothing new to show, Bevy does not update at all.
        let mut busy = false;
        if scene.busy() {
            let step = scene.step();
            if step.fresh {
                cx.invalidate();
            }
            if let Some((total, faces)) = step.landed {
                cx.landed(total, &faces);
            }
            busy = step.busy;
        }
        self.waking = true;
        let next = cx.now() + if busy { FRAME } else { IDLE };
        cx.wake(next);
    }

    fn stop(&mut self) {
        // The window left the screen: Bevy and its renderer go with the part.
        self.scene = None;
    }
}
