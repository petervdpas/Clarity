//! A canvas drawn by code (DESIGN 5.7, conformance/drawing.json): the shapes
//! its draw handler drew (its state's `drawing`), in the canvas's own units
//! (`units`, 100 x 100 without them), scaled to fit its place in proportion
//! and centered. A color is the theme's by name or written out (#rrggbb).
//! A canvas with `on click` runs it with where it was clicked, in its units;
//! `on drag` runs as a pressed pointer moves, with where it was pressed, and
//! `on drop` when it is let go. A drag that moves nothing (the drawing stays
//! the same after its first run), and any drag without `on drag`, pans a
//! canvas with `zoom`, which moves as Clarity's own diagram designer does: it
//! starts whole, the wheel pans, Ctrl and the wheel zoom at the pointer, and
//! the buttons in its corner zoom out, back to whole, and in. With a grid it
//! shows a dot every so many units over the whole view, after the drawing's
//! clear. With `on bend` its links bend: a press on one adds a point there or
//! takes the point under it, dragging moves that point, a double click on a
//! point takes it away, and letting go runs `on bend` with the link's name
//! and its points. With `on connect` (and no mode) the person draws a new
//! line between boxes: the box under the pointer shows a handle in the
//! middle of each side, a press on one (or anywhere on a box while code set
//! `connecting`) drags a line that follows the pointer, and letting go runs
//! `on connect` with where it started and ended; the handles and that line
//! are drawn over the drawing, never in it. Without `on connect` none of it
//! shows or takes the pointer. With `on resize` (and no mode, while code
//! leaves `resizable` true) the box under the pointer shows a grip at its
//! bottom-right corner; dragging it outlines the box at its new size, its
//! top-left staying, and letting go runs `on resize` with where that corner
//! was and is now. With a mode the person drags its objects, paints or draws
//! (`canvas/modes.rs`). A canvas that `runs` an engine beside the screen is
//! drawn by it (`canvas/engine.rs`). The window runs its handlers through
//! its host (`PartHost::pointed`, `acted`).

pub mod engine;
pub mod drawing;
mod modes;
pub mod raster;
pub mod adjust;
pub mod layers;
pub mod toolbox;

pub use modes::{box_of, first_place, painted_with, shape_of, tool_word, with_object_moved, PAINT_TOOLS, VECTOR_TOOLS};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use serde_json::{Map, Value};
use skia_safe::canvas::PointMode;
use skia_safe::font::Edging;
use skia_safe::{Color, Font, FontHinting, Paint, PaintCap, PaintJoin, PaintStyle, Point};

use crate::element::{Cx, Element};
use crate::geometry::Rect;
use crate::input::{Cursor, Modifiers, PointerButton, PointerInput};
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::picture::Picture;
use crate::parts::{Part, PartBase, PartHost};
use crate::theme::{parse_color, ThemeLook};

pub use engine::EngineCanvasPart;

/// A canvas's units when it names none, as on every renderer.
pub const DEFAULT_UNITS: f64 = 100.0;

/// A pointer moved this many pixels is a drag, not a click.
const STILL: f64 = 4.0;

/// How far from a connection handle, in pixels, a press still takes it.
const HANDLE_NEAR: f64 = 7.0;

/// The least width and height, in units, a box is resized to.
const LEAST_SIZE: (f64, f64) = (40.0, 30.0);

/// How many pixels across a box's resize grip is drawn.
const GRIP_SIZE: f64 = 8.0;

/// The bounds of a box drawn on a canvas, in its units: what a new line connects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl UnitBox {
    /// Its connection handles: the middle of its top, right, bottom and left sides.
    pub fn handles(&self) -> [(f64, f64); 4] {
        let (cx, cy) = (self.x + self.width / 2.0, self.y + self.height / 2.0);
        [(cx, self.y), (self.x + self.width, cy), (cx, self.y + self.height), (self.x, cy)]
    }

    /// Its bottom-right corner, where its resize grip is.
    pub fn corner(&self) -> (f64, f64) {
        (self.x + self.width, self.y + self.height)
    }

    /// Its middle, in whole units.
    pub fn center(&self) -> (f64, f64) {
        ((self.x + self.width / 2.0).floor(), (self.y + self.height / 2.0).floor())
    }

    fn contains(&self, other: &UnitBox) -> bool {
        self.x <= other.x && self.y <= other.y && self.x + self.width >= other.x + other.width && self.y + self.height >= other.y + other.height
    }
}

/// The box a shape the canvas hit-tests takes: a rect, a picture, an object, a circle, a ring or an ellipse.
fn box_of_shape(shape: &Shape) -> Option<UnitBox> {
    match shape {
        Shape::Rect { x, y, width, height, .. } | Shape::Image { x, y, width, height, .. } | Shape::Object { x, y, width, height, .. } => {
            Some(UnitBox { x: *x, y: *y, width: *width, height: *height })
        }
        Shape::Circle { x, y, r, .. } | Shape::Ring { x, y, r, .. } => Some(UnitBox { x: x - r, y: y - r, width: 2.0 * r, height: 2.0 * r }),
        Shape::Ellipse { x, y, rx, ry, .. } => Some(UnitBox { x: x - rx, y: y - ry, width: 2.0 * rx, height: 2.0 * ry }),
        _ => None,
    }
}

/// Whether a unit is on a shape the canvas hit-tests (not the paper, lines or text).
fn shape_hit(shape: &Shape, p: (f64, f64)) -> bool {
    match shape {
        Shape::Rect { x, y, width, height, .. } | Shape::Image { x, y, width, height, .. } | Shape::Object { x, y, width, height, .. } => p.0 >= *x && p.0 <= x + width && p.1 >= *y && p.1 <= y + height,
        Shape::Circle { x, y, r, .. } | Shape::Ring { x, y, r, .. } => distance((*x, *y), p) <= *r,
        Shape::Ellipse { x, y, rx, ry, .. } => *rx > 0.0 && *ry > 0.0 && ((p.0 - x) / rx).powi(2) + ((p.1 - y) / ry).powi(2) <= 1.0,
        _ => false,
    }
}

/// The boxes of a drawing of its units, the topmost first: each drawn shape
/// grown to the shapes under it that hold it whole (a diagram's box is its
/// outline, its body and its header). A shape over the whole paper is the
/// paper, no box.
pub fn boxes_of(shown: &[Shape], units: (f64, f64)) -> Vec<(UnitBox, Shape)> {
    let paper = UnitBox { x: 0.0, y: 0.0, width: units.0, height: units.1 };
    let all: Vec<(UnitBox, &Shape)> = shown.iter().filter_map(|s| box_of_shape(s).map(|b| (b, s))).filter(|(b, _)| !b.contains(&paper)).collect();
    (0..all.len())
        .rev()
        .map(|i| {
            let mut grown = all[i].0;
            for (under, _) in all[..i].iter().rev() {
                if under.contains(&grown) {
                    grown = *under;
                }
            }
            (grown, all[i].1.clone())
        })
        .collect()
}

/// What a canvas shows: the pixels a unit takes, and the unit at its top left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub scale: f64,
    pub x: f64,
    pub y: f64,
}

/// A place in a canvas's units, rounded down: where it was clicked.
pub type UnitPoint = (i64, i64);

/// One shape of a drawing (conformance/drawing.json), in the canvas's units.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Clear { color: String },
    Line { x1: f64, y1: f64, x2: f64, y2: f64, color: String, width: f64 },
    Rect { x: f64, y: f64, width: f64, height: f64, color: String },
    Circle { x: f64, y: f64, r: f64, color: String },
    Ring { x: f64, y: f64, r: f64, color: String, width: f64 },
    Ellipse { x: f64, y: f64, rx: f64, ry: f64, color: String },
    /// A text; its style any of "bold", "italic" and "mono" (the theme's code font).
    Words { x: f64, y: f64, text: String, color: String, size: f64, align: String, style: String },
    Image { x: f64, y: f64, width: f64, height: f64, picture: String },
    /// A line people bend: from x1, y1 to x2, y2 through its bend points `via`
    /// (x, y, x, y, ...); what its ends show (an arrow, a crow's foot), a
    /// label halfway along it and its style ("dashed", "dotted").
    Link { name: String, x1: f64, y1: f64, x2: f64, y2: f64, via: Vec<f64>, color: String, width: f64, start: String, end: String, label: String, style: String },
    /// An open line through its points (x, y, x, y, ...).
    Path { points: Vec<f64>, color: String, width: f64 },
    /// A record's box on a canvas with objects: it draws nothing itself.
    Object { record: Value, x: f64, y: f64, width: f64, height: f64 },
}

fn number_of(value: Option<&Value>) -> Option<f64> {
    let n = match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }?;
    n.is_finite().then_some(n)
}

fn wholes(value: Option<&Value>) -> Vec<f64> {
    match value {
        Some(Value::Array(items)) => items.iter().map(|v| number_of(Some(v)).unwrap_or(0.0).round()).collect(),
        _ => Vec::new(),
    }
}

impl Shape {
    /// A shape as a draw handler or an engine sends it; what is wrong with it when it is none.
    pub fn of(sent: &Value) -> Result<Shape, String> {
        let n = |name: &str| number_of(sent.get(name)).ok_or_else(|| format!("a shape's {name} is no number: {sent}"));
        let or = |name: &str, otherwise: f64| number_of(sent.get(name)).unwrap_or(otherwise);
        let t = |name: &str, otherwise: &str| sent.get(name).and_then(Value::as_str).unwrap_or(otherwise).to_string();
        Ok(match sent.get("op").and_then(Value::as_str).unwrap_or("") {
            "clear" => Shape::Clear { color: t("color", "") },
            "line" => Shape::Line { x1: n("x1")?, y1: n("y1")?, x2: n("x2")?, y2: n("y2")?, color: t("color", "text"), width: or("width", 1.0) },
            "rect" => Shape::Rect { x: n("x")?, y: n("y")?, width: n("width")?, height: n("height")?, color: t("color", "text") },
            "circle" => Shape::Circle { x: n("x")?, y: n("y")?, r: n("r")?, color: t("color", "text") },
            "ring" => Shape::Ring { x: n("x")?, y: n("y")?, r: n("r")?, color: t("color", "text"), width: or("width", 1.0) },
            "ellipse" => Shape::Ellipse { x: n("x")?, y: n("y")?, rx: n("rx")?, ry: n("ry")?, color: t("color", "text") },
            "text" => {
                let text = sent.get("text").map(crate::locale::shown).unwrap_or_default();
                Shape::Words { x: n("x")?, y: n("y")?, text, color: t("color", "text"), size: or("size", 10.0), align: t("align", "center"), style: t("style", "") }
            }
            "image" => Shape::Image { x: n("x")?, y: n("y")?, width: n("width")?, height: n("height")?, picture: t("picture", "") },
            "link" => Shape::Link {
                name: t("name", ""),
                x1: n("x1")?,
                y1: n("y1")?,
                x2: n("x2")?,
                y2: n("y2")?,
                via: wholes(sent.get("via")),
                color: t("color", "text"),
                width: or("width", 1.0),
                start: t("start", ""),
                end: t("end", ""),
                label: sent.get("label").map(crate::locale::shown).unwrap_or_default(),
                style: t("style", ""),
            },
            "path" => Shape::Path { points: wholes(sent.get("points")), color: t("color", "text"), width: or("width", 1.0) },
            "object" => Shape::Object { record: sent.get("record").cloned().unwrap_or(Value::Null), x: n("x")?, y: n("y")?, width: n("width")?, height: n("height")? },
            op => return Err(format!("'{op}' is no shape: {sent}")),
        })
    }

