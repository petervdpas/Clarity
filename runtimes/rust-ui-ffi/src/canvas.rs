//! The canvas a custom part draws on, by commands: the library's Skia is
//! not the caller's (C# draws with SkiaSharp's own build), so the part says
//! what to draw and the library draws it with its painter. Colors are
//! 0xAARRGGBB. Clips a part leaves open are closed after its `draw`.

use std::ffi::{c_char, c_void};

use ui::geometry::Rect;
use ui::painter::{Painter, TextAlign};
use ui::skia_safe::{self, Color, Paint, PaintStyle};

use crate::{guard, handle_mut, put, text, ClarityStatus, Failure};

/// The painter of a custom part's `draw`, while it runs.
pub struct ClarityCanvas {
    painter: *const c_void,
    /// The clips open, closed by `clarity_canvas_restore` or after the draw.
    clips: usize,
}

impl ClarityCanvas {
    pub fn new(painter: &Painter) -> ClarityCanvas {
        ClarityCanvas { painter: painter as *const Painter as *const c_void, clips: 0 }
    }

    fn painter(&self) -> &Painter<'_> {
        // The canvas lives only inside the `draw` that holds the painter.
        unsafe { &*(self.painter as *const Painter) }
    }

    /// Closes the clips the part left open.
    pub fn restore_all(&mut self) {
        while self.clips > 0 {
            self.painter().canvas.restore();
            self.clips -= 1;
        }
    }
}

fn color(argb: u32) -> Color {
    Color::new(argb)
}

fn area(x: f64, y: f64, width: f64, height: f64) -> Result<Rect, Failure> {
    if [x, y, width, height].iter().all(|v| v.is_finite()) {
        Ok(Rect::new(x, y, width.max(0.0), height.max(0.0)))
    } else {
        Err(Failure::invalid("the box is no box"))
    }
}

/// Runs a drawing command on a canvas.
unsafe fn on(canvas: *mut ClarityCanvas, draw: impl FnOnce(&mut ClarityCanvas) -> crate::Done) -> ClarityStatus {
    guard(|| draw(unsafe { handle_mut(canvas) }?))
}

fn size_or_look(canvas: &ClarityCanvas, size: f64) -> f64 {
    if size > 0.0 && size.is_finite() { size } else { canvas.painter().look.size("text") }
}

/// # Safety
/// `canvas` is the canvas of a running `draw`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_canvas_fill(canvas: *mut ClarityCanvas, x: f64, y: f64, width: f64, height: f64, argb: u32, radius: f64) -> ClarityStatus {
    unsafe { on(canvas, |c| Ok(c.painter().fill(area(x, y, width, height)?, color(argb), radius.max(0.0)))) }
}

/// # Safety
/// `canvas` is the canvas of a running `draw`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_canvas_stroke(canvas: *mut ClarityCanvas, x: f64, y: f64, width: f64, height: f64, argb: u32, line_width: f64, radius: f64) -> ClarityStatus {
    unsafe { on(canvas, |c| Ok(c.painter().stroke(area(x, y, width, height)?, color(argb), line_width, radius.max(0.0)))) }
}

/// # Safety
/// `canvas` is the canvas of a running `draw`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_canvas_line(canvas: *mut ClarityCanvas, x1: f64, y1: f64, x2: f64, y2: f64, argb: u32, line_width: f64) -> ClarityStatus {
    unsafe { on(canvas, |c| Ok(c.painter().line((x1, y1), (x2, y2), color(argb), line_width))) }
}

fn paint(argb: u32, filled: bool, line_width: f64) -> Paint {
    let mut paint = Paint::default();
    paint.set_color(color(argb));
    paint.set_anti_alias(true);
    paint.set_style(if filled { PaintStyle::Fill } else { PaintStyle::Stroke });
    paint.set_stroke_width(line_width.max(0.0) as f32);
    paint
}

/// # Safety
/// `canvas` is the canvas of a running `draw`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_canvas_oval(canvas: *mut ClarityCanvas, x: f64, y: f64, width: f64, height: f64, argb: u32, filled: i32, line_width: f64) -> ClarityStatus {
    unsafe {
        on(canvas, |c| {
            let b = area(x, y, width, height)?;
            c.painter().canvas.draw_oval(skia_safe::Rect::from_xywh(b.x as f32, b.y as f32, b.width as f32, b.height as f32), &paint(argb, filled != 0, line_width));
            Ok(())
        })
    }
}

