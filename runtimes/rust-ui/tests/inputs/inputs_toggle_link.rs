//! The toggle: a button that stays pressed, bound or kept in its state;
//! pressing it is a change, what code sets is not, a read-only one is not
//! pressed. The link against the cases every renderer shares
//! (conformance/small-parts.json): what a bound link opens for its value; a
//! link opens its address, the one its value holds, or runs its click when
//! it has neither.

use std::rc::Rc;

use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::inputs::link::{address, LinkPart};
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::inputs::toggle::TogglePart;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{near, part_named, window_of, Shot};
use clarity_ui::{Element, Screen};
use serde_json::{json, Value};

fn window(markup: &str, host: Rc<InputsHost>) -> Screen {
    let mut screen = window_of(markup, host);
    Shot::of(&mut screen, 800, 400, 1.0);
    screen
}

fn part<'a, T: Element>(screen: &'a Screen, name: &str) -> &'a T {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<T>()).expect("a part of that kind")
}

fn click(screen: &mut Screen, name: &str) {
    let b = screen.element(part_named(screen, name)).unwrap().node().bounds;
    let (x, y) = (b.x + 4.0, b.y + b.height / 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

#[test]
fn a_bound_toggle_stays_pressed_and_tells_its_change() {
    let host = InputsHost::new();
    host.set("style.bold", json!(false));
    let mut screen = window(r#"<Toggle at="1,1" span="2" name="bold" bind="style.bold" onchange="restyle" />"#, host.clone());
    click(&mut screen, "bold");
    assert!(part::<TogglePart>(&screen, "bold").is_on());
    assert_eq!(vec!["write bold style.bold true", "run bold change restyle"], host.logged());
    click(&mut screen, "bold");
    assert_eq!(Some(json!(false)), host.read("style.bold"));
}

#[test]
fn an_unbound_toggle_keeps_its_state_and_what_code_sets_is_no_change() {
    let host = InputsHost::new();
    let mut screen = window(r#"<Toggle at="1,1" span="2" name="pin" onchange="pinned" />"#, host.clone());
    host.set_state("pin", |s| s.checked = Some(true));
    assert!(part::<TogglePart>(&screen, "pin").is_on());
    assert!(host.logged().is_empty());
    click(&mut screen, "pin");
    assert_eq!(Some(false), host.state(Some("pin")).unwrap().checked);
    assert_eq!(vec!["run pin change pinned"], host.logged());
}

#[test]
fn a_read_only_toggle_is_not_pressed_and_space_presses_a_focused_one() {
    let host = InputsHost::new();
    host.set("v", json!(false));
    host.set("w", json!(false));
    let mut screen = window(
        r#"<Toggle at="1,1" span="2" name="pin" bind="v" readonly="true" onchange="pinned" />
           <Toggle at="1,2" span="2" name="other" bind="w" />"#,
        host.clone(),
    );
    click(&mut screen, "pin");
    assert!(host.logged().is_empty());
    screen.focus(Some(part_named(&screen, "other")));
    screen.key_pressed(KeyInput::plain(Key::Space));
    assert_eq!(Some(json!(true)), host.read("w"));
}

#[test]
fn a_toggle_that_is_on_is_drawn_in_the_soft_accent() {
    let host = InputsHost::new();
    host.set("v", json!(true));
    let mut screen = window(r#"<Toggle at="1,1" span="2" name="pin" bind="v" caption="Pin" /><Toggle at="3,1" span="2" name="off" caption="Off" />"#, host);
    let soft = screen.look().color("accent_soft");
    let mut shot = Shot::of(&mut screen, 400, 100, 1.0);
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join("inputs-toggle.png")).expect("the picture is written");
    }
    let b = part::<TogglePart>(&screen, "pin").node().bounds;
    assert!(near(soft, shot.pixel((b.x + 6.0) as i32, (b.y + 6.0) as i32), 10));
}

fn small_parts() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/small-parts.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("conformance/small-parts.json")).expect("it is JSON")
}

#[test]
fn a_bound_value_opens_as_on_every_renderer() {
    let all = small_parts();
    for case in all["address"].as_array().unwrap() {
        assert_eq!(case["opens"].as_str().map(str::to_string), address(case["value"].as_str()), "{case}");
    }
}

#[test]
fn it_opens_its_address_its_values_or_runs_its_click() {
    let host = InputsHost::new();
    host.set("shop.mail", json!("ada@example.org"));
    host.set("shop.site", json!("lucide.dev"));
    let mut screen = window(
        r#"<Link at="1,1" span="3" name="docs" to="https://example.org/docs" onclick="never" />
           <Link at="1,2" span="3" name="mail" bind="shop.mail" />
           <Link at="1,3" span="3" name="site" bind="shop.site" onclick="never" />
           <Link at="1,4" span="3" name="more" caption="More" onclick="showMore" />"#,
        host.clone(),
    );
    for name in ["docs", "mail", "site", "more"] {
        click(&mut screen, name);
    }
    assert_eq!(vec!["https://example.org/docs", "mailto:ada@example.org"], host.opened.borrow().clone());
    assert_eq!(vec!["run more click showMore"], host.logged());
    assert_eq!("ada@example.org", part::<LinkPart>(&screen, "mail").shown());
    assert_eq!("More", part::<LinkPart>(&screen, "more").shown());
}

#[test]
fn a_disabled_link_does_nothing_and_a_link_draws_in_the_accent() {
    let host = InputsHost::new();
    let mut screen = window(r#"<Link at="1,1" span="3" name="more" caption="More" onclick="showMore" /><Link at="1,2" span="3" name="off" caption="Off" onclick="never" />"#, host.clone());
    host.set_state("off", |s| s.enabled = Some(false));
    click(&mut screen, "off");
    assert!(host.logged().is_empty());
    let mut shot = Shot::of(&mut screen, 400, 100, 1.0);
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join("inputs-link.png")).expect("the picture is written");
    }
    let b = part::<LinkPart>(&screen, "more").node().bounds;
    let background = screen.look().color("background");
    assert!(shot.any_not(b.x as i32, b.y as i32, 40, b.height as i32, background));
}
