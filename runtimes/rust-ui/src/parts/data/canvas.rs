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
//! and its points. With a mode the person drags its objects, paints or draws
//! (`canvas/modes.rs`). A canvas that `runs` an engine beside the screen is
//! drawn by it (`canvas/engine.rs`). The window runs its handlers through
//! its host (`PartHost::pointed`, `acted`).

pub mod engine;
pub mod drawing;
mod modes;

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
use crate::input::{Cursor, Modifiers, PointerInput};
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
    Words { x: f64, y: f64, text: String, color: String, size: f64, align: String },
    Image { x: f64, y: f64, width: f64, height: f64, picture: String },
    /// A line people bend: from x1, y1 to x2, y2 through its bend points `via` (x, y, x, y, ...).
    Link { name: String, x1: f64, y1: f64, x2: f64, y2: f64, via: Vec<f64>, color: String, width: f64 },
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
                Shape::Words { x: n("x")?, y: n("y")?, text, color: t("color", "text"), size: or("size", 10.0), align: t("align", "center") }
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
    let mut font = fonts.font("text", size, false);
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

/// A line through `points`; one point is a dot of its round cap.
fn polyline(canvas: &skia_safe::Canvas, points: &[Point], paint: &Paint) {
    if points.is_empty() {
        return;
    }
    canvas.draw_points(if points.len() == 1 { PointMode::Points } else { PointMode::Polygon }, points, paint);
}

/// Draws one shape of a drawing, in its units, `at` giving each unit's pixel.
pub fn draw_shape(p: &Painter, shape: &Shape, scale: f64, at: &dyn Fn(f64, f64) -> Point, pictures: &dyn Fn(&str) -> Option<Picture>) {
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
        Shape::Link { x1, y1, x2, y2, via, color, width, .. } => {
            through(&link_points(*x1, *y1, *x2, *y2, via), &shape_paint(look, color, Some(width * scale)));
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
        Shape::Words { x, y, text, color, size, align } => {
            let size = size * scale;
            if text.is_empty() || !(size > 0.0) {
                return;
            }
            let font = words_font(p.fonts, size);
            // Shaped as every text is (Arabic joined, Hebrew right to left), unhinted as the zoom needs.
            let shaped = p.fonts.shape_unhinted(text, "text", size, false, true);
            let width = shaped.width();
            let (_, metrics) = font.metrics();
            let a = at(*x, *y);
            // Centered on its point down, by its alignment across.
            let baseline = a.y - (metrics.ascent + metrics.descent) / 2.0;
            shaped.draw(canvas, a.x as f64 - text_left(align, width), baseline as f64, &shape_paint(look, color, None), 0.0);
        }
        // An object's box draws nothing itself: the shapes after it show it; a clear is the drawing's.
        _ => {}
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
    for shape in shapes {
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
        draw_shape(p, shape, v.scale, &at, pictures);
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
    pressed: bool,
    press_at: (f64, f64),
    from: Option<UnitPoint>,
    view_at: Viewport,
    moved: bool,
    // Whether the drag moves the view, and whether its first run said so yet.
    moves_view: bool,
    decided: bool,
    pictures: RefCell<HashMap<String, Option<Picture>>>,
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
    base.state().and_then(|s| s.background).filter(|b| !b.is_empty()).or_else(|| base.spec.text("background").map(str::to_string)).unwrap_or_default()
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
            pressed: false,
            press_at: (0.0, 0.0),
            from: None,
            view_at: Viewport { scale: 1.0, x: 0.0, y: 0.0 },
            moved: false,
            moves_view: false,
            decided: false,
            pictures: RefCell::new(HashMap::new()),
            modes: modes::ModeState::default(),
        }
    }

    /// The width and height of the canvas in its own units.
    pub fn units(&self) -> (f64, f64) {
        units_of(&self.base.spec)
    }

    /// Its drawing as sent: the shapes its draw handler drew last.
    pub fn drawing_sent(&self) -> Vec<Value> {
        self.base.state().and_then(|s| s.drawing).unwrap_or_default()
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
        self.base.state().and_then(|s| s.dot_color).filter(|c| !c.is_empty()).or_else(|| self.base.spec.text("dotColor").map(str::to_string)).unwrap_or_default()
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
        self.mode_shapes(self.drawing())
            .into_iter()
            .map(|shape| match (&self.bending, shape) {
                (Some((bent, via)), Shape::Link { name, x1, y1, x2, y2, color, width, .. }) if &name == bent => {
                    Shape::Link { name, x1, y1, x2, y2, via: via.iter().flat_map(|p| [p.0.round(), p.1.round()]).collect(), color, width }
                }
                (_, shape) => shape,
            })
            .collect()
    }

    /// The link under a unit, the topmost first: its bend point there, else a new one on the segment pressed.
    fn link_at(&self, p: (f64, f64)) -> Option<(String, Vec<(f64, f64)>, usize)> {
        let near = 8.0 / self.viewport().scale;
        for shape in self.shown().iter().rev() {
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

    /// Whether drags, drops and bends on it are edits with undo, so Ctrl+Z takes back its own as a press gives it the focus.
    fn keeps_edits(&self) -> bool {
        self.base.enabled()
            && !self.base.read_only()
            && self.base.name().is_some()
            && !self.base.spec.flag("noUndo")
            && (matches!(self.mode().as_str(), "objects" | "paint") || self.handles("drag") || self.handles("drop") || self.handles("bend"))
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
        let shapes = self.shown();
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
        if self.zoom() {
            self.draw_zoom_buttons(p);
        }
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
        if let Some(button) = self.zoom_button_at(at.screen_x, at.screen_y) {
            if button == "zoomWhole" {
                self.panned = None;
            } else {
                self.zoom_by(if button == "zoomIn" { 1.2 } else { 1.0 / 1.2 }, None, cx);
            }
            cx.invalidate();
            return;
        }
        if at.clicks >= 2 && self.unbend(at.x, at.y, cx) {
            return;
        }
        if self.mode_pressed(at, cx) {
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
        self.moves_view = !self.handles("drag");
        self.decided = self.moves_view;
    }

    fn on_pointer_moved(&mut self, at: PointerInput, cx: &mut Cx) {
        self.pointer_at = Some((at.x, at.y));
        if !self.base.node.pressed {
            return;
        }
        self.heard();
        if self.mode_moved(at, cx) {
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
                let before = self.drawing_sent();
                let mut pointer = pointer_at(self.held(at.x, at.y));
                pointer.insert("fromX".into(), Value::from(from.0));
                pointer.insert("fromY".into(), Value::from(from.1));
                self.point("drag", pointer);
                // The first run says what the drag does: a drawing that stayed the same moves the view.
                if !self.decided {
                    self.decided = true;
                    self.moves_view = self.zoom() && before == self.drawing_sent();
                }
            }
        }
        if self.moves_view && self.zoom() {
            self.panned = Some(scrolled(self.view_at, self.press_at.0 - at.x, self.press_at.1 - at.y));
            cx.invalidate();
        }
    }

    fn on_pointer_released(&mut self, at: PointerInput, cx: &mut Cx) {
        if self.mode_released(at, cx) {
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
        (self.handles("click") && !self.base.read_only()).then_some(Cursor::Hand)
    }

    /// The pointer was taken away mid-gesture: a drag, a bend or a shape being
    /// drawn is dropped, not sent, and what moved shows where it was.
    fn on_pointer_cancelled(&mut self, cx: &mut Cx) {
        self.pressed = false;
        self.moved = false;
        self.bend_press = None;
        self.bending = None;
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
