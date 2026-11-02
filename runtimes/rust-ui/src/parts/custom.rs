//! Custom parts (DESIGN 5.19, 7.9): a control of Clarity.UI that a crate of
//! the program brings, made the way the built-in parts are made. It draws
//! itself in its place, takes the pointer, the wheel, keys and the focus
//! like any element, and speaks with its window in the part's declared
//! words: the value it binds and its properties come in (`set`), what code
//! tells it comes in (`hear`), the person's change of its value goes out
//! (`CustomCx::change`, which runs `on change`) and so do its own events
//! (`CustomCx::tell`, which runs the handler wired to the tell).
//!
//! A crate's type implements `CustomPart` and is registered by the name its
//! declaration gives it, in the generated `src/parts.rs`:
//!
//! ```ignore
//! pub fn register() {
//!     clarity_ui::parts::custom::register("Gauge", || Box::new(gauge::Gauge::default()));
//! }
//! ```
//!
//! The window's `<Custom name="speedo" part="Gauge" bind="speed">` is a place
//! part (`CustomPlace`) that holds the registered part, or says in its place
//! that the program carries no part of that name; the rest of the window
//! goes on.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::Instant;

use serde_json::{Map, Value};

use crate::element::{Cx, Element, ElementId};
use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::input::{KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::painter::{Painter, TextAlign};
use crate::parts::{Part, PartBase, PartHost};
use crate::theme::ThemeLook;
use crate::markup::PartSpec;

/// What a custom part reaches of its window while it handles something:
/// its place, the look, and the words it says to the window.
pub struct CustomCx<'a> {
    base: &'a PartBase,
    id: ElementId,
    cx: &'a mut Cx,
    /// The value it binds as the window holds it now, once it changed it.
    changed: Option<Value>,
}

impl<'a> CustomCx<'a> {
    /// Where it is on the screen.
    pub fn bounds(&self) -> Rect {
        self.base.node.bounds
    }

    pub fn look(&self) -> &ThemeLook {
        self.cx.look()
    }

    pub fn fonts(&self) -> &Fonts {
        self.cx.fonts()
    }

    /// The time now, by the screen's clock.
    pub fn now(&self) -> Instant {
        self.cx.now()
    }

    /// Its name in its window: `speedo`.
    pub fn name(&self) -> Option<&str> {
        self.base.name()
    }

    /// Whether the person may use it.
    pub fn enabled(&self) -> bool {
        self.base.enabled()
    }

    /// It changed: the screen draws it again.
    pub fn invalidate(&mut self) {
        self.cx.invalidate();
    }

    /// It wakes at `at` (its `wake` runs then), once.
    pub fn wake(&mut self, at: Instant) {
        self.cx.wake(self.id, at);
    }

    /// It animates (or no longer does): the screen draws every frame while it does.
    pub fn animate(&mut self, on: bool) {
        self.cx.animate(self.id, on);
    }

    /// It takes the focus.
    pub fn focus(&mut self) {
        self.cx.focus(Some(self.id));
    }

    /// The person changed its value: what it binds takes it (one edit, which Ctrl+Z takes back) and `on change` runs.
    pub fn change(&mut self, value: Value) {
        if let Some(bound) = self.base.bound() {
            self.base.host.write(self.base.name().unwrap_or_default(), bound, value.clone());
            self.changed = Some(value);
        }
        self.base.fire("change");
        self.cx.invalidate();
    }

    /// It tells `tell` with its values: the handler wired to it on its line runs with them; a tell nothing is wired to is let go.
    pub fn tell(&mut self, tell: &str, values: Vec<Value>) {
        let Some(name) = self.base.name() else { return };
        if wired(&self.base.spec, tell) {
            self.base.host.told(name, tell, &values);
        }
    }
}

/// Whether a handler is wired to `tell` on the part's line: `tells='[{"tell":"reached","handler":"..."}]'`.
fn wired(spec: &PartSpec, tell: &str) -> bool {
    let wired = match spec.json("tells") {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::String(text)) => serde_json::from_str::<Vec<Value>>(text).unwrap_or_default(),
        _ => Vec::new(),
    };
    wired.iter().any(|w| w.get("tell").and_then(Value::as_str) == Some(tell))
}

