//! The layers of a painting canvas (docs/IMAGE-PLAN.md): while the canvas is
//! open the person paints on one layer of several, each shown or hidden; the
//! picture kept is the shown layers laid over each other, bottom first. When
//! the picture changes otherwise (Ctrl+Z, another record), the layers fold
//! back into one, so they never disagree with what is kept.

use clarity_image::RgbaImage;

use super::CanvasPart;
use crate::element::Cx;

/// One layer: its name, its pixels, whether it shows.
#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub pixels: RgbaImage,
    pub shown: bool,
}

/// What the layers tool asks of the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerAsk {
    /// A new, empty layer above the one in hand, taken in hand.
    Add,
    /// The layer in hand goes (not the last one).
    Remove,
    /// The layer in hand one place up or down.
    Up,
    Down,
    /// A layer, by its place from the bottom, taken in hand.
    Choose(usize),
    /// A layer shown or hidden.
    Toggle(usize),
}

/// The canvas's layers; none until the person adds one: the picture is then its only layer.
#[derive(Default)]
pub struct Layers {
    pub list: Vec<Layer>,
    pub in_hand: usize,
    /// The picture last sent, the layers composed.
    pub sent: Vec<u8>,
    /// The field's value (and its record) when the layers were last compared with it.
    pub seen: Option<super::modes::PaintedFor>,
}

impl Layers {
    pub fn active(&self) -> bool {
        !self.list.is_empty()
    }

    /// The shown layers laid over each other, bottom first.
    pub fn composed(&self, width: u32, height: u32) -> RgbaImage {
        let mut out = RgbaImage::new(width, height);
        for layer in self.list.iter().filter(|l| l.shown) {
            clarity_image::ops::blend(&mut out, &layer.pixels, 0, 0, 100);
        }
        out
    }

    /// The rows the layers tool shows: each layer's name, whether it shows and whether it is in hand, top first.
    pub fn rows(&self) -> Vec<(String, bool, bool)> {
        self.list.iter().enumerate().rev().map(|(i, l)| (l.name.clone(), l.shown, i == self.in_hand)).collect()
    }
}

impl CanvasPart {
    /// The layers fold back into one when the field's picture changed to one they did not make
    /// (Ctrl+Z, another record): compared once each time the field's value changes.
    pub(super) fn check_layers(&mut self) {
        if !self.modes_layers().active() {
            return;
        }
        let now = self.painted_for();
        if self.modes_layers().seen.as_ref() == Some(&now) {
            return;
        }
        self.modes_layers_mut().seen = Some(now);
        let (w, h) = self.field_pixels_size();
        if self.field_pixels() != self.modes_layers().composed(w, h) {
            *self.modes_layers_mut() = Layers::default();
            self.share_layers();
        }
    }

    /// A layer tool's ask: the layers change, the picture is composed again and kept as one edit.
    pub(super) fn layer_ask(&mut self, ask: LayerAsk, cx: &mut Cx) {
        let picture = self.field_pixels();
        let (w, h) = picture.dimensions();
        {
            let layers = self.modes_layers_mut();
            if !layers.active() {
                layers.list.push(Layer { name: locale_name(1), pixels: picture, shown: true });
                layers.in_hand = 0;
            }
            let n = layers.list.len();
            match ask {
                LayerAsk::Add => {
                    let at = layers.in_hand + 1;
                    layers.list.insert(at, Layer { name: locale_name(n + 1), pixels: RgbaImage::new(w, h), shown: true });
                    layers.in_hand = at;
                }
                LayerAsk::Remove if n > 1 => {
                    layers.list.remove(layers.in_hand);
                    layers.in_hand = layers.in_hand.min(layers.list.len() - 1);
                }
                LayerAsk::Up if layers.in_hand + 1 < n => {
                    layers.list.swap(layers.in_hand, layers.in_hand + 1);
                    layers.in_hand += 1;
                }
                LayerAsk::Down if layers.in_hand > 0 => {
                    layers.list.swap(layers.in_hand, layers.in_hand - 1);
                    layers.in_hand -= 1;
                }
                LayerAsk::Choose(i) if i < n => layers.in_hand = i,
                LayerAsk::Toggle(i) if i < n => layers.list[i].shown = !layers.list[i].shown,
                _ => {}
            }
        }
        self.send_composed(cx);
    }

    /// The layers composed, shown and kept as one edit.
    pub(super) fn send_composed(&mut self, cx: &mut Cx) {
        let (w, h) = self.field_pixels_size();
        let png = clarity_image::encode(&self.modes_layers().composed(w, h));
        self.modes_layers_mut().sent = png.clone();
        self.share_layers();
        self.send_picture(png, cx);
    }
}

/// A new layer's name: Layer 1, Layer 2, in the person's language.
fn locale_name(n: usize) -> String {
    crate::locale::translate("layerName", "Layer {number}").replace("{number}", &n.to_string())
}
