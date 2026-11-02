//! An icon: one of Clarity's Lucide icons, an SVG path in a 24 by 24 box,
//! drawn with a round line in the text's color, scaled to 20 in the middle
//! of its place. A generated application carries the icons it shows, which
//! the host finds by the icon's name (`PartHost::icon_path`). One it does
//! not carry shows as a ring in the border color. An icon written as a
//! whole SVG (it starts with `<`) is drawn as that picture, in its own
//! colors, through resvg (`crate::picture`).

use std::rc::Rc;

use skia_safe::{Color, Paint, PaintCap, PaintJoin, PaintStyle, Path};

use crate::element::Element;
use crate::geometry::Rect;
use crate::markup::PartSpec;
use crate::painter::Painter;
use crate::part_element;
use crate::picture::Picture;
use crate::parts::{Part, PartBase, PartHost};

/// How big an icon part draws its icon.
pub const ICON_SIZE: f64 = 20.0;

pub struct IconPart {
    base: PartBase,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(IconPart { base: PartBase::new(spec, host) })
}

/// The name a generated application gives an icon in code: `triangle-alert`
/// is `TriangleAlert`, and one that starts with a digit gets a `_` before it.
pub fn member(name: &str) -> String {
    let mut member = String::new();
    for word in name.split(|c: char| !c.is_alphanumeric()) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            member.extend(first.to_uppercase());
            member.push_str(chars.as_str());
        }
    }
    if member.is_empty() || member.starts_with(|c: char| c.is_ascii_digit()) {
        member.insert(0, '_');
    }
    member
}

impl IconPart {
    /// The icon it shows, by its name.
    pub fn icon(&self) -> String {
        self.base.spec.text("icon").filter(|i| !i.is_empty()).unwrap_or("circle").to_string()
    }

    /// The path of the icon it shows, as the host gives it; none when there is none.
    pub fn path_of(&self) -> Option<String> {
        self.base.host.icon_path(&self.icon())
    }
}

/// The picture of an icon written as a whole SVG, read once per text on this thread.
fn svg_icon(svg: &str) -> Option<Picture> {
    thread_local! {
        static READ: std::cell::RefCell<std::collections::HashMap<String, Option<Picture>>> = Default::default();
    }
    READ.with(|read| read.borrow_mut().entry(svg.to_string()).or_insert_with(|| Picture::read("icon.svg", svg.as_bytes())).clone())
}

/// Draws the icon of SVG path data `path` in `place`, in `color` (an icon
/// written as a whole SVG in its own colors); false when it draws nothing.
pub fn draw_icon(p: &Painter, path: Option<&str>, place: Rect, color: Color) -> bool {
    if let Some(svg) = path.map(str::trim_start).filter(|d| d.starts_with('<')) {
        let Some(picture) = svg_icon(svg) else { return false };
        picture.draw(p.canvas, crate::parts::containers::image::fit(place, picture.width(), picture.height()));
        return true;
    }
    let Some(shape) = path.filter(|d| !d.is_empty()).and_then(|d| Path::from_svg(d)) else {
        return false;
    };
    let scale = (place.width.min(place.height) / 24.0) as f32;
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(2.0);
    paint.set_stroke_cap(PaintCap::Round);
    paint.set_stroke_join(PaintJoin::Round);
    p.canvas.save();
    p.canvas.translate((
        (place.x + (place.width - 24.0 * scale as f64) / 2.0) as f32,
        (place.y + (place.height - 24.0 * scale as f64) / 2.0) as f32,
    ));
    p.canvas.scale((scale, scale));
    p.canvas.draw_path(&shape, &paint);
    p.canvas.restore();
    true
}

impl Element for IconPart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        let place = Rect::new(bounds.x + (bounds.width - ICON_SIZE) / 2.0, bounds.y + (bounds.height - ICON_SIZE) / 2.0, ICON_SIZE, ICON_SIZE);
        let color = p.look.color(if self.base.enabled() { "text" } else { "muted" });
        if !draw_icon(p, self.path_of().as_deref(), place, color) {
            p.stroke(place.inset(2.0), p.look.color("border"), 1.0, ICON_SIZE / 2.0);
        }
    }
}

impl Part for IconPart {
    fn base(&self) -> &PartBase {
        &self.base
    }
}
