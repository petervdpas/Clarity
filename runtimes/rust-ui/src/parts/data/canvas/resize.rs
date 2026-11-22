//! A canvas with `on resize`: the grip of the box under the pointer and the
//! outline of its new size.

use super::*;

impl CanvasPart {
    /// Whether people resize its boxes by their grip: it has `on resize`,
    /// draws by code (no mode), may be used and changed, and code left
    /// `resizable` true.
    pub fn resizable(&self) -> bool {
        self.handles("resize") && self.mode().is_empty() && self.base.enabled() && !self.base.read_only() && self.base.state().and_then(|s| s.resizable).unwrap_or(true)
    }

    /// Whether a pixel of the part is on the grip of a box: within a few pixels of its bottom-right corner.
    pub(super) fn on_grip(&self, of: &UnitBox, px: f64, py: f64) -> bool {
        let v = self.viewport();
        let (cx, cy) = of.corner();
        ((cx - v.x) * v.scale - px).abs() <= HANDLE_NEAR && ((cy - v.y) * v.scale - py).abs() <= HANDLE_NEAR
    }

    /// The box whose grip shows: the one under the pointer, or the one whose grip is beside it.
    pub(super) fn grip_box(&self, px: f64, py: f64) -> Option<UnitBox> {
        if !self.resizable() {
            return None;
        }
        let boxes: Vec<UnitBox> = self.boxes().iter().map(|(b, _)| *b).collect();
        boxes.iter().find(|b| self.on_grip(b, px, py)).copied().or_else(|| self.box_at(self.unit_at(px, py)))
    }

    /// The box whose grip is under a pixel of the part.
    pub(super) fn grip_at(&self, px: f64, py: f64) -> Option<UnitBox> {
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
    pub(super) fn resize_pressed(&mut self, at: &PointerInput, cx: &mut Cx) -> bool {
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
    pub(super) fn resized_to(&self, press: &ResizePress, x: f64, y: f64) -> (f64, f64) {
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
}
