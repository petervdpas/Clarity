//! A picture field: an image part bound to a record's picture. A new picture
//! shows from its bytes, a stored one is fetched on its own and again for
//! another record, one linked from a file of the field's store shows from
//! there, an empty one shows its text, and a file that is no picture is
//! refused (conformance/pictures.json). Its load and remove buttons show
//! while the pointer or the focus is on it.

#[path = "inputs_test_picker/mod.rs"]
mod picker;

use std::rc::Rc;

use clarity_runtime::picture::Picture;
use clarity_ui::locale;
use clarity_ui::parts::inputs::picture_field::{empty_text, PictureFieldPart};
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::{MissingPart, Part, PartHost};
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::{Element, Screen};
use picker::TestPicker;
use serde_json::{json, Value};

/// A PNG of one pixel.
const PIXEL: [u8; 70] = [
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49,
    0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

const MARKUP: &str = r#"<Image at="1,1" span="6" rows="4" name="photo" bind="person.photo" />"#;

fn pictures() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/pictures.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap()).collect()
}

fn picture(p: Option<Picture>) -> Value {
    p.map(|p| serde_json::to_value(p).unwrap()).unwrap_or(Value::Null)
}

fn make(p: Option<Picture>, markup: &str) -> (Screen, Rc<InputsHost>, std::sync::Arc<TestPicker>) {
    let picker = TestPicker::installed();
    let host = InputsHost::new();
    host.set("person", json!("first"));
    host.set("person.photo", picture(p));
    let mut screen = window_of(markup, host.clone());
    screen.resize(800.0, 300.0, 1.0);
    Shot::of(&mut screen, 800, 300, 1.0);
    (screen, host, picker)
}

fn part(screen: &Screen) -> &PictureFieldPart {
    screen.element(part_named(screen, "photo")).and_then(|e| e.as_any().downcast_ref::<PictureFieldPart>()).expect("a picture field")
}

fn with_part<R>(screen: &mut Screen, act: impl FnOnce(&mut PictureFieldPart) -> R) -> R {
    let id = part_named(screen, "photo");
    screen.act(id, |e, _| act(e.as_any_mut().downcast_mut::<PictureFieldPart>().unwrap())).unwrap()
}

