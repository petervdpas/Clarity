//! The raster tools of a painting canvas (docs/IMAGE-PLAN.md): shapes painted
//! with the brush (line, rectangle, ellipse), selections (rectangle, ellipse,
//! lasso) kept as a mask of the picture's pixels, the selected pixels lifted
//! and moved, and the transforms the toolbox asks for. Each change is one edit.

use clarity_image::{Rgba, RgbaImage};
use skia_safe::{surfaces, AlphaType, Color, ColorType, EncodedImageFormat, Image, ImageInfo, Paint, PaintCap, PaintJoin, PaintStyle, PathEffect, Point, SamplingOptions};

use super::{color_of, CanvasPart, Viewport};
use crate::element::Cx;
use crate::painter::Painter;

/// A selection: which pixels of the picture, as a mask over the box they lie in, and its outline in units to draw.
#[derive(Clone)]
pub struct Selection {
    /// The box of the selected pixels: x, y, width, height in the picture's pixels.
    pub area: (i64, i64, u32, u32),
    /// One byte a pixel of the box, row by row: 255 selected, 0 not.
    pub mask: Vec<u8>,
    /// Its outline in units, as drawn (a closed line).
    pub outline: Vec<(f64, f64)>,
}

impl Selection {
    fn has(&self, x: i64, y: i64) -> bool {
        let (ax, ay, w, h) = self.area;
        let (dx, dy) = (x - ax, y - ay);
        dx >= 0 && dy >= 0 && dx < i64::from(w) && dy < i64::from(h) && self.mask[(dy as usize) * w as usize + dx as usize] > 0
    }
}

/// Pixels lifted from the picture to move: the picture with their place cleared, them on their own, and how far they moved.
struct Lift {
    base: RgbaImage,
    pixels: RgbaImage,
    shown: Option<Image>,
    original: Vec<u8>,
    from: (f64, f64),
    by: (f64, f64),
}

/// What the raster tools keep while the person works.
#[derive(Default)]
pub struct RasterState {
    /// A shape or selection being drawn: its tool, where it started and the points it went through, in units.
    drawing: Option<(&'static str, Vec<(f64, f64)>)>,
    selection: Option<Selection>,
    lift: Option<Lift>,
    /// The colour dialog while it is open (adjust.rs).
    pub(super) adjusting: Option<super::adjust::Open>,
}

impl RasterState {
    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }
}

/// Whether a point lies inside a closed polygon (even-odd).
fn inside(points: &[(f64, f64)], x: f64, y: f64) -> bool {
    let mut is_in = false;
    let mut j = points.len().wrapping_sub(1);
    for i in 0..points.len() {
        let ((xi, yi), (xj, yj)) = (points[i], points[j]);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            is_in = !is_in;
        }
        j = i;
    }
    is_in
}

/// An ellipse's outline in the box from `a` to `b`, as a closed line.
fn ellipse_outline(a: (f64, f64), b: (f64, f64)) -> Vec<(f64, f64)> {
    let (cx, cy, rx, ry) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, (b.0 - a.0).abs() / 2.0, (b.1 - a.1).abs() / 2.0);
    (0..64).map(|i| f64::from(i) / 64.0 * std::f64::consts::TAU).map(|t| (cx + rx * t.cos(), cy + ry * t.sin())).collect()
}

fn rect_outline(a: (f64, f64), b: (f64, f64)) -> Vec<(f64, f64)> {
    vec![(a.0, a.1), (b.0, a.1), (b.0, b.1), (a.0, b.1)]
}

/// The selection a closed outline in units makes of a picture `size` pixels big over `units`.
fn select(outline: Vec<(f64, f64)>, units: (f64, f64), size: (u32, u32)) -> Option<Selection> {
    let (sx, sy) = (f64::from(size.0) / units.0.max(1.0), f64::from(size.1) / units.1.max(1.0));
    let px: Vec<(f64, f64)> = outline.iter().map(|(x, y)| (x * sx, y * sy)).collect();
    let (min_x, max_x) = px.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.0), hi.max(p.0)));
    let (min_y, max_y) = px.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.1), hi.max(p.1)));
    let (x0, y0) = (min_x.floor().max(0.0) as i64, min_y.floor().max(0.0) as i64);
    let (x1, y1) = (max_x.ceil().min(f64::from(size.0)) as i64, max_y.ceil().min(f64::from(size.1)) as i64);
    if x1 - x0 < 2 || y1 - y0 < 2 {
        return None;
    }
    let (w, h) = ((x1 - x0) as u32, (y1 - y0) as u32);
    let mut mask = vec![0u8; (w * h) as usize];
    for dy in 0..h {
        for dx in 0..w {
            if inside(&px, (x0 + i64::from(dx)) as f64 + 0.5, (y0 + i64::from(dy)) as f64 + 0.5) {
                mask[(dy * w + dx) as usize] = 255;
            }
        }
    }
    mask.iter().any(|m| *m > 0).then_some(Selection { area: (x0, y0, w, h), mask, outline })
}

