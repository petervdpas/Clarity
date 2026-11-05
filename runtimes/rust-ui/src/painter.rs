//! What elements draw with: the canvas, the theme and its fonts, and the few
//! shapes every part is made of.

use skia_safe::{Canvas, ClipOp, Color, Paint, PaintStyle};

use crate::fonts::Fonts;
use crate::geometry::Rect;
use crate::theme::ThemeLook;

/// Where a line of text lies across its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// The canvas with the theme and fonts it is drawn in.
pub struct Painter<'a> {
    pub canvas: &'a Canvas,
    pub look: &'a ThemeLook,
    pub fonts: &'a Fonts,
    /// A part's style says bold (its `weight`): every text it draws is bold.
    pub style_bold: bool,
    /// A part's style says where its text sits (its `align`): every text it draws at the left goes there instead.
    pub style_align: Option<TextAlign>,
}

/// The look a part's style gives what it draws: a look of its own (the
/// style's text color, size and font over the window's), and the style's
/// weight and align.
pub struct StyledLook {
    /// The style's look and its fonts; none to keep the window's.
    pub look: Option<(std::sync::Arc<ThemeLook>, std::rc::Rc<Fonts>)>,
    pub bold: bool,
    pub align: Option<TextAlign>,
}

impl StyledLook {
    /// The painter of this look on the canvas of `under`.
    pub fn painter<'b>(&'b self, under: &Painter<'b>) -> Painter<'b> {
        let (look, fonts) = match &self.look {
            Some((look, fonts)) => (look.as_ref(), fonts.as_ref()),
            None => (under.look, under.fonts),
        };
        Painter { canvas: under.canvas, look, fonts, style_bold: self.bold || under.style_bold, style_align: self.align.or(under.style_align) }
    }
}

/// The corners of a push that are round: all of them, or only the outer ones of a joined group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Corners {
    pub left: bool,
    pub right: bool,
}

impl Corners {
    pub const ALL: Corners = Corners { left: true, right: true };
    pub const NONE: Corners = Corners { left: false, right: false };
}

/// Puts the canvas back as it was when it is dropped: after a clip, or the dimmed look.
pub struct Restore<'a> {
    canvas: &'a Canvas,
}

impl Drop for Restore<'_> {
    fn drop(&mut self) {
        self.canvas.restore();
    }
}

impl<'a> Painter<'a> {
    pub fn new(canvas: &'a Canvas, look: &'a ThemeLook, fonts: &'a Fonts) -> Painter<'a> {
        Painter { canvas, look, fonts, style_bold: false, style_align: None }
    }

    fn paint(color: Color, style: PaintStyle, antialias: bool) -> Paint {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.set_style(style);
        paint.set_anti_alias(antialias);
        paint
    }

    /// A filled place, with rounded corners of `radius`.
    pub fn fill(&self, area: Rect, color: Color, radius: f64) {
        let paint = Self::paint(color, PaintStyle::Fill, radius > 0.0);
        if radius > 0.0 {
            self.canvas.draw_round_rect(area.sk(), radius as f32, radius as f32, &paint);
        } else {
            self.canvas.draw_rect(area.sk(), &paint);
        }
    }

    /// A line around the inside of `area`.
    pub fn stroke(&self, area: Rect, color: Color, width: f64, radius: f64) {
        if width <= 0.0 || color.a() == 0 {
            return;
        }
        let mut paint = Self::paint(color, PaintStyle::Stroke, true);
        paint.set_stroke_width(width as f32);
        let inside = area.inset(width / 2.0).sk();
        if radius > 0.0 {
            self.canvas.draw_round_rect(inside, radius as f32, radius as f32, &paint);
        } else {
            self.canvas.draw_rect(inside, &paint);
        }
    }

    /// A straight line from one point to another.
    pub fn line(&self, from: (f64, f64), to: (f64, f64), color: Color, width: f64) {
        let mut paint = Self::paint(color, PaintStyle::Stroke, true);
        paint.set_stroke_width(width as f32);
        self.canvas.draw_line((from.0 as f32, from.1 as f32), (to.0 as f32, to.1 as f32), &paint);
    }

    /// A line of text in `area`, in the middle of its height, cut off at its edges.
    #[allow(clippy::too_many_arguments)]
    pub fn text(&self, text: &str, area: Rect, color: Color, size: f64, role: &str, bold: bool, align: TextAlign) {
        if text.is_empty() {
            return;
        }
        let bold = bold || self.style_bold;
        let align = match (align, self.style_align) {
            (TextAlign::Left, Some(styled)) => styled,
            _ => align,
        };
        let font = self.fonts.font(role, size, bold);
        let paint = Self::paint(color, PaintStyle::Fill, true);
        let width = self.fonts.measure(text, role, size, bold);
        let (_, metrics) = font.metrics();
        let x = match align {
            TextAlign::Center => area.x + (area.width - width) / 2.0,
            TextAlign::Right => area.right() - width,
            TextAlign::Left => area.x,
        };
        let ascent = metrics.ascent as f64;
        let descent = metrics.descent as f64;
        let baseline = area.y + (area.height - (descent - ascent)) / 2.0 - ascent;
        let _clip = self.clip(area);
        // Characters the role's font lacks are drawn in a fallback font, run by run.
        self.fonts.draw(self.canvas, text, x, baseline, role, size, bold, &paint);
    }

    /// A line of text turned a quarter clockwise, read from top to bottom, in the middle of `area` (a tall, narrow one), as a side tab's caption.
    pub fn turned_text(&self, text: &str, area: Rect, color: Color, size: f64) {
        self.turned_line(text, area, color, size, "text", false, TextAlign::Center);
    }

    /// A line of text turned a quarter clockwise in `area`, read from its top
    /// down, in the font and weight given; `align` places it along the area's height.
    #[allow(clippy::too_many_arguments)]
    pub fn turned_line(&self, text: &str, area: Rect, color: Color, size: f64, font: &str, bold: bool, align: TextAlign) {
        if text.is_empty() {
            return;
        }
        let _clip = self.clip(area);
        self.canvas.save();
        self.canvas.translate((area.right() as f32, area.y as f32));
        self.canvas.rotate(90.0, None);
        // Turned, the area's height is the line's width and its width the line's height.
        self.text(text, Rect::new(0.0, 0.0, area.height, area.width), color, size, font, bold, align);
        self.canvas.restore();
    }

    /// A line of text in the theme's text font, left, not bold: the plain case of `text`.
    pub fn plain_text(&self, text: &str, area: Rect, color: Color, size: f64) {
        self.text(text, area, color, size, "text", false, TextAlign::Left);
    }

    /// Draws at the opacity of a disabled part until what it gives is dropped; nothing changes for one that is enabled.
    pub fn dimmed(&self, enabled: bool) -> Option<Restore<'a>> {
        if enabled {
            return None;
        }
        self.canvas.save_layer_alpha_f(None, 140.0 / 255.0);
        Some(Restore { canvas: self.canvas })
    }

