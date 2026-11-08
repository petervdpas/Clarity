//! The picture functions of code (docs/IMAGE-PLAN.md) on the runtime's
//! pictures, through Clarity.Image: each takes a picture in hand (or bytes
//! read from a file) and gives a new one as PNG, leaving its arguments as
//! they were. Every stack gives the same pixels (conformance/picture-functions.json).

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::picture::Picture;
use crate::{Error, Result};

/// What a picture function takes as a picture: one in hand, or read bytes.
pub trait InHand {
    fn picture_bytes(&self) -> Result<&[u8]>;
}

impl InHand for Option<Picture> {
    fn picture_bytes(&self) -> Result<&[u8]> {
        match self {
            Some(Picture::New(bytes)) => Ok(bytes),
            None => Err(Error::Invalid("there is no picture".into())),
            // Generated code reads a record's picture before it gives it to a function.
            Some(Picture::Stored | Picture::InStore(_)) => Err(Error::Invalid("the picture is not read from its record".into())),
        }
    }
}

impl InHand for Vec<u8> {
    fn picture_bytes(&self) -> Result<&[u8]> {
        Ok(self)
    }
}

/// What a picture function takes as a number: whole or decimal.
pub trait Number {
    fn number(&self) -> f64;
}

impl Number for i64 {
    fn number(&self) -> f64 {
        *self as f64
    }
}

impl Number for Decimal {
    fn number(&self) -> f64 {
        self.to_f64().unwrap_or(0.0)
    }
}

fn made(result: std::result::Result<Vec<u8>, clarity_image::Problem>) -> Result<Option<Picture>> {
    result.map(|bytes| Some(Picture::New(bytes))).map_err(|p| Error::Invalid(p.detail()))
}

/// `resize(p, width, height)`.
pub fn resize(p: &impl InHand, width: i64, height: i64) -> Result<Option<Picture>> {
    made(clarity_image::resize(p.picture_bytes()?, width, height))
}

/// `crop(p, x, y, width, height)`.
pub fn crop(p: &impl InHand, x: i64, y: i64, width: i64, height: i64) -> Result<Option<Picture>> {
    made(clarity_image::crop(p.picture_bytes()?, x, y, width, height))
}

/// `rotate(p, degrees)`.
pub fn rotate(p: &impl InHand, degrees: &impl Number) -> Result<Option<Picture>> {
    made(clarity_image::rotate(p.picture_bytes()?, degrees.number()))
}

/// `flip(p, horizontal)`.
pub fn flip(p: &impl InHand, horizontal: bool) -> Result<Option<Picture>> {
    made(clarity_image::flip(p.picture_bytes()?, horizontal))
}

/// `blend(base, over, x, y, opacity)`.
pub fn blend(base: &impl InHand, over: &impl InHand, x: i64, y: i64, opacity: i64) -> Result<Option<Picture>> {
    made(clarity_image::blend(base.picture_bytes()?, over.picture_bytes()?, x, y, opacity))
}

/// `adjust(p, hue, saturation, brightness, contrast)`.
pub fn adjust(p: &impl InHand, hue: &impl Number, saturation: &impl Number, brightness: &impl Number, contrast: &impl Number) -> Result<Option<Picture>> {
    made(clarity_image::adjust(p.picture_bytes()?, hue.number(), saturation.number(), brightness.number(), contrast.number()))
}

/// `pictureWidth(p)`.
pub fn width(p: &impl InHand) -> Result<i64> {
    clarity_image::size(p.picture_bytes()?).map(|(w, _)| i64::from(w)).map_err(|p| Error::Invalid(p.detail()))
}

/// `pictureHeight(p)`.
pub fn height(p: &impl InHand) -> Result<i64> {
    clarity_image::size(p.picture_bytes()?).map(|(_, h)| i64::from(h)).map_err(|p| Error::Invalid(p.detail()))
}