    /// The shapes of a drawing that read as shapes, in order; one that does not draws nothing.
    pub fn all(sent: &[Value]) -> Vec<Shape> {
        sent.iter().filter_map(|s| Shape::of(s).ok()).collect()
    }

    /// The shape moved by `dx`, `dy` units.
    pub fn translated(&self, dx: f64, dy: f64) -> Shape {
        let shift = |points: &[f64]| points.iter().enumerate().map(|(i, p)| p + (if i % 2 == 0 { dx } else { dy }).round()).collect::<Vec<f64>>();
        let mut moved = self.clone();
        match &mut moved {
            Shape::Line { x1, y1, x2, y2, .. } => {
                (*x1, *y1, *x2, *y2) = (*x1 + dx, *y1 + dy, *x2 + dx, *y2 + dy);
            }
            Shape::Rect { x, y, .. }
            | Shape::Circle { x, y, .. }
            | Shape::Ring { x, y, .. }
            | Shape::Words { x, y, .. }
            | Shape::Image { x, y, .. }
            | Shape::Object { x, y, .. }
            | Shape::Ellipse { x, y, .. } => {
                (*x, *y) = (*x + dx, *y + dy);
            }
            Shape::Path { points, .. } => *points = shift(points),
            Shape::Link { x1, y1, x2, y2, via, .. } => {
                (*x1, *y1, *x2, *y2) = (*x1 + dx, *y1 + dy, *x2 + dx, *y2 + dy);
                *via = shift(via);
            }
            Shape::Clear { .. } => {}
        }
        moved
    }
}

/// The whole canvas, fitted in proportion and centered (conformance/drawing.json, views).
pub fn whole(units_width: f64, units_height: f64, width: f64, height: f64) -> Viewport {
    let scale = (width / units_width).min(height / units_height);
    if !(scale > 0.0) || !scale.is_finite() {
        return Viewport { scale: 1.0, x: 0.0, y: 0.0 };
    }
    Viewport { scale, x: (units_width - width / scale) / 2.0, y: (units_height - height / scale) / 2.0 }
}

/// The view after zooming by `factor` (above 1 closer), the unit under the
/// pixel `at_x`, `at_y` staying where it is; from a quarter (or the whole
/// fit, when less) up to 3 (or 4 times the whole fit, when more).
#[allow(clippy::too_many_arguments)]
pub fn zoomed(units_width: f64, units_height: f64, width: f64, height: f64, view: Viewport, factor: f64, at_x: f64, at_y: f64) -> Viewport {
    let fit = (width / units_width).min(height / units_height);
    let scale = (view.scale * factor).max(0.25f64.min(fit)).min(3f64.max(fit * 4.0));
    let ux = view.x + at_x / view.scale;
    let uy = view.y + at_y / view.scale;
    Viewport { scale, x: ux - at_x / scale, y: uy - at_y / scale }
}

/// The view moved by `by_x`, `by_y` pixels, past the canvas's edges too.
pub fn scrolled(view: Viewport, by_x: f64, by_y: f64) -> Viewport {
    Viewport { x: view.x + by_x / view.scale, y: view.y + by_y / view.scale, ..view }
}

/// The unit under a pixel of the view, rounded down; none outside the canvas's units.
pub fn point_in(units_width: f64, units_height: f64, view: Viewport, at_x: f64, at_y: f64) -> Option<UnitPoint> {
    let x = view.x + at_x / view.scale;
    let y = view.y + at_y / view.scale;
    if !(x >= 0.0 && y >= 0.0 && x < units_width && y < units_height) {
        return None;
    }
    Some((x.floor() as i64, y.floor() as i64))
}

/// Where in a drawing of its units a click at a pixel of a part `width` by
/// `height` is, the drawing fitted whole and centered; rounded down. None
/// beside the drawing.
pub fn point_of(units_width: f64, units_height: f64, width: f64, height: f64, at_x: f64, at_y: f64) -> Option<UnitPoint> {
    let scale = (width / units_width).min(height / units_height);
    if !(scale > 0.0) || !scale.is_finite() {
        return None;
    }
    let x = (at_x - (width - units_width * scale) / 2.0) / scale;
    let y = (at_y - (height - units_height * scale) / 2.0) / scale;
    if x < 0.0 || y < 0.0 || x >= units_width || y >= units_height {
        return None;
    }
    Some((x.floor() as i64, y.floor() as i64))
}

/// Where a pointer that is down is, as `point_of` finds it, but beside the drawing at its nearest edge; none when the part shows nothing.
pub fn point_near(units_width: f64, units_height: f64, width: f64, height: f64, at_x: f64, at_y: f64) -> Option<UnitPoint> {
    let scale = (width / units_width).min(height / units_height);
    if !(scale > 0.0) || !scale.is_finite() {
        return None;
    }
    let x = (at_x - (width - units_width * scale) / 2.0) / scale;
    let y = (at_y - (height - units_height * scale) / 2.0) / scale;
    Some((edge(x, units_width), edge(y, units_height)))
}

fn edge(v: f64, size: f64) -> i64 {
    v.floor().max(0.0).min(size.ceil() - 1.0) as i64
}

/// The units between a grid's shown dots: every `grid`, doubled until they are 8 pixels apart (conformance/drawing.json, grids).
pub fn grid_every(grid: f64, scale: f64) -> f64 {
    let mut every = grid;
    while every * scale < 8.0 && every < 1e6 {
        every *= 2.0;
    }
    every
}

/// How far left of its x a text starts: half its width centered (the default), none from its left end, all of it from its right.
pub fn text_left(align: &str, width: f64) -> f64 {
    match align {
        "left" => 0.0,
        "right" => width,
        _ => width / 2.0,
    }
}

/// The color of a shape: written out (#rrggbb), else the theme's by name, its text color for a name it has not.
pub fn color_of(look: &ThemeLook, color: &str) -> Color {
    if color.starts_with('#') {
        if let Some(written) = parse_color(color) {
            return written;
        }
    }
    look.color(color)
}

/// The font of a canvas's text at a size on the screen: unhinted, so its letters and spaces scale with the zoom and keep their widths.
pub fn words_font(fonts: &crate::fonts::Fonts, size: f64) -> Font {
    words_font_of(fonts, "text", size, false)
}

/// The font of a canvas's text of a font role of the theme (`text`, `mono`), bold or not, as `words_font`.
pub fn words_font_of(fonts: &crate::fonts::Fonts, role: &str, size: f64, bold: bool) -> Font {
    let mut font = fonts.font(role, size, bold);
    font.set_hinting(FontHinting::None);
    font.set_linear_metrics(true);
    font.set_subpixel(true);
    font.set_edging(Edging::AntiAlias);
    font
}

/// The points a link goes through: its start, its bend points, its end.
pub fn link_points(x1: f64, y1: f64, x2: f64, y2: f64, via: &[f64]) -> Vec<(f64, f64)> {
    let mut all = vec![(x1, y1)];
    all.extend(via.chunks_exact(2).map(|p| (p[0], p[1])));
    all.push((x2, y2));
    all
}

fn shape_paint(look: &ThemeLook, color: &str, stroke: Option<f64>) -> Paint {
    let mut paint = Paint::default();
    paint.set_color(color_of(look, color));
    paint.set_anti_alias(true);
    match stroke {
        Some(width) => {
            paint.set_style(PaintStyle::Stroke);
            paint.set_stroke_width(width as f32);
            paint.set_stroke_cap(PaintCap::Round);
            paint.set_stroke_join(PaintJoin::Round);
        }
        None => {
            paint.set_style(PaintStyle::Fill);
        }
    }
    paint
}

/// The point halfway along a line through `points`.
fn halfway(points: &[(f64, f64)]) -> (f64, f64) {
    let total: f64 = points.windows(2).map(|w| distance(w[0], w[1])).sum();
    let mut left = total / 2.0;
    for w in points.windows(2) {
        let length = distance(w[0], w[1]);
        if length > 0.0 && left <= length {
            let t = left / length;
            return (w[0].0 + t * (w[1].0 - w[0].0), w[0].1 + t * (w[1].1 - w[0].1));
        }
        left -= length;
    }
    points[0]
}

/// What a link's end at `tip` shows (crate::LINK_ENDS in the spec), the line
/// coming from `from`: an arrow, a hollow triangle, a diamond, or a data
/// model's bars, circle and crow's foot. In units, 12 long and 7 to a side.
#[allow(clippy::too_many_arguments)]
fn link_end(p: &Painter, kind: &str, tip: (f64, f64), from: (f64, f64), color: &str, width: f64, scale: f64, paper: &str, at: &dyn Fn(f64, f64) -> Point) {
    let length = distance(tip, from);
    if kind.is_empty() || length <= 0.0 {
        return;
    }
    // Along the line toward the tip, and across it.
    let (dx, dy) = ((tip.0 - from.0) / length, (tip.1 - from.1) / length);
    let (nx, ny) = (-dy, dx);
    let back = |k: f64, side: f64| at(tip.0 - k * dx + side * nx, tip.1 - k * dy + side * ny);
    let (long, wide) = (12.0, 7.0);
    let stroke = shape_paint(p.look, color, Some(width.max(1.0)));
    let hollow = shape_paint(p.look, if paper.is_empty() { "panel" } else { paper }, None);
    let filled = shape_paint(p.look, color, None);
    let polygon = |points: &[Point], fill: &Paint| {
        let path = skia_safe::Path::polygon(points, true, None, None);
        p.canvas.draw_path(&path, fill);
        p.canvas.draw_path(&path, &stroke);
    };
    let bar = |k: f64| p.canvas.draw_line(back(k, wide), back(k, -wide), &stroke);
    let ring = |k: f64| {
        let r = (4.0 * scale) as f32;
        p.canvas.draw_circle(back(k, 0.0), r, &hollow);
        p.canvas.draw_circle(back(k, 0.0), r, &stroke);
    };
    let crow = || {
        for side in [wide, 0.0, -wide] {
            p.canvas.draw_line(back(long, 0.0), back(0.0, side), &stroke);
        }
    };
    match kind {
        "arrow" => {
            p.canvas.draw_line(back(long, wide), back(0.0, 0.0), &stroke);
            p.canvas.draw_line(back(long, -wide), back(0.0, 0.0), &stroke);
        }
        "triangle" => polygon(&[back(0.0, 0.0), back(long, wide), back(long, -wide)], &hollow),
        "diamond" => polygon(&[back(0.0, 0.0), back(long, wide), back(2.0 * long, 0.0), back(long, -wide)], &hollow),
        "filledDiamond" => polygon(&[back(0.0, 0.0), back(long, wide), back(2.0 * long, 0.0), back(long, -wide)], &filled),
        "one" => {
            bar(6.0);
            bar(11.0);
        }
        "zeroOrOne" => {
            bar(6.0);
            ring(15.0);
        }
        "many" => crow(),
        "oneOrMany" => {
            crow();
            bar(long + 4.0);
        }
        "zeroOrMany" => {
            crow();
            ring(long + 8.0);
        }
        _ => {}
    }
}

