//! The display part: a number in seven-segment digits, as on a calculator.
//! The text a value becomes and the cells it fills follow the cases every
//! renderer shares (conformance/display.json); the segments have the same
//! shapes as the web's and C#'s. Lit segments take the text's color; dark
//! ones show faintly, as on an LCD. The digits are scaled to fit the place,
//! kept in proportion, on the right.

use std::rc::Rc;

use serde_json::Value;
use skia_safe::{Paint, PaintStyle, PathBuilder};

use crate::element::Element;
use crate::geometry::Rect;
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::parts::{Part, PartBase, PartHost};

/// How many digits a display has without `digits`.
pub const DEFAULT_DIGITS: usize = 12;

pub struct DisplayPart {
    base: PartBase,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(DisplayPart { base: PartBase::new(spec, host) })
}

impl DisplayPart {
    /// How many digits it has.
    pub fn digits(&self) -> usize {
        self.base.spec.number("digits").map(|d| d.max(1.0) as usize).unwrap_or(DEFAULT_DIGITS)
    }

    /// What it shows now: what it binds, 0 without a value.
    pub fn text(&self) -> String {
        let value = self.base.bound().and_then(|path| self.base.host.read(path));
        shown_text(&written(value.as_ref()), self.digits())
    }
}

/// A value as a number written out: 0 for none.
fn written(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "0".to_string(),
        Some(Value::String(text)) if text.trim().is_empty() => "0".to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => match (number.as_i64(), number.as_f64()) {
            (Some(whole), _) => whole.to_string(),
            (None, Some(n)) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{n:.0}"),
            _ => number.to_string(),
        },
        Some(other) => other.to_string(),
    }
}

/// The lit segments of a character: a top, b top right, c bottom right, d bottom, e bottom left, f top left, g middle.
pub fn segments(character: char) -> &'static str {
    match character {
        '0' => "abcdef",
        '1' => "bc",
        '2' => "abdeg",
        '3' => "abcdg",
        '4' => "bcfg",
        '5' => "acdfg",
        '6' => "acdefg",
        '7' => "abc",
        '8' => "abcdefg",
        '9' => "abcdfg",
        '-' => "g",
        'E' => "adefg",
        _ => "",
    }
}

/// `digits` plus one, as a decimal text: "0999" gives "1000".
fn plus_one(digits: &str) -> String {
    let mut chars: Vec<u8> = digits.bytes().collect();
    for i in (0..chars.len()).rev() {
        if chars[i] != b'9' {
            chars[i] += 1;
            return String::from_utf8(chars).unwrap_or_default();
        }
        chars[i] = b'0';
    }
    let mut out = String::from("1");
    out.push_str(&String::from_utf8(chars).unwrap_or_default());
    out
}

