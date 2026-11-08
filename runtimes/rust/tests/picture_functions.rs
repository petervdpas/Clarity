//! The picture functions on the runtime's pictures (docs/IMAGE-PLAN.md),
//! through Clarity.Image: a picture in hand or read bytes, a new picture back.

#![cfg(feature = "pictures")]

use clarity_runtime::picture::Picture;
use clarity_runtime::picture_functions as pictures;
use rust_decimal::Decimal;

/// A picture of `width` by `height` red pixels, as PNG.
fn red(width: u32, height: u32) -> Option<Picture> {
    let pixels = clarity_image::RgbaImage::from_pixel(width, height, clarity_image::Rgba([255, 0, 0, 255]));
    Some(Picture::New(clarity_image::encode(&pixels)))
}

fn size(p: &Option<Picture>) -> (i64, i64) {
    (pictures::width(p).unwrap(), pictures::height(p).unwrap())
}

#[test]
fn each_function_gives_a_new_picture() {
    let p = red(4, 2);
    assert_eq!((8, 6), size(&pictures::resize(&p, 8, 6).unwrap()));
    assert_eq!((2, 1), size(&pictures::crop(&p, 1, 1, 2, 5).unwrap()));
    assert_eq!((2, 4), size(&pictures::rotate(&p, &90).unwrap()));
    assert_eq!((2, 4), size(&pictures::rotate(&p, &Decimal::from(270)).unwrap()));
    assert_eq!((4, 2), size(&pictures::flip(&p, true).unwrap()));
    assert_eq!((4, 2), size(&pictures::blend(&p, &red(1, 1), 1, 1, 50).unwrap()));
    assert_eq!((4, 2), size(&pictures::adjust(&p, &30, &Decimal::new(-205, 1), &0, &0).unwrap()));
    assert_eq!((4, 2), size(&p), "the argument is left as it was");
}

#[test]
fn read_bytes_are_a_picture_too() {
    let Some(Picture::New(bytes)) = red(3, 3) else { unreachable!() };
    assert_eq!(3, pictures::width(&bytes).unwrap());
}

#[test]
fn no_picture_is_refused_with_a_message() {
    for none in [None, Some(Picture::Stored), Some(Picture::New(b"no picture".to_vec()))] {
        let refused = pictures::resize(&none, 2, 2).unwrap_err().to_string();
        assert!(!refused.is_empty());
    }
    assert!(pictures::resize(&red(2, 2), 0, 2).is_err(), "a size of 0 is refused");
}
