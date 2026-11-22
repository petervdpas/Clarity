//! A canvas's shapes: the boxes of its drawing and what the pointer hits,
//! links between boxes and their ends, words and colors, and drawing a shape
//! with the painter.

use super::*;

/// The bounds of a box drawn on a canvas, in its units: what a new line connects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl UnitBox {
    /// Its connection handles: the middle of its top, right, bottom and left sides.
    pub fn handles(&self) -> [(f64, f64); 4] {
        let (cx, cy) = (self.x + self.width / 2.0, self.y + self.height / 2.0);
        [(cx, self.y), (self.x + self.width, cy), (cx, self.y + self.height), (self.x, cy)]
    }

    /// Its bottom-right corner, where its resize grip is.
    pub fn corner(&self) -> (f64, f64) {
        (self.x + self.width, self.y + self.height)
    }

    /// Its middle, in whole units.
    pub fn center(&self) -> (f64, f64) {
        ((self.x + self.width / 2.0).floor(), (self.y + self.height / 2.0).floor())
    }

    pub(super) fn contains(&self, other: &UnitBox) -> bool {
        self.x <= other.x && self.y <= other.y && self.x + self.width >= other.x + other.width && self.y + self.height >= other.y + other.height
    }
}

/// The box a shape the canvas hit-tests takes: a rect, a picture, an object, a circle, a ring or an ellipse.
pub(super) fn box_of_shape(shape: &Shape) -> Option<UnitBox> {
    match shape {
        Shape::Rect { x, y, width, height, .. } | Shape::Image { x, y, width, height, .. } | Shape::Object { x, y, width, height, .. } => {
            Some(UnitBox { x: *x, y: *y, width: *width, height: *height })
        }
        Shape::Circle { x, y, r, .. } | Shape::Ring { x, y, r, .. } => Some(UnitBox { x: x - r, y: y - r, width: 2.0 * r, height: 2.0 * r }),
        Shape::Ellipse { x, y, rx, ry, .. } => Some(UnitBox { x: x - rx, y: y - ry, width: 2.0 * rx, height: 2.0 * ry }),
        _ => None,
    }
}

/// Whether a unit is on a shape the canvas hit-tests (not the paper, lines or text).
pub(super) fn shape_hit(shape: &Shape, p: (f64, f64)) -> bool {
    match shape {
        Shape::Rect { x, y, width, height, .. } | Shape::Image { x, y, width, height, .. } | Shape::Object { x, y, width, height, .. } => p.0 >= *x && p.0 <= x + width && p.1 >= *y && p.1 <= y + height,
        Shape::Circle { x, y, r, .. } | Shape::Ring { x, y, r, .. } => distance((*x, *y), p) <= *r,
        Shape::Ellipse { x, y, rx, ry, .. } => *rx > 0.0 && *ry > 0.0 && ((p.0 - x) / rx).powi(2) + ((p.1 - y) / ry).powi(2) <= 1.0,
        _ => false,
    }
}

/// The boxes of a drawing of its units, the topmost first: each drawn shape
/// grown to the shapes under it that hold it whole (a diagram's box is its
/// outline, its body and its header). A shape over the whole paper is the
/// paper, no box.
pub fn boxes_of(shown: &[Shape], units: (f64, f64)) -> Vec<(UnitBox, Shape)> {
    let paper = UnitBox { x: 0.0, y: 0.0, width: units.0, height: units.1 };
    let all: Vec<(UnitBox, &Shape)> = shown.iter().filter_map(|s| box_of_shape(s).map(|b| (b, s))).filter(|(b, _)| !b.contains(&paper)).collect();
    (0..all.len())
        .rev()
        .map(|i| {
            let mut grown = all[i].0;
            for (under, _) in all[..i].iter().rev() {
                if under.contains(&grown) {
                    grown = *under;
                }
            }
            (grown, all[i].1.clone())
        })
        .collect()
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
                Shape::Words { x: n("x")?, y: n("y")?, text, color: t("color", "text"), size: or("size", 10.0), align: t("align", "center"), style: t("style", "") }
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
                start: t("start", ""),
                end: t("end", ""),
                label: sent.get("label").map(crate::locale::shown).unwrap_or_default(),
                style: t("style", ""),
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
    words_font_of(fonts, "text", size, false)
}

