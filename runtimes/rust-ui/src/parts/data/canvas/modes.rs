//! A canvas with a mode (DESIGN 5.31, conformance/canvas-modes.json): what
//! the person does on it. The canvas shows the gesture and sends what was
//! done as one action (a JSON object, `PartHost::acted`); the runtime decides
//! what it does, snaps it and keeps it (`clarity_runtime::canvas::apply`),
//! and the canvas shows the data that comes back.
//!   objects: an `object` shape of the drawing marks a record's box. A press
//!     in it and a drag moves the box and the shapes drawn after it, until
//!     the next object, as a preview; the drop sends {"move": {id, x, y}}.
//!   paint: strokes of the brush, its brush's units wide in its ink, on the
//!     picture of the bound image field; a stroke's end sends
//!     {"stroke": {"data": PNG}}.
//!   vector: the shapes of the bound text field's SVG, with a toolbar of the
//!     mode's tools. Drawing a shape adds it, select and drag moves the
//!     chosen one, Delete deletes it, and the text tool takes its text where
//!     it was pressed until Enter places it.

use std::cell::RefCell;

use clarity_runtime::picture::Picture;
use serde_json::{json, Map, Value};
use skia_safe::{surfaces, AlphaType, BlendMode, Color, ColorType, EncodedImageFormat, Image, ImageInfo, Paint, PaintCap, PaintJoin, PaintStyle, PathEffect, Point, SamplingOptions};

use super::{color_of, draw_shape, polyline, CanvasPart, Shape, Viewport, STILL};
use crate::element::Cx;
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::painter::{Painter, TextAlign};

/// The tools of each mode, in the order its toolbar shows them.
pub const PAINT_TOOLS: [&str; 2] = ["pen", "eraser"];
pub const VECTOR_TOOLS: [&str; 6] = ["select", "line", "rect", "ellipse", "path", "text"];

const TOOL_HEIGHT: f64 = 24.0;

/// A tool's name in the person's language.
pub fn tool_word(tool: &str) -> String {
    match tool {
        "select" => locale::translate("toolSelect", "Select"),
        "line" => locale::translate("toolLine", "Line"),
        "rect" => locale::translate("toolRect", "Rectangle"),
        "ellipse" => locale::translate("toolEllipse", "Ellipse"),
        "path" => locale::translate("toolPath", "Path"),
        "text" => locale::translate("toolText", "Text"),
        "pen" => locale::translate("toolPen", "Pen"),
        "eraser" => locale::translate("toolEraser", "Eraser"),
        other => other.to_string(),
    }
}

/// An object being dragged: the place of its box in the drawing, where it was pressed, where its box was and how far it moved (units).
#[derive(Debug, Clone, Copy)]
struct Dragging {
    place: usize,
    at: (f64, f64),
    from: (f64, f64),
    by: (f64, f64),
    moved: bool,
}

/// A shape being drawn or moved: where it was pressed, where it is now (units), the points it went through, whether it moved.
#[derive(Debug, Clone)]
struct Sketching {
    from: (f64, f64),
    to: (f64, f64),
    points: Vec<(f64, f64)>,
    moved: bool,
}

/// What a canvas keeps of what the person does on it with a mode.
#[derive(Default)]
pub(super) struct ModeState {
    dragging: Option<Dragging>,
    /// The record of each place of the drawing a drag started on, to find it again when the drawing is drawn anew.
    shown_objects: Vec<Value>,
    /// The picture painted on, and the field's value it was read from (or the last stroke left).
    painted: RefCell<Option<(Option<Value>, Option<Image>)>>,
    /// The stroke being painted, in units.
    stroke: Option<Vec<(f64, f64)>>,
    /// The drawing's shapes as the runtime reads its SVG, read once for each text.
    vector: RefCell<Option<(String, Vec<Map<String, Value>>)>>,
    sketching: Option<Sketching>,
    /// Where a text is being typed, in units, and what was typed so far.
    typing: Option<((f64, f64), String)>,
    /// The shape chosen of a canvas without a name, its place in the drawing from 0; a named one's is the window's.
    chosen: Option<usize>,
}

