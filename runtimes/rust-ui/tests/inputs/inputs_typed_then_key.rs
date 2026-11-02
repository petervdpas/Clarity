//! A key that runs a command (Save's Ctrl+S) after the person typed into an
//! input: the input takes what is typed first, so the command sees the new
//! value, as a click on the button does by moving the focus. Every part with
//! a text being typed takes part: a text box, a number, a tag's box, a table's
//! cell, a day, a color, a typed or a search dropdown, a folder.

use super::data_rows_host as data_rows_host;

use std::rc::Rc;

use clarity_ui::element::{chain, find, in_order, ElementId};
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::{FieldFacts, PartHost};
use clarity_ui::testing::part_named;
use clarity_ui::window::view::WindowView;
use clarity_ui::{Rect, Screen, ThemeLook};
use data_rows_host::DataHost;
use serde_json::json;

const SAVE: &str = r#"<Button name="save" at="1,12" span="2" key="Ctrl+S" onclick="save" /><Button name="add" at="3,12" span="2" onclick="add" />"#;

fn window(markup: &str, host: Rc<dyn PartHost>) -> Screen {
    let spec = clarity_ui::markup::read(&format!(r#"<Window name="Keys">{markup}{SAVE}</Window>"#)).expect("the markup reads");
    let mut screen = Screen::new(Box::new(WindowView::new(&spec, host)), ThemeLook::clarity(false));
    screen.resize(900.0, 600.0, 1.0);
    screen
}

fn click(screen: &mut Screen, b: Rect) {
    let (x, y) = (b.x + b.width / 2.0, b.y + b.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn ctrl_s(screen: &mut Screen) {
    screen.key_pressed(KeyInput::new(Key::S, Modifiers::CONTROL));
}

/// The element of a part where text is typed: the first in it that types and takes the focus.
fn typing_in(screen: &Screen, name: &str) -> ElementId {
    let part = screen.element(part_named(screen, name)).expect("the part");
    in_order(part).into_iter().find(|e| e.typing() && e.focusable()).map(|e| e.node().id()).unwrap_or_else(|| panic!("{name} has no input"))
}

/// Puts `text` in a part's input as the person types it, over what it had.
fn type_into(screen: &mut Screen, name: &str, text: &str) {
    let input = typing_in(screen, name);
    screen.focus(Some(input));
    screen.key_pressed(KeyInput::new(Key::A, Modifiers::CONTROL));
    screen.text_typed(text);
}

/// The write that ends with `written` comes before the run of save.
fn written_before_save(log: &[String], written: &str) {
    let write = log.iter().position(|l| l.starts_with("write ") && l.ends_with(written));
    let save = log.iter().position(|l| l.starts_with("run save click"));
    assert!(write.is_some(), "{log:?}");
    assert!(save > write, "{log:?}");
}

#[test]
fn new_then_a_typed_name_then_ctrl_s_saves_the_name() {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("person.name".into(), json!(""));
    let mut screen = window(r#"<Button name="new" at="1,1" span="2" onclick="newPerson" /><Textbox name="name" at="1,2" span="4" bind="person.name" />"#, host.clone());
    let new = screen.element(part_named(&screen, "new")).unwrap().node().bounds;
    click(&mut screen, new);
    let input = typing_in(&screen, "name");
    screen.focus(Some(input));
    screen.text_typed("Ada");
    ctrl_s(&mut screen);
    assert_eq!(Some(json!("Ada")), host.read("person.name"));
    written_before_save(&host.logged(), "\"Ada\"");
    // The input keeps the focus, and pressing again changes nothing more.
    assert_eq!(Some(input), screen.focused());
    let writes = host.logged().iter().filter(|l| l.starts_with("write ")).count();
    ctrl_s(&mut screen);
    assert_eq!(writes, host.logged().iter().filter(|l| l.starts_with("write ")).count());
}

#[test]
fn a_typed_number_is_taken_before_the_key() {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("order.hours".into(), json!(1));
    host.facts("order.hours", FieldFacts { label: "Hours".into(), format: Some("integer".into()), ..Default::default() });
    let mut screen = window(r#"<Number name="hours" at="1,1" span="3" bind="order.hours" />"#, host.clone());
    type_into(&mut screen, "hours", "8");
    ctrl_s(&mut screen);
    assert_eq!(Some(json!(8)), host.read("order.hours"));
    written_before_save(&host.logged(), "8");
}

#[test]
fn a_typed_tag_is_added_before_the_key() {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("album.genres".into(), json!(["live"]));
    host.types.borrow_mut().insert("album.genres".into(), "list<text>".into());
    let mut screen = window(r#"<Tags name="genres" at="1,1" span="8" bind="album.genres" />"#, host.clone());
    let input = typing_in(&screen, "genres");
    screen.focus(Some(input));
    screen.text_typed("demo");
    ctrl_s(&mut screen);
    assert_eq!(Some(json!(["live", "demo"])), host.read("album.genres"));
    written_before_save(&host.logged(), r#"["live","demo"]"#);
}

#[test]
fn a_typed_cell_is_taken_before_the_key() {
    let host = DataHost::new();
    host.set("relations", json!([{ "multiplicity": "1", "label": "" }]));
    let mut screen = window(r#"<Table name="grid" at="1,1" span="12" rows="6" bind="relations" columns="multiplicity, label" edit="multiplicity, label" />"#, host.clone());
    let id = part_named(&screen, "grid");
    let cell = screen.element(id).and_then(|e| e.as_any().downcast_ref::<TablePart>()).and_then(|t| t.cell_box(0, "label")).expect("the label cell");
    click(&mut screen, cell);
    screen.text_typed("owns");
    ctrl_s(&mut screen);
    assert_eq!(json!("owns"), host.get("relations[0].label"));
    let log = host.logged();
    let write = log.iter().position(|l| l.starts_with("write "));
    let save = log.iter().position(|l| l.starts_with("run save click"));
    assert!(write.is_some() && save > write, "{log:?}");
}

#[test]
fn a_typed_dropdowns_text_is_taken_before_the_key() {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("person.city".into(), json!("Rome"));
    host.offer("person.city", &[(json!("Rome"), "Rome"), (json!("Utrecht"), "Utrecht")]);
    let mut screen = window(r#"<Dropdown name="city" at="1,1" span="4" bind="person.city" typed="true" />"#, host.clone());
    type_into(&mut screen, "city", "Oslo");
    ctrl_s(&mut screen);
    assert_eq!(Some(json!("Oslo")), host.read("person.city"));
    written_before_save(&host.logged(), "\"Oslo\"");
}

#[test]
fn a_typed_color_is_taken_before_the_key() {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("tag.color".into(), json!("#123456"));
    let mut screen = window(r#"<Color name="tint" at="1,1" span="4" bind="tag.color" />"#, host.clone());
    type_into(&mut screen, "tint", "#abcdef");
    ctrl_s(&mut screen);
    assert_eq!(Some(json!("#abcdef")), host.read("tag.color"));
    written_before_save(&host.logged(), "\"#abcdef\"");
}

#[test]
fn a_typed_day_is_taken_before_the_key() {
    let host = InputsHost::new();
    host.use_formats("en-US", None);
    host.values.borrow_mut().insert("task.due".into(), json!("2026-01-02"));
    let mut screen = window(r#"<Date name="due" at="1,1" span="4" bind="task.due" />"#, host.clone());
    type_into(&mut screen, "due", "3/4/2026");
    ctrl_s(&mut screen);
    assert_ne!(Some(json!("2026-01-02")), host.read("task.due"));
    let log = host.logged();
    let write = log.iter().position(|l| l.starts_with("write "));
    let save = log.iter().position(|l| l.starts_with("run save click"));
    assert!(write.is_some() && save > write, "{log:?}");
}

#[test]
fn every_part_with_typed_text_takes_it() {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("album.genres".into(), json!([]));
    host.types.borrow_mut().insert("album.genres".into(), "list<text>".into());
    host.offer("task.owner", &[(json!(1), "Ada")]);
    host.facts("task.owner", FieldFacts { label: "Owner".into(), refers: Some("Person".into()), ..Default::default() });
    let screen = window(
        r#"<Textbox name="text" at="1,1" span="4" bind="person.name" />
           <Number name="number" at="1,2" span="4" bind="order.hours" />
           <Tags name="tags" at="1,3" span="8" bind="album.genres" />
           <Date name="date" at="1,4" span="4" bind="task.due" />
           <Color name="color" at="1,5" span="4" bind="tag.color" />
           <Dropdown name="typed" at="1,6" span="4" bind="person.city" typed="true" />
           <Dropdown name="search" at="1,7" span="4" bind="task.owner" search="true" />
           <Folder name="folder" at="1,8" span="4" bind="app.folder" />"#,
        host,
    );
    for name in ["text", "number", "tags", "date", "color", "typed", "search", "folder"] {
        let input = typing_in(&screen, name);
        let root = screen.root();
        let ids = chain(root, input).expect("the input is in the window");
        // What takes the typed text: the input, or the first part around it that does.
        let takes = ids.iter().rev().filter_map(|id| find(root, *id)).any(|e| e.typing());
        assert!(takes, "{name}");
    }
}
