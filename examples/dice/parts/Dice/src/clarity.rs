//! Custom part Dice (DESIGN 5.19, 7.9): a control of Clarity.UI. The canvas
//! it draws on, what it does through Clarity.UI, the trait the crate's `Dice`
//! implements and what it says to its window, in plain Rust: the crate needs
//! nothing of Clarity's. Written by Clarity from the part's declaration, and
//! written again when the declaration changes; the rest of the crate is yours.
//!
//! Every method is called on the screen's thread, one at a time; the part
//! is made when its window opens and lives as long as its window.

#![allow(dead_code)]

pub use serde_json::Value;

/// What `draw` draws on, in the screen's units. Colors are 0xAARRGGBB.
pub trait Canvas {
    fn fill(&mut self, x: f64, y: f64, width: f64, height: f64, color: u32, radius: f64);

    fn stroke(&mut self, x: f64, y: f64, width: f64, height: f64, color: u32, line_width: f64, radius: f64);

    fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, color: u32, line_width: f64);

    fn oval(&mut self, x: f64, y: f64, width: f64, height: f64, color: u32, filled: bool, line_width: f64);

    /// An SVG path's data ("M0 0 L10 10 Z"), filled or stroked.
    fn path(&mut self, svg: &str, color: u32, filled: bool, line_width: f64);

    /// Text in a box: align 0 left, 1 center, 2 right; size 0 is the look's text size.
    fn text(&mut self, text: &str, x: f64, y: f64, width: f64, height: f64, color: u32, size: f64, bold: bool, align: i32);

    /// How wide text is at a size (0: the look's text size).
    fn measure(&mut self, text: &str, size: f64, bold: bool) -> f64;

    /// RGBA pixels (unpremultiplied, rows of `stride` bytes) drawn into a box.
    fn pixels(&mut self, rgba: &[u8], width: u32, height: u32, stride: usize, x: f64, y: f64, box_width: f64, box_height: f64);

    /// Clips what follows to a box until `restore`.
    fn clip(&mut self, x: f64, y: f64, width: f64, height: f64);

    fn restore(&mut self);

    /// A color of the look by its role ("accent", "text", "border", "error").
    fn look_color(&mut self, role: &str) -> u32;
}

/// What the part does through Clarity.UI, valid during the call it is given to.
pub trait Cx {
    /// Its place in the window: x, y, width and height.
    fn bounds(&self) -> (f64, f64, f64, f64);

    fn enabled(&self) -> bool;

    /// Has it drawn again.
    fn invalidate(&mut self);

    /// Has its `wake` called once after `ms` milliseconds.
    fn wake_in(&mut self, ms: f64);

    /// Has it drawn every frame while on.
    fn animate(&mut self, on: bool);

    fn focus(&mut self);

    /// The person changed the value it binds (see `changed` of its tells).
    fn change(&mut self, value: Value);

    /// It tells `tell` with its values (see its tells).
    fn tell(&mut self, tell: &str, values: Vec<Value>);

    /// A text of the program's translations by its key, in the language shown; `english` when there is none.
    fn translate(&self, key: &str, english: &str) -> String;
}

/// Keys held, as bits of `modifiers`.
pub const SHIFT: u32 = 1;
pub const CONTROL: u32 = 2;
pub const ALT: u32 = 4;
pub const META: u32 = 8;

/// Where the pointer is, in the screen's units, with the keys held and the clicks counted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pointer {
    pub x: f64,
    pub y: f64,
    pub modifiers: u32,
    pub clicks: u32,
}

