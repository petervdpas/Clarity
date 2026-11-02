//! An SVG picture of the resources folder is drawn from its shapes at the
//! size it is shown, sharp however large, and fits its place whichever way
//! the SVG gives its size (C# ImageSvgTests).

use clarity_ui::parts::containers::image::ImagePart;
use clarity_ui::picture::Picture;
use clarity_ui::testing::{Shot, TestHost};
use clarity_ui::{Element, Screen, ThemeLook};
use skia_safe::Color;

/// Red squares far bigger than their place, written each way an SVG gives its size.
const SQUARES: [&str; 5] = [
    r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>"##,
    r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>"##,
    r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="50 50 200 200"><rect x="50" y="50" width="200" height="200" fill="#ff0000"/></svg>"##,
    r##"<svg xmlns="http://www.w3.org/2000/svg" width="20cm" height="20cm" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>"##,
    r##"<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%" viewBox="0 0 200 200"><rect width="200" height="200" fill="#ff0000"/></svg>"##,
];

fn red(c: Color) -> bool {
    c.r() > 200 && c.g() < 60 && c.b() < 60
}

/// An image part showing `svg` as the whole of a screen `width` by `height`.
fn shown(svg: &str, width: f64, height: f64) -> Screen {
    let host = TestHost::new();
    host.resources.borrow_mut().insert("square.svg".into(), svg.as_bytes().to_vec());
    let spec = clarity_ui::markup::read(r#"<Window name="Test"><Image name="logo" source="square.svg" /></Window>"#).unwrap();
    let part: Box<dyn Element> = clarity_ui::parts::make(spec.parts[0].clone(), host);
    let mut screen = Screen::new(part, ThemeLook::clarity(false));
    screen.resize(width, height, 1.0);
    screen
}

#[test]
fn an_svg_fits_its_place() {
    for square in SQUARES {
        // A 40 by 40 place in a 120 by 120 picture: the points past the place and below it are inside the picture.
        let mut screen = shown(square, 40.0, 40.0);
        let mut surface = skia_safe::surfaces::raster_n32_premul((120, 120)).unwrap();
        surface.canvas().clear(Color::TRANSPARENT);
        screen.draw(surface.canvas());
        let image = screen.root().as_any().downcast_ref::<ImagePart>().expect("the screen is an image");
        assert!(matches!(image.shown(), Some(Picture::Svg(_))), "{square}");
        let pixmap = surface.peek_pixels().unwrap();
        assert!(red(clarity_ui::testing::pixel_at(&pixmap, 20, 20)), "inside its place: {square}");
        assert!(!red(clarity_ui::testing::pixel_at(&pixmap, 80, 80)), "past its place: {square}");
        assert!(!red(clarity_ui::testing::pixel_at(&pixmap, 20, 80)), "below its place: {square}");
    }
}

#[test]
fn an_svg_shown_large_stays_sharp() {
    // Ten units wide, black on the left half: shown 800 pixels wide, its edge is at 400.
    let halves = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="5" height="10" fill="#000000"/><rect x="5" width="5" height="10" fill="#ffffff"/></svg>"##;
    let mut screen = shown(halves, 800.0, 800.0);
    let mut shot = Shot::of(&mut screen, 800, 800, 1.0);
    // A picture drawn small and blown up blurs pixels around its edge; shapes drawn at the size shown do not.
    assert_eq!(Color::BLACK, shot.pixel(398, 400));
    assert_eq!(Color::WHITE, shot.pixel(401, 400));
}
