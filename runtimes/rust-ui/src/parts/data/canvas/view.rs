//! A canvas's view of its units: the viewport, whole, zoomed and scrolled,
//! points between the screen and the units, the grid's spacing and the
//! zoom's word.

use super::*;

/// What a canvas shows: the pixels a unit takes, and the unit at its top left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub scale: f64,
    pub x: f64,
    pub y: f64,
}

/// A place in a canvas's units, rounded down: where it was clicked.
pub type UnitPoint = (i64, i64);

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

pub(super) fn edge(v: f64, size: f64) -> i64 {
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

/// Clarity's words for the zoom buttons, in the person's language.
pub fn zoom_word(button: &str) -> String {
    match button {
        "zoomOut" => locale::translate("zoomOut", "Zoom out"),
        "zoomWhole" => locale::translate("zoomWhole", "Show it whole"),
        _ => locale::translate("zoomIn", "Zoom in"),
    }
}
