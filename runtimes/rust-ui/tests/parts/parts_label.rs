//! A label, as the C# LabelPartTests check it: its caption, as code set it
//! or as written and translated, with a mark when its input's field is
//! required; a help text shows its input's error in the error color, else
//! its own text or its input's help.

use clarity_runtime::ui::Text;
use clarity_ui::parts::containers::small::HelpPart;
use clarity_ui::parts::label::LabelPart;
use clarity_ui::parts::FieldFacts;
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::Screen;

fn label(screen: &Screen, name: &str) -> String {
    screen.element(part_named(screen, name)).unwrap().as_any().downcast_ref::<LabelPart>().unwrap().text()
}

/// Whether any pixel of a place is near `color`.
fn inked(shot: &mut Shot, place: clarity_ui::Rect, color: skia_safe::Color) -> bool {
    (place.y as i32..place.bottom() as i32).any(|y| (place.x as i32..place.right() as i32).any(|x| near(shot.pixel(x, y), color, 30)))
}

#[test]
fn a_caption_is_translated_and_code_may_set_it() {
    let host = TestHost::new();
    host.texts.borrow_mut().insert("nameCaption".into(), "Name".into());
    let screen = window_of(r#"<Label at="1,1" span="3" name="nameLabel" caption="@nameCaption" />"#, host.clone());
    assert_eq!("Name", label(&screen, "nameLabel"));
    host.set_state("nameLabel", |s| s.caption = Some(Text::plain("Full name")));
    assert_eq!("Full name", label(&screen, "nameLabel"));
}

#[test]
fn a_label_marks_the_input_of_a_required_field() {
    let host = TestHost::new();
    host.binds.borrow_mut().insert("number".into(), "order.number".into());
    host.binds.borrow_mut().insert("note".into(), "order.note".into());
    host.fields.borrow_mut().insert("order.number".into(), FieldFacts { label: "Number".into(), required: true, ..FieldFacts::default() });
    let mut screen = window_of(
        r#"<Label at="1,1" span="3" name="numberLabel" caption="Number" for="number" />
           <Label at="1,2" span="3" name="noteLabel" caption="Note" for="note" />"#,
        host,
    );
    let required = screen.element(part_named(&screen, "numberLabel")).unwrap().as_any().downcast_ref::<LabelPart>().unwrap().required();
    assert!(required);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let error = screen.look().color("error");
    let first = screen.element(part_named(&screen, "numberLabel")).unwrap().node().bounds;
    let second = screen.element(part_named(&screen, "noteLabel")).unwrap().node().bounds;
    assert!(inked(&mut shot, first, error));
    assert!(!inked(&mut shot, second, error));
}

fn help<'a>(screen: &'a Screen, name: &str) -> &'a HelpPart {
    screen.element(part_named(screen, name)).unwrap().as_any().downcast_ref::<HelpPart>().unwrap()
}

#[test]
fn help_shows_its_inputs_error_in_the_error_color_else_its_text() {
    let host = TestHost::new();
    let mut screen = window_of(
        r#"<Textbox at="1,1" span="4" name="email" />
           <Help at="1,2" span="4" name="emailHelp" caption="We never share it" for="email" />"#,
        host.clone(),
    );
    assert_eq!("We never share it", help(&screen, "emailHelp").text());
    assert!(!help(&screen, "emailHelp").shows_error());
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let place = screen.element(part_named(&screen, "emailHelp")).unwrap().node().bounds;
    assert!(inked(&mut shot, place, screen.look().color("muted")));
    host.set_state("email", |s| s.error = Some(Text::plain("Not an address")));
    assert!(help(&screen, "emailHelp").shows_error());
    assert_eq!("Not an address", help(&screen, "emailHelp").text());
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(inked(&mut shot, place, screen.look().color("error")));
}

#[test]
fn help_without_text_of_its_own_shows_its_inputs_help() {
    let host = TestHost::new();
    host.binds.borrow_mut().insert("email".into(), "customer.email".into());
    host.fields.borrow_mut().insert("customer.email".into(), FieldFacts { help: Some("An address we can write to".into()), ..FieldFacts::default() });
    let screen = window_of(
        r#"<Textbox at="1,1" span="4" name="email" bind="customer.email" />
           <Help at="1,2" span="4" name="emailHelp" for="email" />"#,
        host,
    );
    assert_eq!("An address we can write to", help(&screen, "emailHelp").text());
}