/// The selected pixels on their own (the rest transparent), and the picture with their place cleared.
fn lift_out(picture: &RgbaImage, s: &Selection) -> (RgbaImage, RgbaImage) {
    let (ax, ay, w, h) = s.area;
    let mut pixels = RgbaImage::new(w, h);
    let mut base = picture.clone();
    for dy in 0..h {
        for dx in 0..w {
            let (x, y) = (ax + i64::from(dx), ay + i64::from(dy));
            if s.has(x, y) && x < i64::from(picture.width()) && y < i64::from(picture.height()) {
                pixels.put_pixel(dx, dy, *picture.get_pixel(x as u32, y as u32));
                base.put_pixel(x as u32, y as u32, Rgba([0, 0, 0, 0]));
            }
        }
    }
    (pixels, base)
}

/// A skia picture of decoded pixels.
fn image_of(pixels: &RgbaImage) -> Option<Image> {
    Image::from_encoded(skia_safe::Data::new_copy(&clarity_image::encode(pixels)))
}

/// The picture with a shape painted on it, as PNG: a line, or the outline of a rectangle or ellipse, from `a` to `b` in pixels.
fn shape_on(before: &RgbaImage, tool: &str, a: (f64, f64), b: (f64, f64), ink: Color, width: f64) -> Vec<u8> {
    let info = ImageInfo::new((before.width() as i32, before.height() as i32), ColorType::RGBA8888, AlphaType::Premul, None);
    let Some(mut surface) = surfaces::raster(&info, None, None) else { return clarity_image::encode(before) };
    let canvas = surface.canvas();
    canvas.clear(Color::TRANSPARENT);
    if let Some(image) = image_of(before) {
        canvas.draw_image(&image, (0, 0), None);
    }
    let mut pen = Paint::default();
    pen.set_color(ink);
    pen.set_anti_alias(true);
    pen.set_style(PaintStyle::Stroke);
    pen.set_stroke_width(width.max(1.0) as f32);
    pen.set_stroke_cap(PaintCap::Round);
    pen.set_stroke_join(PaintJoin::Round);
    let (a, b) = (Point::new(a.0 as f32, a.1 as f32), Point::new(b.0 as f32, b.1 as f32));
    match tool {
        "line" => canvas.draw_line(a, b, &pen),
        "rect" => canvas.draw_rect(skia_safe::Rect::new(a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)), &pen),
        _ => canvas.draw_oval(skia_safe::Rect::new(a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)), &pen),
    };
    surface.image_snapshot().encode(None, EncodedImageFormat::PNG, None).map(|d| d.as_bytes().to_vec()).unwrap_or_default()
}

thread_local! {
    /// Pixels cut or copied from a picture, with where they were: the program's own picture clipboard, shared by its canvases.
    static CLIPPED: std::cell::RefCell<Option<(RgbaImage, (i64, i64))>> = const { std::cell::RefCell::new(None) };
}

/// What the toolbox asks to do to the picture or its selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transform {
    /// A quarter turn clockwise.
    Rotate,
    FlipAcross,
    FlipDown,
    /// The selection made the whole paper: scaled to fit it, in the middle.
    Crop,
}

impl CanvasPart {
    /// The picture's pixels per unit, across and down.
    fn pixel_scale(&self, pixels: &RgbaImage) -> (f64, f64) {
        let (uw, uh) = self.units();
        (f64::from(pixels.width()) / uw.max(1.0), f64::from(pixels.height()) / uh.max(1.0))
    }

    /// The selection, as the toolbox and code see it; none for none.
    pub fn selection(&self) -> Option<&Selection> {
        self.raster_ref().selection()
    }