#[test]
fn an_image_with_a_bind_is_a_picture_field_and_one_without_is_an_image() {
    let host = InputsHost::new();
    let screen = window_of(&format!(r#"{MARKUP}<Image at="7,1" span="6" name="logo" source="logo.png" />"#), host);
    assert!(part(&screen).base().spec.kind == "image");
    let logo = screen.element(part_named(&screen, "logo")).unwrap();
    assert!(logo.as_any().downcast_ref::<PictureFieldPart>().is_none());
    assert!(logo.as_any().downcast_ref::<MissingPart>().is_none(), "the containers group draws a plain image");
}

#[test]
fn a_new_picture_shows_from_its_bytes() {
    let (screen, _, _) = make(Some(Picture::New(PIXEL.to_vec())), MARKUP);
    assert!(part(&screen).shown().is_some());
    assert!(!part(&screen).is_empty());
}

#[test]
fn a_stored_picture_is_fetched_once_and_again_for_another_record() {
    let (mut screen, host, _) = make(None, MARKUP);
    host.files.borrow_mut().insert("person.photo".into(), PIXEL.to_vec());
    host.set("person.photo", picture(Some(Picture::Stored)));
    Shot::of(&mut screen, 800, 300, 1.0);
    assert!(part(&screen).shown().is_some());
    // Nothing else changed: it is not fetched again; another record is.
    host.files.borrow_mut().clear();
    host.change();
    Shot::of(&mut screen, 800, 300, 1.0);
    assert!(part(&screen).shown().is_some());
    host.set("person", json!("second"));
    Shot::of(&mut screen, 800, 300, 1.0);
    assert!(part(&screen).shown().is_none());
}

#[test]
fn a_picture_linked_from_a_file_shows_from_its_store() {
    let (mut screen, host, _) = make(None, MARKUP);
    host.files.borrow_mut().insert("2024/cover.png".into(), PIXEL.to_vec());
    host.set("person.photo", picture(Some(Picture::InStore("2024/cover.png".into()))));
    Shot::of(&mut screen, 800, 300, 1.0);
    assert!(part(&screen).shown().is_some());
}

#[test]
fn no_picture_shows_its_text() {
    let (mut screen, host, _) = make(Some(Picture::New(PIXEL.to_vec())), MARKUP);
    host.set("person.photo", Value::Null);
    Shot::of(&mut screen, 800, 300, 1.0);
    assert!(part(&screen).shown().is_none());
    assert!(part(&screen).is_empty());
    assert_eq!(locale::translate("noPicture", "No picture"), empty_text());
}

#[test]
fn a_file_is_loaded_only_when_it_is_a_picture() {
    let cases = pictures();
    let (mut screen, host, _) = make(None, MARKUP);
    for h in cases["notPictures"].as_array().unwrap() {
        assert!(!with_part(&mut screen, |p| p.load(hex(h.as_str().unwrap()))));
        assert!(part(&screen).problem());
        assert_eq!(Some(Value::Null), host.read("person.photo"));
    }
    for kind in cases["kinds"].as_array().unwrap() {
        let bytes = hex(kind["bytes"].as_str().unwrap());
        assert!(with_part(&mut screen, |p| p.load(bytes.clone())));
        assert!(!part(&screen).problem());
        assert_eq!(Some(picture(Some(Picture::New(bytes)))), host.read("person.photo"));
    }
    with_part(&mut screen, |p| p.remove());
    assert_eq!(Some(Value::Null), host.read("person.photo"));
}

#[test]
fn its_load_button_offers_pictures_and_takes_the_one_chosen() {
    let (mut screen, host, picker) = make(None, MARKUP);
    picker.next_file("me.png", PIXEL.to_vec());
    let id = part(&screen).base().node.children()[0].node().id();
    screen.act(id, |e, cx| e.on_click(cx));
    screen.finish_later();
    assert_eq!(Some(picture(Some(Picture::New(PIXEL.to_vec())))), host.read("person.photo"));
    let filter = picker.filter().unwrap();
    assert!(filter.patterns.contains(&"*.png".to_string()));
    assert!(filter.patterns.contains(&"*.webp".to_string()));
    assert_eq!(vec![format!("write photo person.photo {}", picture(Some(Picture::New(PIXEL.to_vec()))))], host.logged());
    let remove = part(&screen).base().node.children()[1].node().id();
    screen.act(remove, |e, cx| e.on_click(cx));
    assert_eq!(Some(Value::Null), host.read("person.photo"));
}

#[test]
fn its_buttons_show_while_the_pointer_or_the_focus_is_on_it() {
    let (mut screen, _, _) = make(Some(Picture::New(PIXEL.to_vec())), MARKUP);
    assert!(!part(&screen).revealed());
    let b = part(&screen).base().node.bounds;
    screen.pointer_moved(b.x + 4.0, b.y + 4.0);
    assert!(part(&screen).revealed());
    let load = part(&screen).load_button().node().bounds;
    screen.pointer_moved(load.x + 2.0, load.y + 2.0);
    assert!(part(&screen).revealed());
    screen.pointer_left();
    assert!(!part(&screen).revealed());
    let remove = part(&screen).base().node.children()[1].node().id();
    screen.focus(Some(remove));
    assert!(part(&screen).revealed());
    assert_eq!(locale::translate("loadPicture", "Load a picture"), part(&screen).load_button().text());
    assert_eq!(locale::translate("removePicture", "Remove the picture"), part(&screen).remove_button().text());
}

#[test]
fn remove_shows_only_with_a_picture_and_a_read_only_one_has_no_buttons() {
    let (screen, _, _) = make(None, MARKUP);
    assert!(part(&screen).load_button().is_shown());
    assert!(!part(&screen).remove_button().is_shown());
    let (mut screen, host, _) = make(Some(Picture::New(PIXEL.to_vec())), r#"<Image at="1,1" span="6" rows="4" name="photo" bind="person.photo" readonly="true" />"#);
    assert!(!part(&screen).load_button().is_shown());
    assert!(!part(&screen).remove_button().is_shown());
    assert!(!with_part(&mut screen, |p| p.load(PIXEL.to_vec())));
    assert!(host.logged().is_empty());
}

#[test]
fn it_draws_its_picture_and_its_empty_text() {
    let (mut screen, host, _) = make(Some(Picture::New(PIXEL.to_vec())), MARKUP);
    let b = part(&screen).base().node.bounds;
    screen.pointer_moved(b.x + 4.0, b.y + 4.0);
    let mut shot = Shot::of(&mut screen, 800, 300, 1.0);
    keep(&mut shot, "inputs-picture-field.png");
    host.set("person.photo", Value::Null);
    let mut shot = Shot::of(&mut screen, 800, 300, 1.0);
    keep(&mut shot, "inputs-picture-field-empty.png");
    assert_eq!(800, shot.width());
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}
