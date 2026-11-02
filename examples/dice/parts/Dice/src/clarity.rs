//! Custom part Dice (DESIGN 5.19, 7.9): a control of Clarity.UI. The trait
//! the crate's `Dice` implements, what it says to its window, and the wrapper
//! that makes it a part of clarity-ui. Written by Clarity from the part's
//! declaration, and written again when the declaration changes; the rest of
//! the crate is yours.
//!
//! Every method is called on the screen's thread, one at a time; the part
//! is made when its window opens and lives as long as its window.

#![allow(dead_code)]

pub use clarity_ui::parts::custom::{CustomCx, CustomPart};
pub use clarity_ui::{KeyInput, Modifiers, Painter, PointerInput, Rect};
/// Skia, for drawing on the whole canvas (`painter.canvas`), at the version Clarity.UI is built on.
pub use clarity_ui::skia_safe;
use serde_json::Value;

/// What part Dice does: it draws itself in its place and shows what the
/// window gives it; the input methods it does not write do nothing.
pub trait DicePart: Default + 'static {
    /// Draws itself in `area`, its place in the window, with the theme's look (`painter.look`).
    fn draw(&self, painter: &Painter, area: Rect);

    /// Property count, as the window or code sets it.
    fn show_count(&mut self, count: i64, cx: &mut CustomCx);

    /// What code tells it: roll.
    fn on_roll(&mut self, cx: &mut CustomCx);

    /// Whether it takes the focus: pressed, or reached with Tab.
    fn focusable(&self) -> bool {
        false
    }

    /// The main button went down on it.
    fn pointer_pressed(&mut self, at: PointerInput, cx: &mut CustomCx) {
        let _ = (at, cx);
    }

    /// The pointer moved over it, or anywhere while it holds the pointer.
    fn pointer_moved(&mut self, at: PointerInput, cx: &mut CustomCx) {
        let _ = (at, cx);
    }

    /// The main button went up after it was pressed on it.
    fn pointer_released(&mut self, at: PointerInput, cx: &mut CustomCx) {
        let _ = (at, cx);
    }

    /// Pressed and released with the main button.
    fn click(&mut self, cx: &mut CustomCx) {
        let _ = cx;
    }

    /// The wheel turned over it with keys `held`: true when it took the turn.
    fn wheel(&mut self, dx: f64, dy: f64, held: Modifiers, cx: &mut CustomCx) -> bool {
        let _ = (dx, dy, held, cx);
        false
    }

    /// A key while it has the focus: true when it took the key.
    fn key(&mut self, key: KeyInput, cx: &mut CustomCx) -> bool {
        let _ = (key, cx);
        false
    }

    /// Text typed while it has the focus.
    fn text(&mut self, text: &str, cx: &mut CustomCx) {
        let _ = (text, cx);
    }

    /// It got or lost the focus.
    fn focus_changed(&mut self, focused: bool, cx: &mut CustomCx) {
        let _ = (focused, cx);
    }

    /// The time it asked for came (`cx.wake(at)`).
    fn wake(&mut self, cx: &mut CustomCx) {
        let _ = cx;
    }

    /// Its window opened: called once, after the values of its line, what
    /// code set and what it binds; it may start a timer or an animation here.
    fn start(&mut self, cx: &mut CustomCx) {
        let _ = cx;
    }

    /// Its window left the screen: it stops what it started.
    fn stop(&mut self) {}

    /// What it says about the place at a point when the pointer rests there.
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        let _ = (x, y);
        None
    }
}

/// What part Dice says to its window, from any of its methods: `cx.changed(..)` and one per tell.
pub trait DiceTells {
    /// Tells landed: the handler wired to it runs with these values.
    fn landed(&mut self, total: i64, faces: &str);
}

impl DiceTells for CustomCx<'_> {
    fn landed(&mut self, total: i64, faces: &str) {
        self.tell("landed", vec![Value::from(total), Value::from(faces)]);
    }
}

/// Part Dice as Clarity.UI holds it: the window's words in, typed.
struct Held<P>(P);

impl<P: DicePart> CustomPart for Held<P> {
    fn set(&mut self, name: &str, value: &Value, cx: &mut CustomCx) {
        if name == "enabled" {
            // The window draws it again; `cx.enabled()` says which it is.
            cx.invalidate();
            return;
        }
        match name {
            "count" => self.0.show_count(integer(value), cx),
            _ => {}
        }
    }

    fn hear(&mut self, hear: &str, values: &[Value], cx: &mut CustomCx) {
        // What it hears may take no values.
        let _ = values;
        match hear {
            "roll" => self.0.on_roll(cx),
            _ => {}
        }
    }

    fn draw(&self, painter: &Painter, area: Rect) {
        self.0.draw(painter, area);
    }

    fn focusable(&self) -> bool {
        self.0.focusable()
    }

    fn pointer_pressed(&mut self, at: PointerInput, cx: &mut CustomCx) {
        self.0.pointer_pressed(at, cx);
    }

    fn pointer_moved(&mut self, at: PointerInput, cx: &mut CustomCx) {
        self.0.pointer_moved(at, cx);
    }

    fn pointer_released(&mut self, at: PointerInput, cx: &mut CustomCx) {
        self.0.pointer_released(at, cx);
    }

    fn click(&mut self, cx: &mut CustomCx) {
        self.0.click(cx);
    }

    fn wheel(&mut self, dx: f64, dy: f64, held: Modifiers, cx: &mut CustomCx) -> bool {
        self.0.wheel(dx, dy, held, cx)
    }

    fn key(&mut self, key: KeyInput, cx: &mut CustomCx) -> bool {
        self.0.key(key, cx)
    }

    fn text(&mut self, text: &str, cx: &mut CustomCx) {
        self.0.text(text, cx);
    }

    fn focus_changed(&mut self, focused: bool, cx: &mut CustomCx) {
        self.0.focus_changed(focused, cx);
    }

    fn wake(&mut self, cx: &mut CustomCx) {
        self.0.wake(cx);
    }

    fn start(&mut self, cx: &mut CustomCx) {
        self.0.start(cx);
    }

    fn stop(&mut self) {
        self.0.stop();
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        self.0.tooltip_at(x, y)
    }
}

/// A new part Dice, as the program registers it with Clarity.UI.
pub fn make() -> Box<dyn CustomPart> {
    Box::new(Held(crate::Dice::default()))
}

/// A whole number, from a number or the text of one.
fn integer(value: &Value) -> i64 {
    value.as_i64().or_else(|| value.as_f64().map(|v| v as i64)).or_else(|| value.as_str().and_then(|v| v.trim().parse().ok())).unwrap_or_default()
}

/// True or false; anything else is false.
fn boolean(value: &Value) -> bool {
    value.as_bool().unwrap_or(false)
}

/// Text: a number or a truth as written; nothing is empty.
fn text(value: &Value) -> String {
    match value {
        Value::String(v) => v.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