    /// A press with a raster tool: a selection's pixels are lifted to move when it is pressed inside it; otherwise a shape or a selection starts.
    pub(super) fn raster_pressed(&mut self, tool: &'static str, unit: (f64, f64), cx: &mut Cx) {
        let pixels = self.painted_pixels();
        let (sx, sy) = self.pixel_scale(&pixels);
        let is_select = matches!(tool, "select" | "ellipseSelect" | "lasso");
        let pressed_in = self.raster_ref().selection.as_ref().is_some_and(|s| s.has((unit.0 * sx) as i64, (unit.1 * sy) as i64));
        if is_select && pressed_in {
            let selection = self.raster_ref().selection.clone().expect("pressed in it");
            let (lifted, base) = lift_out(&pixels, &selection);
            let original = clarity_image::encode(&pixels);
            self.show_only(&clarity_image::encode(&base));
            let shown = image_of(&lifted);
            self.raster().lift = Some(Lift { base, pixels: lifted, shown, original, from: unit, by: (0.0, 0.0) });
            cx.invalidate();
            return;
        }
        if is_select {
            self.raster().selection = None;
        }
        self.raster().drawing = Some((tool, vec![unit]));
        cx.invalidate();
    }

    /// The pointer moved with a raster tool: the shape or selection follows it, or the lifted pixels do.
    pub(super) fn raster_moved(&mut self, unit: (f64, f64)) -> bool {
        let state = self.raster();
        if let Some(lift) = state.lift.as_mut() {
            lift.by = (unit.0 - lift.from.0, unit.1 - lift.from.1);
            return true;
        }
        if let Some((tool, points)) = state.drawing.as_mut() {
            // A lasso keeps every point; the others only where it is now.
            if *tool == "lasso" || points.len() < 2 {
                points.push(unit);
            } else if let Some(last) = points.last_mut() {
                *last = unit;
            }
            return true;
        }
        false
    }

    /// Let go: a shape is painted, a selection made, lifted pixels laid down where they moved; each painting is one edit.
    pub(super) fn raster_released(&mut self, cx: &mut Cx) -> bool {
        if let Some(lift) = self.raster().lift.take() {
            let mut picture = lift.base;
            let (sx, sy) = self.pixel_scale(&picture);
            let (dx, dy) = ((lift.by.0 * sx).round() as i64, (lift.by.1 * sy).round() as i64);
            if let Some(s) = self.raster().selection.as_mut() {
                clarity_image::ops::blend(&mut picture, &lift.pixels, s.area.0 + dx, s.area.1 + dy, 100);
                s.area.0 += dx;
                s.area.1 += dy;
                s.outline = s.outline.iter().map(|(x, y)| (x + lift.by.0, y + lift.by.1)).collect();
            }
            if dx == 0 && dy == 0 {
                self.show_only(&lift.original);
                cx.invalidate();
            } else {
                self.show_and_send(clarity_image::encode(&picture), cx);
            }
            return true;
        }
        let Some((tool, points)) = self.raster().drawing.take() else { return false };
        let (a, b) = (points[0], *points.last().unwrap_or(&points[0]));
        let pixels = self.painted_pixels();
        let size = pixels.dimensions();
        match tool {
            "select" => self.raster().selection = select(rect_outline(a, b), self.units(), size),
            "ellipseSelect" => self.raster().selection = select(ellipse_outline(a, b), self.units(), size),
            "lasso" => self.raster().selection = if points.len() > 2 { select(points, self.units(), size) } else { None },
            _ if a == b => {}
            _ => {
                let (sx, sy) = self.pixel_scale(&pixels);
                let ink = color_of(cx.look(), &self.ink());
                let ink = ink.with_a((f64::from(ink.a()) * self.opacity() / 100.0).round() as u8);
                let png = shape_on(&pixels, tool, (a.0 * sx, a.1 * sy), (b.0 * sx, b.1 * sy), ink, self.brush() * sx);
                self.show_and_send(png, cx);
            }
        }
        cx.invalidate();
        true
    }

    pub(super) fn raster_cancelled(&mut self) {
        let state = self.raster();
        state.drawing = None;
        if let Some(lift) = state.lift.take() {
            self.show_only(&lift.original);
        }
    }

