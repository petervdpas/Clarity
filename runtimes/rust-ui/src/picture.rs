//! A picture of the project's resources as the parts draw it: a bitmap
//! Skia decodes (PNG, JPEG, GIF's first frame, BMP, ICO), or an SVG read by
//! resvg (usvg) and drawn by it (tiny-skia) into a bitmap at the size it
//! shows on the screen, then drawn with Skia like any bitmap. An SVG keeps
//! the bitmap it drew last and draws again only at another size.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use resvg::{tiny_skia, usvg};
use skia_safe::{AlphaType, Canvas, ColorType, Data, FilterMode, Image, ImageInfo, MipmapMode, Paint, SamplingOptions};

use crate::geometry::Rect;

/// A picture as the parts draw it.
#[derive(Clone)]
pub enum Picture {
    Bitmap(Image),
    Svg(Rc<SvgPicture>),
}

/// An SVG read into its shapes, with the bitmap it was drawn into last.
pub struct SvgPicture {
    tree: usvg::Tree,
    drawn: RefCell<Option<Image>>,
}

/// The largest side an SVG is drawn at, so a huge place asks no huge bitmap.
const MOST: i32 = 8192;

/// The system's fonts for the texts of SVG pictures, read once.
fn fonts() -> Arc<usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut fonts = usvg::fontdb::Database::new();
            fonts.load_system_fonts();
            Arc::new(fonts)
        })
        .clone()
}

/// Whether a picture is an SVG: by its name, or by what it holds when its name says nothing.
pub fn is_svg(name: &str, bytes: &[u8]) -> bool {
    if name.to_ascii_lowercase().ends_with(".svg") {
        return true;
    }
    let start = String::from_utf8_lossy(&bytes[..bytes.len().min(256)]).trim_start().to_ascii_lowercase();
    start.starts_with("<svg") || (start.starts_with("<?xml") && String::from_utf8_lossy(bytes).contains("<svg"))
}

impl SvgPicture {
    /// The SVG of `bytes` read into its shapes; none when it is no SVG resvg reads, or has no size.
    pub fn read(bytes: &[u8]) -> Option<SvgPicture> {
        let mut options = usvg::Options::default();
        if String::from_utf8_lossy(bytes).contains("<text") {
            options.fontdb = fonts();
        }
        let tree = usvg::Tree::from_data(bytes, &options).ok()?;
        let size = tree.size();
        if size.width() <= 0.0 || size.height() <= 0.0 {
            return None;
        }
        Some(SvgPicture { tree, drawn: RefCell::new(None) })
    }

    /// Its own size, as its width and height (or its view box) say.
    pub fn size(&self) -> (f64, f64) {
        let size = self.tree.size();
        (size.width() as f64, size.height() as f64)
    }

    /// The size of the bitmap it was drawn into last; none before it was drawn.
    pub fn drawn_size(&self) -> Option<(i32, i32)> {
        self.drawn.borrow().as_ref().map(|i| (i.width(), i.height()))
    }

    /// It drawn into a bitmap `width` by `height` pixels, stretched to fill
    /// it; the one drawn last when it has that size.
    pub fn image_at(&self, width: i32, height: i32) -> Option<Image> {
        let (width, height) = (width.clamp(1, MOST), height.clamp(1, MOST));
        if let Some(drawn) = self.drawn.borrow().as_ref().filter(|i| i.width() == width && i.height() == height) {
            return Some(drawn.clone());
        }
        let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32)?;
        let (w, h) = self.size();
        let transform = tiny_skia::Transform::from_scale((width as f64 / w) as f32, (height as f64 / h) as f32);
        resvg::render(&self.tree, transform, &mut pixmap.as_mut());
        // tiny-skia keeps premultiplied RGBA, row after row.
        let info = ImageInfo::new((width, height), ColorType::RGBA8888, AlphaType::Premul, None);
        let image = skia_safe::images::raster_from_data(&info, Data::new_copy(pixmap.data()), width as usize * 4)?;
        *self.drawn.borrow_mut() = Some(image.clone());
        Some(image)
    }
}

impl Picture {
    /// The picture of `bytes` named `name`; none when they are no picture Skia decodes or resvg reads.
    pub fn read(name: &str, bytes: &[u8]) -> Option<Picture> {
        if bytes.is_empty() {
            return None;
        }
        if is_svg(name, bytes) {
            return SvgPicture::read(bytes).map(|svg| Picture::Svg(Rc::new(svg)));
        }
        Image::from_encoded(Data::new_copy(bytes)).map(Picture::Bitmap)
    }

    /// Its own width.
    pub fn width(&self) -> f64 {
        match self {
            Picture::Bitmap(image) => image.width() as f64,
            Picture::Svg(svg) => svg.size().0,
        }
    }

    /// Its own height.
    pub fn height(&self) -> f64 {
        match self {
            Picture::Bitmap(image) => image.height() as f64,
            Picture::Svg(svg) => svg.size().1,
        }
    }

    /// Its bitmap at `width` by `height` pixels: a bitmap as it is, an SVG drawn at that size.
    pub fn image_at(&self, width: i32, height: i32) -> Option<Image> {
        match self {
            Picture::Bitmap(image) => Some(image.clone()),
            Picture::Svg(svg) => svg.image_at(width, height),
        }
    }

    /// Draws it stretched over `place` on `canvas`; an SVG is drawn into a
    /// bitmap of the pixels `place` covers on the screen, so it is sharp at
    /// every zoom and scale.
    pub fn draw(&self, canvas: &Canvas, place: Rect) {
        if place.width <= 0.0 || place.height <= 0.0 {
            return;
        }
        let image = match self {
            Picture::Bitmap(image) => Some(image.clone()),
            Picture::Svg(svg) => {
                let scale = canvas.local_to_device_as_3x3().map_radius(1.0).unwrap_or(1.0).max(0.01) as f64;
                svg.image_at((place.width * scale).round() as i32, (place.height * scale).round() as i32)
            }
        };
        if let Some(image) = image {
            let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear);
            canvas.draw_image_rect_with_sampling_options(&image, None, place.sk(), sampling, &Paint::default());
        }
    }
}