/// The links of a drawing ending at the edge of the box each end lies in,
/// not under it, so what their ends show is seen: a box as `boxes_of` finds them.
pub fn links_to_edges(shapes: Vec<Shape>, boxes: &[UnitBox]) -> Vec<Shape> {
    let inside = |b: &UnitBox, p: (f64, f64)| p.0 > b.x && p.0 < b.x + b.width && p.1 > b.y && p.1 < b.y + b.height;
    // Where the line from p (in the box) to q (outside it) leaves it.
    let edge = |b: &UnitBox, p: (f64, f64), q: (f64, f64)| {
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let across = |d: f64, from: f64, low: f64, high: f64| if d > 0.0 { (high - from) / d } else if d < 0.0 { (low - from) / d } else { f64::INFINITY };
        let t = across(dx, p.0, b.x, b.x + b.width).min(across(dy, p.1, b.y, b.y + b.height)).clamp(0.0, 1.0);
        (p.0 + t * dx, p.1 + t * dy)
    };
    let cut = |p: (f64, f64), q: (f64, f64)| boxes.iter().find(|b| inside(b, p) && !inside(b, q)).map(|b| edge(b, p, q)).unwrap_or(p);
    shapes
        .into_iter()
        .map(|shape| match shape {
            Shape::Link { name, x1, y1, x2, y2, via, color, width, start, end, label, style } => {
                let points = link_points(x1, y1, x2, y2, &via);
                let n = points.len();
                let a = cut(points[0], points[1]);
                let b = cut(points[n - 1], points[n - 2]);
                Shape::Link { name, x1: a.0, y1: a.1, x2: b.0, y2: b.1, via, color, width, start, end, label, style }
            }
            other => other,
        })
        .collect()
}

/// A line through `points`; one point is a dot of its round cap.
fn polyline(canvas: &skia_safe::Canvas, points: &[Point], paint: &Paint) {
    if points.is_empty() {
        return;
    }
    canvas.draw_points(if points.len() == 1 { PointMode::Points } else { PointMode::Polygon }, points, paint);
}

/// Draws one shape of a drawing, in its units, `at` giving each unit's pixel.
/// `paper` (a color, empty for the theme's panel) fills what is hollow at a link's end.
pub fn draw_shape(p: &Painter, shape: &Shape, scale: f64, at: &dyn Fn(f64, f64) -> Point, pictures: &dyn Fn(&str) -> Option<Picture>, paper: &str) {
    let canvas = p.canvas;
    let look = p.look;
    let through = |points: &[(f64, f64)], paint: &Paint| {
        let pixels: Vec<Point> = points.iter().map(|(x, y)| at(*x, *y)).collect();
        polyline(canvas, &pixels, paint);
    };
    match shape {
        Shape::Line { x1, y1, x2, y2, color, width } => {
            canvas.draw_line(at(*x1, *y1), at(*x2, *y2), &shape_paint(look, color, Some(width * scale)));
        }
        Shape::Rect { x, y, width, height, color } => {
            let a = at(*x, *y);
            canvas.draw_rect(skia_safe::Rect::from_xywh(a.x, a.y, (width * scale) as f32, (height * scale) as f32), &shape_paint(look, color, None));
        }
        Shape::Circle { x, y, r, color } => {
            canvas.draw_circle(at(*x, *y), (r * scale) as f32, &shape_paint(look, color, None));
        }
        Shape::Ring { x, y, r, color, width } => {
            canvas.draw_circle(at(*x, *y), (r * scale) as f32, &shape_paint(look, color, Some(width * scale)));
        }
        Shape::Ellipse { x, y, rx, ry, color } => {
            let c = at(*x, *y);
            let (rx, ry) = ((rx * scale) as f32, (ry * scale) as f32);
            canvas.draw_oval(skia_safe::Rect::new(c.x - rx, c.y - ry, c.x + rx, c.y + ry), &shape_paint(look, color, None));
        }
        Shape::Link { x1, y1, x2, y2, via, color, width, start, end, label, style, .. } => {
            let points = link_points(*x1, *y1, *x2, *y2, via);
            let mut line = shape_paint(look, color, Some(width * scale));
            let dash = |on: f64, off: f64| skia_safe::PathEffect::dash(&[(on * scale) as f32, (off * scale) as f32], 0.0);
            match style.as_str() {
                "dashed" => {
                    line.set_path_effect(dash(6.0, 4.0));
                }
                "dotted" => {
                    line.set_path_effect(dash(0.1, 3.0 + width));
                }
                _ => {}
            }
            through(&points, &line);
            let n = points.len();
            link_end(p, start, points[0], points[1], color, width * scale, scale, paper, at);
            link_end(p, end, points[n - 1], points[n - 2], color, width * scale, scale, paper, at);
            if !label.is_empty() {
                let (mx, my) = halfway(&points);
                let words = Shape::Words { x: mx, y: my - 9.0, text: label.clone(), color: "muted".into(), size: 11.0, align: "center".into(), style: "italic".into() };
                draw_shape(p, &words, scale, at, pictures, paper);
            }
        }
        Shape::Path { points, color, width } if points.len() >= 2 => {
            let pairs: Vec<(f64, f64)> = points.chunks_exact(2).map(|q| (q[0], q[1])).collect();
            through(&pairs, &shape_paint(look, color, Some(width * scale)));
        }
        Shape::Image { x, y, width, height, picture } => {
            if let Some(shown) = pictures(picture) {
                let a = at(*x, *y);
                shown.draw(canvas, Rect::new(a.x as f64, a.y as f64, width * scale, height * scale));
            }
        }
        Shape::Words { x, y, text, color, size, align, style } => {
            if text.is_empty() || !(*size > 0.0) || !(scale > 0.0) {
                return;
            }
            // Shaped once at its size in units and drawn scaled, so zooming shapes no text
            // again; unhinted, its letters and spaces scale with it.
            let size = *size;
            let has = |word: &str| style.split_whitespace().any(|w| w == word);
            let role = if has("mono") { "mono" } else { "text" };
            let bold = has("bold");
            let font = words_font_of(p.fonts, role, size, bold);
            // Shaped as every text is (Arabic joined, Hebrew right to left), unhinted as the zoom needs.
            let shaped = p.fonts.shape_unhinted(text, role, size, bold, true);
            let width = shaped.width();
            let (_, metrics) = font.metrics();
            let a = at(*x, *y);
            canvas.save();
            canvas.translate((a.x, a.y));
            canvas.scale((scale as f32, scale as f32));
            // Centered on its point down, by its alignment across, in units.
            let baseline = -(metrics.ascent + metrics.descent) / 2.0;
            let left = -text_left(align, width);
            if has("italic") {
                // Slanted from its baseline, as a face without italics is.
                canvas.translate((left as f32, baseline));
                canvas.skew((-0.2, 0.0));
                shaped.draw(canvas, 0.0, 0.0, &shape_paint(look, color, None), 0.0);
            } else {
                shaped.draw(canvas, left, baseline as f64, &shape_paint(look, color, None), 0.0);
            }
            canvas.restore();
        }
        // An object's box draws nothing itself: the shapes after it show it; a clear is the drawing's.
        _ => {}
    }
}

/// The units a shape may draw on, left, top, right, bottom, with room for a
/// link's ends and label and a text's width, which is not measured here;
/// none for a clear, which fills everything.
fn reach(shape: &Shape) -> Option<(f64, f64, f64, f64)> {
    let around = |points: &[(f64, f64)], room: f64| {
        let (mut l, mut t, mut r, mut b) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for (x, y) in points {
            (l, t, r, b) = (l.min(*x), t.min(*y), r.max(*x), b.max(*y));
        }
        (l - room, t - room, r + room, b + room)
    };
    match shape {
        Shape::Clear { .. } => None,
        Shape::Line { x1, y1, x2, y2, width, .. } => Some(around(&[(*x1, *y1), (*x2, *y2)], *width)),
        Shape::Link { x1, y1, x2, y2, via, width, label, .. } => {
            // Its ends (an arrow, a diamond) and its label reach past its points.
            let room = 30.0 + width + label.chars().count() as f64 * 11.0;
            Some(around(&link_points(*x1, *y1, *x2, *y2, via), room))
        }
        Shape::Path { points, width, .. } => {
            let pairs: Vec<(f64, f64)> = points.chunks_exact(2).map(|q| (q[0], q[1])).collect();
            (!pairs.is_empty()).then(|| around(&pairs, *width))
        }
        Shape::Words { x, y, text, size, .. } => {
            // At most a size across each character, whatever its alignment.
            let wide = text.chars().count() as f64 * size;
            Some((x - wide, y - size, x + wide, y + size))
        }
        Shape::Ring { x, y, r, width, .. } => Some((x - r - width, y - r - width, x + r + width, y + r + width)),
        other => box_of_shape(other).map(|b| (b.x, b.y, b.x + b.width, b.y + b.height)),
    }
}