    /// The toolbox asked for a transform: on the selection when there is one, else on the whole picture, fitted to the paper.
    pub fn transform(&mut self, what: Transform, cx: &mut Cx) {
        let mut picture = self.painted_pixels();
        let (pw, ph) = picture.dimensions();
        let selection = self.raster_ref().selection.clone();
        let made = match (what, selection) {
            (Transform::Crop, None) => return,
            (Transform::Crop, Some(s)) => {
                let (lifted, _) = lift_out(&picture, &s);
                self.raster().selection = None;
                fitted(&lifted, pw, ph)
            }
            (_, Some(s)) => {
                let (lifted, mut base) = lift_out(&picture, &s);
                let turned = turned(&lifted, what);
                let (cx0, cy0) = (s.area.0 + i64::from(s.area.2) / 2, s.area.1 + i64::from(s.area.3) / 2);
                let (x, y) = (cx0 - i64::from(turned.width()) / 2, cy0 - i64::from(turned.height()) / 2);
                clarity_image::ops::blend(&mut base, &turned, x, y, 100);
                // The selection is what was turned, where it lies now.
                let (ux, uy) = self.pixel_scale(&base);
                let mask: Vec<u8> = turned.pixels().map(|p| if p[3] > 0 { 255 } else { 0 }).collect();
                let (w, h) = turned.dimensions();
                let outline = rect_outline((x as f64 / ux, y as f64 / uy), ((x + i64::from(w)) as f64 / ux, (y + i64::from(h)) as f64 / uy));
                self.raster().selection = Some(Selection { area: (x, y, w, h), mask, outline });
                base
            }
            (Transform::Rotate, None) => fitted(&turned(&picture, what), pw, ph),
            (_, None) => {
                picture = turned(&picture, what);
                picture
            }
        };
        self.show_and_send(clarity_image::encode(&made), cx);
    }

    /// Ctrl+C: the selected pixels are copied, for a paste in this canvas or another; false without a selection.
    pub fn copy_selection(&mut self) -> bool {
        let Some(s) = self.raster_ref().selection.clone() else { return false };
        let (lifted, _) = lift_out(&self.painted_pixels(), &s);
        CLIPPED.with(|c| *c.borrow_mut() = Some((lifted, (s.area.0, s.area.1))));
        true
    }

    /// Ctrl+X: copied, then taken out of the picture, as one edit.
    pub fn cut_selection(&mut self, cx: &mut Cx) -> bool {
        if !self.copy_selection() {
            return false;
        }
        self.delete_selection(cx)
    }

    /// Delete: the selected pixels are cleared, as one edit; the selection stays.
    pub fn delete_selection(&mut self, cx: &mut Cx) -> bool {
        let Some(s) = self.raster_ref().selection.clone() else { return false };
        let (_, base) = lift_out(&self.painted_pixels(), &s);
        self.show_and_send(clarity_image::encode(&base), cx);
        true
    }

    /// Ctrl+V: what was cut or copied is laid where it was (inside the picture), and selected, so a drag moves it; one edit.
    pub fn paste(&mut self, cx: &mut Cx) -> bool {
        let Some((pixels, (x, y))) = CLIPPED.with(|c| c.borrow().clone()) else { return false };
        let mut picture = self.painted_pixels();
        let (pw, ph) = (i64::from(picture.width()), i64::from(picture.height()));
        let (w, h) = (i64::from(pixels.width()), i64::from(pixels.height()));
        let (x, y) = (x.clamp(0, (pw - w).max(0)), y.clamp(0, (ph - h).max(0)));
        clarity_image::ops::blend(&mut picture, &pixels, x, y, 100);
        let (ux, uy) = self.pixel_scale(&picture);
        let mask = pixels.pixels().map(|p| if p[3] > 0 { 255 } else { 0 }).collect();
        let outline = rect_outline((x as f64 / ux, y as f64 / uy), ((x + w) as f64 / ux, (y + h) as f64 / uy));
        self.raster().selection = Some(Selection { area: (x, y, pixels.width(), pixels.height()), mask, outline });
        self.show_and_send(clarity_image::encode(&picture), cx);
        true
    }

    /// Ctrl+A: the whole picture is selected.
    pub fn select_all(&mut self, cx: &mut Cx) {
        let size = self.painted_pixels().dimensions();
        let (uw, uh) = self.units();
        self.raster().selection = select(rect_outline((0.0, 0.0), (uw, uh)), (uw, uh), size);
        cx.invalidate();
    }

    /// Escape: nothing is selected any more; false when nothing was.
    pub fn deselect(&mut self, cx: &mut Cx) -> bool {
        let had = self.raster().selection.take().is_some();
        cx.invalidate();
        had
    }

    /// The keys of the raster tools: cut, copy and paste, delete, select all, deselect.
    pub(super) fn raster_key(&mut self, key: crate::input::KeyInput, cx: &mut Cx) -> bool {
        use crate::input::Key;
        let command = key.control() || key.meta();
        match key.key {
            Key::C if command => self.copy_selection(),
            Key::X if command => self.cut_selection(cx),
            Key::V if command => self.paste(cx),
            Key::A if command => {
                self.select_all(cx);
                true
            }
            Key::Delete | Key::Backspace if !command => self.delete_selection(cx),
            Key::Escape => self.deselect(cx),
            _ => false,
        }
    }

