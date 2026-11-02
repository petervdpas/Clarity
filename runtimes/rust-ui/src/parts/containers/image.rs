//! An image: a picture of the project's resources by its name (its
//! `source`, or a text key that names one), fitted into its place in
//! proportion and centered. A picture that is not there shows its name in a
//! dashed frame. Bitmaps (PNG, JPEG, GIF's first frame, BMP, ICO) are
//! drawn, decoded by Skia; an SVG picture is drawn by resvg into a bitmap
//! at the size it shows (`crate::picture`). The host gives the picture's
//! bytes (`PartHost::resource`).

use std::cell::RefCell;
use std::rc::Rc;

use skia_safe::{Image, Paint, PaintStyle, PathEffect};

use crate::element::Element;
use crate::geometry::Rect;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::picture::Picture;
use crate::parts::{Part, PartBase, PartHost};

pub struct ImagePart {
    base: PartBase,
    /// The picture loaded last: its name and the picture, none when it is no picture.
    loaded: RefCell<Option<(String, Option<Picture>)>>,
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(ImagePart { base: PartBase::new(spec, host), loaded: RefCell::new(None) })
}

/// The picture of `bytes` as one bitmap; none when there are none, or they
/// are no picture Skia decodes or resvg reads. An SVG is drawn with its
/// longer side at least 512 pixels (scaled 1 to 8 times), as C# ImagePart
/// does; a part that shows it draws it at its shown size instead (`Picture`).
pub fn picture(name: &str, bytes: Option<&[u8]>) -> Option<Image> {
    match Picture::read(name, bytes?)? {
        Picture::Bitmap(image) => Some(image),
        Picture::Svg(svg) => {
            let (w, h) = svg.size();
            let scale = (512.0 / w.max(h)).clamp(1.0, 8.0);
            svg.image_at((w * scale).ceil() as i32, (h * scale).ceil() as i32)
        }
    }
}

/// Where a picture `width` by `height` is drawn in `place`: as big as fits, in proportion, centered.
pub fn fit(place: Rect, width: f64, height: f64) -> Rect {
    if width <= 0.0 || height <= 0.0 {
        return Rect { width: 0.0, height: 0.0, ..place };
    }
    let scale = (place.width / width).min(place.height / height);
    let (w, h) = (width * scale, height * scale);
    Rect::new(place.x + (place.width - w) / 2.0, place.y + (place.height - h) / 2.0, w, h)
}

impl ImagePart {
    /// The picture's name: its source, else what its caption says in the language shown.
    pub fn named(&self) -> Option<String> {
        if let Some(source) = self.base.spec.text("source").filter(|s| !s.is_empty()) {
            return Some(source.to_string());
        }
        self.base.spec.text("caption").filter(|c| !c.is_empty()).map(|caption| self.base.host.text(Some(caption)))
    }

    /// The picture it shows, as the host gives its bytes, loaded once per name.
    pub fn shown(&self) -> Option<Picture> {
        let name = self.named().unwrap_or_default();
        if let Some((loaded, image)) = self.loaded.borrow().as_ref() {
            if *loaded == name {
                return image.clone();
            }
        }
        let image = if name.is_empty() { None } else { self.base.host.resource(&name).and_then(|bytes| Picture::read(&name, &bytes)) };
        *self.loaded.borrow_mut() = Some((name, image.clone()));
        image
    }

    /// What it says in its frame when it shows no picture: its name.
    pub fn says(&self) -> String {
        self.named().unwrap_or_default()
    }
}

impl Element for ImagePart {
    part_element!();

    fn draw(&self, p: &Painter) {
        let bounds = self.base.node.bounds;
        let look = p.look;
        if let Some(picture) = self.shown() {
            picture.draw(p.canvas, fit(bounds, picture.width(), picture.height()));
            return;
        }
        let mut dashed = Paint::default();
        dashed.set_color(look.color("strong_border"));
        dashed.set_anti_alias(true);
        dashed.set_style(PaintStyle::Stroke);
        dashed.set_stroke_width(look.border() as f32);
        dashed.set_path_effect(PathEffect::dash(&[4.0, 4.0], 0.0));
        let radius = look.radius("panel") as f32;
        p.canvas.draw_round_rect(bounds.inset(look.border() / 2.0).sk(), radius, radius, &dashed);
        let words = bounds.inset(2.0 * look.padding());
        let size = look.size("small");
        let shown = p.fit(&self.says(), words.width, size, "mono", false);
        p.text(&shown, words, look.color("muted"), size, "mono", false, TextAlign::Center);
    }
}

impl Part for ImagePart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        true
    }
}