fn n(shape: &Map<String, Value>, name: &str) -> f64 {
    super::number_of(shape.get(name)).unwrap_or(0.0)
}

fn s<'a>(shape: &'a Map<String, Value>, name: &str, otherwise: &'a str) -> &'a str {
    shape.get(name).and_then(Value::as_str).unwrap_or(otherwise)
}

fn as_value(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        Value::from(n as i64)
    } else {
        Value::from(n)
    }
}

/// A vector shape as the canvas draws it; none for a line of SVG the runtime does not read.
pub fn shape_of(shape: &Map<String, Value>) -> Option<Shape> {
    let color = s(shape, "color", "text").to_string();
    Some(match s(shape, "op", "") {
        "rect" => Shape::Rect { x: n(shape, "x"), y: n(shape, "y"), width: n(shape, "width"), height: n(shape, "height"), color },
        "line" => Shape::Line { x1: n(shape, "x1"), y1: n(shape, "y1"), x2: n(shape, "x2"), y2: n(shape, "y2"), color, width: n(shape, "width") },
        "ellipse" => Shape::Ellipse { x: n(shape, "x"), y: n(shape, "y"), rx: n(shape, "rx"), ry: n(shape, "ry"), color },
        "path" => Shape::Path { points: super::wholes(shape.get("points")), color, width: n(shape, "width") },
        "text" => Shape::Words { x: n(shape, "x"), y: n(shape, "y"), text: s(shape, "text", "").to_string(), color, size: n(shape, "size"), align: "center".into() },
        _ => return None,
    })
}

fn between(x1: f64, y1: f64, x2: f64, y2: f64, grow: f64) -> Rect {
    Rect::new(x1.min(x2) - grow, y1.min(y2) - grow, (x2 - x1).abs() + 2.0 * grow, (y2 - y1).abs() + 2.0 * grow)
}

/// The box a vector shape takes, in units, for choosing it.
pub fn box_of(shape: &Map<String, Value>) -> Option<Rect> {
    match s(shape, "op", "") {
        "rect" => Some(Rect::new(n(shape, "x"), n(shape, "y"), n(shape, "width"), n(shape, "height"))),
        "ellipse" => Some(Rect::new(n(shape, "x") - n(shape, "rx"), n(shape, "y") - n(shape, "ry"), 2.0 * n(shape, "rx"), 2.0 * n(shape, "ry"))),
        "line" => Some(between(n(shape, "x1"), n(shape, "y1"), n(shape, "x2"), n(shape, "y2"), n(shape, "width") / 2.0)),
        "path" => {
            let points = super::wholes(shape.get("points"));
            if points.len() < 2 {
                return None;
            }
            let xs = points.iter().step_by(2);
            let ys = points.iter().skip(1).step_by(2);
            let (x1, x2) = xs.fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
            let (y1, y2) = ys.fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
            Some(between(x1, y1, x2, y2, n(shape, "width") / 2.0))
        }
        "text" => {
            let size = n(shape, "size");
            let length = s(shape, "text", "").chars().count() as f64;
            Some(Rect::new(n(shape, "x") - size * length * 0.3, n(shape, "y") - size * 0.6, size * length * 0.6, size * 1.2))
        }
        _ => None,
    }
}

/// A shape's first place, by which it moves.
pub fn first_place(shape: &Map<String, Value>) -> (f64, f64) {
    match s(shape, "op", "") {
        "line" => (n(shape, "x1"), n(shape, "y1")),
        "path" => {
            let points = super::wholes(shape.get("points"));
            if points.len() >= 2 {
                (points[0], points[1])
            } else {
                (n(shape, "x"), n(shape, "y"))
            }
        }
        _ => (n(shape, "x"), n(shape, "y")),
    }
}

