//! A canvas with a mode (manual/parts.graphics.md, conformance/canvas-modes.json): what
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

mod painting;
mod toolbar;
mod vector;

use std::cell::RefCell;

use crate::wire::FieldContent;
use serde_json::{json, Map, Value};
use skia_safe::{surfaces, AlphaType, BlendMode, Color, ColorType, EncodedImageFormat, Image, ImageInfo, Paint, PaintCap, PaintJoin, PaintStyle, PathEffect, Point, SamplingOptions};

use super::{color_of, draw_shape, polyline, CanvasPart, Shape, Viewport, STILL};
use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Key, KeyInput, Modifiers, PointerInput};
use crate::locale;
use crate::painter::{Painter, TextAlign};
use crate::parts::text_editor::TextEditor;

/// The spray can: fine dots scattered around each point, within the brush's width, the same dots for the same stroke.
pub fn spray(canvas: &skia_safe::Canvas, points: &[Point], brush: f64, pen: &Paint) {
    let mut dot = pen.clone();
    dot.set_style(PaintStyle::Fill);
    let radius = (brush.max(2.0) * 2.0) as f32;
    let per_point = (brush * 3.0).clamp(6.0, 80.0) as usize;
    let mut seed: u32 = 0x9e37_79b9;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f32) / (u32::MAX as f32)
    };
    for p in points {
        for _ in 0..per_point {
            let (angle, reach) = (next() * std::f32::consts::TAU, next().sqrt() * radius);
            canvas.draw_circle((p.x + angle.cos() * reach, p.y + angle.sin() * reach), 0.8, &dot);
        }
    }
}

/// A picture file chosen to import, its bytes; none when the person chose none.
pub(super) struct Imported(pub Option<Vec<u8>>);
/// The exported picture was saved where the person chose, or nowhere.
pub(super) struct Exported;

/// The tools of each mode, in the order its toolbar shows them.
pub const PAINT_TOOLS: [&str; 12] = ["pen", "pencil", "spray", "eraser", "fill", "picker", "select", "ellipseSelect", "lasso", "line", "rect", "ellipse"];
pub const VECTOR_TOOLS: [&str; 6] = ["select", "line", "rect", "ellipse", "path", "text"];
/// A diagram's (mode objects): drag its objects, or draw a new line from one to another.
pub const OBJECT_TOOLS: [&str; 2] = ["select", "connect"];

const TOOL_HEIGHT: f64 = 24.0;
/// How near a colour is to the one pressed for the fill bucket to take it, 0 to 100.
const FILL_TOLERANCE: f64 = 12.0;