    /// The focus ring around `area`, in the theme's focus color.
    pub fn focus_ring(&self, area: Rect, radius: f64) {
        self.stroke(area, self.look.color("focus"), 2.0, radius);
    }

    fn rounded(area: Rect, radius: f64, corners: Corners) -> skia_safe::RRect {
        let r = radius.max(0.0) as f32;
        let left = if corners.left { skia_safe::Vector::new(r, r) } else { skia_safe::Vector::new(0.0, 0.0) };
        let right = if corners.right { skia_safe::Vector::new(r, r) } else { skia_safe::Vector::new(0.0, 0.0) };
        skia_safe::RRect::new_rect_radii(area.sk(), &[left, right, right, left])
    }

    /// A push's plate: filled, with a line around the inside, round at `corners`.
    pub fn plate(&self, area: Rect, fill: Color, line: Color, width: f64, radius: f64, corners: Corners) {
        let paint = Self::paint(fill, PaintStyle::Fill, true);
        self.canvas.draw_rrect(Self::rounded(area, radius, corners), &paint);
        if width <= 0.0 || line.a() == 0 {
            return;
        }
        let mut paint = Self::paint(line, PaintStyle::Stroke, true);
        paint.set_stroke_width(width as f32);
        self.canvas.draw_rrect(Self::rounded(area.inset(width / 2.0), (radius - width / 2.0).max(0.0), corners), &paint);
    }

    /// A small arrow pointing down, in the middle of `area`: a dropdown's and a split button's.
    pub fn down_arrow(&self, area: Rect, color: Color) {
        let cx = area.x + area.width / 2.0;
        let cy = area.y + area.height / 2.0;
        self.line((cx - 4.0, cy - 2.0), (cx, cy + 2.0), color, 1.5);
        self.line((cx, cy + 2.0), (cx + 4.0, cy - 2.0), color, 1.5);
    }

    /// A line cut off with an ellipsis where it is wider than `width`.
    pub fn fit(&self, text: &str, width: f64, size: f64, role: &str, bold: bool) -> String {
        if text.is_empty() || self.fonts.measure(text, role, size, bold) <= width {
            return text.to_string();
        }
        const DOTS: &str = "\u{2026}";
        let chars: Vec<char> = text.chars().collect();
        let (mut low, mut high) = (0usize, chars.len());
        while low < high {
            let mid = (low + high).div_ceil(2);
            let tried: String = chars[..mid].iter().collect::<String>() + DOTS;
            if self.fonts.measure(&tried, role, size, bold) <= width {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        chars[..low].iter().collect::<String>().trim_end().to_string() + DOTS
    }

    /// Pixels made elsewhere (a custom part that renders into a texture),
    /// drawn scaled into `area` with linear sampling: `rgba` holds `height`
    /// rows of `width` RGBA pixels, unpremultiplied, each row `row_bytes`
    /// long (a GPU's readback pads rows). Draws nothing when the bytes do not
    /// hold that many rows, or a row does not hold `width` pixels.
    pub fn pixels(&self, rgba: &[u8], width: u32, height: u32, row_bytes: usize, area: Rect) {
        if width == 0 || height == 0 || row_bytes < width as usize * 4 || rgba.len() < row_bytes * height as usize {
            return;
        }
        let info = skia_safe::ImageInfo::new((width as i32, height as i32), skia_safe::ColorType::RGBA8888, skia_safe::AlphaType::Unpremul, None);
        let data = skia_safe::Data::new_copy(&rgba[..row_bytes * height as usize]);
        let Some(image) = skia_safe::images::raster_from_data(&info, data, row_bytes) else { return };
        let sampling = skia_safe::SamplingOptions::new(skia_safe::FilterMode::Linear, skia_safe::MipmapMode::None);
        self.canvas.draw_image_rect_with_sampling_options(&image, None, area.sk(), sampling, &Paint::default());
    }

    /// Draws only inside `area` until what it gives is dropped.
    pub fn clip(&self, area: Rect) -> Restore<'a> {
        self.canvas.save();
        self.canvas.clip_rect(area.sk(), ClipOp::Intersect, true);
        Restore { canvas: self.canvas }
    }
}