/// The drawing with the object at `place` and what is drawn after it, until the next object, moved.
pub fn with_object_moved(shapes: Vec<Shape>, place: usize, dx: f64, dy: f64) -> Vec<Shape> {
    let mut moving = false;
    shapes
        .into_iter()
        .enumerate()
        .map(|(i, shape)| {
            if matches!(shape, Shape::Object { .. }) {
                moving = i == place;
            }
            if moving {
                shape.translated(dx, dy)
            } else {
                shape
            }
        })
        .collect()
}

/// The picture after a stroke through `points` (in the picture's pixels)
/// with `tool`, as a PNG: the pen paints with `ink`, the eraser takes the
/// paint away.
pub fn painted_with(before: Option<&Image>, width: i32, height: i32, points: &[(f64, f64)], tool: &str, ink: Color, brush: f64) -> Vec<u8> {
    let info = ImageInfo::new((width, height), ColorType::RGBA8888, AlphaType::Premul, None);
    let Some(mut surface) = surfaces::raster(&info, None, None) else { return Vec::new() };
    let canvas = surface.canvas();
    canvas.clear(Color::TRANSPARENT);
    if let Some(before) = before {
        let place = skia_safe::Rect::from_wh(width as f32, height as f32);
        canvas.draw_image_rect_with_sampling_options(before, None, place, SamplingOptions::default(), &Paint::default());
    }
    let mut pen = Paint::default();
    pen.set_color(ink);
    pen.set_anti_alias(true);
    pen.set_style(PaintStyle::Stroke);
    pen.set_stroke_width(brush.max(1.0) as f32);
    pen.set_stroke_cap(PaintCap::Round);
    pen.set_stroke_join(PaintJoin::Round);
    pen.set_blend_mode(if tool == "eraser" { BlendMode::Clear } else { BlendMode::SrcOver });
    let pixels: Vec<Point> = points.iter().map(|(x, y)| Point::new(*x as f32, *y as f32)).collect();
    polyline(canvas, &pixels, &pen);
    surface.image_snapshot().encode(None, EncodedImageFormat::PNG, None).map(|d| d.as_bytes().to_vec()).unwrap_or_default()
}

impl CanvasPart {
    /// What the person does on it: objects, paint or vector; empty: its draw handler draws it.
    pub fn mode(&self) -> String {
        self.base.spec.json("mode").and_then(Value::as_str).unwrap_or("").to_string()
    }

    /// Whether the person acts on it: it has a mode, may be used and changed.
    pub(super) fn acts(&self) -> bool {
        matches!(self.mode().as_str(), "objects" | "paint" | "vector") && !self.base.read_only() && self.base.enabled()
    }

