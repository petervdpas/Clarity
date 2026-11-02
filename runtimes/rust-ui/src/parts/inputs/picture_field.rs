//! A picture field: an image part bound to a record's picture, the same as
//! on every stack (conformance/pictures.json). The picture shows fitted in
//! proportion and centered; without one, its empty text in a dashed frame.
//! A stored picture is fetched on its own, and again when another record
//! shows, since every stored picture is the same value; a new one shows from
//! its bytes until the record is saved. Its buttons to load a picture from a
//! file and to remove it show while the pointer or the focus is on it. A
//! file that is no picture is refused and marks the field; a read-only one
//! has no buttons. Loading and removing write the field.
//!
//! It is the variant of `image` for an image with a bind; any other image is
//! the containers group's plain image.

use std::rc::Rc;

use clarity_runtime::picture::{kind_of, Picture};
use serde_json::Value;
use skia_safe::{Image, PathEffect, SamplingOptions, FilterMode, MipmapMode, Paint, PaintStyle};

use crate::element::{Cx, Element};
use crate::files::FileFilter;
use crate::geometry::Rect;
use crate::later::Done;
use crate::locale;
use crate::markup::PartSpec;
use crate::painter::{Painter, TextAlign};
use crate::part_element;
use crate::parts::containers::container::ContainerButton;
use crate::parts::containers::image;
use crate::parts::inputs::file::Facts;
use crate::parts::{containers, MissingPart, Part, PartBase, PartHost};

const GAP: f64 = 4.0;

/// The kinds of file the load button offers.
const PATTERNS: [&str; 6] = ["*.png", "*.jpg", "*.jpeg", "*.gif", "*.bmp", "*.webp"];

/// An image with a bind shows a record's picture.
pub fn fits(spec: &PartSpec, _host: &dyn PartHost) -> bool {
    spec.text("bind").is_some_and(|b| !b.is_empty())
}

/// An image: the picture field when it fits, else the containers group's plain image.
pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    if fits(&spec, host.as_ref()) {
        return Box::new(PictureFieldPart::new(spec, host));
    }
    plain_image(spec, host)
}

/// The containers group's image, when it draws one.
pub fn plain_image(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    match containers::KINDS.iter().find(|(kind, _)| *kind == "image") {
        Some((_, make)) => make(spec, host),
        None => Box::new(MissingPart::new(spec, host, None)),
    }
}

/// The picture the load button's dialog gave.
struct Loaded(Option<Vec<u8>>);

/// What the empty text says, in Clarity's own words.
pub fn empty_text() -> String {
    locale::translate("noPicture", "No picture")
}

pub struct PictureFieldPart {
    base: PartBase,
    facts: Rc<Facts>,
    shown: Option<Image>,
    /// The picture and the record it was fetched for; none before the first.
    fetched_for: Option<(Option<Value>, Option<Value>)>,
    problem: bool,
}

/// Its buttons, by the order they were added.
const LOAD: usize = 0;
const REMOVE: usize = 1;

impl PictureFieldPart {
    pub fn new(spec: PartSpec, host: Rc<dyn PartHost>) -> PictureFieldPart {
        let mut base = PartBase::new(spec, host);
        let facts = Rc::new(Facts { host: base.host.clone(), spec: base.spec.clone(), part: base.node.id() });
        let (loading, load_shows) = (facts.clone(), facts.clone());
        let load = ContainerButton::new(|| locale::translate("loadPicture", "Load a picture"), move |cx| {
            if !can_load(&loading) {
                return;
            }
            let picker = loading.host.files();
            let title = locale::translate("loadPicture", "Load a picture");
            let filter = FileFilter { name: title.clone(), patterns: PATTERNS.iter().map(|p| p.to_string()).collect() };
            cx.later(loading.part, move || Loaded(picker.open_file(&title, Some(&filter)).map(|c| c.bytes)));
        })
        .shows(move || can_load(&load_shows));
        let (removing, remove_shows) = (facts.clone(), facts.clone());
        let remove = ContainerButton::new(|| locale::translate("removePicture", "Remove the picture"), move |_| remove(&removing))
            .shows(move || can_load(&remove_shows) && value_of(&remove_shows).is_some());
        base.node.add(Box::new(load));
        base.node.add(Box::new(remove));
        let mut part = PictureFieldPart { base, facts, shown: None, fetched_for: None, problem: false };
        part.fetch();
        part
    }

    /// The field's picture; none when it has none.
    pub fn value(&self) -> Option<Picture> {
        value_of(&self.facts)
    }

    /// The picture shown; none while there is none.
    pub fn shown(&self) -> Option<&Image> {
        self.shown.as_ref()
    }

    pub fn is_empty(&self) -> bool {
        self.shown.is_none()
    }

    /// The last file loaded was no picture.
    pub fn problem(&self) -> bool {
        self.problem
    }

    /// Whether the person may load or remove a picture.
    pub fn can_load(&self) -> bool {
        can_load(&self.facts)
    }

    /// Its buttons show: the pointer or the focus is on it.
    pub fn revealed(&self) -> bool {
        crate::element::in_order(self).iter().any(|e| e.node().hovered || e.node().focused)
    }

