//! The paint mode: strokes on the bound image field's picture, filling and
//! picking colors, the layers and their opacity, and sending the picture.

use super::*;

impl CanvasPart {
    /// The picture of the bound image field, read again when the field holds another.
    pub(super) fn painted(&self) -> Option<Image> {
        let now = self.painted_for();
        if let Some((from, image)) = self.modes.painted.borrow().as_ref() {
            if *from == now {
                return image.clone();
            }
        }
        // A stored picture, or one of the field's file store, is fetched before
        // it is painted on (as a picture field fetches it), so a stroke never
        // sends a blank picture over the one the record has.
        let bind = self.base.bound();
        let bytes = match now.0.as_ref().map(FieldContent::read) {
            Some(Some(FieldContent::New(bytes))) => Some(bytes),
            Some(Some(FieldContent::Stored)) => bind.and_then(|b| self.base.host.fetch(b, None)),
            Some(Some(FieldContent::InStore(path))) => bind.and_then(|b| self.base.host.fetch(b, Some(&path))),
            _ => None,
        };
        // Fail safe: a picture that does not load leaves the canvas blank to paint on.
        let image = bytes.and_then(|b| {
            let image = Image::from_encoded(skia_safe::Data::new_copy(&b));
            if image.is_none() {
                eprintln!("{}: its picture did not load", self.base.name().unwrap_or("canvas"));
            }
            image
        });
        *self.modes.painted.borrow_mut() = Some((now, image.clone()));
        image
    }

    /// The bound field's value and the record it is in.
    pub(in crate::parts::data::canvas) fn painted_for(&self) -> PaintedFor {
        let Some(bind) = self.base.bound() else { return (None, None) };
        let value = self.base.host.read(bind);
        let record = bind.rfind('.').filter(|d| *d > 0).and_then(|d| self.base.host.read(&bind[..d]));
        (value, record)
    }

    /// The size of the picture painted on: the picture's own, else the canvas's units.
    pub(super) fn picture_size(&self) -> (i32, i32) {
        match self.painted() {
            Some(p) => (p.width(), p.height()),
            None => {
                let (uw, uh) = self.units();
                ((uw.round() as i32).max(1), (uh.round() as i32).max(1))
            }
        }
    }

    pub(super) fn end_stroke(&mut self, points: Vec<(f64, f64)>, cx: &mut Cx) {
        let (width, height) = self.picture_size();
        let (uw, uh) = self.units();
        // The units in the picture's pixels: the same when it is the canvas's size.
        let sx = width as f64 / uw.max(1.0);
        let sy = height as f64 / uh.max(1.0);
        let scaled: Vec<(f64, f64)> = points.iter().map(|(x, y)| (x * sx, y * sy)).collect();
        let bytes = painted_with(self.painted().as_ref(), width, height, &scaled, self.tool_in_hand(), color_of(cx.look(), &self.ink()), self.brush() * sx, self.modes.toolbox.opacity());
        // Shown as painted at once; the field's picture follows from the runtime.
        *self.modes.painted.borrow_mut() = Some((self.painted_for(), Image::from_encoded(skia_safe::Data::new_copy(&bytes))));
        cx.invalidate();
        let stroke = FieldContent::New(bytes).value();
        self.act(json!({ "stroke": stroke }));
    }

    /// The picture painted on as pixels, a blank one of its size when there is none yet.
    /// The pixels the tools work on: the layer in hand when there are layers, else the picture.
    pub(in crate::parts::data::canvas) fn painted_pixels(&self) -> clarity_image::RgbaImage {
        let layers = &self.modes.layers;
        if layers.active() {
            return layers.list[layers.in_hand].pixels.clone();
        }
        self.field_pixels()
    }

    /// The picture as shown, the layers composed.
    pub(in crate::parts::data::canvas) fn field_pixels(&self) -> clarity_image::RgbaImage {
        let (width, height) = self.picture_size();
        self.painted()
            .and_then(|i| i.encode(None, EncodedImageFormat::PNG, None))
            .and_then(|png| clarity_image::decode(png.as_bytes()).ok())
            .unwrap_or_else(|| clarity_image::RgbaImage::new(width.max(1) as u32, height.max(1) as u32))
    }

    /// The picture's pixel under a point in units.
    pub(super) fn pixel_of(&self, unit: (f64, f64), pixels: &clarity_image::RgbaImage) -> Option<(u32, u32)> {
        let (uw, uh) = self.units();
        let (x, y) = (unit.0 * f64::from(pixels.width()) / uw.max(1.0), unit.1 * f64::from(pixels.height()) / uh.max(1.0));
        (x >= 0.0 && y >= 0.0 && x < f64::from(pixels.width()) && y < f64::from(pixels.height())).then_some((x as u32, y as u32))
    }

