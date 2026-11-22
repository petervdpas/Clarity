//! A canvas with `zoom`: zooming by the wheel and its corner buttons, and the
//! grid's dots over the view.

use super::*;

impl CanvasPart {
    pub(super) fn zoom_by(&mut self, factor: f64, at: Option<(f64, f64)>, cx: &mut Cx) {
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
    pub(super) fn zoom_button_at(&self, x: f64, y: f64) -> Option<&'static str> {
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

    /// The zoom buttons, as the designer's: out, the zoom it is at (back to whole), in.
    pub(super) fn draw_zoom_buttons(&self, p: &Painter) {
        let look = p.look;
        let (zoom_out, back, zoom_in) = self.zoom_buttons();
        let percent = format!("{}%", (self.viewport().scale * 100.0).round());
        for (place, label) in [(zoom_out, "-"), (back, percent.as_str()), (zoom_in, "+")] {
            p.fill(place, look.color("panel"), look.radius("button"));
            p.stroke(place, look.color("border"), look.border(), look.radius("button"));
            p.text(label, place, look.color("text"), look.size("small"), "text", false, TextAlign::Center);
        }
    }

    /// The grid's dots over the whole view, the same size whatever the zoom, thinned to 8 pixels apart or more.
    pub(super) fn draw_grid(&self, p: &Painter, v: Viewport) {
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
}