/// What part Dice does: it draws itself in its place and shows what the
/// window gives it; the input methods it does not write do nothing.
pub trait DicePart: Default + 'static {
    /// Draws itself in its place, with the look's colors (`canvas.look_color`).
    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64);

    /// Property count, as the window or code sets it.
    fn show_count(&mut self, count: i64, cx: &mut dyn Cx);

    /// What code tells it: roll.
    fn on_roll(&mut self, cx: &mut dyn Cx);

    /// Whether it takes the focus: pressed, or reached with Tab.
    fn focusable(&self) -> bool {
        false
    }

    /// The main button went down on it.
    fn pointer_pressed(&mut self, at: Pointer, cx: &mut dyn Cx) {
        let _ = (at, cx);
    }

    /// The pointer moved over it, or anywhere while it holds the pointer.
    fn pointer_moved(&mut self, at: Pointer, cx: &mut dyn Cx) {
        let _ = (at, cx);
    }

    /// The main button went up after it was pressed on it.
    fn pointer_released(&mut self, at: Pointer, cx: &mut dyn Cx) {
        let _ = (at, cx);
    }

    /// Pressed and released with the main button.
    fn click(&mut self, cx: &mut dyn Cx) {
        let _ = cx;
    }

    /// The wheel turned over it with keys held (`SHIFT` and the others): true when it took the turn.
    fn wheel(&mut self, dx: f64, dy: f64, modifiers: u32, cx: &mut dyn Cx) -> bool {
        let _ = (dx, dy, modifiers, cx);
        false
    }

    /// A key by its name ("Enter", "Space", "Left", "A", "+") while it has the focus: true when it took the key.
    fn key(&mut self, name: &str, modifiers: u32, cx: &mut dyn Cx) -> bool {
        let _ = (name, modifiers, cx);
        false
    }

    /// Text typed while it has the focus.
    fn text(&mut self, text: &str, cx: &mut dyn Cx) {
        let _ = (text, cx);
    }

    /// It got or lost the focus.
    fn focus_changed(&mut self, focused: bool, cx: &mut dyn Cx) {
        let _ = (focused, cx);
    }

    /// The time it asked for came (`cx.wake_in(ms)`).
    fn wake(&mut self, cx: &mut dyn Cx) {
        let _ = cx;
    }

    /// Its window opened: called once, after the values of its line, what
    /// code set and what it binds; it may start a timer or an animation here.
    fn start(&mut self, cx: &mut dyn Cx) {
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

impl<C: Cx + ?Sized> DiceTells for C {
    fn landed(&mut self, total: i64, faces: &str) {
        self.tell("landed", vec![Value::from(total), Value::from(faces)]);
    }
}

/// A part as the program's glue holds it, whatever its kind.
pub trait Custom {
    fn set(&mut self, name: &str, value: &Value, cx: &mut dyn Cx);
    fn hear(&mut self, hear: &str, values: &[Value], cx: &mut dyn Cx);
    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64);
    fn focusable(&self) -> bool;
    fn pointer_pressed(&mut self, at: Pointer, cx: &mut dyn Cx);
    fn pointer_moved(&mut self, at: Pointer, cx: &mut dyn Cx);
    fn pointer_released(&mut self, at: Pointer, cx: &mut dyn Cx);
    fn click(&mut self, cx: &mut dyn Cx);
    fn wheel(&mut self, dx: f64, dy: f64, modifiers: u32, cx: &mut dyn Cx) -> bool;
    fn key(&mut self, name: &str, modifiers: u32, cx: &mut dyn Cx) -> bool;
    fn text(&mut self, text: &str, cx: &mut dyn Cx);
    fn focus_changed(&mut self, focused: bool, cx: &mut dyn Cx);
    fn wake(&mut self, cx: &mut dyn Cx);
    fn start(&mut self, cx: &mut dyn Cx);
    fn stop(&mut self);
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String>;
}

/// Part Dice as the glue holds it: the window's words in, typed.
struct Held<P>(P);

impl<P: DicePart> Custom for Held<P> {
    fn set(&mut self, name: &str, value: &Value, cx: &mut dyn Cx) {
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

    fn hear(&mut self, hear: &str, values: &[Value], cx: &mut dyn Cx) {
        // What it hears may take no values.
        let _ = values;
        match hear {
            "roll" => self.0.on_roll(cx),
            _ => {}
        }
    }

    fn draw(&self, canvas: &mut dyn Canvas, x: f64, y: f64, width: f64, height: f64) {
        self.0.draw(canvas, x, y, width, height);
    }

    fn focusable(&self) -> bool {
        self.0.focusable()
    }

    fn pointer_pressed(&mut self, at: Pointer, cx: &mut dyn Cx) {
        self.0.pointer_pressed(at, cx)
    }

    fn pointer_moved(&mut self, at: Pointer, cx: &mut dyn Cx) {
        self.0.pointer_moved(at, cx)
    }

    fn pointer_released(&mut self, at: Pointer, cx: &mut dyn Cx) {
        self.0.pointer_released(at, cx)
    }

    fn click(&mut self, cx: &mut dyn Cx) {
        self.0.click(cx)
    }

    fn wheel(&mut self, dx: f64, dy: f64, modifiers: u32, cx: &mut dyn Cx) -> bool {
        self.0.wheel(dx, dy, modifiers, cx)
    }

    fn key(&mut self, name: &str, modifiers: u32, cx: &mut dyn Cx) -> bool {
        self.0.key(name, modifiers, cx)
    }

    fn text(&mut self, text: &str, cx: &mut dyn Cx) {
        self.0.text(text, cx)
    }

    fn focus_changed(&mut self, focused: bool, cx: &mut dyn Cx) {
        self.0.focus_changed(focused, cx)
    }

    fn wake(&mut self, cx: &mut dyn Cx) {
        self.0.wake(cx)
    }

    fn start(&mut self, cx: &mut dyn Cx) {
        self.0.start(cx)
    }

    fn stop(&mut self) {
        self.0.stop()
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        self.0.tooltip_at(x, y)
    }
}

/// A new part Dice, as the program registers it with Clarity.UI.
pub fn make() -> Box<dyn Custom> {
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
