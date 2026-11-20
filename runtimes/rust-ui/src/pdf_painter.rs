//! Draws a laid-out page of Clarity.Pdf (a report, a Markdown text) with
//! Skia (docs/ledger/reports-pdf.md, step 5): its glyph runs in the carried
//! fonts, its shapes and its clips, sharp at any zoom. `scale` is pixels
//! per point; `origin` where the page's top left lies on the canvas.

use std::cell::RefCell;
use std::collections::HashMap;

use clarity_pdf::{Item, PathOp, Rgba};
use skia_safe::{Canvas, Color, Data, Font, FontMgr, Matrix, Paint, PaintStyle, Path, PathBuilder, Point, Typeface};

thread_local! {
    /// This thread's typefaces of Clarity.Pdf's carried fonts, by their number.
    static TYPEFACES: RefCell<HashMap<usize, Option<Typeface>>> = RefCell::new(HashMap::new());
}

fn typeface(font: usize) -> Option<Typeface> {
    TYPEFACES.with(|cell| {
        cell.borrow_mut()
            .entry(font)
            .or_insert_with(|| {
                let (data, index) = clarity_pdf::font_data(font)?;
                FontMgr::new().new_from_data(Data::new_copy(data), index)
            })
            .clone()
    })
}

fn color(c: Rgba) -> Color {
    Color::from_argb(c[3], c[0], c[1], c[2])
}

fn matrix(t: &[f32; 6]) -> Matrix {
    // [sx, ky, kx, sy, tx, ty] as Skia's scale-x, skew-x, translate-x, skew-y, scale-y, translate-y.
    Matrix::new_all(t[0], t[2], t[4], t[1], t[3], t[5], 0.0, 0.0, 1.0)
}

fn path(ops: &[PathOp]) -> Path {
    let mut path = PathBuilder::new();
    for op in ops {
        match *op {
            PathOp::Move(x, y) => {
                path.move_to((x, y));
            }
            PathOp::Line(x, y) => {
                path.line_to((x, y));
            }
            PathOp::Cubic(ax, ay, bx, by, cx, cy) => {
                path.cubic_to((ax, ay), (bx, by), (cx, cy));
            }
            PathOp::Close => {
                path.close();
            }
        }
    }
    path.detach()
}

/// Draws a page's items.
pub fn paint(canvas: &Canvas, items: &[Item], scale: f32, origin: (f32, f32)) {
    canvas.save();
    canvas.translate(origin);
    canvas.scale((scale, scale));
    let base = canvas.local_to_device_as_3x3();
    let mut depth = 0;
    for item in items {
        match item {
            Item::Push { clip, transform } => {
                canvas.save();
                depth += 1;
                if let Some(clip) = clip {
                    canvas.set_matrix(&(base * matrix(transform)).into());
                    canvas.clip_path(&path(clip), None, true);
                }
            }
            Item::Pop => {
                if depth > 0 {
                    canvas.restore();
                    depth -= 1;
                }
            }
            Item::Glyphs { font, size, color: c, transform, glyphs } => {
                let Some(face) = typeface(*font) else { continue };
                canvas.set_matrix(&(base * matrix(transform)).into());
                let mut font = Font::from_typeface(face, *size);
                font.set_subpixel(true);
                font.set_edging(skia_safe::font::Edging::AntiAlias);
                let ids: Vec<u16> = glyphs.iter().map(|g| g.0).collect();
                let places: Vec<Point> = glyphs.iter().map(|g| Point::new(g.1, g.2)).collect();
                let mut paint = Paint::default();
                paint.set_anti_alias(true).set_color(color(*c));
                canvas.draw_glyphs_at(&ids, places.as_slice(), Point::new(0.0, 0.0), &font, &paint);
            }
            Item::Image { data, svg, width, height, transform } => {
                canvas.set_matrix(&(base * matrix(transform)).into());
                let name = if *svg { "picture.svg" } else { "picture.png" };
                if let Some(picture) = crate::picture::Picture::read(name, data) {
                    picture.draw(canvas, crate::geometry::Rect::new(0.0, 0.0, f64::from(*width), f64::from(*height)));
                }
            }
            Item::Shape { path: ops, fill, stroke, transform } => {
                canvas.set_matrix(&(base * matrix(transform)).into());
                let shape = path(ops);
                if let Some(fill) = fill {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true).set_color(color(*fill)).set_style(PaintStyle::Fill);
                    canvas.draw_path(&shape, &paint);
                }
                if let Some((c, width)) = stroke {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true).set_color(color(*c)).set_style(PaintStyle::Stroke).set_stroke_width(*width);
                    canvas.draw_path(&shape, &paint);
                }
            }
        }
    }
    while depth > 0 {
        canvas.restore();
        depth -= 1;
    }
    canvas.restore();
}
