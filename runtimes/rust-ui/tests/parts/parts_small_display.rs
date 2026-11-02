//! The small parts that show, as the C# SmallDisplayTests check them: a
//! progress bar's fill (conformance/small-parts.json `progress`), a busy
//! indicator with its text, an icon of an SVG path, and an image of the
//! resources by its name, fitted in proportion.

use std::rc::Rc;

use clarity_ui::markup::PartSpec;
use clarity_ui::parts::containers::icon::{self, IconPart};
use clarity_ui::parts::containers::image::{self, ImagePart};
use clarity_ui::parts::containers::small::{BusyPart, ProgressPart};
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::{Rect, Screen};
use serde_json::{json, Value};
use skia_safe::Color;

fn host() -> Rc<TestHost> {
    TestHost::new()
}

fn conformance(file: &str) -> Value {
    let path = format!("{}/../../conformance/{file}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).expect("the conformance file reads")).expect("the conformance file is JSON")
}

fn window(markup: &str, host: Rc<TestHost>, width: f64, height: f64) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(width, height, 1.0);
    screen
}

fn part<'a, T: 'static>(screen: &'a Screen, name: &str) -> &'a T {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<T>()).expect("a part of that kind")
}

fn bounds(screen: &Screen, name: &str) -> Rect {
    screen.element(part_named(screen, name)).unwrap().node().bounds
}

