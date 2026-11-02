//! A custom part that renders elsewhere (a game engine into a texture) puts
//! its pixels on Clarity.UI's canvas through `Painter::pixels`: RGBA bytes,
//! rows padded as a GPU's readback pads them, drawn scaled into a place.

use clarity_ui::Rect;
use clarity_ui::testing::{TestHost, near, window_of};
use skia_safe::Color;

/// 4 x 2 pixels, the left half red and the right half blue, each row padded to 256 bytes.
fn halves() -> Vec<u8> {
    let mut rgba = vec![0u8; 256 * 2];
    for y in 0..2 {
        for x in 0..4 {
            let at = y * 256 + x * 4;
            let pixel = if x < 2 { [255, 0, 0, 255] } else { [0, 0, 255, 255] };
            rgba[at..at + 4].copy_from_slice(&pixel);
        }
    }
    rgba
}

#[test]
fn pixels_are_drawn_scaled_into_their_place() {
    let screen = window_of(r#"<Label at="1,1" span="1" name="l" caption="" />"#, TestHost::new());
    let mut surface = skia_safe::surfaces::raster_n32_premul((40, 20)).unwrap();
    surface.canvas().clear(Color::WHITE);
    let fonts = clarity_ui::fonts::Fonts::new(screen.look().clone());
    let painter = clarity_ui::Painter::new(surface.canvas(), screen.look(), &fonts);
    painter.pixels(&halves(), 4, 2, 256, Rect::new(0.0, 0.0, 40.0, 20.0));
    let mut pixel = |x: i32, y: i32| {
        let mut out = [0u8; 4];
        let info = skia_safe::ImageInfo::new((1, 1), skia_safe::ColorType::RGBA8888, skia_safe::AlphaType::Unpremul, None);
        assert!(surface.read_pixels(&info, &mut out, 4, (x, y)));
        Color::from_argb(out[3], out[0], out[1], out[2])
    };
    assert!(near(pixel(2, 10), Color::RED, 8), "{:?}", pixel(2, 10));
    assert!(near(pixel(37, 10), Color::BLUE, 8), "{:?}", pixel(37, 10));
}

#[test]
fn too_few_bytes_draw_nothing() {
    let screen = window_of(r#"<Label at="1,1" span="1" name="l" caption="" />"#, TestHost::new());
    let mut surface = skia_safe::surfaces::raster_n32_premul((40, 20)).unwrap();
    surface.canvas().clear(Color::WHITE);
    let fonts = clarity_ui::fonts::Fonts::new(screen.look().clone());
    let painter = clarity_ui::Painter::new(surface.canvas(), screen.look(), &fonts);
    let short = &halves()[..300];
    painter.pixels(short, 4, 2, 256, Rect::new(0.0, 0.0, 40.0, 20.0));
    // Nor a width that does not fit the rows.
    painter.pixels(&halves(), 100, 2, 256, Rect::new(0.0, 0.0, 40.0, 20.0));
    let mut out = [0u8; 4];
    let info = skia_safe::ImageInfo::new((1, 1), skia_safe::ColorType::RGBA8888, skia_safe::AlphaType::Unpremul, None);
    assert!(surface.read_pixels(&info, &mut out, 4, (2, 10)));
    assert_eq!(out, [255, 255, 255, 255]);
}

/// A part crate reaches Skia's whole canvas through Clarity.UI, without a dependency of its own.
#[test]
fn skia_is_reexported_for_parts() {
    let _: clarity_ui::skia_safe::Color = clarity_ui::skia_safe::Color::RED;
}
