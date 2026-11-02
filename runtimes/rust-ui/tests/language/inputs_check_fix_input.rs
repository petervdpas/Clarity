//! The inputs as the person meets them: a read-only checkbox or switch is
//! greyed as a disabled one; the pointer is a hand over a link it can
//! follow; a spin box's arrows are drawn in the theme's text color (muted
//! while disabled) and are no Tab stops, the Down arrow too; a read-only
//! dropdown takes no Up, Down or letter; a read-only radio group's choices
//! are no Tab stops; a required text cleared is empty, as a required day
//! cleared is no day; and an unbound tags part keeps its list when its
//! window is made again.

use super::inputs_program as program;

use std::rc::Rc;

use clarity_ui::element::{self, Cx};
use clarity_ui::input::{Cursor, Key, KeyInput, Modifiers};
use clarity_ui::parts::check_box::BOX_SIZE;
use clarity_ui::parts::inputs::dropdown::DropdownPart;
use clarity_ui::parts::inputs::number::NumberPart;
use clarity_ui::parts::inputs::radio::RadioPart;
use clarity_ui::parts::inputs::tags::TagsPart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::text_box::TextBoxPart;
use clarity_ui::parts::{FieldFacts, PartHost};
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::window::WindowStack;
use clarity_ui::{Element, Screen, ThemeLook};
use serde_json::{json, Map, Value};
use skia_safe::Color;

fn near(a: Color, b: Color) -> bool {
    let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
    d(a.r(), b.r()) + d(a.g(), b.g()) + d(a.b(), b.b()) < 24
}

fn press(screen: &mut Screen, key: Key) {
    screen.key_pressed(KeyInput::plain(key));
}

fn window(markup: &str, host: Rc<InputsHost>) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn bounds(screen: &Screen, name: &str) -> clarity_ui::geometry::Rect {
    screen.element(part_named(screen, name)).unwrap().node().bounds
}

#[test]
fn a_read_only_checkbox_or_switch_is_greyed_as_a_disabled_one() {
    let _alone = super::one_at_a_time();
    let host = InputsHost::new();
    for path in ["a", "b", "c", "d", "e", "f"] {
        host.set(path, json!(true));
    }
    let mut screen = window(
        r#"<Checkbox at="1,1" span="3" name="plain" bind="a" />
           <Checkbox at="1,2" span="3" name="reading" bind="b" readonly="true" />
           <Checkbox at="1,3" span="3" name="off" bind="c" />
           <Switch at="1,4" span="3" name="plainSwitch" bind="d" />
           <Switch at="1,5" span="3" name="readingSwitch" bind="e" readonly="true" />
           <Switch at="1,6" span="3" name="offSwitch" bind="f" />"#,
        host.clone(),
    );
    host.set_state("off", |s| s.enabled = Some(false));
    host.set_state("offSwitch", |s| s.enabled = Some(false));
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let accent = screen.look().color("accent");
    // In the ticked box, away from the tick; in the switch's track, left of the knob.
    let at_box = |shot: &mut Shot, name: &str| {
        let b = bounds(&screen, name);
        shot.pixel((b.x + 1.0 + 4.0) as i32, (b.y + (b.height - BOX_SIZE) / 2.0 + 3.0) as i32)
    };
    let at_track = |shot: &mut Shot, name: &str| {
        let b = bounds(&screen, name);
        shot.pixel((b.x + 6.0) as i32, (b.y + b.height / 2.0) as i32)
    };
    assert!(near(accent, at_box(&mut shot, "plain")));
    assert!(!near(accent, at_box(&mut shot, "reading")), "a read-only box is greyed");
    assert_eq!(at_box(&mut shot, "off"), at_box(&mut shot, "reading"));
    assert!(near(accent, at_track(&mut shot, "plainSwitch")));
    assert!(!near(accent, at_track(&mut shot, "readingSwitch")), "a read-only switch is greyed");
    assert_eq!(at_track(&mut shot, "offSwitch"), at_track(&mut shot, "readingSwitch"));
}

