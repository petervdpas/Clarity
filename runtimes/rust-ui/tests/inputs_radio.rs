//! The radio group: one button per choice, side by side or one under the
//! other; picking another choice is a change, the one it has is not, and
//! what code sets is not; the arrow keys pick the next or previous choice.

use std::rc::Rc;

use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::inputs::radio::RadioPart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{near, part_named, window_of, Shot};
use clarity_ui::Screen;
use serde_json::json;

fn sizes() -> Rc<InputsHost> {
    let host = InputsHost::new();
    host.set("task.size", json!("Small"));
    host.offer("task.size", &[(json!("Small"), "Small"), (json!("Medium"), "Medium"), (json!("Large"), "Large")]);
    host
}

fn window(markup: &str, host: Rc<InputsHost>) -> Screen {
    let mut screen = window_of(markup, host);
    Shot::of(&mut screen, 800, 400, 1.0);
    screen
}

fn radio<'a>(screen: &'a Screen, name: &str) -> &'a RadioPart {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<RadioPart>()).expect("a radio part")
}

fn click(screen: &mut Screen, place: Rect) {
    let (x, y) = (place.x + 4.0, place.y + place.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

#[test]
fn its_choices_stand_side_by_side_or_one_under_the_other() {
    let screen = window(
        r#"<Radio at="1,1" span="12" name="across" bind="task.size" />
           <Radio at="1,2" span="12" rows="3" name="down" bind="task.size" orientation="vertical" />"#,
        sizes(),
    );
    let across = radio(&screen, "across").places();
    assert_eq!(3, across.len());
    assert!(across[1].x > across[0].right() && across[1].y == across[0].y);
    let down = radio(&screen, "down").places();
    assert!(down[1].y >= down[0].bottom() && down[1].x == down[0].x);
}

#[test]
fn picking_another_choice_changes_it_and_the_one_it_has_does_not() {
    let host = sizes();
    let mut screen = window(r#"<Radio at="1,1" span="12" name="size" bind="task.size" onchange="sized" />"#, host.clone());
    let places = radio(&screen, "size").places().to_vec();
    click(&mut screen, places[0]);
    assert!(host.logged().is_empty());
    click(&mut screen, places[2]);
    assert_eq!(Some(json!("Large")), host.read("task.size"));
    assert_eq!(vec![r#"write size task.size "Large""#, "run size change sized"], host.logged());
    assert_eq!(Some(2), radio(&screen, "size").chosen());
}

#[test]
fn what_code_sets_is_no_change() {
    let host = sizes();
    let screen = window(r#"<Radio at="1,1" span="12" name="size" bind="task.size" onchange="sized" />"#, host.clone());
    host.set("task.size", json!("Medium"));
    assert_eq!(Some(1), radio(&screen, "size").chosen());
    assert!(host.logged().is_empty());
}

#[test]
fn the_arrow_keys_pick_the_next_and_the_previous_choice() {
    let host = sizes();
    let mut screen = window(r#"<Radio at="1,1" span="12" name="size" bind="task.size" />"#, host.clone());
    screen.key_pressed(KeyInput::plain(Key::Tab));
    screen.key_pressed(KeyInput::plain(Key::Right));
    assert_eq!(Some(json!("Medium")), host.read("task.size"));
    screen.key_pressed(KeyInput::plain(Key::Left));
    screen.key_pressed(KeyInput::plain(Key::Left));
    assert_eq!(Some(json!("Large")), host.read("task.size"));
}

#[test]
fn a_disabled_or_read_only_group_cannot_be_picked() {
    let host = sizes();
    let mut screen = window(
        r#"<Radio at="1,1" span="12" name="first" bind="task.size" readonly="true" />
           <Radio at="1,2" span="12" name="second" bind="task.size" />"#,
        host.clone(),
    );
    host.set_state("second", |s| s.enabled = Some(false));
    let first = radio(&screen, "first").places()[1];
    let second = radio(&screen, "second").places()[1];
    click(&mut screen, first);
    click(&mut screen, second);
    assert!(host.logged().is_empty());
}

#[test]
fn the_chosen_button_is_drawn_in_the_accent_color() {
    let host = sizes();
    let mut screen = window(
        r#"<Radio at="1,1" span="12" name="size" bind="task.size" />
           <Radio at="1,2" span="4" rows="3" name="down" bind="task.size" orientation="vertical" />"#,
        host,
    );
    screen.key_pressed(KeyInput::plain(Key::Tab));
    let accent = screen.look().color("accent");
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join("inputs-radio.png")).expect("the picture is written");
    }
    let first = radio(&screen, "size").places()[0];
    let middle = (first.x + 1.0 + 8.0, first.y + first.height / 2.0);
    assert!(near(accent, shot.pixel(middle.0 as i32, middle.1 as i32), 10));
}