/// Whether a text is a number written plainly: an optional minus, digits, and a point with digits.
fn plain_number(text: &str) -> bool {
    let body = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = match body.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (body, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    digits(whole) && fraction.is_none_or(digits)
}

/// The text `value` (a number as text) shows in `digits` digits: rounded
/// half away from zero to fit, without trailing zeros; E when it cannot fit.
pub fn shown_text(value: &str, digits: usize) -> String {
    let text = value.trim();
    if !plain_number(text) {
        return "E".to_string();
    }
    let negative = text.starts_with('-');
    let text = text.trim_start_matches('-');
    let (integer, fraction) = text.split_once('.').unwrap_or((text, ""));
    let integer = match integer.trim_start_matches('0') {
        "" => "0",
        kept => kept,
    };
    if integer.len() > digits {
        return "E".to_string();
    }
    let places = digits - integer.len();
    let mut kept = integer.to_string();
    if fraction.len() > places {
        kept.push_str(&fraction[..places]);
    } else {
        kept.push_str(fraction);
        kept.push_str(&"0".repeat(places - fraction.len()));
    }
    if fraction.len() > places && fraction.as_bytes()[places] >= b'5' {
        kept = plus_one(&kept);
    }
    let whole = match kept[..kept.len() - places].trim_start_matches('0') {
        "" => "0",
        w => w,
    };
    if whole.len() > digits {
        return "E".to_string();
    }
    let rest = kept[kept.len() - places..].trim_end_matches('0');
    let shown = if rest.is_empty() { whole.to_string() } else { format!("{whole}.{rest}") };
    if negative && shown.bytes().any(|b| (b'1'..=b'9').contains(&b)) { format!("-{shown}") } else { shown }
}

/// What each of the `digits` + 1 cells shows, from the left: a character and
/// whether its point is lit; the extra cell is for the minus sign.
pub fn cells(text: &str, digits: usize) -> Vec<(char, bool)> {
    let mut lit: Vec<(char, bool)> = Vec::new();
    for c in text.chars() {
        match lit.last_mut() {
            Some(last) if c == '.' => last.1 = true,
            _ => lit.push((c, false)),
        }
    }
    let dark = (digits + 1).saturating_sub(lit.len());
    let mut all = vec![(' ', false); dark];
    all.extend(lit);
    all
}

// A cell is 13 units wide and 20 high, as on the web.
const CELL_WIDTH: f64 = 13.0;
const CELL_HEIGHT: f64 = 20.0;
const T: f64 = 1.0;
const G: f64 = 0.6;

fn across(x1: f64, x2: f64, y: f64) -> [(f64, f64); 6] {
    [(x1, y), (x1 + T, y - T), (x2 - T, y - T), (x2, y), (x2 - T, y + T), (x1 + T, y + T)]
}

fn down(x: f64, y1: f64, y2: f64) -> [(f64, f64); 6] {
    [(x, y1), (x + T, y1 + T), (x + T, y2 - T), (x, y2), (x - T, y2 - T), (x - T, y1 + T)]
}

/// The shape of each segment in a cell, by its letter.
fn shapes() -> [(char, [(f64, f64); 6]); 7] {
    [
        ('a', across(2.0 + G, 10.0 - G, 2.0)),
        ('b', down(10.0, 2.0 + G, 10.0 - G)),
        ('c', down(10.0, 10.0 + G, 18.0 - G)),
        ('d', across(2.0 + G, 10.0 - G, 18.0)),
        ('e', down(2.0, 10.0 + G, 18.0 - G)),
        ('f', down(2.0, 2.0 + G, 10.0 - G)),
        ('g', across(2.0 + G, 10.0 - G, 10.0)),
    ]
}

/// Where the cells are drawn in `place`: the left edge, the top and the scale of a unit.
pub fn fit(place: Rect, cells: usize) -> (f64, f64, f64) {
    let inner = Rect::new(place.x + 10.0, place.y + 6.0, (place.width - 20.0).max(0.0), (place.height - 12.0).max(0.0));
    let width = cells as f64 * CELL_WIDTH + 1.0;
    let scale = (inner.width / width).min(inner.height / CELL_HEIGHT);
    (inner.right() - width * scale, inner.y + (inner.height - CELL_HEIGHT * scale) / 2.0, scale)
}

impl Element for DisplayPart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        let look = p.look;
        let radius = look.radius("input");
        p.fill(bounds, look.color("raised"), radius);
        p.stroke(bounds, look.color("border"), look.border(), radius);
        let cells = cells(&self.text(), self.digits());
        let (left, top, scale) = fit(bounds, cells.len());
        if scale <= 0.0 {
            return;
        }
        let lit = look.color("text");
        let paint = |color| {
            let mut paint = Paint::default();
            paint.set_color(color);
            paint.set_anti_alias(true);
            paint.set_style(PaintStyle::Fill);
            paint
        };
        let on = paint(lit);
        let off = paint(lit.with_a(26));
        for (i, (character, point)) in cells.iter().enumerate() {
            let at = i as f64 * CELL_WIDTH;
            let segments = segments(*character);
            for (name, shape) in shapes() {
                let mut path = PathBuilder::new();
                for (k, (x, y)) in shape.iter().enumerate() {
                    let point = ((left + (x + at) * scale) as f32, (top + y * scale) as f32);
                    if k == 0 {
                        path.move_to(point);
                    } else {
                        path.line_to(point);
                    }
                }
                path.close();
                p.canvas.draw_path(&path.detach(), if segments.contains(name) { &on } else { &off });
            }
            let dot = ((left + (11.5 + at) * scale) as f32, (top + 18.0 * scale) as f32);
            p.canvas.draw_circle(dot, scale as f32, if *point { &on } else { &off });
        }
    }
}

impl Part for DisplayPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
