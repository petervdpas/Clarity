//! A canvas drawn by code (manual/parts.graphics.md, conformance/drawing.json): the shapes
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
mod shapes;
mod view;
mod bend;
mod carry;
mod connect;
mod resize;
mod zoom;
pub mod layers;
pub mod toolbox;

pub use self::{shapes::*, view::*};

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

    // -- New lines between boxes -----------------------------------------------------

    // -- Boxes people resize ----------------------------------------------------------

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