/// A tool's name in the person's language.
pub fn tool_word(tool: &str) -> String {
    match tool {
        "select" => locale::translate("toolSelect", "Select"),
        "connect" => locale::translate("toolConnect", "Connect"),
        "line" => locale::translate("toolLine", "Line"),
        "rect" => locale::translate("toolRect", "Rectangle"),
        "ellipse" => locale::translate("toolEllipse", "Ellipse"),
        "path" => locale::translate("toolPath", "Path"),
        "text" => locale::translate("toolText", "Text"),
        "pen" => locale::translate("toolPen", "Pen"),
        "eraser" => locale::translate("toolEraser", "Eraser"),
        "pencil" => locale::translate("toolPencil", "Pencil"),
        "fill" => locale::translate("toolFill", "Fill"),
        "picker" => locale::translate("toolPicker", "Pick a colour"),
        "ellipseSelect" => locale::translate("toolEllipseSelect", "Ellipse select"),
        "lasso" => locale::translate("toolLasso", "Lasso"),
        "spray" => locale::translate("toolSpray", "Spray can"),
        "rotate" => locale::translate("rotatePicture", "Turn a quarter clockwise"),
        "flipAcross" => locale::translate("flipAcross", "Mirror left to right"),
        "flipDown" => locale::translate("flipDown", "Mirror top to bottom"),
        "crop" => locale::translate("cropPicture", "Crop to the selection"),
        "import" => locale::translate("importPicture", "Import a picture"),
        "export" => locale::translate("exportPicture", "Export the picture"),
        "zoomIn" => locale::translate("zoomIn", "Zoom in"),
        "zoomOut" => locale::translate("zoomOut", "Zoom out"),
        "adjust" => locale::translate("adjustColors", "Colours"),
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

/// A bound field's value and the record it is in.
pub(super) type PaintedFor = (Option<Value>, Option<Value>);

/// What a canvas keeps of what the person does on it with a mode.
#[derive(Default)]
pub(super) struct ModeState {
    dragging: Option<Dragging>,
    /// The record of each place of the drawing a drag started on, to find it again when the drawing is drawn anew.
    shown_objects: Vec<Value>,
    /// The picture painted on, and the field's value and its record it was read for (or the last stroke left):
    /// every stored picture is the same value, so another record reads its own.
    painted: RefCell<Option<(PaintedFor, Option<Image>)>>,
    /// The stroke being painted, in units.
    stroke: Option<Vec<(f64, f64)>>,
    /// The drawing's shapes as the runtime reads its SVG, read once for each text.
    vector: RefCell<Option<(String, Vec<Map<String, Value>>)>>,
    sketching: Option<Sketching>,
    /// Where a text is being typed, in units, and the text input it is typed in
    /// (a caret that moves, a selection, the clipboard, as in every text box).
    typing: Option<((f64, f64), TextEditor)>,
    /// Its toolbar's buttons as last drawn, on the screen.
    toolbar: RefCell<Vec<(&'static str, Rect)>>,
    /// The shape chosen of a canvas without a name, its place in the drawing from 0; a named one's is the window's.
    chosen: Option<usize>,
    /// What the toolbox holds beyond the canvas's state: opacity, the background colour, where it is (toolbox.rs).
    pub(super) toolbox: std::rc::Rc<super::toolbox::Toolbox>,
    /// The raster tools' shape, selection and lifted pixels (raster.rs).
    raster: super::raster::RasterState,
    /// The layers while the person works with them (layers.rs).
    layers: super::layers::Layers,
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
        "text" => Shape::Words { x: n(shape, "x"), y: n(shape, "y"), text: s(shape, "text", "").to_string(), color, size: n(shape, "size"), align: "center".into(), style: String::new() },
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

/// The drawing with the object at `place` and what is drawn after it, until
/// the next object, moved; a link that ends in the object's box follows it
/// there, its end in the box moving and its bend points staying.
pub fn with_object_moved(shapes: Vec<Shape>, place: usize, dx: f64, dy: f64) -> Vec<Shape> {
    let held = match shapes.get(place) {
        Some(Shape::Object { x, y, width, height, .. }) => Some((*x, *y, *width, *height)),
        _ => None,
    };
    let inside = |px: f64, py: f64| held.is_some_and(|(x, y, w, h)| px >= x && px <= x + w && py >= y && py <= y + h);
    let mut moving = false;
    shapes
        .into_iter()
        .enumerate()
        .map(|(i, shape)| {
            if matches!(shape, Shape::Object { .. }) {
                moving = i == place;
            }
            match shape {
                Shape::Link { name, x1, y1, x2, y2, via, color, width, start, end, label, style } => {
                    let (x1, y1) = if inside(x1, y1) { (x1 + dx, y1 + dy) } else { (x1, y1) };
                    let (x2, y2) = if inside(x2, y2) { (x2 + dx, y2 + dy) } else { (x2, y2) };
                    Shape::Link { name, x1, y1, x2, y2, via, color, width, start, end, label, style }
                }
                shape if moving => shape.translated(dx, dy),
                shape => shape,
            }
        })
        .collect()
}

/// The picture after a stroke through `points` (in the picture's pixels)
/// with `tool`, as a PNG: the pen paints with `ink`, the eraser takes the
/// paint away.
pub fn painted_with(before: Option<&Image>, width: i32, height: i32, points: &[(f64, f64)], tool: &str, ink: Color, brush: f64, opacity: f64) -> Vec<u8> {
    let info = ImageInfo::new((width, height), ColorType::RGBA8888, AlphaType::Premul, None);
    let Some(mut surface) = surfaces::raster(&info, None, None) else { return Vec::new() };
    let canvas = surface.canvas();
    canvas.clear(Color::TRANSPARENT);
    if let Some(before) = before {
        let place = skia_safe::Rect::from_wh(width as f32, height as f32);
        canvas.draw_image_rect_with_sampling_options(before, None, place, SamplingOptions::default(), &Paint::default());
    }
    let mut pen = Paint::default();
    // Opacity (0 to 100) thins the paint; the pencil paints hard pixels, the pen soft edges.
    let alpha = (f64::from(ink.a()) * opacity.clamp(0.0, 100.0) / 100.0).round() as u8;
    pen.set_color(ink.with_a(alpha));
    pen.set_anti_alias(tool != "pencil");
    pen.set_style(PaintStyle::Stroke);
    pen.set_stroke_width(brush.max(1.0) as f32);
    pen.set_stroke_cap(PaintCap::Round);
    pen.set_stroke_join(PaintJoin::Round);
    pen.set_blend_mode(if tool == "eraser" { BlendMode::Clear } else { BlendMode::SrcOver });
    let pixels: Vec<Point> = points.iter().map(|(x, y)| Point::new(*x as f32, *y as f32)).collect();
    if tool == "spray" {
        spray(canvas, &pixels, brush, &pen);
    } else {
        polyline(canvas, &pixels, &pen);
    }
    surface.image_snapshot().encode(None, EncodedImageFormat::PNG, None).map(|d| d.as_bytes().to_vec()).unwrap_or_default()
}

impl CanvasPart {
    /// What the person does on it: objects, paint or vector; empty: its draw handler draws it.
    pub fn mode(&self) -> String {
        self.base.spec.json("mode").and_then(Value::as_str).unwrap_or("").to_string()
    }

    /// Whether the person acts on it: it has a mode, may be used and changed.
    pub(in crate::parts::data::canvas) fn acts(&self) -> bool {
        matches!(self.mode().as_str(), "objects" | "paint" | "vector") && !self.base.read_only() && self.base.enabled()
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
    pub fn typing(&self) -> Option<String> {
        self.modes.typing.as_ref().map(|(_, editor)| editor.text())
    }

    /// The text input the text tool types in; none while no text is being typed.
    pub fn typing_editor(&self) -> Option<&TextEditor> {
        self.modes.typing.as_ref().map(|(_, editor)| editor)
    }

    fn act(&self, action: Value) {
        let Value::Object(action) = action else { return };
        if let Some(name) = self.base.name().filter(|_| self.acts()) {
            self.base.host.acted(name, &action);
        }
    }

    // -- The toolbar -------------------------------------------------------------

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
    pub(in crate::parts::data::canvas) fn drawing_changed(&mut self) {
        let Some(held) = self.modes.dragging else { return };
        let record = self.modes.shown_objects.get(held.place).cloned().unwrap_or(Value::Null);
        let place = self.drawing().iter().position(|s| matches!(s, Shape::Object { record: r, .. } if *r == record));
        self.modes.shown_objects = self.object_records();
        self.modes.dragging = place.map(|place| Dragging { place, ..held });
    }

    /// The drawing as it shows: an object being dragged moved with what is drawn after it.
    pub(in crate::parts::data::canvas) fn mode_shapes(&self, shapes: Vec<Shape>) -> Vec<Shape> {
        match self.modes.dragging {
            Some(d) if d.moved => with_object_moved(shapes, d.place, d.by.0, d.by.1),
            _ => shapes,
        }
    }

    // -- paint -------------------------------------------------------------------

    // -- vector ------------------------------------------------------------------

    pub(in crate::parts::data::canvas) fn mode_text(&mut self, text: &str, cx: &mut Cx) {
        if let Some((_, editor)) = self.modes.typing.as_mut() {
            editor.typed(text, cx);
            self.place_typing(cx);
            cx.invalidate();
        }
    }

    pub(in crate::parts::data::canvas) fn mode_key(&mut self, key: KeyInput, cx: &mut Cx) -> bool {
        if let Some((_, editor)) = self.modes.typing.as_mut() {
            // Enter places the text, Escape drops it; every other key is the text input's.
            if matches!(key.key, Key::Enter | Key::Escape) {
                self.end_typing(key.key == Key::Enter, cx);
                return true;
            }
            let took = editor.key(key, cx);
            self.place_typing(cx);
            cx.invalidate();
            return took;
        }
        // A painting canvas's selection: cut, copy, paste, delete, select all, deselect.
        if self.acts() && self.mode() == "paint" && self.raster_key(key, cx) {
            return true;
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
    pub(in crate::parts::data::canvas) fn mode_pressed(&mut self, at: PointerInput, cx: &mut Cx) -> bool {
        if !self.acts() {
            return false;
        }
        if self.typing_pressed(at, cx) {
            return true;
        }
        let buttons = self.tool_buttons(cx.fonts(), cx.look());
        if let Some((tool, _)) = buttons.iter().find(|(_, place)| place.contains(at.screen_x, at.screen_y)) {
            if let Some(name) = self.base.name() {
                let tool = tool.to_string();
                crate::parts::change_state(&*self.base.host, name, |s| s.tool = Some(tool.clone()));
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
            "paint" => match self.tool_in_hand() {
                "fill" => self.fill_at(unit, cx),
                "picker" => self.pick_at(unit),
                tool @ ("select" | "ellipseSelect" | "lasso" | "line" | "rect" | "ellipse") => self.raster_pressed(tool, unit, cx),
                _ => self.modes.stroke = Some(vec![unit]),
            },
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

    pub(in crate::parts::data::canvas) fn mode_moved(&mut self, at: PointerInput, cx: &mut Cx) -> bool {
        if self.typing_dragged(at, false, cx) {
            return true;
        }
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
        if self.raster_moved(unit) {
            cx.invalidate();
            return true;
        }
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

    pub(in crate::parts::data::canvas) fn mode_released(&mut self, at: PointerInput, cx: &mut Cx) -> bool {
        if self.typing_dragged(at, true, cx) {
            return true;
        }
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
        if self.raster_released(cx) {
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

    /// The pointer was taken away: a drag, a stroke or a shape being drawn is dropped, not sent.
    pub(in crate::parts::data::canvas) fn mode_cancelled(&mut self) {
        self.modes.dragging = None;
        self.modes.stroke = None;
        self.modes.sketching = None;
        self.raster_cancelled();
    }

    // -- drawing ----------------------------------------------------------------------

    /// What a mode draws over the drawing: the painted picture and the stroke, or the vector shapes, the chosen one's frame and the one being drawn; and its toolbar.
    pub(in crate::parts::data::canvas) fn draw_mode(&self, p: &Painter, v: Viewport) {
        let at = |x: f64, y: f64| self.at(v, x, y);
        let pictures = |name: &str| self.picture(name);
        let canvas = p.canvas;
        let (uw, uh) = self.units();
        match self.mode().as_str() {
            "paint" => {
                // The picture is a sheet of paper on a desk: the paper its units, in the canvas's
                // background colour (white without one), with a light shadow; the desk around it.
                let start = at(0.0, 0.0);
                let sheet = Rect::new(f64::from(start.x), f64::from(start.y), uw * v.scale, uh * v.scale);
                {
                    let _clip = p.clip(self.base.node.bounds);
                    p.fill(self.base.node.bounds, p.look.color("raised"), 0.0);
                    for i in 1..=4 {
                        let reach = f64::from(i);
                        p.fill(Rect::new(sheet.x + reach, sheet.y + reach, sheet.width, sheet.height), Color::from_argb(18, 0, 0, 0), 0.0);
                    }
                    let paper = self.paper();
                    p.fill(sheet, color_of(p.look, if paper.is_empty() { "#ffffff" } else { &paper }), 0.0);
                }
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
                    // As it will be painted: thinned by the opacity, hard-edged with the pencil.
                    let alpha = (f64::from(ink.a()) * self.modes.toolbox.opacity() / 100.0).round() as u8;
                    pen.set_color(ink.with_a(alpha));
                    pen.set_anti_alias(self.tool_in_hand() != "pencil");
                    pen.set_style(PaintStyle::Stroke);
                    pen.set_stroke_width((self.brush().max(1.0) * v.scale) as f32);
                    pen.set_stroke_cap(PaintCap::Round);
                    pen.set_stroke_join(PaintJoin::Round);
                    let pixels: Vec<Point> = stroke.iter().map(|(x, y)| at(*x, *y)).collect();
                    if self.tool_in_hand() == "spray" {
                        spray(canvas, &pixels, self.brush() * v.scale, &pen);
                    } else {
                        polyline(canvas, &pixels, &pen);
                    }
                }
                self.draw_raster(p, v);
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
                        draw_shape(p, &shape, v.scale, &at, &pictures, &self.paper());
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
                        draw_shape(p, &preview, v.scale, &at, &pictures, &self.paper());
                    }
                }
                if let Some((_, editor)) = &self.modes.typing {
                    // The text being typed, in its input where it goes.
                    crate::element::draw_safely(editor, p);
                }
            }
            _ => {}
        }
        // With a toolbox, the tools are in it, not on the canvas.
        if !self.tools_shown().is_empty() && self.toolbox_shows().is_none() {
            self.draw_toolbar(p);
        }
    }
}
