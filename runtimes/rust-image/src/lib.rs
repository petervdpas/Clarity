//! Clarity.Image: the picture functions of code (docs/IMAGE-PLAN.md), on
//! pictures as their encoded bytes, giving PNG. The pixel work is in
//! `ops`, on decoded RGBA pictures, which Clarity.UI's painting canvas uses too.

pub mod ops;

/// The decoded picture and its pixel, for the painting canvas that works on them.
pub use image::{Rgba, RgbaImage};


/// Why a picture could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The bytes are no PNG, JPEG or WebP picture.
    NotAPicture(String),
    /// A size of 0, or more than 30000 pixels a side.
    BadSize,
}

impl Problem {
    pub fn detail(&self) -> String {
        match self {
            Problem::NotAPicture(why) => why.clone(),
            Problem::BadSize => "a picture is 1 to 30000 pixels a side".to_string(),
        }
    }
}

/// The largest side a picture made here may have.
pub const MAX_SIDE: u32 = 30000;

/// The pixels of an encoded picture.
pub fn decode(bytes: &[u8]) -> Result<RgbaImage, Problem> {
    image::load_from_memory(bytes).map(|i| i.to_rgba8()).map_err(|e| Problem::NotAPicture(e.to_string()))
}

/// A picture as PNG.
pub fn encode(picture: &RgbaImage) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    // Writing PNG into memory fails only when out of memory.
    let _ = picture.write_to(&mut out, image::ImageFormat::Png);
    out.into_inner()
}

fn size_ok(width: i64, height: i64) -> Result<(u32, u32), Problem> {
    let ok = |n: i64| (1..=i64::from(MAX_SIDE)).contains(&n);
    if ok(width) && ok(height) { Ok((width as u32, height as u32)) } else { Err(Problem::BadSize) }
}

/// `resize(p, width, height)`.
pub fn resize(bytes: &[u8], width: i64, height: i64) -> Result<Vec<u8>, Problem> {
    let (w, h) = size_ok(width, height)?;
    Ok(encode(&ops::resize(&decode(bytes)?, w, h)))
}

/// `crop(p, x, y, width, height)`, clamped to the picture; a crop outside it gives a 1 by 1 transparent picture.
pub fn crop(bytes: &[u8], x: i64, y: i64, width: i64, height: i64) -> Result<Vec<u8>, Problem> {
    Ok(encode(&ops::crop(&decode(bytes)?, x, y, width, height)))
}

/// `rotate(p, degrees)`.
pub fn rotate(bytes: &[u8], degrees: f64) -> Result<Vec<u8>, Problem> {
    Ok(encode(&ops::rotate(&decode(bytes)?, degrees)))
}

/// `flip(p, horizontal)`.
pub fn flip(bytes: &[u8], horizontal: bool) -> Result<Vec<u8>, Problem> {
    Ok(encode(&ops::flip(&decode(bytes)?, horizontal)))
}

/// `blend(base, over, x, y, opacity)`.
pub fn blend(base: &[u8], over: &[u8], x: i64, y: i64, opacity: i64) -> Result<Vec<u8>, Problem> {
    let mut base = decode(base)?;
    ops::blend(&mut base, &decode(over)?, x, y, opacity);
    Ok(encode(&base))
}

/// `adjust(p, hue, saturation, brightness, contrast)`.
pub fn adjust(bytes: &[u8], hue: f64, saturation: f64, brightness: f64, contrast: f64) -> Result<Vec<u8>, Problem> {
    let mut picture = decode(bytes)?;
    ops::adjust(&mut picture, hue, saturation, brightness, contrast);
    Ok(encode(&picture))
}

/// `pictureWidth(p)` and `pictureHeight(p)`: its size in pixels.
pub fn size(bytes: &[u8]) -> Result<(u32, u32), Problem> {
    let picture = decode(bytes)?;
    Ok(picture.dimensions())
}
