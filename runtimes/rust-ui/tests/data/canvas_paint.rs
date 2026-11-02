//! A paint canvas bound to a record's picture paints on the picture the
//! record has: a stored one is fetched on its own (as a picture field fetches
//! it), one of the field's file store by its path, and another record's
//! again. The fetch comes before the first stroke, so a stroke never sends a
//! blank picture over the one stored. (Fetching is synchronous here, so the
//! C# case of a stroke while the picture is on its way is a stroke before the
//! canvas was ever drawn.)

use std::cell::RefCell;
use std::rc::Rc;

use clarity_runtime::picture::Picture;
use clarity_runtime::ui::PartState;
use clarity_ui::input::PointerButton;
use clarity_ui::markup::HandlerCall;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{Shot, TestHost};
use clarity_ui::{Element, Screen, ThemeLook};
use serde_json::{json, Map, Value};
use skia_safe::Color;

const MARKUP: &str = r#"<Canvas name="pad" units="{&quot;width&quot;:100,&quot;height&quot;:100}" mode="paint" bind="sheet.picture" />"#;

/// A window whose record holds a picture, fetched by `fetcher`; each fetch and each action logged.
struct PaintHost {
    inner: TestHost,
    fetcher: Box<dyn Fn(&PaintHost, &str, Option<&str>) -> Option<Vec<u8>>>,
    asked: RefCell<Vec<(String, Option<String>, Value)>>,
    actions: RefCell<Vec<Value>>,
}

impl PartHost for PaintHost {
    fn read(&self, path: &str) -> Option<Value> {
        self.inner.read(path)
    }
    fn write(&self, part: &str, path: &str, value: Value) {
        self.inner.write(part, path, value)
    }
    fn edited(&self, part: &str) {
        self.inner.edited(part)
    }
    fn state(&self, name: Option<&str>) -> Option<PartState> {
        self.inner.state(name)
    }
    fn set_state(&self, name: &str, state: PartState) {
        clarity_ui::PartHost::set_state(&self.inner, name, state)
    }
    fn run(&self, part: Option<&str>, call: &HandlerCall) -> bool {
        self.inner.run(part, call)
    }
    fn text(&self, written: Option<&str>) -> String {
        self.inner.text(written)
    }
    fn changes(&self) -> u64 {
        self.inner.changes()
    }
    fn fetch(&self, bind: &str, store_path: Option<&str>) -> Option<Vec<u8>> {
        let record = self.inner.read("sheet").unwrap_or(Value::Null);
        self.asked.borrow_mut().push((bind.to_string(), store_path.map(str::to_string), record));
        (self.fetcher)(self, bind, store_path)
    }
    fn acted(&self, _part: &str, action: &Map<String, Value>) -> Option<bool> {
        self.actions.borrow_mut().push(Value::Object(action.clone()));
        Some(true)
    }
}

/// A PNG 100 by 100, clear but for one red pixel at 80, 20.
fn marked() -> Vec<u8> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((100, 100)).unwrap();
    surface.canvas().clear(Color::TRANSPARENT);
    let mut paint = skia_safe::Paint::default();
    paint.set_color(Color::RED);
    surface.canvas().draw_rect(skia_safe::Rect::from_xywh(80.0, 20.0, 1.0, 1.0), &paint);
    surface.image_snapshot().encode(None, skia_safe::EncodedImageFormat::PNG, 100).unwrap().as_bytes().to_vec()
}

fn canvas(picture: Option<Picture>, drawn: bool, fetcher: impl Fn(&PaintHost, &str, Option<&str>) -> Option<Vec<u8>> + 'static) -> (Rc<PaintHost>, Screen) {
    let host = Rc::new(PaintHost { inner: TestHost::default(), fetcher: Box::new(fetcher), asked: RefCell::default(), actions: RefCell::default() });
    host.inner.set("sheet", json!("first"));
    host.inner.set("sheet.picture", picture.map(|p| serde_json::to_value(p).unwrap()).unwrap_or(Value::Null));
    let spec = clarity_ui::markup::read(&format!("<Window name=\"Test\">{MARKUP}</Window>")).expect("the markup reads");
    let part: Box<dyn Element> = clarity_ui::parts::make(spec.parts[0].clone(), host.clone());
    let mut screen = Screen::new(part, ThemeLook::clarity(false));
    screen.resize(800.0, 400.0, 1.0);
    if drawn {
        // Drawn once, as a window shows it.
        let _ = Shot::of(&mut screen, 800, 400, 1.0);
    }
    (host, screen)
}

