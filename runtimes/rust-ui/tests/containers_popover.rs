//! A popover lies over the window while its part's state is visible, under
//! the part named in its `for` (above it when there is no room below);
//! a press outside it or Escape closes it, and its state hears so. In the
//! window's grid it takes nothing.

use std::rc::Rc;

use clarity_ui::element::{in_order, ElementId};
use clarity_ui::parts::containers::popover::PopoverPart;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{near, part_name, window_of, Shot, TestHost};
use clarity_ui::{Element, Key, KeyInput, PointerButton, Rect, Screen};

const WINDOW: &str = r#"
    <Label name="save" at="1,2" span="3" caption="Save" />
    <Popover name="tip" for="save" at="7,6" span="4" rows="2"><Label name="inside" at="1,1" span="4" caption="Inside" /></Popover>"#;

fn made(visible: bool) -> (Screen, Rc<TestHost>) {
    let host = TestHost::new();
    host.set_state("tip", |s| s.visible = Some(visible));
    let mut screen = window_of(WINDOW, host.clone());
    screen.resize(808.0, 400.0, 1.0);
    (screen, host)
}

fn popover(screen: &Screen) -> &PopoverPart {
    clarity_ui::element::find_type::<PopoverPart>(screen.root(), |_| true).expect("a popover")
}

/// The id of the part named `name` in the overlay on top.
fn in_overlay(screen: &Screen, name: &str) -> ElementId {
    let top = screen.overlays().last().expect("an overlay");
    in_order(top.content.as_ref()).into_iter().find(|e| part_name(*e) == Some(name)).map(|e| e.node().id()).expect("the part is in the overlay")
}

fn visible(host: &TestHost) -> Option<bool> {
    host.state(Some("tip")).unwrap().visible
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn it_opens_while_its_state_is_visible_and_closes_when_it_is_not() {
    let (mut screen, host) = made(false);
    assert!(screen.overlays().is_empty());
    host.set_state("tip", |s| s.visible = Some(true));
    screen.resize(808.0, 400.0, 1.0);
    assert_eq!(1, screen.overlays().len());
    assert_eq!(Some(screen.overlays()[0].id()), popover(&screen).opened());
    let look = screen.look().clone();
    let (width, height) = popover(&screen).size(&look);
    assert_eq!(4.0 * PopoverPart::COLUMN + 3.0 * 8.0 + 2.0 * (2.0 * look.padding() + look.border()), width);
    let place = screen.overlays()[0].content.node().bounds;
    assert_eq!((width, height), (place.width, place.height));
    // Its parts are inside it, and take the pointer there.
    let inside = in_overlay(&screen, "inside");
    let b = screen.element(inside).unwrap().node().bounds;
    assert_eq!(Some(inside), screen.at(b.x + 5.0, b.y + 5.0));
    host.set_state("tip", |s| s.visible = Some(false));
    screen.resize(808.0, 400.0, 1.0);
    assert!(screen.overlays().is_empty());
    assert_eq!(None, popover(&screen).opened());
}

#[test]
fn a_press_outside_or_escape_closes_it_and_its_state_hears() {
    let (mut screen, host) = made(true);
    assert_eq!(1, screen.overlays().len());
    screen.pointer_pressed(700.0, 380.0, PointerButton::Left, 1);
    screen.pointer_released(700.0, 380.0, PointerButton::Left);
    assert!(screen.overlays().is_empty());
    assert_eq!(Some(false), visible(&host));
    host.set_state("tip", |s| s.visible = Some(true));
    screen.resize(808.0, 400.0, 1.0);
    assert_eq!(1, screen.overlays().len());
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(screen.overlays().is_empty());
    assert_eq!(Some(false), visible(&host));
    // Drawn again, it stays closed.
    screen.resize(808.0, 400.0, 1.0);
    assert!(screen.overlays().is_empty());
}

#[test]
fn in_the_windows_grid_it_takes_no_room_and_no_pointer() {
    let (screen, _) = made(false);
    let part = popover(&screen);
    assert!(!part.is_shown());
    assert!(!clarity_ui::Part::grows(part));
    let b = part.node().bounds;
    assert_ne!(Some(part.node().id()), screen.at(b.x + 5.0, b.y + 5.0));
}

#[test]
fn one_without_a_name_never_opens() {
    let mut screen = window_of(&WINDOW.replace(r#"name="tip" "#, ""), TestHost::new());
    screen.resize(808.0, 400.0, 1.0);
    screen.resize(808.0, 400.0, 1.0);
    assert!(screen.overlays().is_empty());
}

#[test]
fn it_lies_under_its_part_above_when_there_is_no_room_below_kept_off_the_edges() {
    let whole = Rect::new(0.0, 0.0, 808.0, 400.0);
    let own = Rect::new(404.0, 220.0, 344.0, 80.0);
    let size = (360.0, 100.0);
    // Under the part, at its left, kept off the window's edge.
    let save = Rect::new(0.0, 44.0, 196.0, 36.0);
    assert_eq!(Rect::new(4.0, 44.0 + 36.0 + 4.0, 360.0, 100.0), PopoverPart::place(Some(save), own, size, whole));
    // Above it when there is no room under it.
    let low = Rect::new(600.0, 352.0, 196.0, 36.0);
    assert_eq!(Rect::new(808.0 - 360.0 - 4.0, 352.0 - 4.0 - 100.0, 360.0, 100.0), PopoverPart::place(Some(low), own, size, whole));
    // Without a part it is for, at its own place.
    assert_eq!(Rect::new(404.0, 220.0, 360.0, 100.0), PopoverPart::place(None, own, size, whole));
}

#[test]
#[ignore = "needs the place of the part its `for` names from the screen (g8-shared.md popover-anchor)"]
fn it_opens_under_the_part_its_for_names() {
    let (screen, _) = made(true);
    let place = screen.overlays()[0].content.node().bounds;
    assert_eq!(4.0, place.x);
    assert_eq!(44.0 + 36.0 + 4.0, place.y);
}

#[test]
fn it_draws_as_a_panel_with_a_strong_border_over_the_window() {
    let (mut screen, _) = made(true);
    let (panel, strong) = (screen.look().color("panel"), screen.look().color("strong_border"));
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    keep(&mut shot, "popover.png");
    let b = screen.overlays()[0].content.node().bounds;
    assert!(near(panel, shot.pixel((b.x + b.width - 6.0) as i32, (b.y + b.height - 6.0) as i32), 10));
    assert!(near(strong, shot.pixel((b.x + b.width / 2.0) as i32, b.y as i32), 40));
}