/// Whether any pixel of a place is near `color`.
fn inked(shot: &mut Shot, place: Rect, color: Color) -> bool {
    (place.y as i32..place.bottom() as i32).any(|y| (place.x as i32..place.right() as i32).any(|x| near(shot.pixel(x, y), color, 30)))
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn a_progress_bar_is_as_full_as_on_every_renderer() {
    for case in conformance("small-parts.json")["progress"].as_array().unwrap() {
        let value = Some(&case["value"]).filter(|v| !v.is_null());
        let max = case["max"].as_str();
        let full = ProgressPart::full(value, max);
        assert!((full - case["full"].as_f64().unwrap()).abs() < 1e-6, "{case}: {full}");
    }
    assert_eq!(0.5, ProgressPart::full(Some(&json!("125")), Some("250")));
    assert_eq!(0.0, ProgressPart::full(Some(&json!(" ")), None));
}

#[test]
fn a_progress_bar_fills_in_the_accent_as_far_as_its_value() {
    let host = host();
    host.set("done", json!(125));
    let mut screen = window(r#"<Progress at="1,1" span="12" name="done" bind="done" max="250" />"#, host.clone(), 808.0, 400.0);
    assert_eq!(0.5, part::<ProgressPart>(&screen, "done").shown());
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    let b = bounds(&screen, "done");
    let middle = (b.y + b.height / 2.0) as i32;
    let look = screen.look().clone();
    assert_eq!(look.color("accent"), shot.pixel((b.x + b.width / 4.0) as i32, middle));
    assert_eq!(look.color("raised"), shot.pixel((b.x + b.width * 3.0 / 4.0) as i32, middle));
    keep(&mut shot, "progress.png");
    // What code sets shows on the next draw.
    host.set("done", json!(250));
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    assert_eq!(look.color("accent"), shot.pixel((b.x + b.width * 3.0 / 4.0) as i32, middle));
}

#[test]
fn a_busy_indicator_says_what_it_binds_else_its_caption_and_turns() {
    let host = host();
    host.texts.borrow_mut().insert("loading".into(), "Loading orders".into());
    let mut screen = window(
        r#"<Busy at="1,1" span="4" name="loading" caption="@loading" />
           <Busy at="1,2" span="4" name="status" bind="status" />"#,
        host.clone(),
        808.0,
        400.0,
    );
    assert_eq!("Loading orders", part::<BusyPart>(&screen, "loading").text());
    host.set("status", json!("Saving"));
    assert_eq!("Saving", part::<BusyPart>(&screen, "status").text());
    // It turns while it shows: the screen draws every frame.
    assert_eq!(2, screen.animating());
    part::<BusyPart>(&screen, "loading").clock.set(Some(0.0));
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    let b = bounds(&screen, "loading");
    let top = b.y + (b.height - 16.0) / 2.0;
    // At the start of a turn its arc runs from the top to the right, in the accent.
    assert!(inked(&mut shot, Rect::new(b.x + 9.0, top, 6.0, 4.0), screen.look().color("accent")));
    assert!(!inked(&mut shot, Rect::new(b.x + 1.0, top + 12.0, 6.0, 4.0), screen.look().color("accent")));
    keep(&mut shot, "busy.png");
}

#[test]
fn an_icon_is_found_by_its_name_as_the_generated_icons_class_names_it() {
    assert_eq!("TriangleAlert", icon::member("triangle-alert"));
    assert_eq!("ArrowUp10", icon::member("arrow-up-1-0"));
    assert_eq!("_1", icon::member("1"));
}

#[test]
fn an_icon_draws_its_path_in_the_texts_color() {
    let host = host();
    host.icons.borrow_mut().insert("minus".into(), "M 2 12 L 22 12".into());
    host.icons.borrow_mut().insert("check".into(), "M20 6 9 17l-5-5".into());
    let mut screen = window(
        r#"<Icon at="1,1" span="1" name="minus" icon="minus" />
           <Icon at="2,1" span="1" name="check" icon="check" />
           <Icon at="3,1" span="1" name="gone" icon="not-there" />"#,
        host,
        808.0,
        400.0,
    );
    assert_eq!("minus", part::<IconPart>(&screen, "minus").icon());
    assert_eq!(None, part::<IconPart>(&screen, "gone").path_of());
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    let b = bounds(&screen, "minus");
    let look = screen.look().clone();
    // A line across the middle of a 20 high icon in the middle of its place.
    assert!(shot.pixel((b.x + b.width / 2.0) as i32, (b.y + b.height / 2.0) as i32).r() < 100);
    assert_eq!(look.color("background"), shot.pixel((b.x + b.width / 2.0) as i32, (b.y + 4.0) as i32));
    // One the application does not carry shows a ring in the border color, not the text's.
    let gone = bounds(&screen, "gone");
    assert!(inked(&mut shot, gone, look.color("border")));
    assert!(!inked(&mut shot, gone, look.color("text")));
    keep(&mut shot, "icon.png");
}

fn red_png(width: i32, height: i32) -> Vec<u8> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((width, height)).unwrap();
    surface.canvas().clear(Color::RED);
    surface.image_snapshot().encode(None, skia_safe::EncodedImageFormat::PNG, None).unwrap().as_bytes().to_vec()
}

#[test]
fn an_image_is_fitted_in_proportion_and_centered() {
    assert_eq!(Rect::new(76.0, 0.0, 248.0, 124.0), image::fit(Rect::new(0.0, 0.0, 400.0, 124.0), 200.0, 100.0));
    assert_eq!(Rect::new(10.0, 50.0, 100.0, 50.0), image::fit(Rect::new(10.0, 0.0, 100.0, 150.0), 200.0, 100.0));
    assert_eq!(0.0, image::fit(Rect::new(0.0, 0.0, 100.0, 100.0), 0.0, 10.0).width);
}

#[test]
fn an_image_shows_its_picture_fitted_in_proportion() {
    let host = host();
    host.resources.borrow_mut().insert("red.png".into(), red_png(200, 100));
    host.resources.borrow_mut().insert("broken.png".into(), b"not a picture".to_vec());
    host.texts.borrow_mut().insert("logoFile".into(), "red.png".into());
    let mut screen = window(
        r#"<Image at="1,1" span="6" rows="3" name="red" source="red.png" />
           <Image at="7,1" span="6" rows="3" name="gone" source="not-there.png" />
           <Image at="1,4" span="6" rows="3" name="keyed" caption="@logoFile" />
           <Image at="7,4" span="6" rows="3" name="broken" source="broken.png" />"#,
        host,
        808.0,
        400.0,
    );
    assert_eq!(Some("red.png".to_string()), part::<ImagePart>(&screen, "keyed").named());
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    let b = bounds(&screen, "red");
    assert!(part::<ImagePart>(&screen, "red").shown().is_some());
    assert!(part::<ImagePart>(&screen, "keyed").shown().is_some());
    assert!(part::<ImagePart>(&screen, "gone").shown().is_none());
    assert!(part::<ImagePart>(&screen, "broken").shown().is_none());
    assert!(clarity_ui::Part::grows(part::<ImagePart>(&screen, "red")));
    assert!(clarity_ui::Part::grows(part::<ImagePart>(&screen, "gone")));
    assert_eq!(Color::RED, shot.pixel((b.x + b.width / 2.0) as i32, (b.y + b.height / 2.0) as i32));
    let fitted = image::fit(b, 200.0, 100.0);
    if fitted.x - b.x > 4.0 {
        assert_eq!(screen.look().color("background"), shot.pixel((b.x + 2.0) as i32, (b.y + b.height / 2.0) as i32));
    }
    // One that is not there shows its name in a dashed frame.
    let gone = bounds(&screen, "gone");
    assert!(inked(&mut shot, gone, screen.look().color("muted")));
    keep(&mut shot, "image.png");
}

#[test]
fn an_svg_picture_is_drawn_and_its_frame_says_only_its_name() {
    // Drawing is tested in tests/core_svg.rs; here the part shows it and says no more than its name.
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="80"><rect x="10" y="10" width="20" height="60" fill="#202020"/></svg>"##;
    assert!(image::picture("figure.svg", Some(svg.as_bytes())).is_some());
    assert!(image::picture("figure.png", None).is_none());
    let host = host();
    host.resources.borrow_mut().insert("figure.svg".into(), svg.as_bytes().to_vec());
    let screen = window(r#"<Image at="1,1" span="6" rows="3" name="figure" source="figure.svg" />"#, host, 808.0, 400.0);
    assert!(part::<ImagePart>(&screen, "figure").shown().is_some());
    assert_eq!("figure.svg", part::<ImagePart>(&screen, "figure").says());
}

#[test]
fn a_part_made_from_its_spec_alone_draws_without_a_window() {
    // Each kind is made by its group's maker from a spec, as a host makes it.
    for kind in ["help", "badge", "progress", "busy", "display", "icon", "image"] {
        let made = clarity_ui::parts::make(PartSpec::new(kind, serde_json::Map::new()), host());
        assert!(made.as_any().downcast_ref::<clarity_ui::parts::MissingPart>().is_none(), "{kind} is drawn");
    }
}
