//! An SVG picture (an image part's, a canvas image shape's, an icon written
//! as a whole SVG) is drawn from its shapes by resvg, into a bitmap at the
//! size it shows, and drawn with Skia (C# SvgPictureTests).

use clarity_ui::parts::containers::icon;
use clarity_ui::parts::containers::image::{self, ImagePart};
use clarity_ui::picture::Picture;
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::{Rect, Screen};
use serde_json::json;
use skia_safe::Color;

const FIGURE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="80" viewBox="0 0 40 80"><rect x="10" y="10" width="20" height="60" fill="#202020"/></svg>"##;

const SHOTS: &str = "/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots";

fn dark() -> Color {
    Color::from_rgb(0x20, 0x20, 0x20)
}

#[test]
fn an_svg_picture_is_drawn_from_its_shapes() {
    let picture = image::picture("walk-right-1.svg", Some(FIGURE.as_bytes())).expect("an SVG is a picture");
    assert!(picture.width() >= 40 && picture.height() >= 80);
    assert_eq!(picture.width() * 2, picture.height());
    let pixels = |x: i32, y: i32| {
        let mut out = [0u8; 4];
        let info = skia_safe::ImageInfo::new((1, 1), skia_safe::ColorType::RGBA8888, skia_safe::AlphaType::Unpremul, None);
        assert!(picture.read_pixels(&info, &mut out, 4, (x, y), skia_safe::image::CachingHint::Allow));
        out
    };
    // The figure is dark in the middle, and nothing is drawn around it.
    assert_eq!([0x20, 0x20, 0x20, 0xff], pixels(picture.width() / 2, picture.height() / 2));
    assert_eq!(0, pixels(1, 1)[3]);
}

#[test]
fn an_svg_that_is_no_svg_is_no_picture() {
    assert!(image::picture("broken.svg", Some(b"not svg")).is_none());
    assert!(Picture::read("broken.svg", b"not svg").is_none());
}

#[test]
fn an_svg_is_drawn_at_the_size_it_shows() {
    let picture = Picture::read("figure.svg", FIGURE.as_bytes()).expect("an SVG is a picture");
    assert_eq!((40.0, 80.0), (picture.width(), picture.height()));
    // Read by what it holds too, whatever its name says.
    assert!(matches!(Picture::read("figure", FIGURE.as_bytes()), Some(Picture::Svg(_))));
    let at = picture.image_at(100, 200).expect("it draws");
    assert_eq!((100, 200), (at.width(), at.height()));
    let Picture::Svg(svg) = &picture else { panic!("an SVG") };
    assert_eq!(Some((100, 200)), svg.drawn_size());
    // The same size again is the bitmap already drawn.
    assert_eq!(at.unique_id(), picture.image_at(100, 200).unwrap().unique_id());
}

fn image_screen(scale: f32) -> (Screen, Shot, Rect) {
    let host = TestHost::new();
    host.resources.borrow_mut().insert("figure.svg".into(), FIGURE.as_bytes().to_vec());
    let mut screen = window_of(r#"<Image at="1,1" span="6" rows="3" name="figure" source="figure.svg" />"#, host);
    let shot = Shot::of(&mut screen, 808, 400, scale);
    let bounds = screen.element(part_named(&screen, "figure")).unwrap().node().bounds;
    (screen, shot, bounds)
}

#[test]
fn an_image_part_draws_its_svg_picture() {
    let (screen, mut shot, b) = image_screen(1.0);
    let part = screen.element(part_named(&screen, "figure")).and_then(|e| e.as_any().downcast_ref::<ImagePart>()).unwrap();
    assert!(part.shown().is_some(), "an SVG shows");
    // Fitted in proportion: twice as high as wide, centered, dark in the middle.
    let place = image::fit(b, 40.0, 80.0);
    assert!(near(dark(), shot.pixel((place.x + place.width / 2.0) as i32, (place.y + place.height / 2.0) as i32), 8));
    // The SVG's own empty margin shows the window behind it.
    assert_eq!(screen.look().color("background"), shot.pixel((place.x + 2.0) as i32, (place.y + 2.0) as i32));
    shot.save(format!("{SHOTS}/finish-svg-image.png")).ok();
    // Drawn on a screen of twice the pixels, it is drawn at twice the size.
    let (screen, _, b) = image_screen(2.0);
    let part = screen.element(part_named(&screen, "figure")).and_then(|e| e.as_any().downcast_ref::<ImagePart>()).unwrap();
    let place = image::fit(b, 40.0, 80.0);
    let Some(Picture::Svg(svg)) = part.shown() else { panic!("an SVG") };
    let (w, h) = svg.drawn_size().expect("drawn");
    assert_eq!(((place.width * 2.0).round() as i32, (place.height * 2.0).round() as i32), (w, h));
}

#[test]
fn a_canvas_image_shape_draws_an_svg_picture() {
    let host = TestHost::new();
    host.resources.borrow_mut().insert("figure.svg".into(), FIGURE.as_bytes().to_vec());
    host.set_state("pad", |s| s.drawing = Some(vec![json!({"op": "image", "x": 0, "y": 0, "width": 40, "height": 80, "picture": "figure.svg"})]));
    let mut screen = window_of(r#"<Canvas name="pad" at="1,1" span="12" rows="8" units="{&quot;width&quot;:40,&quot;height&quot;:80}" />"#, host);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let b = screen.element(part_named(&screen, "pad")).unwrap().node().bounds;
    let place = image::fit(b, 40.0, 80.0);
    assert!(near(dark(), shot.pixel((place.x + place.width / 2.0) as i32, (place.y + place.height / 2.0) as i32), 8));
    shot.save(format!("{SHOTS}/finish-svg-canvas.png")).ok();
}

#[test]
fn an_icon_written_as_a_whole_svg_draws_it() {
    let host = TestHost::new();
    host.icons.borrow_mut().insert("figure".into(), FIGURE.to_string());
    let mut screen = window_of(r#"<Icon name="mark" at="1,1" span="2" rows="2" icon="figure" />"#, host);
    let mut shot = Shot::of(&mut screen, 400, 200, 1.0);
    let b = screen.element(part_named(&screen, "mark")).unwrap().node().bounds;
    let place = Rect::new(b.x + (b.width - icon::ICON_SIZE) / 2.0, b.y + (b.height - icon::ICON_SIZE) / 2.0, icon::ICON_SIZE, icon::ICON_SIZE);
    assert!(near(dark(), shot.pixel((place.x + place.width / 2.0) as i32, (place.y + place.height / 2.0) as i32), 8));
}
