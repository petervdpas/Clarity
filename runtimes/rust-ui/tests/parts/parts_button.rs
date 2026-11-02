//! The buttons, as the C# ButtonPartTests check them: a button runs its
//! click from the pointer, Enter and Space; it is drawn in the theme's button
//! look with its hover and pressed looks and the focus ring; a disabled one
//! does nothing. A toggle stays pressed, bound or in its state; a button
//! group joins its buttons; a split button opens its menu from its arrow.

use clarity_runtime::ui::Text;
use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::painter::Corners;
use clarity_ui::parts::button::ButtonPart;
use clarity_ui::parts::containers::buttons::ButtonsPart;
use clarity_ui::parts::inputs::choice_list::LINE;
use clarity_ui::parts::inputs::toggle::TogglePart;
use clarity_ui::parts::PartHost;
use clarity_ui::Element;
use serde_json::json;
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::Screen;

fn click(screen: &mut Screen, name: &str) {
    screen.resize(800.0, 400.0, 1.0);
    let b = screen.element(part_named(screen, name)).unwrap().node().bounds;
    screen.click(b.x + 6.0, b.y + 6.0);
}

#[test]
fn a_click_runs_its_handlers() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Button at="1,1" span="2" name="seven" caption="7" onclick="digit(7)" />"#, host.clone());
    click(&mut screen, "seven");
    assert_eq!(vec!["run seven click digit(7)"], host.logged());
}

#[test]
fn a_click_also_opens_the_window_it_names() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Button at="1,1" span="2" name="users" opens="Users" />"#, host.clone());
    click(&mut screen, "users");
    assert_eq!(vec!["run users click Users"], host.logged());
}

#[test]
fn enter_and_space_click_the_focused_button() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Button at="1,1" span="2" name="go" onclick="go" />"#, host.clone());
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert_eq!(Some(part_named(&screen, "go")), screen.focused());
    screen.key_pressed(KeyInput::plain(Key::Enter));
    screen.key_pressed(KeyInput::plain(Key::Space));
    assert_eq!(vec!["run go click go", "run go click go"], host.logged());
}

#[test]
fn a_disabled_button_does_nothing_and_takes_no_focus() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Button at="1,1" span="2" name="go" onclick="go" />"#, host.clone());
    host.set_state("go", |s| s.enabled = Some(false));
    click(&mut screen, "go");
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert!(host.logged().is_empty());
    assert_eq!(None, screen.focused());
}

#[test]
fn its_caption_is_the_project_s_text_or_what_code_sets() {
    let host = TestHost::new();
    host.texts.borrow_mut().insert("saveCaption".into(), "Opslaan".into());
    let screen = window_of(r#"<Button at="1,1" span="2" name="save" caption="@saveCaption" />"#, host.clone());
    let button = |s: &Screen| s.element(part_named(s, "save")).unwrap().as_any().downcast_ref::<ButtonPart>().unwrap().caption();
    assert_eq!("Opslaan", button(&screen));
    host.set_state("save", |s| s.caption = Some(Text::plain("Bewaar")));
    assert_eq!("Bewaar", button(&screen));
}

#[test]
fn it_is_drawn_in_the_theme_s_button_look_hovered_and_pressed() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Button at="1,1" span="2" caption="" />"#, host);
    let look = screen.look().clone();
    let mut plain = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(near(look.part("button", None).background.unwrap(), plain.pixel(6, 18), 2));
    screen.pointer_moved(6.0, 18.0);
    let mut hovered = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(near(look.part("button", Some("hover")).border.unwrap(), hovered.pixel(0, 18), 40));
    screen.pointer_pressed(6.0, 18.0, PointerButton::Left, 1);
    let mut pressed = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(near(look.part("button", Some("pressed")).background.unwrap(), pressed.pixel(6, 18), 2));
}

