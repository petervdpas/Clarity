//! Carrying a canvas's boxes: which it carries, snapping to its grid, and
//! where a carried box goes.

use super::*;

impl CanvasPart {
    /// Whether it carries a box itself: it has `on drop` and no `on drag`
    /// (which gets every move), draws by code, may be used and changed.
    pub(super) fn carries(&self) -> bool {
        self.handles("drop") && !self.handles("drag") && self.mode().is_empty() && self.base.enabled() && !self.base.read_only()
    }

    /// Whether a box carried moves by whole grid steps: as code set `snap`, else its line.
    pub(super) fn snaps(&self) -> bool {
        self.base.state().and_then(|s| s.snap).unwrap_or_else(|| self.base.spec.flag("snap"))
    }

    /// A press on a box of a canvas that carries boxes takes it.
    pub(super) fn carry_pressed(&mut self, at: &PointerInput) -> bool {
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
    pub(super) fn carried_by(&self, press: &CarryPress, x: f64, y: f64) -> (f64, f64) {
        let scale = self.viewport().scale;
        let (dx, dy) = ((x - press.at_x) / scale, (y - press.at_y) / scale);
        let grid = self.grid();
        if self.snaps() && grid > 0.0 {
            ((dx / grid).round() * grid, (dy / grid).round() * grid)
        } else {
            (dx.round(), dy.round())
        }
    }
}