    /// The fill bucket: the area of near colours under the point takes the ink, as one edit.
    pub(super) fn fill_at(&mut self, unit: (f64, f64), cx: &mut Cx) {
        let mut pixels = self.painted_pixels();
        let Some((x, y)) = self.pixel_of(unit, &pixels) else { return };
        let ink = color_of(cx.look(), &self.ink());
        let alpha = (f64::from(ink.a()) * self.modes.toolbox.opacity() / 100.0).round() as u8;
        clarity_image::ops::fill(&mut pixels, x, y, clarity_image::Rgba([ink.r(), ink.g(), ink.b(), alpha]), FILL_TOLERANCE);
        self.show_and_send(clarity_image::encode(&pixels), cx);
    }

    /// A new picture painted: shown at once, and sent as one edit; the field's picture follows from the runtime.
    pub(in crate::parts::data::canvas) fn show_and_send(&mut self, png: Vec<u8>, cx: &mut Cx) {
        // With layers, what the tools made is the layer in hand; the layers composed are kept.
        if self.modes.layers.active() {
            if let Ok(pixels) = clarity_image::decode(&png) {
                let at = self.modes.layers.in_hand;
                self.modes.layers.list[at].pixels = pixels;
            }
            self.send_composed(cx);
            return;
        }
        self.send_picture(png, cx);
    }

    /// The picture shown and kept as one edit.
    pub(in crate::parts::data::canvas) fn send_picture(&mut self, png: Vec<u8>, cx: &mut Cx) {
        *self.modes.painted.borrow_mut() = Some((self.painted_for(), Image::from_encoded(skia_safe::Data::new_copy(&png))));
        cx.invalidate();
        self.act(json!({ "stroke": FieldContent::New(png).value() }));
    }

    /// A picture shown while the person works (a selection lifted), not sent; with layers, the layer in hand as it would be.
    pub(in crate::parts::data::canvas) fn show_only(&self, png: &[u8]) {
        let shown = if self.modes.layers.active() {
            let (w, h) = self.field_pixels_size();
            let mut layers: Vec<super::super::layers::Layer> = self.modes.layers.list.clone();
            if let Ok(pixels) = clarity_image::decode(png) {
                layers[self.modes.layers.in_hand].pixels = pixels;
            }
            let all = super::super::layers::Layers { list: layers, in_hand: self.modes.layers.in_hand, sent: Vec::new(), seen: None };
            clarity_image::encode(&all.composed(w, h))
        } else {
            png.to_vec()
        };
        *self.modes.painted.borrow_mut() = Some((self.painted_for(), Image::from_encoded(skia_safe::Data::new_copy(&shown))));
    }

    /// The picture's size in pixels.
    pub(in crate::parts::data::canvas) fn field_pixels_size(&self) -> (u32, u32) {
        let (w, h) = self.picture_size();
        (w.max(1) as u32, h.max(1) as u32)
    }

    pub(in crate::parts::data::canvas) fn modes_layers(&self) -> &super::super::layers::Layers {
        &self.modes.layers
    }

    pub(in crate::parts::data::canvas) fn modes_layers_mut(&mut self) -> &mut super::super::layers::Layers {
        &mut self.modes.layers
    }

    /// The toolbox's layers tool sees the layers as they are.
    pub(in crate::parts::data::canvas) fn share_layers(&self) {
        self.modes.toolbox.set_layer_rows(self.modes.layers.rows());
    }

    /// The opacity the toolbox paints with, 0 to 100.
    pub(in crate::parts::data::canvas) fn opacity(&self) -> f64 {
        self.modes.toolbox.opacity()
    }

    /// What the raster tools keep while the person works (raster.rs).
    pub(in crate::parts::data::canvas) fn raster(&mut self) -> &mut super::super::raster::RasterState {
        &mut self.modes.raster
    }

    pub(in crate::parts::data::canvas) fn raster_ref(&self) -> &super::super::raster::RasterState {
        &self.modes.raster
    }

    /// The colour picker: the pixel under the point becomes the ink, written out.
    pub(super) fn pick_at(&mut self, unit: (f64, f64)) {
        let pixels = self.painted_pixels();
        let Some((x, y)) = self.pixel_of(unit, &pixels) else { return };
        let p = pixels.get_pixel(x, y);
        let ink = format!("#{:02x}{:02x}{:02x}", p[0], p[1], p[2]);
        if let Some(name) = self.base.name() {
            crate::parts::change_state(&*self.base.host, name, |s| s.ink = Some(ink.clone()));
        }
    }
}