#[test]
fn the_focused_button_shows_the_focus_ring_and_a_disabled_one_is_dimmed() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Button at="1,1" span="2" name="go" caption="" />"#, host.clone());
    screen.key_pressed(KeyInput::plain(Key::Tab));
    let mut focused = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(near(screen.look().color("focus"), focused.pixel(1, 18), 40));
    screen.focus(None);
    let enabled = Shot::of(&mut screen, 800, 400, 1.0).pixel(0, 18);
    host.set_state("go", |s| s.enabled = Some(false));
    let mut dimmed = Shot::of(&mut screen, 800, 400, 1.0);
    assert_ne!(enabled, dimmed.pixel(0, 18));
}

#[test]
fn a_default_button_has_the_accent_ring() {
    let host = TestHost::new();
    let mut screen = window_of(r#"<Button at="1,1" span="2" caption="" default="true" />"#, host);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    assert!(near(screen.look().color("accent"), shot.pixel(0, 18), 40));
}

#[test]
fn a_read_only_toggle_is_not_pressed() {
    let host = TestHost::new();
    host.set("v", json!(false));
    let mut screen = window_of(r#"<Toggle at="1,1" span="2" name="pin" bind="v" readonly="true" onchange="pinned" />"#, host.clone());
    click(&mut screen, "pin");
    assert!(host.logged().is_empty());
}

#[test]
fn a_button_group_joins_its_buttons_with_only_its_outer_corners_round() {
    let host = TestHost::new();
    let mut screen = window_of(
        r#"<Buttons at="1,1" span="4">
             <Button at="1,1" span="2" name="left" onclick="goLeft" />
             <Toggle at="3,1" span="2" name="right" />
           </Buttons>"#,
        host.clone(),
    );
    screen.resize(800.0, 400.0, 1.0);
    let group = clarity_ui::element::find_type::<ButtonsPart>(screen.root(), |_| true).unwrap();
    let pushes = group.inner().node().children();
    assert_eq!(2, pushes.len());
    let (a, b) = (pushes[0].node().bounds, pushes[1].node().bounds);
    assert!((a.right() - b.x).abs() < 0.001);
    assert!((group.node().bounds.right() - b.right()).abs() < 0.001);
    let left = pushes[0].as_any().downcast_ref::<ButtonPart>().unwrap().corners;
    let right = pushes[1].as_any().downcast_ref::<TogglePart>().unwrap().corners;
    assert_eq!((Corners { left: true, right: false }, Corners { left: false, right: true }), (left, right));
    screen.click(a.x + a.width / 2.0, a.y + a.height / 2.0);
    screen.click(b.x + b.width / 2.0, b.y + b.height / 2.0);
    assert_eq!("run left click goLeft", host.logged()[0]);
    assert_eq!(Some(true), host.state(Some("right")).unwrap().checked);
}

#[test]
fn a_split_button_runs_its_click_and_opens_its_menu_from_the_arrow() {
    let host = TestHost::new();
    let mut screen = window_of(
        r#"<Button at="1,1" span="4" name="save" split="true" onclick="save">
             <Menu><Item caption="Save as draft" onclick="saveDraft" /><Item kind="separator" /><Item caption="Save a copy" onclick="saveCopy(1)" /></Menu>
           </Button>"#,
        host.clone(),
    );
    screen.resize(800.0, 400.0, 1.0);
    let id = part_named(&screen, "save");
    let bounds = split_button(&screen, id).node().bounds;
    screen.click(bounds.x + 10.0, 18.0);
    assert_eq!(vec!["run save click save"], host.logged());
    let arrow = split_button(&screen, id).arrow();
    screen.click(arrow.x + 10.0, 18.0);
    assert_eq!(1, screen.overlays().len());
    let place = screen.overlays()[0].place;
    assert!(place.y >= bounds.bottom());
    screen.click(place.x + 10.0, place.y + 1.0 + LINE * 1.5);
    assert_eq!(vec!["run save click save", "run save click saveCopy(1)"], host.logged());
    assert!(screen.overlays().is_empty());
    assert!(!split_button(&screen, id).menu_open());
}

fn split_button(screen: &Screen, id: clarity_ui::element::ElementId) -> &ButtonPart {
    screen.element(id).unwrap().as_any().downcast_ref::<ButtonPart>().unwrap()
}
