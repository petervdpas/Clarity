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
    canvas_of(MARKUP, picture, drawn, fetcher)
}

fn canvas_of(markup: &str, picture: Option<Picture>, drawn: bool, fetcher: impl Fn(&PaintHost, &str, Option<&str>) -> Option<Vec<u8>> + 'static) -> (Rc<PaintHost>, Screen) {
    let host = Rc::new(PaintHost { inner: TestHost::default(), fetcher: Box::new(fetcher), asked: RefCell::default(), actions: RefCell::default() });
    host.inner.set("sheet", json!("first"));
    host.inner.set("sheet.picture", picture.map(|p| serde_json::to_value(p).unwrap()).unwrap_or(Value::Null));
    let spec = clarity_ui::markup::read(&format!("<Window name=\"Test\">{markup}</Window>")).expect("the markup reads");
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

const WITH_TOOLBOX: &str = r#"<Canvas name="pad" units="{&quot;width&quot;:100,&quot;height&quot;:100}" mode="paint" toolbox="popup" bind="sheet.picture" />"#;

fn toolbox_panel(screen: &Screen) -> &clarity_ui::parts::data::canvas::toolbox::ToolboxPanel {
    screen.overlays().iter().find_map(|o| o.content.as_any().downcast_ref::<clarity_ui::parts::data::canvas::toolbox::ToolboxPanel>()).expect("the toolbox shows")
}

fn click_at(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
    let _ = Shot::of(screen, 800, 400, 1.0);
}

/// `toolbox popup`: the paint tools are in a toolbox over the window, not on
/// the canvas; a tool chosen there is the canvas's tool.
#[test]
fn a_toolbox_holds_the_paint_tools_and_chooses_one() {
    let (host, mut screen) = canvas_of(WITH_TOOLBOX, None, true, |_, _, _| None);
    let fill = toolbox_panel(&screen).tool_place("fill").expect("the fill tool");
    click_at(&mut screen, fill.x + fill.width / 2.0, fill.y + fill.height / 2.0);
    assert_eq!(Some("fill".to_string()), host.inner.state(Some("pad")).and_then(|s| s.tool));
}

/// The fill bucket paints the area under the press in the ink, as one edit.
#[test]
fn the_fill_bucket_fills_the_area_it_is_pressed_in() {
    let (host, mut screen) = canvas_of(WITH_TOOLBOX, Some(Picture::Stored), true, |_, _, _| Some(marked()));
    clarity_ui::parts::change_state(&*host, "pad", |s| {
        s.tool = Some("fill".into());
        s.ink = Some("#0000ff".into());
    });
    click_at(&mut screen, 242.0, 42.0);
    let mut after = sent(&host);
    assert_eq!(Color::BLUE, px(&mut after, 10, 10), "the clear area is filled");
    assert_eq!(Color::RED, px(&mut after, 80, 20), "the red pixel is another colour and stays");
}

/// The colour picker takes the colour of the pixel pressed as the ink.
#[test]
fn the_picker_takes_the_colour_pressed_as_the_ink() {
    let (host, mut screen) = canvas_of(WITH_TOOLBOX, Some(Picture::Stored), true, |_, _, _| Some(marked()));
    clarity_ui::parts::change_state(&*host, "pad", |s| s.tool = Some("picker".into()));
    click_at(&mut screen, 522.0, 82.0);
    assert_eq!(Some("#ff0000".to_string()), host.inner.state(Some("pad")).and_then(|s| s.ink));
}

/// The toolbox shows the brush and ink the canvas's line gives, until the person changes them.
#[test]
fn the_toolbox_shows_the_canvas_s_own_brush_and_ink() {
    let markup = r##"<Canvas name="pad" units="{&quot;width&quot;:100,&quot;height&quot;:100}" mode="paint" toolbox="popup" brush="8" ink="#8a5a2b" bind="sheet.picture" />"##;
    let (_host, screen) = canvas_of(markup, None, true, |_, _, _| None);
    let canvas = clarity_ui::element::find_type::<clarity_ui::parts::data::canvas::CanvasPart>(screen.root(), |_| true).expect("the canvas");
    assert_eq!((8, "#8a5a2b".to_string()), canvas.toolbox().shown());
}

/// An imported picture is laid on the paper, scaled to fit, in the middle, as one edit.
#[test]
fn an_imported_picture_is_laid_in_the_middle_of_the_paper() {
    let (host, mut screen) = canvas_of(WITH_TOOLBOX, None, true, |_, _, _| None);
    let red = clarity_image::encode(&clarity_image::RgbaImage::from_pixel(10, 10, clarity_image::Rgba([255, 0, 0, 255])));
    let id = clarity_ui::element::find_type::<clarity_ui::parts::data::canvas::CanvasPart>(screen.root(), |_| true).expect("the canvas").node().id();
    screen.act(id, |e, cx| e.as_any_mut().downcast_mut::<clarity_ui::parts::data::canvas::CanvasPart>().unwrap().import(&red, cx));
    let mut after = sent(&host);
    assert_eq!(Color::RED, px(&mut after, 50, 50));
    assert_eq!(Color::TRANSPARENT, px(&mut after, 5, 5));
}

/// Holding a group's button shows its other tools; choosing one puts it in hand.
#[test]
fn holding_a_tool_group_shows_its_other_tools() {
    let (host, mut screen) = canvas_of(WITH_TOOLBOX, None, true, |_, _, _| None);
    let select = toolbox_panel(&screen).tool_place("select").expect("the select group");
    let start = std::time::Instant::now();
    screen.now = Box::new(move || start);
    screen.pointer_pressed(select.x + 10.0, select.y + 10.0, PointerButton::Left, 1);
    screen.now = Box::new(move || start + std::time::Duration::from_millis(500));
    let _ = Shot::of(&mut screen, 800, 400, 1.0);
    screen.pointer_released(select.x + 10.0, select.y + 10.0, PointerButton::Left);
    let _ = Shot::of(&mut screen, 800, 400, 1.0);
    let flyout = screen.overlays().iter().find_map(|o| o.content.as_any().downcast_ref::<clarity_ui::parts::tools::ToolFlyout>()).expect("the group's tools show");
    let tools: Vec<String> = flyout.tools().iter().map(|(t, _)| t.clone()).collect();
    assert_eq!(vec!["select", "ellipseSelect", "lasso"], tools);
    let lasso = flyout.tools()[2].1;
    click_at(&mut screen, lasso.x + 10.0, lasso.y + 10.0);
    assert_eq!(Some("lasso".to_string()), host.inner.state(Some("pad")).and_then(|s| s.tool));
}

/// A rectangle selection's pixels move with a drag inside it, as one edit.
#[test]
fn a_selection_s_pixels_move_with_a_drag() {
    let (host, mut screen) = canvas_of(WITH_TOOLBOX, Some(Picture::Stored), true, |_, _, _| Some(marked()));
    clarity_ui::parts::change_state(&*host, "pad", |s| s.tool = Some("select".into()));
    // Select units 75..85 across, 15..25 down: around the red pixel at 80, 20.
    screen.pointer_pressed(500.0, 60.0, PointerButton::Left, 1);
    screen.pointer_moved(540.0, 100.0);
    screen.pointer_released(540.0, 100.0, PointerButton::Left);
    // Drag from inside it 10 units to the left.
    screen.pointer_pressed(520.0, 80.0, PointerButton::Left, 1);
    screen.pointer_moved(480.0, 80.0);
    screen.pointer_released(480.0, 80.0, PointerButton::Left);
    let mut after = sent(&host);
    assert_eq!(Color::RED, px(&mut after, 70, 20), "the pixel moved with the selection");
    assert_eq!(Color::TRANSPARENT, px(&mut after, 80, 20), "its old place is cleared");
}

/// Mirroring without a selection mirrors the whole picture.
#[test]
fn a_flip_mirrors_the_whole_picture() {
    let (host, mut screen) = canvas_of(WITH_TOOLBOX, Some(Picture::Stored), true, |_, _, _| Some(marked()));
    let id = clarity_ui::element::find_type::<clarity_ui::parts::data::canvas::CanvasPart>(screen.root(), |_| true).expect("the canvas").node().id();
    screen.act(id, |e, cx| e.as_any_mut().downcast_mut::<clarity_ui::parts::data::canvas::CanvasPart>().unwrap().transform(clarity_ui::parts::data::canvas::raster::Transform::FlipAcross, cx));
    let mut after = sent(&host);
    assert_eq!(Color::RED, px(&mut after, 19, 20));
}

/// Ctrl+X takes the selected pixels out, as one edit; Ctrl+V lays them back where they were, selected.
#[test]
fn a_selection_is_cut_and_pasted_back() {
    use clarity_ui::input::{Key, KeyInput, Modifiers};
    let (host, mut screen) = canvas_of(WITH_TOOLBOX, Some(Picture::Stored), true, |_, _, _| Some(marked()));
    clarity_ui::parts::change_state(&*host, "pad", |s| s.tool = Some("select".into()));
    screen.pointer_pressed(500.0, 60.0, PointerButton::Left, 1);
    screen.pointer_moved(540.0, 100.0);
    screen.pointer_released(540.0, 100.0, PointerButton::Left);
    screen.key_pressed(KeyInput::new(Key::X, Modifiers::CONTROL));
    screen.key_pressed(KeyInput::new(Key::V, Modifiers::CONTROL));
    let actions = host.actions.borrow().clone();
    assert_eq!(2, actions.len(), "{actions:?}");
    let picture = |i: usize| {
        let Ok(Picture::New(bytes)) = serde_json::from_value::<Picture>(actions[i]["stroke"].clone()) else { panic!("a picture") };
        clarity_image::decode(&bytes).unwrap()
    };
    assert_eq!(0, picture(0).get_pixel(80, 20)[3], "cut: the pixel is gone");
    assert_eq!([255, 0, 0, 255], picture(1).get_pixel(80, 20).0, "pasted back where it was");
}

/// A toolbox page placed in the canvas: its coded tool runs its handler, its size part sets the brush.
#[test]
fn a_designed_toolbox_runs_its_coded_tools_and_sets_the_brush() {
    let markup = r#"<Canvas name="pad" units="{&quot;width&quot;:100,&quot;height&quot;:100}" mode="paint" toolbox="popup" toolboxColumns="4" bind="sheet.picture"><Tool name="starTool" at="1,1" icon="star" onclick="stamp" for="pad" /><Tool name="fillTool" at="2,1" does="fill" for="pad" /><ToolSize name="sizeOption" at="1,2" span="4" for="pad" /></Canvas>"#;
    let (host, mut screen) = canvas_of(markup, None, true, |_, _, _| None);
    let panel = toolbox_panel(&screen);
    assert!(panel.tool_place("fill").is_some(), "the designed tool is in the toolbox");
    assert!(panel.tool_place("lasso").is_none(), "and only what the toolbox page has");
    let star = clarity_ui::element::in_order(panel).into_iter().find(|e| clarity_ui::testing::part_name(*e) == Some("starTool")).map(|e| e.node().bounds).expect("the coded tool");
    let size = clarity_ui::element::in_order(panel).into_iter().find(|e| clarity_ui::testing::part_name(*e) == Some("sizeOption")).map(|e| e.node().bounds).expect("the size part");
    click_at(&mut screen, star.x + 10.0, star.y + 10.0);
    assert!(host.inner.logged().iter().any(|l| l.contains("stamp")), "{:?}", host.inner.logged());
    // The plus button at its right end makes the brush larger.
    click_at(&mut screen, size.right() - 6.0, size.y + size.height / 2.0);
    assert_eq!(Some(3), host.inner.state(Some("pad")).and_then(|s| s.brush));
}

/// The toolbox is as high as its parts: the last row (colours, import, export) lies inside it, not cut off.
#[test]
fn the_toolbox_is_as_high_as_its_parts() {
    let (_host, screen) = canvas_of(WITH_TOOLBOX, None, true, |_, _, _| None);
    let panel = toolbox_panel(&screen);
    let export = panel.tool_place("export").expect("the export tool");
    assert!(export.height > 0.0 && export.bottom() <= panel.node().bounds.bottom(), "{export:?} inside {:?}", panel.node().bounds);
}

/// The colour dialog: brightness made more shows on the picture, and OK keeps it as one edit.
#[test]
fn the_colour_dialog_brightens_the_picture_and_ok_keeps_it() {
    let grey = clarity_image::encode(&clarity_image::RgbaImage::from_pixel(100, 100, clarity_image::Rgba([100, 100, 100, 255])));
    let (host, mut screen) = canvas_of(WITH_TOOLBOX, Some(Picture::Stored), true, move |_, _, _| Some(grey.clone()));
    let adjust = toolbox_panel(&screen).tool_place("adjust").expect("the colours tool");
    click_at(&mut screen, adjust.x + 10.0, adjust.y + 10.0);
    let dialog = || screen.overlays().iter().find_map(|o| o.content.as_any().downcast_ref::<clarity_ui::parts::data::canvas::adjust::AdjustDialog>()).map(|d| (d.place_of("moreBrightness").unwrap(), d.place_of("ok").unwrap()));
    let (more, ok) = dialog().expect("the colour dialog opens");
    click_at(&mut screen, more.x + 5.0, more.y + 5.0);
    click_at(&mut screen, more.x + 5.0, more.y + 5.0);
    assert!(host.actions.borrow().is_empty(), "nothing is kept before OK");
    click_at(&mut screen, ok.x + 10.0, ok.y + 10.0);
    let mut after = sent(&host);
    let p = px(&mut after, 50, 50);
    assert!(p.r() > 110, "brighter: {p:?}");
    assert!(screen.overlays().iter().all(|o| o.content.as_any().downcast_ref::<clarity_ui::parts::data::canvas::adjust::AdjustDialog>().is_none()), "the dialog goes");
}

/// Layers: a new layer is painted on; hidden, what is on it leaves the picture kept; the picture is the layers composed.
#[test]
fn a_layer_is_painted_on_and_hiding_it_leaves_the_picture() {
    let (host, mut screen) = canvas_of(WITH_TOOLBOX, Some(Picture::Stored), true, |_, _, _| Some(marked()));
    let id = clarity_ui::element::find_type::<clarity_ui::parts::data::canvas::CanvasPart>(screen.root(), |_| true).expect("the canvas").node().id();
    let toolbox = clarity_ui::element::find_type::<clarity_ui::parts::data::canvas::CanvasPart>(screen.root(), |_| true).unwrap().toolbox().clone();
    use clarity_ui::parts::data::canvas::layers::LayerAsk;
    toolbox.ask_layers(LayerAsk::Add);
    let _ = Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!(2, toolbox.layer_rows().len(), "the picture and the new layer");
    // Fill the new layer (clear) blue: only it changes; the red pixel below still shows in the picture kept.
    clarity_ui::parts::change_state(&*host, "pad", |s| {
        s.tool = Some("fill".into());
        s.ink = Some("#0000ff".into());
    });
    click_at(&mut screen, 242.0, 42.0);
    let last = |host: &PaintHost| {
        let actions = host.actions.borrow();
        let Ok(Picture::New(bytes)) = serde_json::from_value::<Picture>(actions.last().unwrap()["stroke"].clone()) else { panic!("a picture") };
        clarity_image::decode(&bytes).unwrap()
    };
    assert_eq!([0, 0, 255, 255], last(&host).get_pixel(80, 20).0, "the new layer covers the picture");
    // Hide the new layer (top row, place 1 from the bottom): the picture is the bottom layer again.
    screen.act(id, |_, _| ());
    toolbox.ask_layers(LayerAsk::Toggle(1));
    let _ = Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!([255, 0, 0, 255], last(&host).get_pixel(80, 20).0, "hidden, it leaves the picture as it was");
}