/// A stroke from unit 10, 10 to 20, 20 (the canvas shows 4 pixels a unit, from 200 across).
fn stroke(screen: &mut Screen) {
    screen.pointer_pressed(240.0, 40.0, PointerButton::Left, 1);
    screen.pointer_moved(280.0, 80.0);
    screen.pointer_released(280.0, 80.0, PointerButton::Left);
}

/// The one picture sent, drawn on a clear surface of its size.
fn sent(host: &PaintHost) -> skia_safe::Surface {
    let actions = host.actions.borrow();
    assert_eq!(1, actions.len(), "{actions:?}");
    let Ok(Picture::New(bytes)) = serde_json::from_value::<Picture>(actions[0]["stroke"].clone()) else { panic!("a stroke is a picture") };
    let image = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&bytes)).expect("the stroke is a PNG");
    let mut surface = skia_safe::surfaces::raster_n32_premul((image.width(), image.height())).unwrap();
    surface.canvas().clear(Color::TRANSPARENT);
    surface.canvas().draw_image(&image, (0, 0), None);
    surface
}

#[test]
fn a_stored_picture_is_fetched_and_a_stroke_paints_on_it() {
    let (host, mut screen) = canvas(Some(Picture::Stored), true, |_, _, _| Some(marked()));
    assert_eq!(vec![("sheet.picture".to_string(), None, json!("first"))], *host.asked.borrow());
    stroke(&mut screen);
    let mut after = sent(&host);
    // What the record had stays, with the stroke over it.
    assert_eq!(Color::RED, px(&mut after, 80, 20));
    assert_eq!(ThemeLook::clarity(false).color("text"), px(&mut after, 15, 15));
    // Nothing else changed: it is not fetched again.
    host.inner.change();
    let _ = Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!(1, host.asked.borrow().len());
}

#[test]
fn a_picture_of_the_field_s_file_store_is_fetched_by_its_path() {
    let (host, mut screen) = canvas(Some(Picture::InStore("2024/sketch.png".into())), true, |_, _, _| Some(marked()));
    assert_eq!(Some("2024/sketch.png".to_string()), host.asked.borrow()[0].1);
    stroke(&mut screen);
    assert_eq!(Color::RED, px(&mut sent(&host), 80, 20));
}

#[test]
fn another_record_paints_on_its_own_picture() {
    let (host, mut screen) = canvas(Some(Picture::Stored), true, |_, _, _| Some(marked()));
    host.inner.set("sheet", json!("second"));
    let _ = Shot::of(&mut screen, 800, 400, 1.0);
    let records: Vec<Value> = host.asked.borrow().iter().map(|a| a.2.clone()).collect();
    assert_eq!(vec![json!("first"), json!("second")], records);
}

#[test]
fn a_stroke_before_the_canvas_was_drawn_still_paints_on_the_stored_picture() {
    let (host, mut screen) = canvas(Some(Picture::Stored), false, |_, _, _| Some(marked()));
    stroke(&mut screen);
    assert_eq!(Color::RED, px(&mut sent(&host), 80, 20));
}

#[test]
fn a_picture_that_does_not_load_leaves_the_canvas_blank_to_paint_on() {
    let (host, mut screen) = canvas(Some(Picture::New(vec![1, 2, 3])), true, |_, _, _| None);
    stroke(&mut screen);
    let mut after = sent(&host);
    assert_eq!((100, 100), (after.width(), after.height()));
    assert_eq!(0, px(&mut after, 80, 20).a());
    // A stored picture whose fetch finds nothing is blank as well.
    let (host, mut screen) = canvas(Some(Picture::Stored), true, |_, _, _| None);
    stroke(&mut screen);
    assert_eq!(0, px(&mut sent(&host), 80, 20).a());
}

/// A pixel of a picture, read through the checked reader every test uses.
fn px(picture: &mut skia_safe::Surface, x: i32, y: i32) -> Color {
    clarity_ui::testing::pixel_at(&picture.peek_pixels().expect("raster pixels"), x, y)
}
