//! Custom part Dice: dice on a felt table, drawn in 3D with Bevy, as a
//! control of Clarity.UI, drawn through its library. Told to roll, the dice tumble, bounce and land,
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

use scene::Scene;

/// How often the part updates while the table moves, in milliseconds: about 60 times a second.
const FRAME_MS: f64 = 16.0;
/// How often it looks whether its place changed size while the table rests.
const IDLE_MS: f64 = 250.0;
/// How far a drag turns the table, in radians per pixel.
const TURN_PER_PIXEL: f32 = 0.01;
/// How far a press may move and still be a click, in pixels.
const CLICK_SLOP: f64 = 4.0;

#[derive(Default)]
pub struct Dice {
    /// Bevy's table, made at the first wake-up, once the part's size is known.
    scene: Option<Scene>,
    /// Why Bevy could not start, said in the part's place in the language shown.
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
    fn kick(&mut self, cx: &mut dyn Cx) {
        if !self.waking {
            self.waking = true;
            cx.wake_in(0.0);
        }
    }

    /// Makes the table at the part's size, the first time; says why when it cannot.
    fn ensure_scene(&mut self, cx: &mut dyn Cx) {
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
                let said = cx.translate("customPartFailed", "this part could not draw");
                self.failed = Some(format!("{said}: {why}"));
                cx.invalidate();
            }
        }
    }

    fn roll(&mut self, cx: &mut dyn Cx) {
        match self.scene.as_mut() {
            Some(scene) => scene.roll(),
            None => self.roll_asked = true,
        }
        self.kick(cx);
    }
}

/// The part's place in whole pixels.
fn pixels_of(cx: &dyn Cx) -> (u32, u32) {
    let (_, _, width, height) = cx.bounds();
    (width.round().max(1.0) as u32, height.round().max(1.0) as u32)
}

impl DicePart for Dice {
    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64) {
        if let Some(failed) = &self.failed {
            let error = canvas.look_color("error");
            canvas.stroke(x, y, width, height, error, 1.0, 4.0);
            canvas.text(failed, x + 6.0, y + 6.0, width - 12.0, height - 12.0, error, 0.0, false, 0);
            return;
        }
        match self.scene.as_ref().and_then(Scene::pixels) {
            Some((rgba, w, h, row_bytes)) => canvas.pixels(rgba, w, h, row_bytes, x, y, width, height),
            // Until the first picture comes back, the felt's place stays the window's.
            None => {
                let border = canvas.look_color("border");
                canvas.stroke(x, y, width, height, border, 1.0, 4.0);
            }
        }
    }

    fn show_count(&mut self, count: i64, cx: &mut dyn Cx) {
        self.count = Some(count);
        if let Some(scene) = self.scene.as_mut() {
            scene.set_count(count);
        }
        self.kick(cx);
    }

    fn on_roll(&mut self, cx: &mut dyn Cx) {
        self.roll(cx);
    }

    fn start(&mut self, cx: &mut dyn Cx) {
        // The window opened: the first wake-up makes the table.
        self.kick(cx);
    }

    fn focusable(&self) -> bool {
        true
    }

    fn pointer_pressed(&mut self, at: Pointer, cx: &mut dyn Cx) {
        self.pressed = Some((at.x, at.y));
        self.dragged = 0.0;
        cx.focus();
    }

    fn pointer_moved(&mut self, at: Pointer, cx: &mut dyn Cx) {
        let Some((x, y)) = self.pressed else { return };
        let dx = at.x - x;
        self.dragged += dx.abs() + (at.y - y).abs();
        self.pressed = Some((at.x, at.y));
        if let Some(scene) = self.scene.as_mut() {
            scene.turn(-dx as f32 * TURN_PER_PIXEL);
            self.kick(cx);
        }
    }

    fn pointer_released(&mut self, _at: Pointer, _cx: &mut dyn Cx) {
        self.pressed = None;
    }

    fn click(&mut self, cx: &mut dyn Cx) {
        // A drag turned the table; only a click in place rolls.
        if self.dragged <= CLICK_SLOP {
            self.roll(cx);
        }
    }

    fn wheel(&mut self, _dx: f64, dy: f64, _modifiers: u32, cx: &mut dyn Cx) -> bool {
        let Some(scene) = self.scene.as_mut() else { return false };
        // The wheel turned down (dy above 0) goes farther, up comes nearer.
        scene.zoom(if dy > 0.0 { 1.0 / 0.9 } else { 0.9 });
        self.kick(cx);
        true
    }

    fn key(&mut self, name: &str, modifiers: u32, cx: &mut dyn Cx) -> bool {
        if modifiers != 0 {
            return false;
        }
        match name {
            "Space" | "Enter" => self.roll(cx),
            "Left" | "Right" | "+" | "Equal" | "-" => {
                let Some(scene) = self.scene.as_mut() else { return false };
                match name {
                    "Left" => scene.turn(0.15),
                    "Right" => scene.turn(-0.15),
                    "-" => scene.zoom(1.0 / 0.9),
                    _ => scene.zoom(0.9),
                }
                self.kick(cx);
            }
            _ => return false,
        }
        true
    }

    fn focus_changed(&mut self, _focused: bool, cx: &mut dyn Cx) {
        cx.invalidate();
    }

    fn wake(&mut self, cx: &mut dyn Cx) {
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
        cx.wake_in(if busy { FRAME_MS } else { IDLE_MS });
    }

    fn stop(&mut self) {
        // The window left the screen: Bevy and its renderer go with the part.
        self.scene = None;
    }
}
