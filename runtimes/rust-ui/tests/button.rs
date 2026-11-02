//! A button: its click runs its handlers, Enter and Space click it when it
//! has the focus, a disabled one does nothing, and it is drawn in the
//! theme's button look, hovered, pressed, focused and dimmed.

use clarity_runtime::ui::Text;
use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::button::ButtonPart;
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
