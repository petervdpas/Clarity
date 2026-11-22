//! A canvas with `on connect`: the box under the pointer, its handles, the
//! line the person draws to another box, and the line drawn over the drawing.

use super::*;

impl CanvasPart {
    /// Whether the person draws new lines on it: it has `on connect`, draws by code (no mode) or holds objects, may be used and changed.
    pub(super) fn connects(&self) -> bool {
        self.handles("connect") && matches!(self.mode().as_str(), "" | "objects") && self.base.enabled() && !self.base.read_only()
    }

    /// Whether a press anywhere on a box draws a new line: code set `connecting`, or a diagram's connect tool is in hand; only with `on connect`.
    pub fn connecting(&self) -> bool {
        self.connects() && (self.base.state().and_then(|s| s.connecting).unwrap_or(false) || (self.mode() == "objects" && self.tool_in_hand() == "connect"))
    }

    /// The box whose handles show: the one under the pointer, or the one with a handle beside it.
    pub(super) fn hovered_box(&self, px: f64, py: f64) -> Option<UnitBox> {
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
    pub(super) fn handle_of(&self, of: &UnitBox, px: f64, py: f64) -> Option<(f64, f64)> {
        let v = self.viewport();
        of.handles().into_iter().find(|h| distance(((h.0 - v.x) * v.scale, (h.1 - v.y) * v.scale), (px, py)) <= HANDLE_NEAR)
    }

    /// The handle under a pixel of the part, and its box.
    pub(super) fn handle_at(&self, px: f64, py: f64) -> Option<(UnitBox, (f64, f64))> {
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
    pub(super) fn connect_pressed(&mut self, at: &PointerInput, cx: &mut Cx) -> bool {
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

    /// Over the drawing, never in it: the handles of the box under the
    /// pointer, or the new line being drawn (dashed, in the accent) and the
    /// box it would end on outlined.
    pub(super) fn draw_connect(&self, p: &Painter, v: Viewport) {
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
