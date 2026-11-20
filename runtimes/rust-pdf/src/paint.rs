//! A laid-out page as a list of what to draw (docs/ledger/reports-pdf.md, step 5):
//! glyph runs by carried font, shapes with their fill and stroke, and clips,
//! each with the transform it is drawn in, in points from the page's top
//! left. Clarity.UI's painter draws the list with Skia, sharp at any zoom;
//! nothing here knows Skia.

use typst::layout::{Frame, FrameItem, Point, Transform};
use typst::visualize::{Curve, CurveItem, Geometry, Paint};

/// A colour as red, green, blue and alpha.
pub type Rgba = [u8; 4];

/// A step of a path, in points.
#[derive(Debug, Clone, PartialEq)]
pub enum PathOp {
    Move(f32, f32),
    Line(f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// One thing to draw. A transform is `[sx, ky, kx, sy, tx, ty]`: a point
/// (x, y) is drawn at (sx·x + kx·y + tx, ky·x + sy·y + ty).
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// Glyphs of a carried font (`font_data`), each its id and place.
    Glyphs { font: usize, size: f32, color: Rgba, transform: [f32; 6], glyphs: Vec<(u16, f32, f32)> },
    /// A shape: its path, filled or stroked (colour and width).
    Shape { path: Vec<PathOp>, fill: Option<Rgba>, stroke: Option<(Rgba, f32)>, transform: [f32; 6] },
    /// What follows until its `Pop` is drawn inside `clip`, when there is one.
    Push { clip: Option<Vec<PathOp>>, transform: [f32; 6] },
    /// A picture as its file holds it (PNG, JPEG, GIF, WebP or SVG), drawn in a box of `width` by `height` points.
    Image { data: Vec<u8>, svg: bool, width: f32, height: f32, transform: [f32; 6] },
    Pop,
}

fn matrix(t: Transform) -> [f32; 6] {
    [t.sx.get() as f32, t.ky.get() as f32, t.kx.get() as f32, t.sy.get() as f32, t.tx.to_pt() as f32, t.ty.to_pt() as f32]
}

fn rgba(paint: &Paint) -> Rgba {
    match paint {
        Paint::Solid(color) => color.to_vec4_u8(),
        // Gradients and tilings are not laid out by a report or Markdown; drawn as black.
        _ => [0, 0, 0, 255],
    }
}

fn point(p: Point) -> (f32, f32) {
    (p.x.to_pt() as f32, p.y.to_pt() as f32)
}

fn path(curve: &Curve) -> Vec<PathOp> {
    curve
        .0
        .iter()
        .map(|item| match item {
            CurveItem::Move(p) => {
                let (x, y) = point(*p);
                PathOp::Move(x, y)
            }
            CurveItem::Line(p) => {
                let (x, y) = point(*p);
                PathOp::Line(x, y)
            }
            CurveItem::Cubic(a, b, c) => {
                let ((ax, ay), (bx, by), (cx, cy)) = (point(*a), point(*b), point(*c));
                PathOp::Cubic(ax, ay, bx, by, cx, cy)
            }
            CurveItem::Close => PathOp::Close,
        })
        .collect()
}

/// A shape's geometry as a path.
fn geometry(geometry: &Geometry) -> Vec<PathOp> {
    match geometry {
        Geometry::Line(to) => {
            let (x, y) = point(*to);
            vec![PathOp::Move(0.0, 0.0), PathOp::Line(x, y)]
        }
        Geometry::Rect(size) => {
            let (w, h) = (size.x.to_pt() as f32, size.y.to_pt() as f32);
            vec![PathOp::Move(0.0, 0.0), PathOp::Line(w, 0.0), PathOp::Line(w, h), PathOp::Line(0.0, h), PathOp::Close]
        }
        Geometry::Curve(curve) => path(curve),
    }
}

/// The items of a frame drawn in `ts`.
pub(crate) fn items(frame: &Frame, ts: Transform, fonts: &[typst::text::Font], out: &mut Vec<Item>) {
    for (place, item) in frame.items() {
        let at = ts.pre_concat(Transform::translate(place.x, place.y));
        match item {
            FrameItem::Group(group) => {
                let inner = at.pre_concat(group.transform);
                out.push(Item::Push { clip: group.clip.as_ref().map(path), transform: matrix(inner) });
                items(&group.frame, inner, fonts, out);
                out.push(Item::Pop);
            }
            FrameItem::Text(text) => {
                let Some(font) = fonts.iter().position(|f| f == text.font.font()) else { continue };
                let size = text.size.to_pt();
                let mut x = 0.0;
                let glyphs = text
                    .glyphs
                    .iter()
                    .map(|g| {
                        let placed = (g.id, ((x + g.x_offset.get()) * size) as f32, (-g.y_offset.get() * size) as f32);
                        x += g.x_advance.get();
                        placed
                    })
                    .collect();
                out.push(Item::Glyphs { font, size: size as f32, color: rgba(&text.fill), transform: matrix(at), glyphs });
            }
            FrameItem::Shape(shape, _) => {
                out.push(Item::Shape {
                    path: geometry(&shape.geometry),
                    fill: shape.fill.as_ref().map(rgba),
                    stroke: shape.stroke.as_ref().map(|s| (rgba(&s.paint), s.thickness.to_pt() as f32)),
                    transform: matrix(at),
                });
            }
            FrameItem::Image(image, size, _) => {
                let (data, svg) = match image.kind() {
                    typst::visualize::ImageKind::Raster(raster) => (raster.data().to_vec(), false),
                    typst::visualize::ImageKind::Svg(svg) => (svg.data().to_vec(), true),
                    // A report holds no PDF pictures.
                    _ => continue,
                };
                out.push(Item::Image { data, svg, width: size.x.to_pt() as f32, height: size.y.to_pt() as f32, transform: matrix(at) });
            }
            _ => {}
        }
    }
}
