//! A place on the screen, in the screen's units (pixels at scale 1), from
//! its top left.

use std::fmt;

/// A place on the screen: its top left corner and its size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect { x, y, width, height }
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Whether a point lies in it: its left and top edges are in, its right and bottom edges out.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// The same place moved by `x` and `y`.
    pub fn offset(&self, x: f64, y: f64) -> Rect {
        Rect { x: self.x + x, y: self.y + y, ..*self }
    }

    /// The place `by` in from every edge; a negative `by` is out.
    pub fn inset(&self, by: f64) -> Rect {
        Rect::new(self.x + by, self.y + by, (self.width - 2.0 * by).max(0.0), (self.height - 2.0 * by).max(0.0))
    }

    /// The same place as Skia's rectangle.
    pub fn sk(&self) -> skia_safe::Rect {
        skia_safe::Rect::new(self.x as f32, self.y as f32, self.right() as f32, self.bottom() as f32)
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn n(v: f64) -> String {
            let text = format!("{v:.3}");
            text.trim_end_matches('0').trim_end_matches('.').to_string()
        }
        write!(f, "[{}, {}, {}, {}]", n(self.x), n(self.y), n(self.width), n(self.height))
    }
}