/// The values written on a part's line (`values` in its markup), each
/// read by its kind as the web screen and C# read them: a number, true or
/// false, else the text as written.
fn line_values(spec: &PartSpec) -> Vec<(String, Value)> {
    let values = match spec.json("values") {
        Some(Value::Object(map)) => map.clone(),
        Some(Value::String(text)) => serde_json::from_str::<serde_json::Map<String, Value>>(text).unwrap_or_default(),
        _ => serde_json::Map::new(),
    };
    values
        .into_iter()
        .map(|(name, written)| {
            let text = written.get("value").map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).unwrap_or_default();
            let value = match written.get("kind").and_then(Value::as_str) {
                Some("number" | "integer" | "decimal") => text.parse::<i64>().map(Value::from).or_else(|_| text.parse::<f64>().map(Value::from)).unwrap_or(Value::Null),
                Some("bool") => Value::Bool(text == "true"),
                _ => Value::from(text),
            };
            (name, value)
        })
        .collect()
}

/// A custom part: what a crate of the program implements. Every method but
/// `set`, `hear` and `draw` has a default that does nothing.
pub trait CustomPart: std::any::Any {
    /// A property as the window or code sets it: `value` for what it binds,
    /// `enabled`, or one its declaration names. Given when its window starts
    /// and again when it changed.
    fn set(&mut self, name: &str, value: &Value, cx: &mut CustomCx);

    /// What code told it, with its values: `gauge.reset(1)`.
    fn hear(&mut self, hear: &str, values: &[Value], cx: &mut CustomCx);

    /// Draws itself in `area`, its place in the window.
    fn draw(&self, painter: &Painter, area: Rect);

    /// Its window opened: called once, after the values of its line, what
    /// code set and what it binds were given; it may start a timer or an
    /// animation of its own here.
    fn start(&mut self, cx: &mut CustomCx) {
        let _ = cx;
    }

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

    /// The time it asked for came (`CustomCx::wake`).
    fn wake(&mut self, cx: &mut CustomCx) {
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

/// Makes a new part of a kind a crate registered.
pub type MakeCustom = Box<dyn Fn() -> Box<dyn CustomPart> + Send + Sync>;

static REGISTERED: Mutex<Option<HashMap<String, MakeCustom>>> = Mutex::new(None);

/// Adds a custom part of `kind` (its declaration's name, `Gauge`), as the generated `src/parts.rs` does.
pub fn register(kind: &str, make: impl Fn() -> Box<dyn CustomPart> + Send + Sync + 'static) {
    let mut registered = REGISTERED.lock().unwrap_or_else(|p| p.into_inner());
    registered.get_or_insert_with(HashMap::new).insert(kind.to_string(), Box::new(make));
}

/// A new part of `kind`; none when the program registered none of that name.
pub fn make_custom(kind: &str) -> Option<Box<dyn CustomPart>> {
    let registered = REGISTERED.lock().unwrap_or_else(|p| p.into_inner());
    registered.as_ref().and_then(|r| r.get(kind)).map(|make| make())
}

/// The place of a custom part in its window: it holds the part the
/// program's crate brings, or says in its place that there is none.
pub struct CustomPlace {
    base: PartBase,
    inner: Option<Box<dyn CustomPart>>,
    /// Why it holds no part: none registered of its name, or one that failed.
    pub why: Option<String>,
    started: bool,
    shown_value: Option<Option<Value>>,
    shown_enabled: Option<bool>,
    shown_values: Map<String, Value>,
}

/// Makes the place of a `<Custom>` part.
pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    let kind = spec.text("part").unwrap_or_default().to_string();
    let made = std::panic::catch_unwind(|| make_custom(&kind));
    let (inner, why) = match made {
        Ok(Some(part)) => (Some(part), None),
        Ok(None) => (None, Some(locale::fill(&locale::translate("customPartMissing", "the program carries no part {part}"), "part", &kind))),
        Err(panic) => (None, Some(panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default())),
    };
    Box::new(CustomPlace { base: PartBase::new(spec, host), inner, why, started: false, shown_value: None, shown_enabled: None, shown_values: Map::new() })
}

impl CustomPlace {
    /// The part it holds, as its own type; none when it holds none of that type.
    pub fn inner<T: CustomPart>(&self) -> Option<&T> {
        self.inner.as_ref().and_then(|p| (p.as_ref() as &dyn std::any::Any).downcast_ref::<T>())
    }

    /// Runs `act` on the part it holds with its context; none when it holds none.
    fn with<R>(&mut self, cx: &mut Cx, act: impl FnOnce(&mut dyn CustomPart, &mut CustomCx) -> R) -> Option<R> {
        let id = self.base.node.id();
        let inner = self.inner.as_mut()?;
        let mut ccx = CustomCx { base: &self.base, id, cx, changed: None };
        let result = act(inner.as_mut(), &mut ccx);
        if let Some(value) = ccx.changed {
            self.shown_value = Some(Some(value));
        }
        Some(result)
    }

