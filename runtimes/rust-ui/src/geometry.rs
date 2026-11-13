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

/// How far `content` goes beyond `room`, the room a scroll area scrolls:
/// none when it fits, and none for less than half a pixel, which is what
/// sums of row heights leave over in their last digits, not content.
pub fn scroll_room(content: f64, room: f64) -> f64 {
    let over = content - room;
    if over < 0.5 { 0.0 } else { over }
}

/// The least length of a scroll bar's thumb, so it is always there to take.
pub const LEAST_THUMB: f64 = 24.0;

/// A scroll bar's thumb along its track, one way: every part with a bar
/// measures, pages and drags it alike. `track` is the bar's length, `content`
/// how long what scrolls is, `scroll` how far it is scrolled, `max` how far it can.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thumb {
    pub track: f64,
    pub content: f64,
    pub scroll: f64,
    pub max: f64,
}

impl Thumb {
    /// As long as the share of the content in view, never shorter than LEAST_THUMB.
    pub fn length(&self) -> f64 {
        if self.content <= 0.0 { self.track } else { self.track.min(LEAST_THUMB.max(self.track * self.track / self.content)) }
    }

    /// Where it starts along the track.
    pub fn at(&self) -> f64 {
        if self.max <= 0.0 { 0.0 } else { (self.track - self.length()) * self.scroll / self.max }
    }

    /// A press `along` the track: on the thumb, where on it the pointer took
    /// it (`Ok`); beside it, the scroll a `page` that way (`Err`).
    pub fn pressed(&self, along: f64, page: f64) -> Result<f64, f64> {
        let at = self.at();
        if along >= at && along < at + self.length() {
            Ok(along - at)
        } else {
            Err((self.scroll + if along < at { -page } else { page }).clamp(0.0, self.max))
        }
    }

    /// The scroll with the thumb dragged to `along`, taken `grab` from its start.
    pub fn dragged(&self, along: f64, grab: f64) -> f64 {
        let travel = self.track - self.length();
        if travel <= 0.0 { 0.0 } else { ((along - grab) / travel * self.max).clamp(0.0, self.max) }
    }
}