    fn button(&self, index: usize) -> &ContainerButton {
        self.base.node.children()[index].as_any().downcast_ref::<ContainerButton>().expect("a picture field's button")
    }

    /// Loads a picture from a file.
    pub fn load_button(&self) -> &ContainerButton {
        self.button(LOAD)
    }

    /// Removes the picture.
    pub fn remove_button(&self) -> &ContainerButton {
        self.button(REMOVE)
    }

    /// Fetches the picture when the field or its record changed.
    pub fn fetch(&mut self) {
        let bind = self.facts.bound().map(str::to_string);
        let picture = bind.as_deref().and_then(|b| self.facts.host.read(b));
        let record = bind.as_deref().and_then(|b| b.rfind('.').filter(|d| *d > 0).map(|d| b[..d].to_string())).and_then(|r| self.facts.host.read(&r));
        let key = (picture, record);
        if self.fetched_for.as_ref() == Some(&key) {
            return;
        }
        self.fetched_for = Some(key);
        let bytes = self.value().and_then(|p| self.facts.bytes_of(&p));
        self.shown = image::picture("picture", bytes.as_deref());
    }

    /// Takes the bytes as the new picture when they are one; false when they are not, or it may not.
    pub fn load(&mut self, bytes: Vec<u8>) -> bool {
        if !self.can_load() || self.facts.bound().is_none() {
            return false;
        }
        self.problem = kind_of(&bytes).is_none();
        if self.problem {
            return false;
        }
        self.facts.write(serde_json::to_value(Picture::New(bytes)).unwrap_or(Value::Null));
        true
    }

    /// The picture removed.
    pub fn remove(&mut self) {
        if self.can_load() {
            self.problem = false;
        }
        remove(&self.facts);
    }
}

fn value_of(facts: &Facts) -> Option<Picture> {
    serde_json::from_value(facts.host.read(facts.bound()?)?).ok()
}

fn can_load(facts: &Facts) -> bool {
    !facts.read_only() && facts.enabled()
}

fn remove(facts: &Facts) {
    if can_load(facts) && facts.bound().is_some() && value_of(facts).is_some() {
        facts.write(Value::Null);
    }
}

impl Element for PictureFieldPart {
    part_element!();

    /// Its buttons in a row, centered at its bottom, each as wide as its words.
    fn arrange(&mut self, area: Rect, cx: &mut Cx) {
        self.base.node.bounds = area;
        self.fetch();
        let look = cx.look();
        let height = (look.size("text") + 4.0 * look.padding()).min((area.height - 2.0 * GAP).max(0.0));
        let widths: Vec<f64> = self.base.node.children().iter().map(|b| if b.is_shown() { b.as_any().downcast_ref::<ContainerButton>().map(|b| b.width_for(cx)).unwrap_or(0.0) } else { 0.0 }).collect();
        let count = widths.iter().filter(|w| **w > 0.0).count();
        let total: f64 = widths.iter().sum::<f64>() + GAP * count.saturating_sub(1) as f64;
        let mut x = area.x + (area.width - total) / 2.0;
        let y = area.bottom() - GAP - height;
        for (button, width) in self.base.node.children_mut().iter_mut().zip(widths) {
            button.arrange(Rect::new(x, y, width, height), cx);
            if width > 0.0 {
                x += width + GAP;
            }
        }
    }

    fn draw(&self, p: &Painter) {
        let look = p.look;
        let b = self.base.node.bounds;
        match &self.shown {
            Some(picture) => {
                let place = image::fit(b, picture.width() as f64, picture.height() as f64);
                p.canvas.draw_image_rect_with_sampling_options(picture, None, place.sk(), SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear), &Paint::default());
            }
            None => {
                let mut dashed = Paint::default();
                dashed.set_color(look.color("strong_border"));
                dashed.set_anti_alias(true);
                dashed.set_style(PaintStyle::Stroke);
                dashed.set_stroke_width(look.border() as f32);
                dashed.set_path_effect(PathEffect::dash(&[4.0, 4.0], 0.0));
                let radius = look.radius("panel") as f32;
                p.canvas.draw_round_rect(b.inset(look.border() / 2.0).sk(), radius, radius, &dashed);
                p.text(&empty_text(), b.inset(2.0 * look.padding()), look.color("muted"), look.size("small"), "text", false, TextAlign::Center);
            }
        }
        // A file that was no picture marks the field in the error color.
        if self.problem {
            p.stroke(b, look.color("error"), 2.0, look.radius("panel"));
        }
        if self.revealed() {
            crate::element::draw_children(&self.base.node, p);
        }
    }

    fn on_later(&mut self, done: Done, cx: &mut Cx) {
        if let Ok(loaded) = done.downcast::<Loaded>() {
            if let Some(bytes) = loaded.0 {
                self.load(bytes);
            }
        }
        cx.invalidate();
    }
}

impl Part for PictureFieldPart {
    fn base(&self) -> &PartBase {
        &self.base
    }

    fn grows(&self) -> bool {
        true
    }
}