    /// What the raster tools show over the picture: the shape or selection being drawn, the selection's outline, the lifted pixels.
    pub(super) fn draw_raster(&self, p: &Painter, v: Viewport) {
        let state = self.raster_ref();
        let at = |x: f64, y: f64| self.at(v, x, y);
        let canvas = p.canvas;
        if let Some(lift) = state.lift.as_ref() {
            if let (Some(image), Some(s)) = (lift.shown.as_ref(), state.selection.as_ref()) {
                let (sx, sy) = self.pixel_scale(&lift.base);
                let (x, y) = (s.area.0 as f64 / sx + lift.by.0, s.area.1 as f64 / sy + lift.by.1);
                let (w, h) = (f64::from(s.area.2) / sx, f64::from(s.area.3) / sy);
                let (a, b) = (at(x, y), at(x + w, y + h));
                let sampling = SamplingOptions::new(skia_safe::FilterMode::Linear, skia_safe::MipmapMode::None);
                canvas.draw_image_rect_with_sampling_options(image, None, skia_safe::Rect::new(a.x, a.y, b.x, b.y), sampling, &Paint::default());
            }
        }
        // Marching ants: a dashed line on a light one, so it shows on any picture.
        let ants = |outline: &[(f64, f64)], by: (f64, f64)| {
            let pts: Vec<Point> = outline.iter().map(|(x, y)| at(x + by.0, y + by.1)).collect();
            if pts.len() < 2 {
                return;
            }
            let path = skia_safe::Path::polygon(&pts, true, None, None);
            let mut under = Paint::default();
            under.set_color(Color::WHITE);
            under.set_style(PaintStyle::Stroke);
            under.set_stroke_width(1.0);
            canvas.draw_path(&path, &under);
            let mut over = under.clone();
            over.set_color(Color::BLACK);
            over.set_path_effect(PathEffect::dash(&[4.0, 4.0], 0.0));
            canvas.draw_path(&path, &over);
        };
        if let Some(s) = state.selection.as_ref() {
            ants(&s.outline, state.lift.as_ref().map_or((0.0, 0.0), |l| l.by));
        }
        if let Some((tool, points)) = state.drawing.as_ref() {
            let (a, b) = (points[0], *points.last().unwrap_or(&points[0]));
            match *tool {
                "select" => ants(&rect_outline(a, b), (0.0, 0.0)),
                "ellipseSelect" => ants(&ellipse_outline(a, b), (0.0, 0.0)),
                "lasso" => ants(points, (0.0, 0.0)),
                _ => {
                    let ink = color_of(p.look, &self.ink());
                    let mut pen = Paint::default();
                    pen.set_color(ink.with_a((f64::from(ink.a()) * self.opacity() / 100.0).round() as u8));
                    pen.set_anti_alias(true);
                    pen.set_style(PaintStyle::Stroke);
                    pen.set_stroke_width((self.brush().max(1.0) * v.scale) as f32);
                    pen.set_stroke_cap(PaintCap::Round);
                    let (pa, pb) = (at(a.0, a.1), at(b.0, b.1));
                    let r = skia_safe::Rect::new(pa.x.min(pb.x), pa.y.min(pb.y), pa.x.max(pb.x), pa.y.max(pb.y));
                    match *tool {
                        "line" => canvas.draw_line(pa, pb, &pen),
                        "rect" => canvas.draw_rect(r, &pen),
                        _ => canvas.draw_oval(r, &pen),
                    };
                }
            }
        }
    }
}

/// A picture turned or mirrored.
fn turned(p: &RgbaImage, what: Transform) -> RgbaImage {
    match what {
        Transform::Rotate => clarity_image::ops::rotate(p, 90.0),
        Transform::FlipAcross => clarity_image::ops::flip(p, true),
        Transform::FlipDown => clarity_image::ops::flip(p, false),
        Transform::Crop => p.clone(),
    }
}

/// A picture scaled to fit `w` by `h`, in the middle of a transparent one that size.
fn fitted(p: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let fit = (f64::from(w) / f64::from(p.width().max(1))).min(f64::from(h) / f64::from(p.height().max(1)));
    let (fw, fh) = (((f64::from(p.width()) * fit).round() as u32).max(1), ((f64::from(p.height()) * fit).round() as u32).max(1));
    let mut out = RgbaImage::new(w, h);
    clarity_image::ops::blend(&mut out, &clarity_image::ops::resize(p, fw, fh), i64::from((w - fw.min(w)) / 2), i64::from((h - fh.min(h)) / 2), 100);
    out
}