/// # Safety
/// `canvas` is the canvas of a running `draw`; `svg` is (pointer, length).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_canvas_path(canvas: *mut ClarityCanvas, svg: *const c_char, svg_len: usize, argb: u32, filled: i32, line_width: f64) -> ClarityStatus {
    let svg = unsafe { text(svg, svg_len) };
    unsafe {
        on(canvas, |c| {
            let path = skia_safe::utils::parse_path::from_svg(svg?).ok_or_else(|| Failure::invalid("the path's data does not read"))?;
            c.painter().canvas.draw_path(&path, &paint(argb, filled != 0, line_width));
            Ok(())
        })
    }
}

/// # Safety
/// `canvas` is the canvas of a running `draw`; `words` is (pointer, length).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_canvas_text(canvas: *mut ClarityCanvas, words: *const c_char, words_len: usize, x: f64, y: f64, width: f64, height: f64, argb: u32, size: f64, bold: i32, align: i32) -> ClarityStatus {
    let words = unsafe { text(words, words_len) };
    unsafe {
        on(canvas, |c| {
            let align = match align {
                1 => TextAlign::Center,
                2 => TextAlign::Right,
                _ => TextAlign::Left,
            };
            let size = size_or_look(c, size);
            c.painter().text(words?, area(x, y, width, height)?, color(argb), size, "text", bold != 0, align);
            Ok(())
        })
    }
}

/// # Safety
/// `canvas` is the canvas of a running `draw`; `words` is (pointer, length); `width` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_canvas_measure(canvas: *mut ClarityCanvas, words: *const c_char, words_len: usize, size: f64, bold: i32, width: *mut f64) -> ClarityStatus {
    let words = unsafe { text(words, words_len) };
    unsafe {
        on(canvas, |c| {
            let size = size_or_look(c, size);
            let measured = c.painter().fonts.measure(words?, "text", size, bold != 0);
            put(width, measured)
        })
    }
}

/// # Safety
/// `canvas` is the canvas of a running `draw`; `rgba` points at `stride * height` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_canvas_pixels(canvas: *mut ClarityCanvas, rgba: *const u8, width: u32, height: u32, stride: usize, x: f64, y: f64, box_width: f64, box_height: f64) -> ClarityStatus {
    unsafe {
        on(canvas, |c| {
            if rgba.is_null() || stride < width as usize * 4 {
                return Err(Failure::invalid("the pixels are no picture of that size"));
            }
            let bytes = std::slice::from_raw_parts(rgba, stride * height as usize);
            c.painter().pixels(bytes, width, height, stride, area(x, y, box_width, box_height)?);
            Ok(())
        })
    }
}

/// # Safety
/// `canvas` is the canvas of a running `draw`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_canvas_clip(canvas: *mut ClarityCanvas, x: f64, y: f64, width: f64, height: f64) -> ClarityStatus {
    unsafe {
        on(canvas, |c| {
            let b = area(x, y, width, height)?;
            let sk = c.painter().canvas;
            sk.save();
            sk.clip_rect(skia_safe::Rect::from_xywh(b.x as f32, b.y as f32, b.width as f32, b.height as f32), skia_safe::ClipOp::Intersect, true);
            c.clips += 1;
            Ok(())
        })
    }
}

/// # Safety
/// `canvas` is the canvas of a running `draw`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_canvas_restore(canvas: *mut ClarityCanvas) -> ClarityStatus {
    unsafe {
        on(canvas, |c| {
            if c.clips == 0 {
                return Err(Failure::invalid("no clip is open"));
            }
            c.painter().canvas.restore();
            c.clips -= 1;
            Ok(())
        })
    }
}

/// # Safety
/// `canvas` is the canvas of a running `draw`; `role` is (pointer, length); `argb` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clarity_canvas_look_color(canvas: *mut ClarityCanvas, role: *const c_char, role_len: usize, argb: *mut u32) -> ClarityStatus {
    let role = unsafe { text(role, role_len) };
    unsafe {
        on(canvas, |c| {
            let found = c.painter().look.color(role?);
            put(argb, (found.a() as u32) << 24 | (found.r() as u32) << 16 | (found.g() as u32) << 8 | found.b() as u32)
        })
    }
}