/// Draws `shapes` of a canvas of its units in `bounds`, in the view `v`: a
/// clear fills its units (the whole view on a sheet that zooms) with its
/// color, else `paper`, else the theme's panel; the grid goes after the
/// clear, first when there is none.
#[allow(clippy::too_many_arguments)]
pub fn paint(
    p: &Painter,
    bounds: Rect,
    shapes: &[Shape],
    v: Viewport,
    units: (f64, f64),
    paper: &str,
    sheet: bool,
    pictures: &dyn Fn(&str) -> Option<Picture>,
    grid: Option<&dyn Fn(&Painter)>,
) {
    let at = |x: f64, y: f64| Point::new((bounds.x + (x - v.x) * v.scale) as f32, (bounds.y + (y - v.y) * v.scale) as f32);
    // A canvas that shows whole draws only on its units, never on the margins beside them, so a
    // text near its edge is cut there; a sheet that zooms draws over its whole place.
    let origin = at(0.0, 0.0);
    let _clip = p.clip(if sheet { bounds } else { Rect::new(origin.x as f64, origin.y as f64, units.0 * v.scale, units.1 * v.scale) });
    let has_clear = shapes.iter().any(|s| matches!(s, Shape::Clear { .. }));
    let mut grid_drawn = grid.is_none();
    if let Some(grid) = grid.filter(|_| !has_clear) {
        grid(p);
        grid_drawn = true;
    }
    // What shows of the units: a shape wholly outside it is not drawn.
    let seen = (v.x, v.y, v.x + bounds.width / v.scale, v.y + bounds.height / v.scale);
    for shape in shapes {
        if reach(shape).is_some_and(|(left, top, right, bottom)| right < seen.0 || bottom < seen.1 || left > seen.2 || top > seen.3) {
            continue;
        }
        if let Shape::Clear { color } = shape {
            let named = if !color.is_empty() { color.as_str() } else if !paper.is_empty() { paper } else { "panel" };
            let start = at(0.0, 0.0);
            let place = if sheet { bounds } else { Rect::new(start.x as f64, start.y as f64, units.0 * v.scale, units.1 * v.scale) };
            p.fill(place, color_of(p.look, named), 0.0);
            if !grid_drawn {
                grid_drawn = true;
                if let Some(grid) = grid {
                    grid(p);
                }
            }
            continue;
        }
        draw_shape(p, shape, v.scale, &at, pictures, paper);
    }
}

/// Clarity's words for the zoom buttons, in the person's language.
pub fn zoom_word(button: &str) -> String {
    match button {
        "zoomOut" => locale::translate("zoomOut", "Zoom out"),
        "zoomWhole" => locale::translate("zoomWhole", "Show it whole"),
        _ => locale::translate("zoomIn", "Zoom in"),
    }
}

/// A press on a link: which, its bend points, the one in hand, where it started, where the pointer was and whether it moved.
#[derive(Debug, Clone)]
struct BendPress {
    name: String,
    via: Vec<(f64, f64)>,
    point: usize,
    start: (f64, f64),
    at_x: f64,
    at_y: f64,
    moved: bool,
}

/// A new line being drawn from a box: where it started (in whole units, the
/// box's middle from a handle), where its line shows from (the handle, or
/// where it was pressed), the box, where the pointer was pressed, where it is
/// now in units and whether it moved.
#[derive(Debug, Clone)]
struct ConnectPress {
    from: (f64, f64),
    shown_from: (f64, f64),
    from_box: UnitBox,
    at_x: f64,
    at_y: f64,
    to: (f64, f64),
    moved: bool,
}

/// A box the canvas carries itself (`on drop` without `on drag`): the box,
/// where it was pressed in whole units and on the part, how far it is
/// carried (in whole grid steps when it snaps) and whether it moved.
#[derive(Debug, Clone)]
struct CarryPress {
    from_box: UnitBox,
    from: UnitPoint,
    at_x: f64,
    at_y: f64,
    by: (f64, f64),
    moved: bool,
}

/// A box being resized by its grip: the box, where the pointer was pressed
/// on the part, its new width and height in units and whether it moved.
#[derive(Debug, Clone)]
struct ResizePress {
    from_box: UnitBox,
    at_x: f64,
    at_y: f64,
    size: (f64, f64),
    moved: bool,
}

/// A shape of a box carried by `by`: one that lies in the box moves, a link
/// keeps an end outside it where it is. Lines and text count as in it when
/// all their points are.
fn carried_shape(shape: Shape, of: &UnitBox, by: (f64, f64)) -> Shape {
    let inside = |x: f64, y: f64| x >= of.x - 0.5 && x <= of.x + of.width + 0.5 && y >= of.y - 0.5 && y <= of.y + of.height + 0.5;
    let all_inside = |points: &[f64]| points.chunks(2).all(|p| p.len() == 2 && inside(p[0], p[1]));
    let moves = match &shape {
        Shape::Link { x1, y1, x2, y2, via, .. } => {
            let (a, b) = (inside(*x1, *y1), inside(*x2, *y2));
            if !(a || b) {
                return shape;
            }
            let (x1, y1) = if a { (x1 + by.0, y1 + by.1) } else { (*x1, *y1) };
            let (x2, y2) = if b { (x2 + by.0, y2 + by.1) } else { (*x2, *y2) };
            let via = if a && b { via.chunks(2).flat_map(|p| [p[0] + by.0, p.get(1).copied().unwrap_or(0.0) + by.1]).collect() } else { via.clone() };
            let mut moved = shape.clone();
            if let Shape::Link { x1: a1, y1: b1, x2: a2, y2: b2, via: v, .. } = &mut moved {
                (*a1, *b1, *a2, *b2, *v) = (x1, y1, x2, y2, via);
            }
            return moved;
        }
        Shape::Line { x1, y1, x2, y2, .. } => all_inside(&[*x1, *y1, *x2, *y2]),
        Shape::Path { points, .. } => !points.is_empty() && all_inside(points),
        Shape::Words { x, y, .. } => inside(*x, *y),
        other => box_of_shape(other).is_some_and(|b| of.contains(&b)),
    };
    if moves {
        shape.translated(by.0, by.1)
    } else {
        shape
    }
}

/// What the kept shapes were made of: the window's change count, the box
/// carried and how far, and the link being bent with its points.
type KeptKey = (u64, Option<(UnitBox, (f64, f64))>, Option<(String, Vec<(f64, f64)>)>);

/// The shapes as shown, the boxes in them, and the shapes as painted (links ending at their boxes).
#[derive(Clone)]
struct Kept {
    shown: Rc<Vec<Shape>>,
    boxes: Rc<Vec<(UnitBox, Shape)>>,
    painted: Rc<Vec<Shape>>,
}

pub struct CanvasPart {
    base: PartBase,
    /// Where the person zoomed and panned to; none while it shows whole.
    panned: Option<Viewport>,
    /// Where the pointer was last over it, from its top left; the wheel with Ctrl zooms there.
    pointer_at: Option<(f64, f64)>,
    /// The drawing last seen, to know when it was drawn anew.
    seen: Option<Vec<Value>>,
    seen_changes: u64,
    /// A link being bent: its points as the pointer has them, until the drawing comes back with them.
    bending: Option<(String, Vec<(f64, f64)>)>,
    /// The link last pressed, whose bend points show.
    in_hand: Option<String>,
    bend_press: Option<BendPress>,
    connect_press: Option<ConnectPress>,
    /// Whether the new line being drawn came from the right button: let go without moving, it opens the menu.
    connect_right: bool,
    carry_press: Option<CarryPress>,
    resize_press: Option<ResizePress>,
    /// A box carried and let go, shown where it was put until the drawing comes back.
    carried: Option<(UnitBox, (f64, f64))>,
    /// The box whose handles show, as last drawn, to draw anew only when it changes.
    hover_seen: Option<UnitBox>,
    pressed: bool,
    press_at: (f64, f64),
    from: Option<UnitPoint>,
    view_at: Viewport,
    moved: bool,
    // Whether the drag moves the view, and whether its first run said so yet.
    moves_view: bool,
    pictures: RefCell<HashMap<String, Option<Picture>>>,
    /// The drawing as shapes, its boxes and what is painted, kept until the
    /// window changes or what is carried or bent moves: a frame that only
    /// pans or zooms reads and measures nothing again.
    kept: RefCell<Option<(KeptKey, Kept)>>,
    modes: modes::ModeState,
}

/// Makes a canvas: one its code draws, or one an engine beside the screen draws when it `runs` one.
pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    if spec.text("runs").is_some_and(|r| !r.is_empty()) {
        Box::new(EngineCanvasPart::new(spec, host))
    } else {
        Box::new(CanvasPart::new(spec, host))
    }
}

/// A canvas's width and height in its own units: `units`, 100 x 100 without them.
pub fn units_of(spec: &PartSpec) -> (f64, f64) {
    let side = |name: &str| match spec.json("units") {
        Some(Value::Object(units)) => number_of(units.get(name)).filter(|n| *n > 0.0).unwrap_or(DEFAULT_UNITS),
        _ => DEFAULT_UNITS,
    };
    (side("width"), side("height"))
}

/// What a clear without a color fills a canvas with: its state's background, else its line's; empty is the theme's panel.
pub fn paper_of(base: &PartBase) -> String {
    base.state().and_then(|s| s.background.clone()).filter(|b| !b.is_empty()).or_else(|| base.spec.text("background").map(str::to_string)).unwrap_or_default()
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn distance_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0.0, 1.0) };
    distance(p, (a.0 + t * dx, a.1 + t * dy))
}

/// A pointer as a canvas's handler reads it: `x` and `y` in its units.
pub fn pointer_at(at: UnitPoint) -> Map<String, Value> {
    let mut pointer = Map::new();
    pointer.insert("x".into(), Value::from(at.0));
    pointer.insert("y".into(), Value::from(at.1));
    pointer
}