    /// Gives the part what changed since it last heard: on the first time
    /// the values written on its line and set by code, then what it binds
    /// and whether it is enabled, then what code told it.
    fn sync(&mut self, cx: &mut Cx) {
        if self.inner.is_none() {
            return;
        }
        // The first time, the values written on its line, before what code set.
        if !self.started {
            for (name, value) in line_values(&self.base.spec) {
                self.with(cx, |part, ccx| part.set(&name, &value, ccx));
            }
        }
        let state = self.base.state();
        let values = state.as_ref().and_then(|s| s.values.clone()).unwrap_or_default();
        let mut changed: Vec<(String, Value)> = values.iter().filter(|(k, v)| self.shown_values.get(*k) != Some(*v)).map(|(k, v)| (k.clone(), v.clone())).collect();
        // A value code no longer sets (an undo took it back) goes back to the one on its line.
        let line = line_values(&self.base.spec);
        for name in self.shown_values.keys().filter(|k| !values.contains_key(*k)) {
            let written = line.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()).unwrap_or(Value::Null);
            changed.push((name.clone(), written));
        }
        self.shown_values = values;
        for (name, value) in changed {
            self.with(cx, |part, ccx| part.set(&name, &value, ccx));
        }
        if let Some(bound) = self.base.bound().map(str::to_string) {
            let value = self.base.host.read(&bound);
            if self.shown_value.as_ref() != Some(&value) {
                self.shown_value = Some(value.clone());
                let value = value.unwrap_or(Value::Null);
                self.with(cx, |part, ccx| part.set("value", &value, ccx));
            }
        }
        let enabled = self.base.enabled();
        if self.shown_enabled != Some(enabled) {
            self.shown_enabled = Some(enabled);
            self.with(cx, |part, ccx| part.set("enabled", &Value::Bool(enabled), ccx));
        }
        if let Some(name) = self.base.name().map(str::to_string) {
            for hear in self.base.host.hears(&name) {
                self.with(cx, |part, ccx| part.hear(&hear.hear, &hear.values, ccx));
            }
        }
        if !self.started {
            self.started = true;
            self.with(cx, |part, ccx| part.start(ccx));
        }
    }
}

impl Element for CustomPlace {
    crate::part_element!();

    fn focusable(&self) -> bool {
        self.base.enabled() && self.inner.as_ref().is_some_and(|p| p.focusable())
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.sync(cx);
    }

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        match &self.inner {
            Some(part) => {
                // A part that fails to draw says why in its place, and the rest of the window goes on.
                let drawn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _clip = p.clip(bounds);
                    part.draw(p, bounds);
                }));
                if drawn.is_err() {
                    let why = locale::translate("customPartFailed", "this part could not draw");
                    p.text(&why, bounds.inset(6.0), p.look.color("error"), p.look.size("small"), "text", false, TextAlign::Left);
                }
            }
            None => {
                p.stroke(bounds, p.look.color("error"), 1.0, p.look.radius("input"));
                let why = self.why.clone().unwrap_or_default();
                p.text(&why, bounds.inset(6.0), p.look.color("error"), p.look.size("small"), "text", false, TextAlign::Left);
            }
        }
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        if self.base.enabled() {
            self.with(cx, |part, ccx| part.pointer_pressed(at, ccx));
        }
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.with(cx, |part, ccx| part.pointer_moved(at, ccx));
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        self.with(cx, |part, ccx| part.pointer_released(at, ccx));
    }

    fn on_click(&mut self, cx: &mut Cx) {
        if self.base.enabled() {
            self.with(cx, |part, ccx| part.click(ccx));
        }
    }

    fn on_wheel(&mut self, dx: f64, dy: f64, held: Modifiers, cx: &mut Cx) -> bool {
        self.base.enabled() && self.with(cx, |part, ccx| part.wheel(dx, dy, held, ccx)).unwrap_or(false)
    }

    fn on_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        self.base.enabled() && self.with(cx, |part, ccx| part.key(key, ccx)).unwrap_or(false)
    }

    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        if self.base.enabled() {
            self.with(cx, |part, ccx| part.text(text, ccx));
        }
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        self.with(cx, |part, ccx| part.focus_changed(focused, ccx));
    }

    fn on_wake(&mut self, cx: &mut Cx) {
        self.with(cx, |part, ccx| part.wake(ccx));
    }

    fn on_detached(&mut self, _cx: &mut Cx) {
        if let Some(part) = self.inner.as_mut() {
            part.stop();
        }
    }

    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        self.inner.as_ref().and_then(|p| p.tooltip_at(x, y))
    }
}

impl Part for CustomPlace {
    fn base(&self) -> &PartBase {
        &self.base
    }

    /// It grows when its declaration says `grows`, which its markup carries.
    fn grows(&self) -> bool {
        self.base.spec.flag("grows")
    }
}