#[test]
fn the_pointer_is_a_hand_over_a_link_it_can_follow() {
    let _alone = super::one_at_a_time();
    let host = InputsHost::new();
    let mut screen = window(
        r#"<Link at="1,1" span="3" name="docs" to="https://example.org/docs" caption="Docs" />
           <Link at="1,2" span="3" name="gone" to="https://example.org/gone" caption="Gone" />
           <Label at="1,3" span="3" name="plain" caption="Plain" />"#,
        host.clone(),
    );
    host.set_state("gone", |s| s.enabled = Some(false));
    Shot::of(&mut screen, 800, 400, 1.0);
    let over = |screen: &mut Screen, name: &str| {
        let b = bounds(screen, name);
        screen.pointer_moved(b.x + 6.0, b.y + b.height / 2.0);
    };
    over(&mut screen, "docs");
    assert_eq!(Cursor::Hand, screen.cursor());
    over(&mut screen, "plain");
    assert_eq!(Cursor::Default, screen.cursor());
    over(&mut screen, "gone");
    assert_eq!(Cursor::Default, screen.cursor());
}

#[test]
fn a_spin_boxs_arrows_are_drawn_in_the_themes_text_color_and_are_no_tab_stops() {
    let _alone = super::one_at_a_time();
    for dark in [false, true] {
        let host = InputsHost::new();
        host.set("count", json!(3));
        host.set("other", json!(3));
        host.types.borrow_mut().insert("count".into(), "integer".into());
        host.types.borrow_mut().insert("other".into(), "integer".into());
        let mut screen = window(
            r#"<Spinbox at="1,1" span="3" name="count" bind="count" min="0" max="9" />
               <Spinbox at="1,2" span="3" name="other" bind="other" min="0" max="9" />"#,
            host.clone(),
        );
        host.set_state("other", |s| s.enabled = Some(false));
        screen.show(ThemeLook::clarity(dark));
        let mut shot = Shot::of(&mut screen, 1600, 800, 2.0);
        let look = screen.look().clone();
        let arrows = |name: &str| {
            let part = screen.element(part_named(&screen, name)).and_then(|e| e.as_any().downcast_ref::<NumberPart>()).unwrap();
            [part.up().unwrap().bounds, part.down().unwrap().bounds]
        };
        let shows = |shot: &mut Shot, b: clarity_ui::geometry::Rect, color: Color| {
            for x in (b.x * 2.0) as i32..(b.right() * 2.0) as i32 {
                for y in (b.y * 2.0) as i32..(b.bottom() * 2.0) as i32 {
                    if shot.pixel(x, y) == color {
                        return true;
                    }
                }
            }
            false
        };
        for b in arrows("count") {
            assert!(shows(&mut shot, b, look.color("text")), "an arrow in {} is drawn in the text color", if dark { "dark" } else { "light" });
        }
        for b in arrows("other") {
            assert!(shows(&mut shot, b, look.color("muted")));
            assert!(!shows(&mut shot, b, look.color("text")));
        }
        // Neither arrow is a Tab stop: only the box is.
        assert_eq!(vec![part_named(&screen, "count")], screen.tab_order());
    }
}