    /// The tools its toolbar shows: the canvas's own, of its mode, in the mode's order; all of them when it names none.
    pub fn tools_shown(&self) -> Vec<&'static str> {
        let all: &[&'static str] = match self.mode().as_str() {
            "paint" => &PAINT_TOOLS,
            "vector" => &VECTOR_TOOLS,
            _ => return Vec::new(),
        };
        let named = self.base.spec.names("tools");
        all.iter().copied().filter(|t| named.is_empty() || named.iter().any(|n| n == t)).collect()
    }

    /// The tool in hand: the one its state names, when the canvas has it, else its first.
    pub fn tool_in_hand(&self) -> &'static str {
        let tools = self.tools_shown();
        let named = self.base.state().and_then(|s| s.tool).unwrap_or_default();
        tools.iter().copied().find(|t| *t == named).or_else(|| tools.first().copied()).unwrap_or("")
    }

    /// How wide it paints or draws, in its units.
    pub fn brush(&self) -> f64 {
        self.base.state().and_then(|s| s.brush).map(|b| b as f64).or_else(|| self.base.spec.number("brush")).unwrap_or(2.0)
    }

    /// The color it paints or draws with: the theme's by name, or written out.
    pub fn ink(&self) -> String {
        self.base.state().and_then(|s| s.ink).filter(|i| !i.is_empty()).or_else(|| self.base.spec.text("ink").map(str::to_string)).unwrap_or_else(|| "text".to_string())
    }

    /// The shape chosen, its place in the drawing from 0, as the window keeps it (`selected`, as a table's chosen row); none for none.
    pub fn chosen(&self) -> Option<usize> {
        match self.base.name() {
            Some(name) => self.base.host.selected(name),
            None => self.modes.chosen,
        }
    }

    fn choose(&mut self, place: Option<usize>) {
        match self.base.name() {
            Some(name) => self.base.host.select(name, place),
            None => self.modes.chosen = place,
        }
    }

    /// The text typed with the text tool so far; none while no text is being typed.
    pub fn typing(&self) -> Option<&str> {
        self.modes.typing.as_ref().map(|(_, t)| t.as_str())
    }

    fn act(&self, action: Value) {
        let Value::Object(action) = action else { return };
        if let Some(name) = self.base.name().filter(|_| self.acts()) {
            self.base.host.acted(name, &action);
        }
    }

    // -- The toolbar -------------------------------------------------------------

    /// Its toolbar's buttons on the screen, in the order of its tools.
    pub fn tool_buttons(&self, cx_fonts: &crate::fonts::Fonts, look: &crate::theme::ThemeLook) -> Vec<(&'static str, Rect)> {
        let b = self.base.node.bounds;
        let mut x = b.x + 6.0;
        self.tools_shown()
            .into_iter()
            .map(|tool| {
                let width = cx_fonts.measure(&tool_word(tool), "text", look.size("small"), false) + 16.0;
                let place = Rect::new(x, b.y + 6.0, width, TOOL_HEIGHT);
                x += width + 2.0;
                (tool, place)
            })
            .collect()
    }

    fn draw_toolbar(&self, p: &Painter) {
        let look = p.look;
        let in_hand = self.tool_in_hand();
        for (tool, place) in self.tool_buttons(p.fonts, look) {
            let chosen = tool == in_hand;
            p.fill(place, look.color(if chosen { "accent" } else { "panel" }), look.radius("button"));
            p.stroke(place, look.color("border"), look.border(), look.radius("button"));
            p.text(&tool_word(tool), place, look.color(if chosen { "on_accent" } else { "text" }), look.size("small"), "text", false, TextAlign::Center);
        }
    }

    // -- objects -----------------------------------------------------------------

    fn object_records(&self) -> Vec<Value> {
        self.drawing().into_iter().map(|s| if let Shape::Object { record, .. } = s { record } else { Value::Null }).collect()
    }

    /// The topmost object whose box holds a unit.
    fn object_at(&self, p: (f64, f64)) -> Option<usize> {
        self.drawing().iter().enumerate().rev().find_map(|(i, s)| match s {
            Shape::Object { x, y, width, height, .. } if p.0 >= *x && p.0 <= x + width && p.1 >= *y && p.1 <= y + height => Some(i),
            _ => None,
        })
    }

    /// A drawing drawn anew (after any handler, a timer's tick too) keeps a drag going, on the same record's object in it.
    pub(super) fn drawing_changed(&mut self) {
        let Some(held) = self.modes.dragging else { return };
        let record = self.modes.shown_objects.get(held.place).cloned().unwrap_or(Value::Null);
        let place = self.drawing().iter().position(|s| matches!(s, Shape::Object { record: r, .. } if *r == record));
        self.modes.shown_objects = self.object_records();
        self.modes.dragging = place.map(|place| Dragging { place, ..held });
    }

    /// The drawing as it shows: an object being dragged moved with what is drawn after it.
    pub(super) fn mode_shapes(&self, shapes: Vec<Shape>) -> Vec<Shape> {
        match self.modes.dragging {
            Some(d) if d.moved => with_object_moved(shapes, d.place, d.by.0, d.by.1),
            _ => shapes,
        }
    }

    // -- paint -------------------------------------------------------------------

    /// The picture of the bound image field, read again when the field holds another.
    fn painted(&self) -> Option<Image> {
        let value = self.base.bound().and_then(|path| self.base.host.read(path));
        if let Some((from, image)) = self.modes.painted.borrow().as_ref() {
            if *from == value {
                return image.clone();
            }
        }
        // Fail safe: a picture that does not load leaves the canvas blank to paint on.
        let bytes = match value.clone().map(serde_json::from_value::<Picture>) {
            Some(Ok(Picture::New(bytes))) => Some(bytes),
            Some(Ok(Picture::Stored)) => self.base.bound().and_then(|bind| self.base.host.fetch(bind, None)),
            _ => None,
        };
        let image = bytes.and_then(|b| Image::from_encoded(skia_safe::Data::new_copy(&b)));
        *self.modes.painted.borrow_mut() = Some((value, image.clone()));
        image
    }

    /// The size of the picture painted on: the picture's own, else the canvas's units.
    fn picture_size(&self) -> (i32, i32) {
        match self.painted() {
            Some(p) => (p.width(), p.height()),
            None => {
                let (uw, uh) = self.units();
                ((uw.round() as i32).max(1), (uh.round() as i32).max(1))
            }
        }
    }

    fn end_stroke(&mut self, points: Vec<(f64, f64)>, cx: &mut Cx) {
        let (width, height) = self.picture_size();
        let (uw, uh) = self.units();
        // The units in the picture's pixels: the same when it is the canvas's size.
        let sx = width as f64 / uw.max(1.0);
        let sy = height as f64 / uh.max(1.0);
        let scaled: Vec<(f64, f64)> = points.iter().map(|(x, y)| (x * sx, y * sy)).collect();
        let bytes = painted_with(self.painted().as_ref(), width, height, &scaled, self.tool_in_hand(), color_of(cx.look(), &self.ink()), self.brush() * sx);
        // Shown as painted at once; the field's picture follows from the runtime.
        let value = self.base.bound().and_then(|path| self.base.host.read(path));
        *self.modes.painted.borrow_mut() = Some((value, Image::from_encoded(skia_safe::Data::new_copy(&bytes))));
        cx.invalidate();
        let stroke = serde_json::to_value(Picture::New(bytes)).unwrap_or(Value::Null);
        self.act(json!({ "stroke": stroke }));
    }

    // -- vector ------------------------------------------------------------------

    /// The shapes of the bound drawing, as the runtime reads its SVG.
    pub fn vector_shapes(&self) -> Vec<Map<String, Value>> {
        let svg = self.base.bound().and_then(|path| self.base.host.read(path)).and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
        if let Some((kept, shapes)) = self.modes.vector.borrow().as_ref() {
            if *kept == svg {
                return shapes.clone();
            }
        }
        let (uw, uh) = self.units();
        let (_, shapes) = clarity_runtime::canvas::drawing_shapes(&svg, [uw as i64, uh as i64]);
        *self.modes.vector.borrow_mut() = Some((svg, shapes.clone()));
        shapes
    }

    /// The topmost shape whose box holds a unit, a few pixels around it too.
    fn vector_shape_at(&self, p: (f64, f64)) -> Option<usize> {
        let near = 4.0 / self.viewport().scale.max(1e-6);
        self.vector_shapes().iter().enumerate().rev().find_map(|(i, shape)| {
            let b = box_of(shape)?;
            (p.0 >= b.x - near && p.0 <= b.right() + near && p.1 >= b.y - near && p.1 <= b.bottom() + near).then_some(i)
        })
    }

    /// The shape `tool` draws from one unit to another, in the canvas's ink and brush; none when it has no size.
    pub fn sketched(&self, tool: &str, from: (f64, f64), to: (f64, f64), points: &[(f64, f64)]) -> Option<Value> {
        let (left, top) = (from.0.min(to.0), from.1.min(to.1));
        let (w, h) = ((to.0 - from.0).abs(), (to.1 - from.1).abs());
        if tool != "path" && w == 0.0 && h == 0.0 {
            return None;
        }
        let (ink, brush) = (self.ink(), as_value(self.brush()));
        let v = as_value;
        Some(match tool {
            "line" => json!({"op": "line", "x1": v(from.0), "y1": v(from.1), "x2": v(to.0), "y2": v(to.1), "color": ink, "width": brush}),
            "rect" => json!({"op": "rect", "x": v(left), "y": v(top), "width": v(w), "height": v(h), "color": ink}),
            "ellipse" => json!({"op": "ellipse", "x": v(left + w / 2.0), "y": v(top + h / 2.0), "rx": v(w / 2.0), "ry": v(h / 2.0), "color": ink}),
            "path" if points.len() >= 2 => {
                let all: Vec<Value> = points.iter().flat_map(|p| [v(p.0), v(p.1)]).collect();
                json!({"op": "path", "points": all, "color": ink, "width": brush})
            }
            _ => return None,
        })
    }

    fn start_typing(&mut self, unit: (f64, f64), cx: &mut Cx) {
        self.end_typing(true, cx);
        self.modes.typing = Some((unit, String::new()));
        cx.invalidate();
    }

    /// Places the text typed (`place`) or drops it.
    pub fn end_typing(&mut self, place: bool, cx: &mut Cx) {
        let Some((at, text)) = self.modes.typing.take() else { return };
        cx.invalidate();
        if place && !text.trim().is_empty() {
            let size = as_value((self.brush() * 4.0).max(10.0));
            self.act(json!({"add": {"op": "text", "x": as_value(at.0), "y": as_value(at.1), "text": text, "color": self.ink(), "size": size}}));
        }
    }

    pub(super) fn mode_text(&mut self, text: &str, cx: &mut Cx) {
        if let Some((_, typed)) = self.modes.typing.as_mut() {
            typed.push_str(text);
            cx.invalidate();
        }
    }

    pub(super) fn mode_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if let Some((_, typed)) = self.modes.typing.as_mut() {
            return match key.key {
                Key::Enter | Key::Escape => {
                    self.end_typing(key.key == Key::Enter, cx);
                    true
                }
                Key::Backspace => {
                    typed.pop();
                    cx.invalidate();
                    true
                }
                _ => false,
            };
        }
        // Delete deletes the chosen shape of a vector canvas.
        if !self.acts() || self.mode() != "vector" || !matches!(key.key, Key::Delete | Key::Backspace) || key.modifiers != Modifiers::NONE {
            return false;
        }
        let Some(chosen) = self.chosen() else { return false };
        self.act(json!({ "delete": chosen + 1 }));
        true
    }

    // -- the pointer ----------------------------------------------------------------

    /// A press of a canvas with a mode; true when the mode took it.
    pub(super) fn mode_pressed(&mut self, at: PointerInput, cx: &mut Cx) -> bool {
        if !self.acts() {
            return false;
        }
        let buttons = self.tool_buttons(cx.fonts(), cx.look());
        if let Some((tool, _)) = buttons.iter().find(|(_, place)| place.contains(at.screen_x, at.screen_y)) {
            if let Some(name) = self.base.name() {
                let tool = tool.to_string();
                self.base.host.change_state(name, &mut |s| s.tool = Some(tool.clone()));
            }
            cx.invalidate();
            return true;
        }
        let unit = self.unit_at(at.x, at.y);
        match self.mode().as_str() {
            "objects" => {
                let Some(place) = self.object_at(unit) else { return false };
                let Some(Shape::Object { x, y, .. }) = self.drawing().get(place).cloned() else { return false };
                self.modes.shown_objects = self.object_records();
                self.modes.dragging = Some(Dragging { place, at: (at.x, at.y), from: (x, y), by: (0.0, 0.0), moved: false });
            }
            "paint" => self.modes.stroke = Some(vec![unit]),
            "vector" => {
                let tool = self.tool_in_hand();
                if tool == "text" {
                    self.start_typing(unit, cx);
                    return true;
                }
                self.end_typing(true, cx);
                if tool == "select" {
                    let place = self.vector_shape_at(unit);
                    self.choose(place);
                    if place.is_none() {
                        cx.invalidate();
                        return true;
                    }
                }
                self.modes.sketching = Some(Sketching { from: unit, to: unit, points: vec![unit], moved: false });
            }
            _ => return false,
        }
        cx.invalidate();
        true
    }

    pub(super) fn mode_moved(&mut self, at: PointerInput, cx: &mut Cx) -> bool {
        if let Some(d) = self.modes.dragging {
            if !d.moved && ((at.x - d.at.0).powi(2) + (at.y - d.at.1).powi(2)).sqrt() < STILL {
                return true;
            }
            let scale = self.viewport().scale;
            self.modes.dragging = Some(Dragging { by: ((at.x - d.at.0) / scale, (at.y - d.at.1) / scale), moved: true, ..d });
            cx.invalidate();
            return true;
        }
        let unit = self.unit_at(at.x, at.y);
        if let Some(stroke) = self.modes.stroke.as_mut() {
            stroke.push(unit);
            cx.invalidate();
            return true;
        }
        if let Some(w) = self.modes.sketching.as_mut() {
            w.points.push(unit);
            w.to = unit;
            w.moved = true;
            cx.invalidate();
            return true;
        }
        false
    }

    pub(super) fn mode_released(&mut self, at: PointerInput, cx: &mut Cx) -> bool {
        let _ = at;
        if let Some(d) = self.modes.dragging.take() {
            if d.moved {
                if let Some(Shape::Object { record, .. }) = self.drawing().get(d.place) {
                    let (x, y) = (d.from.0 + d.by.0, d.from.1 + d.by.1);
                    self.act(json!({"move": {"id": record, "x": as_value(x), "y": as_value(y)}}));
                }
            }
            // What the runtime kept shows from its drawing; a refused drop puts the box back.
            cx.invalidate();
            return true;
        }
        if let Some(stroke) = self.modes.stroke.take() {
            self.end_stroke(stroke, cx);
            return true;
        }
        if let Some(w) = self.modes.sketching.take() {
            let tool = self.tool_in_hand();
            if tool == "select" {
                let shapes = self.vector_shapes();
                if let Some(place) = self.chosen().filter(|p| w.moved && *p < shapes.len()) {
                    let first = first_place(&shapes[place]);
                    let (x, y) = (first.0 + w.to.0 - w.from.0, first.1 + w.to.1 - w.from.1);
                    self.act(json!({"move": {"index": place + 1, "x": as_value(x), "y": as_value(y)}}));
                }
            } else if w.moved {
                if let Some(shape) = self.sketched(tool, w.from, w.to, &w.points) {
                    self.act(json!({ "add": shape }));
                }
            }
            cx.invalidate();
            return true;
        }
        false
    }

    // -- drawing ----------------------------------------------------------------------

    /// What a mode draws over the drawing: the painted picture and the stroke, or the vector shapes, the chosen one's frame and the one being drawn; and its toolbar.
    pub(super) fn draw_mode(&self, p: &Painter, v: Viewport) {
        let at = |x: f64, y: f64| self.at(v, x, y);
        let pictures = |name: &str| self.picture(name);
        let canvas = p.canvas;
        let (uw, uh) = self.units();
        match self.mode().as_str() {
            "paint" => {
                if let Some(picture) = self.painted() {
                    let start = at(0.0, 0.0);
                    let place = skia_safe::Rect::from_xywh(start.x, start.y, (uw * v.scale) as f32, (uh * v.scale) as f32);
                    let sampling = SamplingOptions::new(skia_safe::FilterMode::Linear, skia_safe::MipmapMode::None);
                    canvas.draw_image_rect_with_sampling_options(&picture, None, place, sampling, &Paint::default());
                }
                if let Some(stroke) = self.modes.stroke.as_ref().filter(|s| !s.is_empty()) {
                    let paper = self.paper();
                    let ink = if self.tool_in_hand() == "eraser" { color_of(p.look, if paper.is_empty() { "panel" } else { &paper }) } else { color_of(p.look, &self.ink()) };
                    let mut pen = Paint::default();
                    pen.set_color(ink);
                    pen.set_anti_alias(true);
                    pen.set_style(PaintStyle::Stroke);
                    pen.set_stroke_width((self.brush().max(1.0) * v.scale) as f32);
                    pen.set_stroke_cap(PaintCap::Round);
                    pen.set_stroke_join(PaintJoin::Round);
                    let pixels: Vec<Point> = stroke.iter().map(|(x, y)| at(*x, *y)).collect();
                    polyline(canvas, &pixels, &pen);
                }
            }
            "vector" => {
                let shapes = self.vector_shapes();
                let chosen = self.chosen();
                let (by_x, by_y) = match &self.modes.sketching {
                    Some(w) if w.moved && self.tool_in_hand() == "select" => (w.to.0 - w.from.0, w.to.1 - w.from.1),
                    _ => (0.0, 0.0),
                };
                for (i, shape) in shapes.iter().enumerate() {
                    if let Some(shape) = shape_of(shape) {
                        let shape = if Some(i) == chosen { shape.translated(by_x, by_y) } else { shape };
                        draw_shape(p, &shape, v.scale, &at, &pictures);
                    }
                }
                if let Some(b) = chosen.and_then(|c| shapes.get(c)).and_then(box_of) {
                    let (a, z) = (at(b.x + by_x, b.y + by_y), at(b.right() + by_x, b.bottom() + by_y));
                    let mut frame = Paint::default();
                    frame.set_color(p.look.color("accent"));
                    frame.set_anti_alias(true);
                    frame.set_style(PaintStyle::Stroke);
                    frame.set_stroke_width(1.5);
                    frame.set_path_effect(PathEffect::dash(&[4.0, 3.0], 0.0));
                    canvas.draw_rect(skia_safe::Rect::new(a.x - 3.0, a.y - 3.0, z.x + 3.0, z.y + 3.0), &frame);
                }
                if let Some(w) = self.modes.sketching.as_ref().filter(|w| w.moved && self.tool_in_hand() != "select") {
                    if let Some(preview) = self.sketched(self.tool_in_hand(), w.from, w.to, &w.points).as_ref().and_then(Value::as_object).and_then(shape_of) {
                        draw_shape(p, &preview, v.scale, &at, &pictures);
                    }
                }
                if let Some(((x, y), typed)) = &self.modes.typing {
                    // The text being typed, where it goes, with its caret.
                    let point = at(*x, *y);
                    let size = p.look.size("text");
                    let width = (p.fonts.measure(typed, "text", size, false) + 12.0).max(160.0);
                    let field = Rect::new(point.x as f64, point.y as f64 - size, width, size * 2.0);
                    p.fill(field, p.look.color("input"), p.look.radius("input"));
                    p.stroke(field, p.look.color("accent"), p.look.border(), p.look.radius("input"));
                    let (shown, color) = if typed.is_empty() { (locale::translate("canvasText", "The text"), "muted") } else { (format!("{typed}|"), "text") };
                    p.text(&shown, field.inset(4.0), p.look.color(color), size, "text", false, TextAlign::Left);
                }
            }
            _ => {}
        }
        if !self.tools_shown().is_empty() {
            self.draw_toolbar(p);
        }
    }
}