impl CanvasPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> CanvasPart {
        CanvasPart {
            base: PartBase::new(spec, host),
            panned: None,
            pointer_at: None,
            seen: None,
            seen_changes: u64::MAX,
            bending: None,
            in_hand: None,
            bend_press: None,
            connect_press: None,
            connect_right: false,
            carry_press: None,
            resize_press: None,
            carried: None,
            hover_seen: None,
            pressed: false,
            press_at: (0.0, 0.0),
            from: None,
            view_at: Viewport { scale: 1.0, x: 0.0, y: 0.0 },
            moved: false,
            moves_view: false,
            pictures: RefCell::new(HashMap::new()),
            kept: RefCell::new(None),
            modes: modes::ModeState::default(),
        }
    }

    /// The width and height of the canvas in its own units.
    pub fn units(&self) -> (f64, f64) {
        units_of(&self.base.spec)
    }

    /// Its drawing as sent: the shapes its draw handler drew last.
    pub fn drawing_sent(&self) -> Vec<Value> {
        self.base.state().and_then(|s| s.drawing.clone()).unwrap_or_default()
    }

    /// Its drawing: the shapes its draw handler drew last that read as shapes.
    pub fn drawing(&self) -> Vec<Shape> {
        Shape::all(&self.drawing_sent())
    }

    /// Whether it zooms and pans.
    pub fn zoom(&self) -> bool {
        self.base.spec.flag("zoom")
    }

    /// Its grid in units, 0 for none: as code set it, else as its line says.
    pub fn grid(&self) -> f64 {
        self.base.state().and_then(|s| s.grid).map(|g| g as f64).or_else(|| self.base.spec.number("grid")).unwrap_or(0.0)
    }

    /// What a clear without a color fills it with; empty is the theme's panel.
    pub fn paper(&self) -> String {
        paper_of(&self.base)
    }

    pub fn dot_size(&self) -> f64 {
        self.base.state().and_then(|s| s.dot_size).map(|d| d as f64).or_else(|| self.base.spec.number("dotSize")).unwrap_or(2.0)
    }

    pub fn dot_color(&self) -> String {
        self.base.state().and_then(|s| s.dot_color.clone()).filter(|c| !c.is_empty()).or_else(|| self.base.spec.text("dotColor").map(str::to_string)).unwrap_or_default()
    }

    fn handles(&self, event: &str) -> bool {
        self.base.spec.on(event).next().is_some()
    }

    /// Why the last drawing failed; none when it drew.
    pub fn failed(&self) -> Option<String> {
        self.base.node.failed()
    }

    // -- The view ------------------------------------------------------------------

    /// What it shows: the canvas whole, fitted and centered, until the person zooms or pans.
    pub fn viewport(&self) -> Viewport {
        match self.panned.filter(|_| self.zoom()) {
            Some(v) => v,
            None => {
                let (uw, uh) = self.units();
                let b = self.base.node.bounds;
                whole(uw, uh, b.width, b.height)
            }
        }
    }

    /// The unit under a pixel of the part (from its top left), not rounded.
    fn unit_at(&self, x: f64, y: f64) -> (f64, f64) {
        let v = self.viewport();
        (v.x + x / v.scale, v.y + y / v.scale)
    }

    /// The unit under a pointer that is down, at the canvas's nearest edge beside it.
    fn held(&self, x: f64, y: f64) -> UnitPoint {
        let (ux, uy) = self.unit_at(x, y);
        let (uw, uh) = self.units();
        (edge(ux, uw), edge(uy, uh))
    }

    /// A toolbox's zoom tool: closer or farther, about the middle.
    pub(super) fn zoom_step(&mut self, closer: bool, cx: &mut Cx) {
        self.zoom_by(if closer { 1.25 } else { 1.0 / 1.25 }, None, cx);
    }

    fn zoom_by(&mut self, factor: f64, at: Option<(f64, f64)>, cx: &mut Cx) {
        let (uw, uh) = self.units();
        let b = self.base.node.bounds;
        let (ax, ay) = at.unwrap_or((b.width / 2.0, b.height / 2.0));
        self.panned = Some(zoomed(uw, uh, b.width, b.height, self.viewport(), factor, ax, ay));
        self.place_typing(cx);
        cx.invalidate();
    }

    /// The zoom buttons in its top right corner, on the screen, as the designer's: out, back to whole, in.
    pub fn zoom_buttons(&self) -> (Rect, Rect, Rect) {
        let b = self.base.node.bounds;
        let top = b.y + 6.0;
        let right = b.right() - 6.0;
        let zoom_in = Rect::new(right - 26.0, top, 26.0, 24.0);
        let back = Rect::new(zoom_in.x - 2.0 - 52.0, top, 52.0, 24.0);
        let zoom_out = Rect::new(back.x - 2.0 - 26.0, top, 26.0, 24.0);
        (zoom_out, back, zoom_in)
    }

    /// Which zoom button a point of the screen is on; none off them, or on a canvas that does not zoom.
    fn zoom_button_at(&self, x: f64, y: f64) -> Option<&'static str> {
        if !self.zoom() {
            return None;
        }
        let (zoom_out, back, zoom_in) = self.zoom_buttons();
        if zoom_out.contains(x, y) {
            Some("zoomOut")
        } else if back.contains(x, y) {
            Some("zoomWhole")
        } else if zoom_in.contains(x, y) {
            Some("zoomIn")
        } else {
            None
        }
    }

    // -- Links that bend -----------------------------------------------------------

    /// The drawing as it shows: an object dragged moved, a link being bent with the points the pointer gives it.
    pub fn shown(&self) -> Vec<Shape> {
        let carry = self.carry_press.as_ref().filter(|c| c.moved).map(|c| (c.from_box, c.by)).or(self.carried);
        self.mode_shapes(self.drawing())
            .into_iter()
            .map(|shape| match carry {
                Some((of, by)) => carried_shape(shape, &of, by),
                None => shape,
            })
            .map(|shape| match (&self.bending, shape) {
                (Some((bent, via)), Shape::Link { name, x1, y1, x2, y2, color, width, start, end, label, style, .. }) if &name == bent => {
                    Shape::Link { name, x1, y1, x2, y2, via: via.iter().flat_map(|p| [p.0.round(), p.1.round()]).collect(), color, width, start, end, label, style }
                }
                (_, shape) => shape,
            })
            .collect()
    }

    /// The shapes as shown, their boxes and the shapes as painted: kept while
    /// the window, the box carried and the link bent stay as they were; made
    /// anew every time on a canvas with a mode, whose objects move apart.
    fn kept(&self) -> Kept {
        let changes = self.base.host.changes();
        let carry = self.carry_press.as_ref().filter(|c| c.moved).map(|c| (c.from_box, c.by)).or(self.carried);
        let key: KeptKey = (changes, carry, self.bending.clone());
        let keeps = changes != 0 && self.mode().is_empty();
        if keeps {
            if let Some((at, kept)) = self.kept.borrow().as_ref() {
                if *at == key {
                    return kept.clone();
                }
            }
        }
        let shown = self.shown();
        let boxes = boxes_of(&shown, self.units());
        let edges: Vec<UnitBox> = boxes.iter().map(|(b, _)| *b).collect();
        let painted = links_to_edges(shown.clone(), &edges);
        let kept = Kept { shown: Rc::new(shown), boxes: Rc::new(boxes), painted: Rc::new(painted) };
        if keeps {
            *self.kept.borrow_mut() = Some((key, kept.clone()));
        }
        kept
    }

    /// Whether something is drawn at a unit: a box, a circle, an ellipse, a ring, a picture or an object (not the paper, lines or text).
    fn drawn_at(&self, p: (f64, f64)) -> bool {
        self.kept().shown.iter().any(|shape| shape_hit(shape, p))
    }

    /// The boxes a new line joins, the topmost first: each drawn shape grown
    /// to the shapes under it that hold it whole (a diagram's box is its
    /// outline, its body and its header). A shape over the whole paper is
    /// the paper, no box.
    fn boxes(&self) -> Rc<Vec<(UnitBox, Shape)>> {
        self.kept().boxes
    }

    /// The box at a unit: of the topmost shape drawn there, grown as `boxes` grows it.
    pub fn box_at(&self, p: (f64, f64)) -> Option<UnitBox> {
        self.boxes().iter().find(|(_, shape)| shape_hit(shape, p)).map(|(b, _)| *b)
    }

    // -- Boxes the canvas carries -----------------------------------------------------

    /// Whether it carries a box itself: it has `on drop` and no `on drag`
    /// (which gets every move), draws by code, may be used and changed.
    fn carries(&self) -> bool {
        self.handles("drop") && !self.handles("drag") && self.mode().is_empty() && self.base.enabled() && !self.base.read_only()
    }

    /// Whether a box carried moves by whole grid steps: as code set `snap`, else its line.
    fn snaps(&self) -> bool {
        self.base.state().and_then(|s| s.snap).unwrap_or_else(|| self.base.spec.flag("snap"))
    }

    /// A press on a box of a canvas that carries boxes takes it.
    fn carry_pressed(&mut self, at: &PointerInput) -> bool {
        if !self.carries() {
            return false;
        }
        let p = self.unit_at(at.x, at.y);
        let Some(from_box) = self.box_at(p) else { return false };
        self.carried = None;
        self.carry_press = Some(CarryPress { from_box, from: (p.0.floor() as i64, p.1.floor() as i64), at_x: at.x, at_y: at.y, by: (0.0, 0.0), moved: false });
        true
    }

    /// How far the pointer carried the box, in whole units, in whole grid steps when it snaps.
    fn carried_by(&self, press: &CarryPress, x: f64, y: f64) -> (f64, f64) {
        let scale = self.viewport().scale;
        let (dx, dy) = ((x - press.at_x) / scale, (y - press.at_y) / scale);
        let grid = self.grid();
        if self.snaps() && grid > 0.0 {
            ((dx / grid).round() * grid, (dy / grid).round() * grid)
        } else {
            (dx.round(), dy.round())
        }
    }

    // -- New lines between boxes -----------------------------------------------------

    /// Whether the person draws new lines on it: it has `on connect`, draws by code (no mode) or holds objects, may be used and changed.
    fn connects(&self) -> bool {
        self.handles("connect") && matches!(self.mode().as_str(), "" | "objects") && self.base.enabled() && !self.base.read_only()
    }

    /// Whether a press anywhere on a box draws a new line: code set `connecting`, or a diagram's connect tool is in hand; only with `on connect`.
    pub fn connecting(&self) -> bool {
        self.connects() && (self.base.state().and_then(|s| s.connecting).unwrap_or(false) || (self.mode() == "objects" && self.tool_in_hand() == "connect"))
    }

    /// The box whose handles show: the one under the pointer, or the one with a handle beside it.
    fn hovered_box(&self, px: f64, py: f64) -> Option<UnitBox> {
        // A diagram shows a box's handles only while its connect tool is in hand: else a press drags the box.
        if !self.connects() || (self.mode() == "objects" && !self.connecting()) {
            return None;
        }
        if let Some(found) = self.box_at(self.unit_at(px, py)) {
            return Some(found);
        }
        self.boxes().iter().map(|(b, _)| *b).find(|b| self.handle_of(b, px, py).is_some())
    }

    /// The handle of a box beside a pixel of the part, in units.
    fn handle_of(&self, of: &UnitBox, px: f64, py: f64) -> Option<(f64, f64)> {
        let v = self.viewport();
        of.handles().into_iter().find(|h| distance(((h.0 - v.x) * v.scale, (h.1 - v.y) * v.scale), (px, py)) <= HANDLE_NEAR)
    }

    /// The handle under a pixel of the part, and its box.
    fn handle_at(&self, px: f64, py: f64) -> Option<(UnitBox, (f64, f64))> {
        let of = self.hovered_box(px, py)?;
        self.handle_of(&of, px, py).map(|h| (of, h))
    }

    /// The connection handles showing, in units: those of the box under the pointer while nothing is pressed.
    pub fn connect_handles(&self) -> Vec<(f64, f64)> {
        if self.connect_press.is_some() || self.base.node.pressed || !self.base.node.hovered {
            return Vec::new();
        }
        self.pointer_at.and_then(|(px, py)| self.hovered_box(px, py)).map(|b| b.handles().to_vec()).unwrap_or_default()
    }

    /// The new line being drawn, in units: from its handle (or where it was pressed) to the pointer.
    pub fn connect_line(&self) -> Option<((f64, f64), (f64, f64))> {
        self.connect_press.as_ref().filter(|c| c.moved).map(|c| (c.shown_from, c.to))
    }

    /// The box the new line would end on: the one under the pointer, other than the one it started on.
    pub fn connect_target(&self) -> Option<UnitBox> {
        let press = self.connect_press.as_ref().filter(|c| c.moved)?;
        self.box_at(press.to).filter(|b| *b != press.from_box)
    }

    /// A press that starts a new line: on a handle, anywhere on a box with
    /// the right button or while `connecting`.
    fn connect_pressed(&mut self, at: &PointerInput, cx: &mut Cx) -> bool {
        if !self.connects() {
            return false;
        }
        let p = self.unit_at(at.x, at.y);
        let right = at.button == PointerButton::Right;
        // From a handle the line starts at its box's middle, which any hit test finds in the box; else where it was pressed.
        let start = match self.handle_at(at.x, at.y) {
            Some((of, handle)) => Some((of, of.center(), handle)),
            None if self.connecting() || right => self.box_at(p).map(|of| (of, (p.0.floor(), p.1.floor()), p)),
            None => None,
        };
        self.connect_right = right;
        let Some((from_box, from, shown_from)) = start else { return false };
        self.connect_press = Some(ConnectPress { from, shown_from, from_box, at_x: at.x, at_y: at.y, to: p, moved: false });
        cx.invalidate();
        true
    }

    // -- Boxes people resize ----------------------------------------------------------

    /// Whether people resize its boxes by their grip: it has `on resize`,
    /// draws by code (no mode), may be used and changed, and code left
    /// `resizable` true.
    pub fn resizable(&self) -> bool {
        self.handles("resize") && self.mode().is_empty() && self.base.enabled() && !self.base.read_only() && self.base.state().and_then(|s| s.resizable).unwrap_or(true)
    }

    /// Whether a pixel of the part is on the grip of a box: within a few pixels of its bottom-right corner.
    fn on_grip(&self, of: &UnitBox, px: f64, py: f64) -> bool {
        let v = self.viewport();
        let (cx, cy) = of.corner();
        ((cx - v.x) * v.scale - px).abs() <= HANDLE_NEAR && ((cy - v.y) * v.scale - py).abs() <= HANDLE_NEAR
    }

    /// The box whose grip shows: the one under the pointer, or the one whose grip is beside it.
    fn grip_box(&self, px: f64, py: f64) -> Option<UnitBox> {
        if !self.resizable() {
            return None;
        }
        let boxes: Vec<UnitBox> = self.boxes().iter().map(|(b, _)| *b).collect();
        boxes.iter().find(|b| self.on_grip(b, px, py)).copied().or_else(|| self.box_at(self.unit_at(px, py)))
    }

    /// The box whose grip is under a pixel of the part.
    fn grip_at(&self, px: f64, py: f64) -> Option<UnitBox> {
        self.grip_box(px, py).filter(|b| self.on_grip(b, px, py))
    }

    /// The resize grip showing, in units: the bottom-right corner of the box under the pointer while nothing is pressed.
    pub fn resize_grip(&self) -> Option<(f64, f64)> {
        if self.resize_press.is_some() || self.connect_press.is_some() || self.base.node.pressed || !self.base.node.hovered {
            return None;
        }
        self.pointer_at.and_then(|(px, py)| self.grip_box(px, py)).map(|b| b.corner())
    }

    /// The box being resized at its new size, in units, once the pointer moved.
    pub fn resize_outline(&self) -> Option<UnitBox> {
        self.resize_press.as_ref().filter(|r| r.moved).map(|r| UnitBox { width: r.size.0, height: r.size.1, ..r.from_box })
    }

    /// A left press on a box's grip starts resizing it.
    fn resize_pressed(&mut self, at: &PointerInput, cx: &mut Cx) -> bool {
        if at.button != PointerButton::Left {
            return false;
        }
        let Some(from_box) = self.grip_at(at.x, at.y) else { return false };
        self.resize_press = Some(ResizePress { from_box, at_x: at.x, at_y: at.y, size: (from_box.width, from_box.height), moved: false });
        cx.invalidate();
        true
    }

    /// The new width and height of the box as the pointer has them, its
    /// top-left staying: whole units, its right and bottom edges on the grid
    /// when it snaps, never less than the least size.
    fn resized_to(&self, press: &ResizePress, x: f64, y: f64) -> (f64, f64) {
        let scale = self.viewport().scale;
        let b = press.from_box;
        let (right, bottom) = (b.x + b.width + (x - press.at_x) / scale, b.y + b.height + (y - press.at_y) / scale);
        let grid = self.grid();
        let snaps = self.snaps() && grid > 0.0;
        let side = |start: f64, end: f64, least: f64| {
            let end = if snaps { (end / grid).round() * grid } else { end.round() };
            let least_end = if snaps { ((start + least) / grid).ceil() * grid } else { (start + least).ceil() };
            end.max(least_end) - start
        };
        (side(b.x, right, LEAST_SIZE.0), side(b.y, bottom, LEAST_SIZE.1))
    }

    /// The link under a unit, the topmost first: its bend point there, else a new one on the segment pressed.
    fn link_at(&self, p: (f64, f64)) -> Option<(String, Vec<(f64, f64)>, usize)> {
        let near = 8.0 / self.viewport().scale;
        for shape in self.kept().shown.iter().rev() {
            // A box drawn over the press takes it: the link under it is not pressed.
            if let Shape::Rect { x, y, width, height, .. } = shape {
                if p.0 >= *x && p.0 <= x + width && p.1 >= *y && p.1 <= y + height {
                    return None;
                }
            }
            let Shape::Link { name, x1, y1, x2, y2, via, .. } = shape else { continue };
            let all = link_points(*x1, *y1, *x2, *y2, via);
            let mut bends: Vec<(f64, f64)> = all[1..all.len() - 1].to_vec();
            if let Some(on) = bends.iter().position(|v| distance(*v, p) <= near) {
                return Some((name.clone(), bends, on));
            }
            for i in 0..all.len() - 1 {
                if distance_to_segment(p, all[i], all[i + 1]) <= near {
                    bends.insert(i, p);
                    return Some((name.clone(), bends, i));
                }
            }
        }
        None
    }

    /// The bend points of the link in hand.
    fn bend_handles(&self) -> Vec<(f64, f64)> {
        let Some(in_hand) = self.in_hand.as_ref().filter(|_| self.handles("bend")) else { return Vec::new() };
        self.shown()
            .into_iter()
            .find_map(|s| match s {
                Shape::Link { name, x1, y1, x2, y2, via, .. } if &name == in_hand => {
                    let all = link_points(x1, y1, x2, y2, &via);
                    Some(all[1..all.len() - 1].to_vec())
                }
                _ => None,
            })
            .unwrap_or_default()
    }

    fn bent(&mut self, name: String, via: Vec<(f64, f64)>, cx: &mut Cx) {
        self.seen = Some(self.drawing_sent());
        let mut at = pointer_at((0, 0));
        at.insert("bent".into(), Value::from(name.clone()));
        at.insert("via".into(), Value::Array(via.iter().flat_map(|p| [Value::from(p.0.round() as i64), Value::from(p.1.round() as i64)]).collect()));
        self.bending = Some((name, via));
        cx.invalidate();
        self.point("bend", at);
    }

    /// A double click on a bend point of the link in hand takes it away.
    fn unbend(&mut self, x: f64, y: f64, cx: &mut Cx) -> bool {
        let Some(name) = self.in_hand.clone().filter(|_| self.handles("bend")) else { return false };
        let p = self.unit_at(x, y);
        let near = 8.0 / self.viewport().scale;
        let mut handles = self.bend_handles();
        let Some(point) = handles.iter().position(|v| distance(*v, p) <= near) else { return false };
        handles.remove(point);
        self.bent(name, handles, cx);
        true
    }

    // -- The pointer ---------------------------------------------------------------

    /// Runs its handler of `event` with where it was: through the window's command, else with the pointer set in its state.
    fn point(&self, event: &str, at: Map<String, Value>) -> bool {
        if let Some(name) = self.base.name() {
            if let Some(ran) = self.base.host.pointed(name, event, &at) {
                return ran;
            }
            crate::parts::change_state(&*self.base.host, name, |s| s.pointer = Some(at.clone()));
        }
        self.base.fire(event)
    }

    /// Whether drags, drops, bends, new lines and resizes on it are edits with undo, so Ctrl+Z takes back its own as a press gives it the focus.
    fn keeps_edits(&self) -> bool {
        self.base.enabled()
            && !self.base.read_only()
            && self.base.name().is_some()
            && !self.base.spec.flag("noUndo")
            && (matches!(self.mode().as_str(), "objects" | "paint") || self.handles("drag") || self.handles("drop") || self.handles("bend") || self.handles("connect") || self.handles("resize"))
    }

    /// The window changed: a drawing drawn anew keeps a drag going and stops showing a bend of its own.
    fn heard(&mut self) {
        let changes = self.base.host.changes();
        if changes == self.seen_changes {
            return;
        }
        self.seen_changes = changes;
        let drawing = self.drawing_sent();
        if self.seen.as_ref() != Some(&drawing) {
            self.seen = Some(drawing);
            self.bending = None;
            self.carried = None;
            self.drawing_changed();
        }
    }

    // -- Drawing ---------------------------------------------------------------------

    /// A picture of the resources folder by its name, loaded once.
    fn picture(&self, name: &str) -> Option<Picture> {
        if let Some(kept) = self.pictures.borrow().get(name) {
            return kept.clone();
        }
        let loaded = self.base.host.resource(name).and_then(|bytes| Picture::read(name, &bytes));
        self.pictures.borrow_mut().insert(name.to_string(), loaded.clone());
        loaded
    }

    /// The pixel on the screen of a unit, in the view `v`.
    fn at(&self, v: Viewport, x: f64, y: f64) -> Point {
        let b = self.base.node.bounds;
        Point::new((b.x + (x - v.x) * v.scale) as f32, (b.y + (y - v.y) * v.scale) as f32)
    }

    /// The grid's dots over the whole view, the same size whatever the zoom, thinned to 8 pixels apart or more.
    fn draw_grid(&self, p: &Painter, v: Viewport) {
        let every = grid_every(self.grid(), v.scale);
        let dot_color = self.dot_color();
        let mut dot = Paint::default();
        dot.set_color(color_of(p.look, if dot_color.is_empty() { "border" } else { &dot_color }));
        dot.set_anti_alias(true);
        let r = (self.dot_size().max(1.0) / 2.0) as f32;
        let b = self.base.node.bounds;
        let right = v.x + b.width / v.scale;
        let bottom = v.y + b.height / v.scale;
        let mut x = (v.x / every).ceil() * every;
        while x <= right {
            let mut y = (v.y / every).ceil() * every;
            while y <= bottom {
                p.canvas.draw_circle(self.at(v, x, y), r, &dot);
                y += every;
            }
            x += every;
        }
    }

    /// The zoom buttons, as the designer's: out, the zoom it is at (back to whole), in.
    fn draw_zoom_buttons(&self, p: &Painter) {
        let look = p.look;
        let (zoom_out, back, zoom_in) = self.zoom_buttons();
        let percent = format!("{}%", (self.viewport().scale * 100.0).round());
        for (place, label) in [(zoom_out, "-"), (back, percent.as_str()), (zoom_in, "+")] {
            p.fill(place, look.color("panel"), look.radius("button"));
            p.stroke(place, look.color("border"), look.border(), look.radius("button"));
            p.text(label, place, look.color("text"), look.size("small"), "text", false, TextAlign::Center);
        }
    }

    fn draw_canvas(&self, p: &Painter) {
        let v = self.viewport();
        if !(v.scale > 0.0) || !v.scale.is_finite() {
            return;
        }
        let shapes = self.kept().painted;
        let grid = |p: &Painter| self.draw_grid(p, v);
        let pictures = |name: &str| self.picture(name);
        let grid: Option<&dyn Fn(&Painter)> = if self.grid() > 0.0 { Some(&grid) } else { None };
        paint(p, self.base.node.bounds, &shapes, v, self.units(), &self.paper(), self.zoom(), &pictures, grid);
        self.draw_mode(p, v);
        // The bend points of the link in hand: drag one to move it, double-click it to take it away.
        let mut face = Paint::default();
        face.set_color(p.look.color("panel"));
        face.set_anti_alias(true);
        let mut rim = Paint::default();
        rim.set_color(p.look.color("accent"));
        rim.set_anti_alias(true);
        rim.set_style(PaintStyle::Stroke);
        rim.set_stroke_width(1.5);
        for handle in self.bend_handles() {
            let at = self.at(v, handle.0, handle.1);
            p.canvas.draw_circle(at, 5.0, &face);
            p.canvas.draw_circle(at, 5.0, &rim);
        }
        self.draw_connect(p, v);
        self.draw_resize(p, v);
        if self.zoom() {
            self.draw_zoom_buttons(p);
        }
    }

    /// Over the drawing, never in it: the handles of the box under the
    /// pointer, or the new line being drawn (dashed, in the accent) and the
    /// box it would end on outlined.
    fn draw_connect(&self, p: &Painter, v: Viewport) {
        let accent = p.look.color("accent");
        if let Some((from, to)) = self.connect_line() {
            if let Some(target) = self.connect_target() {
                let mut outline = Paint::default();
                outline.set_color(accent);
                outline.set_anti_alias(true);
                outline.set_style(PaintStyle::Stroke);
                outline.set_stroke_width(2.0);
                let (a, b) = (self.at(v, target.x, target.y), self.at(v, target.x + target.width, target.y + target.height));
                p.canvas.draw_rect(skia_safe::Rect::new(a.x - 2.0, a.y - 2.0, b.x + 2.0, b.y + 2.0), &outline);
            }
            let mut line = Paint::default();
            line.set_color(accent);
            line.set_anti_alias(true);
            line.set_style(PaintStyle::Stroke);
            line.set_stroke_width(2.0);
            line.set_path_effect(skia_safe::PathEffect::dash(&[6.0, 4.0], 0.0));
            p.canvas.draw_line(self.at(v, from.0, from.1), self.at(v, to.0, to.1), &line);
            return;
        }
        let mut dot = Paint::default();
        dot.set_color(accent);
        dot.set_anti_alias(true);
        let mut rim = Paint::default();
        rim.set_color(p.look.color("panel"));
        rim.set_anti_alias(true);
        rim.set_style(PaintStyle::Stroke);
        rim.set_stroke_width(1.5);
        for handle in self.connect_handles() {
            let at = self.at(v, handle.0, handle.1);
            p.canvas.draw_circle(at, 4.5, &dot);
            p.canvas.draw_circle(at, 4.5, &rim);
        }
    }
}

