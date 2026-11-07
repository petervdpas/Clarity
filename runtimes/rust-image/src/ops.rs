//! The pixel work on decoded RGBA pictures (conformance/picture-functions.json).

use image::{Rgba, RgbaImage, imageops};

/// Smoothly scaled to `width` by `height`.
pub fn resize(p: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    imageops::resize(p, width, height, imageops::FilterType::Triangle)
}

/// The part at `x, y`, `width` by `height`, clamped to the picture; outside it, 1 by 1 transparent.
pub fn crop(p: &RgbaImage, x: i64, y: i64, width: i64, height: i64) -> RgbaImage {
    let (pw, ph) = (i64::from(p.width()), i64::from(p.height()));
    let (x0, y0) = (x.clamp(0, pw), y.clamp(0, ph));
    let (x1, y1) = ((x + width).clamp(0, pw), (y + height).clamp(0, ph));
    if x1 <= x0 || y1 <= y0 {
        return RgbaImage::new(1, 1);
    }
    imageops::crop_imm(p, x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32).to_image()
}

/// Turned clockwise by `degrees`: a quarter turn exactly; any other angle on a
/// transparent ground the size of the turned picture, each pixel taken from
/// the nearest pixel of the picture.
pub fn rotate(p: &RgbaImage, degrees: f64) -> RgbaImage {
    let turn = degrees.rem_euclid(360.0);
    if turn == 0.0 {
        return p.clone();
    }
    if turn == 90.0 {
        return imageops::rotate90(p);
    }
    if turn == 180.0 {
        return imageops::rotate180(p);
    }
    if turn == 270.0 {
        return imageops::rotate270(p);
    }
    let (w, h) = (f64::from(p.width()), f64::from(p.height()));
    let (sin, cos) = turn.to_radians().sin_cos();
    let out_w = (w * cos.abs() + h * sin.abs()).round().max(1.0);
    let out_h = (w * sin.abs() + h * cos.abs()).round().max(1.0);
    let mut out = RgbaImage::new(out_w as u32, out_h as u32);
    let (cx, cy, ox, oy) = (w / 2.0, h / 2.0, out_w / 2.0, out_h / 2.0);
    for (x, y, pixel) in out.enumerate_pixels_mut() {
        // The point of the picture this pixel's middle comes from, turned back.
        let (dx, dy) = (f64::from(x) + 0.5 - ox, f64::from(y) + 0.5 - oy);
        let (sx, sy) = (dx * cos + dy * sin + cx, -dx * sin + dy * cos + cy);
        if sx >= 0.0 && sy >= 0.0 && sx < w && sy < h {
            *pixel = *p.get_pixel(sx as u32, sy as u32);
        }
    }
    out
}

/// Mirrored left to right (`horizontal`) or top to bottom.
pub fn flip(p: &RgbaImage, horizontal: bool) -> RgbaImage {
    if horizontal { imageops::flip_horizontal(p) } else { imageops::flip_vertical(p) }
}

/// `over` laid onto `base` with its top left at `x, y`, its alpha times `opacity` (0 to 100): source over.
pub fn blend(base: &mut RgbaImage, over: &RgbaImage, x: i64, y: i64, opacity: i64) {
    let opacity = opacity.clamp(0, 100) as f64 / 100.0;
    for (ox, oy, top) in over.enumerate_pixels() {
        let (bx, by) = (x + i64::from(ox), y + i64::from(oy));
        if bx < 0 || by < 0 || bx >= i64::from(base.width()) || by >= i64::from(base.height()) {
            continue;
        }
        let under = base.get_pixel_mut(bx as u32, by as u32);
        *under = over_of(*under, *top, opacity);
    }
}

/// One pixel laid over another, with `opacity` (0 to 1) on the top one's alpha.
pub fn over_of(under: Rgba<u8>, top: Rgba<u8>, opacity: f64) -> Rgba<u8> {
    let ta = f64::from(top[3]) / 255.0 * opacity;
    let ua = f64::from(under[3]) / 255.0;
    let out_a = ta + ua * (1.0 - ta);
    if out_a <= 0.0 {
        return Rgba([0, 0, 0, 0]);
    }
    let channel = |i: usize| ((f64::from(top[i]) * ta + f64::from(under[i]) * ua * (1.0 - ta)) / out_a).round().clamp(0.0, 255.0) as u8;
    Rgba([channel(0), channel(1), channel(2), (out_a * 255.0).round() as u8])
}

