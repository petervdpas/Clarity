//! What parts take from the facts of the fields they bind, and the person's
//! edits of an unbound part (C# FieldFactsTests): a table's headers in the
//! fields' labels, a slider's range, a tree's parent field, a label's
//! required mark; a button that opens a window; an unbound input's own text
//! and tick told to the host as an edit.

use clarity_ui::parts::check_box::CheckPart;
use clarity_ui::parts::data::table::TablePart;
use clarity_ui::parts::inputs::slider::SliderPart;
use clarity_ui::parts::label::LabelPart;
use clarity_ui::parts::FieldFacts;
use clarity_ui::testing::{part_named, window_of, TestHost};
use clarity_ui::{PartHost, Screen};
use serde_json::json;

/// The window laid out and drawn once, as a host shows it.
fn laid(mut screen: Screen) -> Screen {
    screen.resize(800.0, 500.0, 1.0);
    clarity_ui::testing::Shot::of(&mut screen, 800, 500, 1.0);
    screen
}

#[test]
fn a_tables_headers_are_its_fields_labels() {
    let host = TestHost::new();
    host.texts.borrow_mut().insert("taskTitle".into(), "Title".into());
    host.set("tasks", json!([{ "title": "Write", "due": 3 }]));
    host.fields.borrow_mut().insert("tasks.title".into(), FieldFacts { label: "@taskTitle".into(), ..FieldFacts::default() });
    let screen = laid(window_of(r#"<Table name="tasks" at="1,1" span="12" rows="4" bind="tasks" columns="title, due" />"#, host));
    let table = screen.element(part_named(&screen, "tasks")).unwrap().as_any().downcast_ref::<TablePart>().unwrap();
    let labels: Vec<&str> = table.columns().iter().map(|c| c.label.as_str()).collect();
    assert_eq!(vec!["Title", "due"], labels);
}

#[test]
fn a_slider_takes_its_fields_range_and_a_label_its_required_mark() {
    let host = TestHost::new();
    host.set("order.rating", json!(3));
    host.binds.borrow_mut().insert("rating".into(), "order.rating".into());
    host.fields.borrow_mut().insert("order.rating".into(), FieldFacts { label: "Rating".into(), required: true, min: Some("1".into()), max: Some("5".into()), ..FieldFacts::default() });
    let screen = laid(window_of(
        r#"<Label name="ratingLabel" at="1,1" span="2" caption="Rating" for="rating" />
           <Slider name="rating" at="3,1" span="4" bind="order.rating" />"#,
        host,
    ));
    let label = screen.element(part_named(&screen, "ratingLabel")).unwrap().as_any().downcast_ref::<LabelPart>().unwrap();
    assert!(label.required());
    let range = screen.element(part_named(&screen, "rating")).unwrap().as_any().downcast_ref::<SliderPart>().unwrap().range();
    assert_eq!((Some("1".to_string()), Some("5".to_string())), (range.min, range.max));
}

#[test]
fn a_trees_parent_field_is_its_entitys() {
    let host = TestHost::new();
    host.set("folders", json!([{ "id": 1, "name": "a", "up": null }, { "id": 2, "name": "b", "up": 1 }]));
    host.fields.borrow_mut().insert("folders.id".into(), FieldFacts { label: "Id".into(), parent: Some("up".into()), ..FieldFacts::default() });
    let mut screen = laid(window_of(r#"<Tree name="folders" at="1,1" span="12" rows="4" bind="folders" show="name" />"#, host));
    let id = part_named(&screen, "folders");
    let table = screen.element(id).unwrap().as_any().downcast_ref::<TablePart>().unwrap();
    assert_eq!("up", table.parent_field());
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<TablePart>().unwrap().toggle(0));
    let table = screen.element(id).unwrap().as_any().downcast_ref::<TablePart>().unwrap();
    assert_eq!(1, table.tree_place(1).expect("the child shows under its open parent").1);
}

#[test]
fn a_button_and_a_menu_item_open_the_window_they_name() {
    let host = TestHost::new();
    let mut screen = laid(window_of(r#"<Button name="users" at="1,1" span="3" caption="Users" opens="Users" />"#, host.clone()));
    let id = part_named(&screen, "users");
    let b = screen.element(id).unwrap().node().bounds;
    screen.click(b.x + b.width / 2.0, b.y + b.height / 2.0);
    assert_eq!(vec!["run users click Users"], host.logged());
}

#[test]
fn an_unbound_inputs_own_text_and_tick_are_edits() {
    let host = TestHost::new();
    let mut screen = laid(window_of(
        r#"<Textbox name="note" at="1,1" span="4" />
           <Checkbox name="agree" at="1,2" span="4" caption="Agree" />
           <Toggle name="bold" at="1,3" span="2" caption="B" />"#,
        host.clone(),
    ));
    let note = part_named(&screen, "note");
    screen.focus(Some(note));
    screen.text_typed("hi");
    // Leaving the box ends its typing, which is then one edit.
    screen.key_pressed(clarity_ui::KeyInput::plain(clarity_ui::Key::Tab));
    let agree = part_named(&screen, "agree");
    screen.act(agree, |e, cx| e.as_any_mut().downcast_mut::<CheckPart>().unwrap().tick(cx));
    let bold = part_named(&screen, "bold");
    let b = screen.element(bold).unwrap().node().bounds;
    screen.click(b.x + b.width / 2.0, b.y + b.height / 2.0);
    assert_eq!(vec!["note", "agree", "bold"], host.edits.borrow().clone());
    assert_eq!(Some("hi".to_string()), host.state(Some("note")).and_then(|s| s.text));
}