impl CanvasPart {
    /// Over the drawing, never in it: the grip of the box under the pointer
    /// (a small square in the accent), or the box being resized at its new
    /// size, outlined dashed in the accent.
    fn draw_resize(&self, p: &Painter, v: Viewport) {
        let accent = p.look.color("accent");
        if let Some(outline) = self.resize_outline() {
            let mut line = Paint::default();
            line.set_color(accent);
            line.set_anti_alias(true);
            line.set_style(PaintStyle::Stroke);
            line.set_stroke_width(2.0);
            line.set_path_effect(skia_safe::PathEffect::dash(&[6.0, 4.0], 0.0));
            let (a, b) = (self.at(v, outline.x, outline.y), self.at(v, outline.x + outline.width, outline.y + outline.height));
            p.canvas.draw_rect(skia_safe::Rect::new(a.x, a.y, b.x, b.y), &line);
            return;
        }
        let Some((x, y)) = self.resize_grip() else { return };
        let at = self.at(v, x, y);
        let half = (GRIP_SIZE / 2.0) as f32;
        let square = skia_safe::Rect::new(at.x - half, at.y - half, at.x + half, at.y + half);
        let mut fill = Paint::default();
        fill.set_color(accent);
        fill.set_anti_alias(true);
        p.canvas.draw_rect(square, &fill);
        let mut rim = Paint::default();
        rim.set_color(p.look.color("panel"));
        rim.set_anti_alias(true);
        rim.set_style(PaintStyle::Stroke);
        rim.set_stroke_width(1.5);
        p.canvas.draw_rect(square, &rim);
    }
}