/// Hue turned by `hue` degrees; `saturation`, `brightness` and `contrast` from
/// -100 to 100, 0 leaving the picture as it is. Brightness adds up to the full
/// range, contrast stretches or flattens around the middle grey, saturation
/// moves each colour away from or toward its grey. Alpha stays.
pub fn adjust(p: &mut RgbaImage, hue: f64, saturation: f64, brightness: f64, contrast: f64) {
    let (s, b, c) = (saturation.clamp(-100.0, 100.0) / 100.0, brightness.clamp(-100.0, 100.0) / 100.0, contrast.clamp(-100.0, 100.0) / 100.0);
    let factor = if c >= 0.0 { 1.0 + c * 3.0 } else { 1.0 + c };
    let (sin, cos) = hue.to_radians().sin_cos();
    for pixel in p.pixels_mut() {
        let mut rgb = [f64::from(pixel[0]), f64::from(pixel[1]), f64::from(pixel[2])];
        if hue != 0.0 {
            rgb = turn_hue(rgb, sin, cos);
        }
        if s != 0.0 {
            let grey = 0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2];
            let k = 1.0 + s;
            rgb = rgb.map(|v| grey + (v - grey) * k);
        }
        if b != 0.0 {
            rgb = rgb.map(|v| v + b * 255.0);
        }
        if c != 0.0 {
            rgb = rgb.map(|v| (v - 127.5) * factor + 127.5);
        }
        for (i, v) in rgb.iter().enumerate() {
            pixel[i] = v.round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// A colour's hue turned around the grey axis (the YIQ rotation, as browsers' hue-rotate).
fn turn_hue(rgb: [f64; 3], sin: f64, cos: f64) -> [f64; 3] {
    let m = [
        [0.213 + cos * 0.787 - sin * 0.213, 0.715 - cos * 0.715 - sin * 0.715, 0.072 - cos * 0.072 + sin * 0.928],
        [0.213 - cos * 0.213 + sin * 0.143, 0.715 + cos * 0.285 + sin * 0.140, 0.072 - cos * 0.072 - sin * 0.283],
        [0.213 - cos * 0.213 - sin * 0.787, 0.715 - cos * 0.715 + sin * 0.715, 0.072 + cos * 0.928 + sin * 0.072],
    ];
    [0, 1, 2].map(|r| m[r][0] * rgb[0] + m[r][1] * rgb[1] + m[r][2] * rgb[2])
}

/// The area around `x, y` of colours within `tolerance` (0 to 100) of the one
/// there, as its 4 neighbours reach it, painted `color`: a fill bucket.
pub fn fill(p: &mut RgbaImage, x: u32, y: u32, color: Rgba<u8>, tolerance: f64) {
    if x >= p.width() || y >= p.height() {
        return;
    }
    let start = *p.get_pixel(x, y);
    if start == color {
        return;
    }
    let reach = tolerance.clamp(0.0, 100.0) / 100.0 * 255.0;
    let near = |c: &Rgba<u8>| (0..4).all(|i| (f64::from(c[i]) - f64::from(start[i])).abs() <= reach);
    let (w, h) = p.dimensions();
    let mut seen = vec![false; (w * h) as usize];
    let mut todo = vec![(x, y)];
    while let Some((cx, cy)) = todo.pop() {
        let at = (cy * w + cx) as usize;
        if seen[at] || !near(p.get_pixel(cx, cy)) {
            continue;
        }
        seen[at] = true;
        p.put_pixel(cx, cy, color);
        if cx > 0 {
            todo.push((cx - 1, cy));
        }
        if cy > 0 {
            todo.push((cx, cy - 1));
        }
        if cx + 1 < w {
            todo.push((cx + 1, cy));
        }
        if cy + 1 < h {
            todo.push((cx, cy + 1));
        }
    }
}