/// The font of a canvas's text of a font role of the theme (`text`, `mono`), bold or not, as `words_font`.
pub fn words_font_of(fonts: &crate::fonts::Fonts, role: &str, size: f64, bold: bool) -> Font {
    let mut font = fonts.font(role, size, bold);
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

pub(super) fn shape_paint(look: &ThemeLook, color: &str, stroke: Option<f64>) -> Paint {
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

/// The point halfway along a line through `points`.
pub(super) fn halfway(points: &[(f64, f64)]) -> (f64, f64) {
    let total: f64 = points.windows(2).map(|w| distance(w[0], w[1])).sum();
    let mut left = total / 2.0;
    for w in points.windows(2) {
        let length = distance(w[0], w[1]);
        if length > 0.0 && left <= length {
            let t = left / length;
            return (w[0].0 + t * (w[1].0 - w[0].0), w[0].1 + t * (w[1].1 - w[0].1));
        }
        left -= length;
    }
    points[0]
}

/// What a link's end at `tip` shows (crate::LINK_ENDS in the spec), the line
/// coming from `from`: an arrow, a hollow triangle, a diamond, or a data
/// model's bars, circle and crow's foot. In units, 12 long and 7 to a side.
#[allow(clippy::too_many_arguments)]
pub(super) fn link_end(p: &Painter, kind: &str, tip: (f64, f64), from: (f64, f64), color: &str, width: f64, scale: f64, paper: &str, at: &dyn Fn(f64, f64) -> Point) {
    let length = distance(tip, from);
    if kind.is_empty() || length <= 0.0 {
        return;
    }
    // Along the line toward the tip, and across it.
    let (dx, dy) = ((tip.0 - from.0) / length, (tip.1 - from.1) / length);
    let (nx, ny) = (-dy, dx);
    let back = |k: f64, side: f64| at(tip.0 - k * dx + side * nx, tip.1 - k * dy + side * ny);
    let (long, wide) = (12.0, 7.0);
    let stroke = shape_paint(p.look, color, Some(width.max(1.0)));
    let hollow = shape_paint(p.look, if paper.is_empty() { "panel" } else { paper }, None);
    let filled = shape_paint(p.look, color, None);
    let polygon = |points: &[Point], fill: &Paint| {
        let path = skia_safe::Path::polygon(points, true, None, None);
        p.canvas.draw_path(&path, fill);
        p.canvas.draw_path(&path, &stroke);
    };
    let bar = |k: f64| p.canvas.draw_line(back(k, wide), back(k, -wide), &stroke);
    let ring = |k: f64| {
        let r = (4.0 * scale) as f32;
        p.canvas.draw_circle(back(k, 0.0), r, &hollow);
        p.canvas.draw_circle(back(k, 0.0), r, &stroke);
    };
    let crow = || {
        for side in [wide, 0.0, -wide] {
            p.canvas.draw_line(back(long, 0.0), back(0.0, side), &stroke);
        }
    };
    match kind {
        "arrow" => {
            p.canvas.draw_line(back(long, wide), back(0.0, 0.0), &stroke);
            p.canvas.draw_line(back(long, -wide), back(0.0, 0.0), &stroke);
        }
        "triangle" => polygon(&[back(0.0, 0.0), back(long, wide), back(long, -wide)], &hollow),
        "diamond" => polygon(&[back(0.0, 0.0), back(long, wide), back(2.0 * long, 0.0), back(long, -wide)], &hollow),
        "filledDiamond" => polygon(&[back(0.0, 0.0), back(long, wide), back(2.0 * long, 0.0), back(long, -wide)], &filled),
        "one" => {
            bar(6.0);
            bar(11.0);
        }
        "zeroOrOne" => {
            bar(6.0);
            ring(15.0);
        }
        "many" => crow(),
        "oneOrMany" => {
            crow();
            bar(long + 4.0);
        }
        "zeroOrMany" => {
            crow();
            ring(long + 8.0);
        }
        _ => {}
    }
}

/// The links of a drawing ending at the edge of the box each end lies in,
/// not under it, so what their ends show is seen: a box as `boxes_of` finds them.
pub fn links_to_edges(shapes: Vec<Shape>, boxes: &[UnitBox]) -> Vec<Shape> {
    let inside = |b: &UnitBox, p: (f64, f64)| p.0 > b.x && p.0 < b.x + b.width && p.1 > b.y && p.1 < b.y + b.height;
    // Where the line from p (in the box) to q (outside it) leaves it.
    let edge = |b: &UnitBox, p: (f64, f64), q: (f64, f64)| {
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let across = |d: f64, from: f64, low: f64, high: f64| if d > 0.0 { (high - from) / d } else if d < 0.0 { (low - from) / d } else { f64::INFINITY };
        let t = across(dx, p.0, b.x, b.x + b.width).min(across(dy, p.1, b.y, b.y + b.height)).clamp(0.0, 1.0);
        (p.0 + t * dx, p.1 + t * dy)
    };
    let cut = |p: (f64, f64), q: (f64, f64)| boxes.iter().find(|b| inside(b, p) && !inside(b, q)).map(|b| edge(b, p, q)).unwrap_or(p);
    shapes
        .into_iter()
        .map(|shape| match shape {
            Shape::Link { name, x1, y1, x2, y2, via, color, width, start, end, label, style } => {
                let points = link_points(x1, y1, x2, y2, &via);
                let n = points.len();
                let a = cut(points[0], points[1]);
                let b = cut(points[n - 1], points[n - 2]);
                Shape::Link { name, x1: a.0, y1: a.1, x2: b.0, y2: b.1, via, color, width, start, end, label, style }
            }
            other => other,
        })
        .collect()
}

/// A line through `points`; one point is a dot of its round cap.
pub(super) fn polyline(canvas: &skia_safe::Canvas, points: &[Point], paint: &Paint) {
    if points.is_empty() {
        return;
    }
    canvas.draw_points(if points.len() == 1 { PointMode::Points } else { PointMode::Polygon }, points, paint);
}

/// Draws one shape of a drawing, in its units, `at` giving each unit's pixel.
/// `paper` (a color, empty for the theme's panel) fills what is hollow at a link's end.
pub fn draw_shape(p: &Painter, shape: &Shape, scale: f64, at: &dyn Fn(f64, f64) -> Point, pictures: &dyn Fn(&str) -> Option<Picture>, paper: &str) {
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
        Shape::Link { x1, y1, x2, y2, via, color, width, start, end, label, style, .. } => {
            let points = link_points(*x1, *y1, *x2, *y2, via);
            let mut line = shape_paint(look, color, Some(width * scale));
            let dash = |on: f64, off: f64| skia_safe::PathEffect::dash(&[(on * scale) as f32, (off * scale) as f32], 0.0);
            match style.as_str() {
                "dashed" => {
                    line.set_path_effect(dash(6.0, 4.0));
                }
                "dotted" => {
                    line.set_path_effect(dash(0.1, 3.0 + width));
                }
                _ => {}
            }
            through(&points, &line);
            let n = points.len();
            link_end(p, start, points[0], points[1], color, width * scale, scale, paper, at);
            link_end(p, end, points[n - 1], points[n - 2], color, width * scale, scale, paper, at);
            if !label.is_empty() {
                let (mx, my) = halfway(&points);
                let words = Shape::Words { x: mx, y: my - 9.0, text: label.clone(), color: "muted".into(), size: 11.0, align: "center".into(), style: "italic".into() };
                draw_shape(p, &words, scale, at, pictures, paper);
            }
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
        Shape::Words { x, y, text, color, size, align, style } => {
            if text.is_empty() || !(*size > 0.0) || !(scale > 0.0) {
                return;
            }
            // Shaped once at its size in units and drawn scaled, so zooming shapes no text
            // again; unhinted, its letters and spaces scale with it.
            let size = *size;
            let has = |word: &str| style.split_whitespace().any(|w| w == word);
            let role = if has("mono") { "mono" } else { "text" };
            let bold = has("bold");
            let font = words_font_of(p.fonts, role, size, bold);
            // Shaped as every text is (Arabic joined, Hebrew right to left), unhinted as the zoom needs.
            let shaped = p.fonts.shape_unhinted(text, role, size, bold, true);
            let width = shaped.width();
            let (_, metrics) = font.metrics();
            let a = at(*x, *y);
            canvas.save();
            canvas.translate((a.x, a.y));
            canvas.scale((scale as f32, scale as f32));
            // Centered on its point down, by its alignment across, in units.
            let baseline = -(metrics.ascent + metrics.descent) / 2.0;
            let left = -text_left(align, width);
            if has("italic") {
                // Slanted from its baseline, as a face without italics is.
                canvas.translate((left as f32, baseline));
                canvas.skew((-0.2, 0.0));
                shaped.draw(canvas, 0.0, 0.0, &shape_paint(look, color, None), 0.0);
            } else {
                shaped.draw(canvas, left, baseline as f64, &shape_paint(look, color, None), 0.0);
            }
            canvas.restore();
        }
        // An object's box draws nothing itself: the shapes after it show it; a clear is the drawing's.
        _ => {}
    }
}

/// The units a shape may draw on, left, top, right, bottom, with room for a
/// link's ends and label and a text's width, which is not measured here;
/// none for a clear, which fills everything.
pub(super) fn reach(shape: &Shape) -> Option<(f64, f64, f64, f64)> {
    let around = |points: &[(f64, f64)], room: f64| {
        let (mut l, mut t, mut r, mut b) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for (x, y) in points {
            (l, t, r, b) = (l.min(*x), t.min(*y), r.max(*x), b.max(*y));
        }
        (l - room, t - room, r + room, b + room)
    };
    match shape {
        Shape::Clear { .. } => None,
        Shape::Line { x1, y1, x2, y2, width, .. } => Some(around(&[(*x1, *y1), (*x2, *y2)], *width)),
        Shape::Link { x1, y1, x2, y2, via, width, label, .. } => {
            // Its ends (an arrow, a diamond) and its label reach past its points.
            let room = 30.0 + width + label.chars().count() as f64 * 11.0;
            Some(around(&link_points(*x1, *y1, *x2, *y2, via), room))
        }
        Shape::Path { points, width, .. } => {
            let pairs: Vec<(f64, f64)> = points.chunks_exact(2).map(|q| (q[0], q[1])).collect();
            (!pairs.is_empty()).then(|| around(&pairs, *width))
        }
        Shape::Words { x, y, text, size, .. } => {
            // At most a size across each character, whatever its alignment.
            let wide = text.chars().count() as f64 * size;
            Some((x - wide, y - size, x + wide, y + size))
        }
        Shape::Ring { x, y, r, width, .. } => Some((x - r - width, y - r - width, x + r + width, y + r + width)),
        other => box_of_shape(other).map(|b| (b.x, b.y, b.x + b.width, b.y + b.height)),
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
    // What shows of the units: a shape wholly outside it is not drawn.
    let seen = (v.x, v.y, v.x + bounds.width / v.scale, v.y + bounds.height / v.scale);
    for shape in shapes {
        if reach(shape).is_some_and(|(left, top, right, bottom)| right < seen.0 || bottom < seen.1 || left > seen.2 || top > seen.3) {
            continue;
        }
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
        draw_shape(p, shape, v.scale, &at, pictures, paper);
    }
}

pub(super) fn number_of(value: Option<&Value>) -> Option<f64> {
    let n = match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }?;
    n.is_finite().then_some(n)
}

pub(super) fn wholes(value: Option<&Value>) -> Vec<f64> {
    match value {
        Some(Value::Array(items)) => items.iter().map(|v| number_of(Some(v)).unwrap_or(0.0).round()).collect(),
        _ => Vec::new(),
    }
}
