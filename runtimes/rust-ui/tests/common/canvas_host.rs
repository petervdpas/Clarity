//! What the canvas tests share (C# CanvasPartHost): a window that runs a
//! canvas's commands as a generated program has them and logs each, whose
//! reads fail once it is told to, as a broken view model would. A canvas of
//! 100 by 100 units fills 400 by 400 pixels of an 800 by 400 screen, from
//! 200 across: unit x, y is pixel 200 + 4x, 4y.

#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use clarity_runtime::ui::PartState;
use clarity_ui::markup::HandlerCall;
use clarity_ui::parts::data::canvas::CanvasPart;
use clarity_ui::parts::{FieldFacts, PartHost};
use clarity_ui::testing::{Shot, TestHost};
use clarity_ui::{Element, Screen, ThemeLook};
use serde_json::{json, Map, Value};
use skia_safe::Color;

pub const UNITS: &str = r#"units="{&quot;width&quot;:100,&quot;height&quot;:100}""#;

#[derive(Default)]
pub struct CanvasHost {
    pub inner: TestHost,
    /// Each pointer event a handler ran for: the part, the event, where.
    pub points: RefCell<Vec<(String, String, Map<String, Value>)>>,
    /// Each action of a canvas with a mode.
    pub actions: RefCell<Vec<Value>>,
    /// Once set, reads, rows and resources fail, as a broken view model's would.
    pub failing: Cell<bool>,
}

impl CanvasHost {
    /// Draws `shapes` on the canvas's state, as its draw handler does.
    pub fn draw(&self, part: &str, shapes: Vec<Value>) {
        self.inner.set_state(part, |s| s.drawing = Some(shapes));
    }

    pub fn events(&self) -> Vec<String> {
        self.points.borrow().iter().map(|p| p.1.clone()).collect()
    }

    fn fail(&self, what: &str) {
        if self.failing.get() {
            panic!("{what}");
        }
    }
}

impl PartHost for CanvasHost {
    fn read(&self, path: &str) -> Option<Value> {
        self.fail("the value is gone");
        self.inner.read(path)
    }
    fn write(&self, part: &str, path: &str, value: Value) {
        self.inner.write(part, path, value)
    }
    fn edited(&self, part: &str) {
        self.inner.edited(part)
    }
    fn state(&self, name: Option<&str>) -> Option<PartState> {
        self.inner.state(name)
    }
    fn set_state(&self, name: &str, state: PartState) {
        clarity_ui::PartHost::set_state(&self.inner, name, state)
    }
    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.inner.run(part, call)
    }
    fn text(&self, written: Option<&str>) -> String {
        self.inner.text(written)
    }
    fn field(&self, path: &str) -> Option<FieldFacts> {
        self.inner.field(path)
    }
    fn changes(&self) -> u64 {
        self.inner.changes()
    }
    fn resource(&self, name: &str) -> Option<Vec<u8>> {
        self.fail("the picture store is gone");
        self.inner.resource(name)
    }
    fn pointed(&self, part: &str, event: &str, at: &Map<String, Value>) -> Option<bool> {
        self.points.borrow_mut().push((part.to_string(), event.to_string(), at.clone()));
        self.inner.set_state(part, |s| s.pointer = Some(at.clone()));
        Some(true)
    }
    fn selected(&self, part: &str) -> Option<usize> {
        self.inner.selected(part)
    }
    fn select(&self, part: &str, place: Option<usize>) {
        self.inner.select(part, place)
    }
    fn acted(&self, _part: &str, action: &Map<String, Value>) -> Option<bool> {
        self.actions.borrow_mut().push(Value::Object(action.clone()));
        Some(true)
    }
}

/// A part of `markup` as the whole of a screen 800 by 400, so its pixels are the screen's.
pub fn alone(markup: &str, host: Rc<dyn PartHost>) -> Screen {
    let spec = clarity_ui::markup::read(&format!("<Window name=\"Test\">{markup}</Window>")).expect("the markup reads");
    let part: Box<dyn Element> = clarity_ui::parts::make(spec.parts[0].clone(), host);
    let mut screen = Screen::new(part, ThemeLook::clarity(false));
    screen.resize(800.0, 400.0, 1.0);
    screen
}