#[test]
fn a_read_only_dropdown_takes_no_up_down_or_letter() {
    let _alone = super::one_at_a_time();
    let host = InputsHost::new();
    host.set("task.owner", json!(1));
    host.offer("task.owner", &[(Value::Null, ""), (json!(1), "Ada"), (json!(2), "Grace")]);
    let mut screen = window(r#"<Dropdown at="1,1" span="4" name="owner" bind="task.owner" readonly="true" onchange="ownerChanged" />"#, host.clone());
    let id = part_named(&screen, "owner");
    screen.focus(Some(id));
    press(&mut screen, Key::Down);
    press(&mut screen, Key::Up);
    screen.text_typed("g");
    let took = screen.act(id, |e, cx: &mut Cx| {
        let took = e.on_key(KeyInput::plain(Key::Down), cx);
        e.on_text("g", cx);
        took
    });
    assert_eq!(Some(false), took);
    let dropdown = screen.element(id).and_then(|e| e.as_any().downcast_ref::<DropdownPart>()).unwrap();
    assert!(dropdown.list().is_none());
    assert_eq!(Some(json!(1)), host.read("task.owner"));
    assert!(host.logged().is_empty());
}

#[test]
fn a_read_only_radio_groups_choices_are_no_tab_stops() {
    let _alone = super::one_at_a_time();
    let host = InputsHost::new();
    host.set("task.size", json!("Small"));
    host.offer("task.size", &[(json!("Small"), "Small"), (json!("Medium"), "Medium"), (json!("Large"), "Large")]);
    let mut screen = window(
        r#"<Radio at="1,1" span="12" name="reading" bind="task.size" readonly="true" />
           <Radio at="1,2" span="12" name="picking" bind="task.size" />"#,
        host,
    );
    Shot::of(&mut screen, 800, 400, 1.0);
    let buttons = |name: &str| screen.element(part_named(&screen, name)).and_then(|e| e.as_any().downcast_ref::<RadioPart>()).unwrap().buttons();
    let (reading, picking) = (buttons("reading"), buttons("picking"));
    let order = screen.tab_order();
    assert!(order.iter().all(|e| !reading.contains(e)));
    assert_eq!(picking.len(), order.iter().filter(|e| picking.contains(e)).count());
}

#[test]
fn a_required_text_cleared_is_empty_as_every_input_takes_an_empty_value() {
    let _alone = super::one_at_a_time();
    let host = InputsHost::new();
    host.set("task.name", json!("Ada"));
    host.facts("task.name", FieldFacts { label: "Name".into(), required: true, ..FieldFacts::default() });
    let mut screen = window(r#"<Textbox at="1,1" span="4" name="name" bind="task.name" />"#, host.clone());
    let id = part_named(&screen, "name");
    screen.focus(Some(id));
    screen.act(id, |e, _| e.as_any_mut().downcast_mut::<TextBoxPart>().unwrap().editor_mut().select_all());
    press(&mut screen, Key::Delete);
    press(&mut screen, Key::Enter);
    assert_eq!(Some(json!("")), host.read("task.name"));
}

const MOODS_MODEL: &str = r#"{"project": {"locale": {"language": "en"}, "translations": {"en": {}, "nl": {}}}}"#;

#[test]
fn an_unbound_tags_part_keeps_its_list_when_its_window_is_made_again() {
    let _alone = super::one_at_a_time();
    let files = program::files("Moods", r#"<Tags at="1,1" span="8" name="moods" onchange="moodsChanged" />"#, MOODS_MODEL);
    clarity_ui::choices::start(&files, None);
    let (mut screen, program) = program::shown_from(files, Map::new());
    let tags = |screen: &Screen| element::find_type::<TagsPart>(screen.root(), |_| true).map(|t| (t.node().id(), t.items(), t.chips().iter().map(|c| c.label.clone()).collect::<Vec<_>>())).unwrap();
    let (first, _, _) = tags(&screen);
    let typed = element::find_type::<TagsPart>(screen.root(), |_| true).and_then(|t| t.box_id()).unwrap();
    screen.focus(Some(typed));
    screen.text_typed("calm");
    press(&mut screen, Key::Enter);
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!(vec![json!("calm")], tags(&screen).1);
    assert_eq!(vec!["moodsChanged".to_string()], program.ran.lock().unwrap().clone());

    clarity_ui::choices::choose_language("nl");
    screen.act_root::<WindowStack, _>(|_, _| ());
    Shot::of(&mut screen, 800, 400, 1.0);

    let (again, items, chips) = tags(&screen);
    assert_ne!(first, again);
    assert_eq!(vec![json!("calm")], items);
    assert_eq!(vec!["calm".to_string()], chips);
    clarity_ui::choices::choose_language("en");
    screen.act_root::<WindowStack, _>(|_, _| ());
    let _ = Modifiers::NONE;
}