impl Element for CanvasPart {
    part_element!();

    fn focusable(&self) -> bool {
        if self.mode() == "vector" {
            self.acts()
        } else {
            self.keeps_edits()
        }
    }

    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.heard();
        self.place_typing(cx);
        self.follow_toolbox(cx);
    }

    /// Leaving the screen, its toolbox goes with it.
    fn on_detached(&mut self, cx: &mut Cx) {
        self.modes.toolbox.close(cx);
    }

    /// A picture the toolbox asked to import, chosen by the person, comes back here.
    fn on_later(&mut self, done: crate::later::Done, cx: &mut Cx) {
        if let Ok(imported) = done.downcast::<modes::Imported>() {
            if let Some(bytes) = imported.0 {
                self.import(&bytes, cx);
            }
        }
    }

    fn on_wheel(&mut self, dx: f64, dy: f64, held: Modifiers, cx: &mut Cx) -> bool {
        if !self.zoom() {
            return false;
        }
        // The wheel with Ctrl zooms, closer as it turns up, at the pointer; without, it pans.
        if held.has(Modifiers::CONTROL) && dy != 0.0 {
            self.zoom_by(if dy < 0.0 { 1.2 } else { 1.0 / 1.2 }, self.pointer_at, cx);
            return true;
        }
        self.panned = Some(scrolled(self.viewport(), dx, dy));
        self.place_typing(cx);
        cx.invalidate();
        true
    }

    /// A zoom button says what it does when the pointer rests on it.
    fn tooltip_at(&self, x: f64, y: f64) -> Option<String> {
        self.zoom_button_at(x, y).map(zoom_word)
    }

    fn on_pointer_pressed(&mut self, at: PointerInput, cx: &mut Cx) {
        self.heard();
        if !self.base.enabled() {
            return;
        }
        if let Some(button) = self.zoom_button_at(at.screen_x, at.screen_y).filter(|_| at.button == PointerButton::Left) {
            if button == "zoomWhole" {
                self.panned = None;
            } else {
                self.zoom_by(if button == "zoomIn" { 1.2 } else { 1.0 / 1.2 }, None, cx);
            }
            cx.invalidate();
            return;
        }
        if self.resize_pressed(&at, cx) {
            return;
        }
        if self.connect_pressed(&at, cx) {
            return;
        }
        // A right press that draws no line waits for its release, which opens the menu.
        if at.button == PointerButton::Right {
            return;
        }
        if at.clicks >= 2 && self.unbend(at.x, at.y, cx) {
            return;
        }
        if self.mode_pressed(at, cx) {
            return;
        }
        if self.carry_pressed(&at) {
            return;
        }
        if self.handles("bend") {
            if let Some((name, via, point)) = self.link_at(self.unit_at(at.x, at.y)) {
                self.in_hand = Some(name.clone());
                let start = via[point];
                self.bend_press = Some(BendPress { name, via, point, start, at_x: at.x, at_y: at.y, moved: false });
                cx.invalidate();
                return;
            }
            self.in_hand = None;
        }
        self.pressed = self.handles("click") || self.handles("drag") || self.zoom();
        if !self.pressed {
            return;
        }
        self.press_at = (at.x, at.y);
        self.view_at = self.viewport();
        let (uw, uh) = self.units();
        self.from = point_in(uw, uh, self.view_at, at.x, at.y);
        self.moved = false;
        // What a drag does is known where it starts: on something drawn it is the
        // drag handler's, on empty paper it moves the view of a canvas that zooms.
        self.moves_view = !self.handles("drag") || (self.zoom() && !(self.from.is_some() && self.drawn_at(self.unit_at(at.x, at.y))));
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer_at = Some((at.x, at.y));
        if !self.base.node.pressed {
            // The handles and the grip follow the pointer from box to box.
            let hovered = self.hovered_box(at.x, at.y).or_else(|| self.grip_box(at.x, at.y));
            if hovered != self.hover_seen {
                self.hover_seen = hovered;
                cx.invalidate();
            }
            return;
        }
        self.heard();
        if let Some(press) = self.resize_press.clone() {
            if !press.moved && distance((at.x, at.y), (press.at_x, press.at_y)) < STILL {
                return;
            }
            let size = self.resized_to(&press, at.x, at.y);
            if let Some(resizing) = self.resize_press.as_mut() {
                resizing.moved = true;
                resizing.size = size;
            }
            cx.invalidate();
            return;
        }
        if let Some(press) = self.connect_press.as_mut() {
            if !press.moved && distance((at.x, at.y), (press.at_x, press.at_y)) < STILL {
                return;
            }
            press.moved = true;
            let v = self.viewport();
            if let Some(press) = self.connect_press.as_mut() {
                press.to = (v.x + at.x / v.scale, v.y + at.y / v.scale);
            }
            cx.invalidate();
            return;
        }
        if self.mode_moved(at, cx) {
            return;
        }
        if let Some(press) = self.carry_press.clone() {
            if !press.moved && distance((at.x, at.y), (press.at_x, press.at_y)) < STILL {
                return;
            }
            let by = self.carried_by(&press, at.x, at.y);
            if let Some(carrying) = self.carry_press.as_mut() {
                carrying.moved = true;
                if carrying.by != by {
                    carrying.by = by;
                    cx.invalidate();
                }
            }
            return;
        }
        if let Some(press) = self.bend_press.as_mut() {
            if !press.moved && distance((at.x, at.y), (press.at_x, press.at_y)) < STILL {
                return;
            }
            press.moved = true;
            let press = press.clone();
            let scale = self.viewport().scale;
            let mut via = press.via.clone();
            via[press.point] = (press.start.0 + (at.x - press.at_x) / scale, press.start.1 + (at.y - press.at_y) / scale);
            self.bending = Some((press.name, via));
            cx.invalidate();
            return;
        }
        if !self.pressed {
            return;
        }
        if !self.moved && distance((at.x, at.y), self.press_at) < STILL {
            return;
        }
        self.moved = true;
        if !self.moves_view && self.handles("drag") {
            if let Some(from) = self.from {
                let mut pointer = pointer_at(self.held(at.x, at.y));
                pointer.insert("fromX".into(), Value::from(from.0));
                pointer.insert("fromY".into(), Value::from(from.1));
                self.point("drag", pointer);
            }
        }
        if self.moves_view && self.zoom() {
            self.panned = Some(scrolled(self.view_at, self.press_at.0 - at.x, self.press_at.1 - at.y));
            cx.invalidate();
        }
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        if let Some(press) = self.resize_press.take() {
            cx.invalidate();
            // Let go at its size, nothing runs; else the handler reads where the corner was and is now.
            let b = press.from_box;
            if press.moved && press.size != (b.width, b.height) {
                let (was_x, was_y) = b.corner();
                let mut pointer = pointer_at(((b.x + press.size.0).round() as i64, (b.y + press.size.1).round() as i64));
                pointer.insert("fromX".into(), Value::from(was_x.round() as i64));
                pointer.insert("fromY".into(), Value::from(was_y.round() as i64));
                self.point("resize", pointer);
            }
            return;
        }
        if let Some(press) = self.connect_press.take() {
            cx.invalidate();
            // A line let go where it was pressed is none; else the handler decides what it connects.
            if press.moved {
                let mut pointer = pointer_at(self.held(at.x, at.y));
                pointer.insert("fromX".into(), Value::from(press.from.0 as i64));
                pointer.insert("fromY".into(), Value::from(press.from.1 as i64));
                self.point("connect", pointer);
            }
            return;
        }
        if self.mode_released(at, cx) {
            return;
        }
        if let Some(press) = self.carry_press.take() {
            cx.invalidate();
            // Let go where it was, it stays; else the drop handler puts it where it was carried.
            if press.moved && press.by != (0.0, 0.0) {
                self.carried = Some((press.from_box, press.by));
                self.seen = Some(self.drawing_sent());
                let mut pointer = pointer_at((press.from.0 + press.by.0 as i64, press.from.1 + press.by.1 as i64));
                pointer.insert("fromX".into(), Value::from(press.from.0));
                pointer.insert("fromY".into(), Value::from(press.from.1));
                self.point("drop", pointer);
            }
            return;
        }
        if let Some(bent) = self.bend_press.take() {
            if bent.moved {
                if let Some((_, via)) = self.bending.clone() {
                    self.bent(bent.name, via, cx);
                }
            }
            return;
        }
        if !self.pressed {
            return;
        }
        self.pressed = false;
        let (uw, uh) = self.units();
        if self.moved {
            if !self.moves_view && self.handles("drag") && self.handles("drop") {
                if let Some(from) = self.from {
                    let mut pointer = pointer_at(self.held(at.x, at.y));
                    pointer.insert("fromX".into(), Value::from(from.0));
                    pointer.insert("fromY".into(), Value::from(from.1));
                    self.point("drop", pointer);
                }
            }
            return;
        }
        let b = self.base.node.bounds;
        if at.x < 0.0 || at.y < 0.0 || at.x > b.width || at.y > b.height || !self.handles("click") {
            return;
        }
        if let Some(point) = point_in(uw, uh, self.viewport(), at.x, at.y) {
            self.point("click", pointer_at(point));
        }
    }

    /// The right button draws a new line from a box on a canvas with `on
    /// connect`; let go without moving, or on a canvas without, it opens
    /// the canvas's menu with the pointer where it was pressed.
    fn right_drags(&self, _x: f64, _y: f64) -> bool {
        self.base.enabled() && (self.connects() || self.base.spec.menu.is_some())
    }

    fn on_right_released(&mut self, at: PointerInput, cx: &mut Cx) -> bool {
        let press = self.connect_press.take();
        cx.invalidate();
        if let Some(press) = press.as_ref().filter(|p| p.moved) {
            let mut pointer = pointer_at(self.held(at.x, at.y));
            pointer.insert("fromX".into(), Value::from(press.from.0 as i64));
            pointer.insert("fromY".into(), Value::from(press.from.1 as i64));
            self.point("connect", pointer);
            return true;
        }
        // The menu's handler finds where it was opened as the canvas's x and y, and the
        // line there as `canvas.bent` (empty on a box or the paper), which it takes in hand.
        let (x, y) = press.map(|p| (p.at_x, p.at_y)).unwrap_or((at.x, at.y));
        // The line's own points: a right click adds no bend point.
        let line = self.link_at(self.unit_at(x, y)).map(|(name, _, _)| {
            let via = self.shown().iter().find_map(|s| match s {
                Shape::Link { name: n, x1, y1, x2, y2, via, .. } if *n == name => {
                    let all = link_points(*x1, *y1, *x2, *y2, via);
                    Some(all[1..all.len() - 1].to_vec())
                }
                _ => None,
            });
            (name, via.unwrap_or_default())
        });
        self.in_hand = line.as_ref().map(|(name, _)| name.clone());
        if let (Some(name), Some(point)) = (self.base.name(), point_in(self.units().0, self.units().1, self.viewport(), x, y)) {
            let mut pointer = pointer_at(point);
            pointer.insert("bent".into(), Value::from(line.as_ref().map(|(n, _)| n.clone()).unwrap_or_default()));
            let via = line.map(|(_, via)| via).unwrap_or_default();
            pointer.insert("via".into(), Value::Array(via.iter().flat_map(|p| [Value::from(p.0.round() as i64), Value::from(p.1.round() as i64)]).collect()));
            crate::parts::change_state(&*self.base.host, name, |s| s.pointer = Some(pointer));
        }
        false
    }

    fn on_text(&mut self, text: &str, cx: &mut Cx) {
        self.mode_text(text, cx);
    }

    fn on_key(&mut self, key: crate::input::KeyInput, cx: &mut Cx) -> bool {
        self.mode_key(key, cx)
    }

    fn on_focus_changed(&mut self, focused: bool, cx: &mut Cx) {
        // A text being typed is placed when the canvas loses the keyboard.
        if !focused {
            self.end_typing(true, cx);
        }
    }

    /// A drawing that fails says why in its place, in a red outline, and the
    /// window draws on (the screen draws every element fail-safe).
    fn draw(&self, p: &Painter) {
        let _clip = p.clip(self.base.node.bounds);
        self.draw_canvas(p);
    }

    /// The hand over its zoom buttons, its toolbar's buttons while the person
    /// may use them and the whole of a canvas they click; the I-beam over the
    /// text being typed.
    fn cursor_at(&self, x: f64, y: f64) -> Option<Cursor> {
        if !self.base.enabled() {
            return None;
        }
        if self.zoom_button_at(x, y).is_some() {
            return Some(Cursor::Hand);
        }
        if self.acts() && self.toolbar_drawn().iter().any(|(_, b)| b.contains(x, y)) {
            return Some(Cursor::Hand);
        }
        if self.typing_editor().is_some_and(|e| e.node().bounds.contains(x, y)) {
            return Some(Cursor::Text);
        }
        // The resize cursor over a box's grip and while resizing one.
        if self.resize_press.is_some() {
            return Some(Cursor::ResizeCorner);
        }
        if self.resizable() {
            let b = self.base.node.bounds;
            if self.grip_at(x - b.x, y - b.y).is_some() {
                return Some(Cursor::ResizeCorner);
            }
        }
        // The crosshair where a press starts a new line: a handle, or any box while `connecting`.
        if self.connects() {
            let b = self.base.node.bounds;
            let (px, py) = (x - b.x, y - b.y);
            if self.connect_press.is_some() || self.handle_at(px, py).is_some() || (self.connecting() && self.box_at(self.unit_at(px, py)).is_some()) {
                return Some(Cursor::Crosshair);
            }
        }
        // The move cursor over a box it carries, and while carrying one.
        if self.carry_press.is_some() {
            return Some(Cursor::Move);
        }
        if self.carries() {
            let b = self.base.node.bounds;
            if self.box_at(self.unit_at(x - b.x, y - b.y)).is_some() {
                return Some(Cursor::Move);
            }
        }
        (self.handles("click") && !self.base.read_only()).then_some(Cursor::Hand)
    }

    /// The pointer was taken away mid-gesture: a drag, a bend or a shape being
    /// drawn is dropped, not sent, and what moved shows where it was.
    fn on_pointer_cancelled(&mut self, cx: &mut Cx) {
        self.pressed = false;
        self.moved = false;
        self.bend_press = None;
        self.bending = None;
        self.connect_press = None;
        self.carry_press = None;
        self.resize_press = None;
        self.mode_cancelled();
        cx.invalidate();
    }

    /// While a text is typed the keyboard types: an input method composes in it.
    fn typing(&self) -> bool {
        self.typing_editor().is_some()
    }

    fn caret_area(&self) -> Option<Rect> {
        self.typing_editor().and_then(|e| e.caret_area())
    }

    fn on_compose(&mut self, composing: Option<crate::element::Composition>, cx: &mut Cx) {
        self.typing_compose(composing, cx);
    }
}

impl Part for CanvasPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    /// A canvas grows into the height its window has left, on every stack.
    fn grows(&self) -> bool {
        true
    }
}