/// A canvas named pad of 100 by 100 units with `attributes`, on a host of its own.
pub fn canvas(attributes: &str) -> (Rc<CanvasHost>, Screen) {
    canvas_on(attributes, Rc::new(CanvasHost::default()))
}

pub fn canvas_on(attributes: &str, host: Rc<CanvasHost>) -> (Rc<CanvasHost>, Screen) {
    let screen = alone(&format!(r#"<Canvas name="pad" {UNITS} {attributes} />"#), host.clone());
    (host, screen)
}

pub fn the_canvas(screen: &Screen) -> &CanvasPart {
    screen.root().as_any().downcast_ref::<CanvasPart>().expect("the screen is a canvas")
}

pub fn shot(screen: &mut Screen) -> Shot {
    Shot::of(screen, 800, 400, 1.0)
}

pub fn look() -> std::sync::Arc<ThemeLook> {
    ThemeLook::clarity(false)
}

/// Unit x, y as a pixel of the screen.
pub fn px(x: f64, y: f64) -> (i32, i32) {
    ((200.0 + 4.0 * x) as i32, (4.0 * y) as i32)
}

pub fn press(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, clarity_ui::PointerButton::Left, 1);
}

pub fn release(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_released(x, y, clarity_ui::PointerButton::Left);
}

pub fn rect(x: f64, y: f64, w: f64, h: f64, color: &str) -> Value {
    json!({"op": "rect", "x": x, "y": y, "width": w, "height": h, "color": color})
}

pub fn clear(color: &str) -> Value {
    json!({"op": "clear", "color": color})
}

pub fn object(id: Value, x: f64, y: f64, w: f64, h: f64) -> Value {
    json!({"op": "object", "record": id, "x": x, "y": y, "width": w, "height": h})
}

pub fn link(via: Value, width: f64) -> Value {
    json!({"op": "link", "name": "a-b", "x1": 10, "y1": 50, "x2": 90, "y2": 50, "via": via, "color": "text", "width": width})
}

/// A PNG of one color, `size` pixels square.
pub fn png(color: Color, size: i32) -> Vec<u8> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((size, size)).unwrap();
    surface.canvas().clear(color);
    surface.image_snapshot().encode(None, skia_safe::EncodedImageFormat::PNG, 100).unwrap().as_bytes().to_vec()
}

/// The text of a vector drawing with `adds` added, as the runtime keeps it.
pub fn drawn(adds: &[Value]) -> String {
    use clarity_runtime::canvas::{apply, CanvasArgs};
    let mut state = clarity_runtime::ui::WindowState::default();
    state.data.insert("drawing".into(), json!(""));
    let args = CanvasArgs { part: "pad".into(), mode: "vector".into(), path: "drawing".into(), units: Some([100, 100]), ..CanvasArgs::default() };
    for add in adds {
        let action = json!({ "add": add });
        apply(&mut state, &args, action.as_object().unwrap()).expect("the shape adds");
    }
    state.data["drawing"].as_str().unwrap_or_default().to_string()
}

/// How far apart two colours are, channel by channel.
pub fn distance(a: Color, b: Color) -> i32 {
    (a.r() as i32 - b.r() as i32).abs() + (a.g() as i32 - b.g() as i32).abs() + (a.b() as i32 - b.b() as i32).abs()
}

/// A pixel drawn in `color`, at least partly: nearer to it than to `behind`.
pub fn toward(pixel: Color, color: Color, behind: Color) -> bool {
    distance(pixel, color) < distance(pixel, behind)
}

/// A pixel mostly of `color`: a thin line's edge blends a little of what is under it.
pub fn near(pixel: Color, color: Color) -> bool {
    distance(pixel, color) < 120
}
