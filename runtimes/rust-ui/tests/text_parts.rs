//! The parts that show text: a label (translated, set by code, marking the
//! input of a required field), a heading in the heading size, and a long
//! line cut off with an ellipsis.

use clarity_runtime::ui::Text;
use clarity_ui::parts::heading::HeadingPart;
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

#[test]
fn a_heading_draws_its_caption_in_the_heading_size() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Heading at="1,1" span="6" name="title" caption="Orders" />"#, host);
    let heading = screen.element(part_named(&screen, "title")).unwrap().as_any().downcast_ref::<HeadingPart>().unwrap().text();
    assert_eq!("Orders", heading);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let text = screen.look().color("text");
    assert!(inked(&mut shot, clarity_ui::Rect::new(0.0, 0.0, 120.0, 36.0), text));
    shot.save("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/heading.png").unwrap();
}

#[test]
fn a_long_line_is_cut_off_with_an_ellipsis() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Label at="1,1" span="1" name="long" caption="A caption far too long for one narrow column" />"#, host);
    screen.resize(400.0, 200.0, 1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((400, 200)).unwrap();
    screen.draw(surface.canvas());
    let fonts = clarity_ui::fonts::Fonts::new(screen.look().clone());
    let painter = clarity_ui::Painter::new(surface.canvas(), screen.look(), &fonts);
    let cut = painter.fit("A caption far too long for one narrow column", 60.0, 14.0, "text", false);
    assert!(cut.ends_with('\u{2026}'));
    assert!(fonts.measure(&cut, "text", 14.0, false) <= 60.0);
    assert_eq!("short", painter.fit("short", 60.0, 14.0, "text", false));
}
