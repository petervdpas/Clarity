//! What the screen does for every element in its tree: one inside a hidden
//! element draws nothing and takes neither Tab nor the pointer; an element
//! that animates gets a frame drawn on each turn; a press on what an
//! overlay opened from is no press outside it. Also the opener of addresses
//! each system gets, and a part state's own look.

use std::rc::Rc;

use clarity_ui::element;
use clarity_ui::input::{Key, KeyInput, PointerButton};
use clarity_ui::parts::inputs::dropdown::DropdownPart;
use clarity_ui::parts::inputs::testing::InputsHost;
use clarity_ui::parts::text_editor::TextEditor;
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use clarity_ui::theme::{LookValues, ThemeLook};
use clarity_ui::window::address::{command_for, Os};
use clarity_ui::Screen;
use serde_json::json;

fn shown(markup: &str, host: Rc<dyn clarity_ui::PartHost>) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(800.0, 400.0, 1.0);
    screen
}

fn part_named_in(screen: &Screen, name: &str) -> u64 {
    part_named(screen, name)
}

fn editor_of(screen: &Screen, name: &str) -> u64 {
    // What takes the focus in a text box: its editor where it is in the tree, else the text box itself.
    let id = part_named_in(screen, name);
    element::in_order(screen.element(id).unwrap()).into_iter().find(|e| e.focusable()).map(|e| e.node().id()).unwrap_or(id)
}

#[test]
fn an_element_inside_a_hidden_part_takes_no_tab_and_draws_nothing() {
    let host = TestHost::new();
    let mut screen = shown(
        r#"<Textbox name="first" at="1,1" span="4" />
            <Panel name="box" at="1,2" span="6" rows="3" caption="Box"><Textbox name="inner" at="1,1" span="4" /></Panel>
            <Textbox name="last" at="1,5" span="4" />"#,
        host.clone(),
    );
    Shot::of(&mut screen, 800, 400, 1.0);
    let inner = part_named(&screen, "inner");
    let at = screen.element(inner).unwrap().node().bounds;
    host.set_state("box", |s| s.visible = Some(false));
    assert!(!screen.shows(inner));
    let first = editor_of(&screen, "first");
    assert!(screen.shows(first));
    screen.focus(Some(first));
    screen.key_pressed(KeyInput::plain(Key::Tab));
    assert_eq!(screen.focused(), Some(editor_of(&screen, "last")));
    // Nothing of the panel is drawn: its place is the window's background.
    let mut pixels = Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!(pixels.pixel((at.x + 4.0) as i32, (at.y + at.height / 2.0) as i32), screen.look().color("background"));
    let under = screen.at(at.x + 4.0, at.y + at.height / 2.0).and_then(|id| screen.element(id));
    assert!(!under.is_some_and(|e| e.as_any().is::<TextEditor>()));
}

#[test]
fn a_busy_indicator_asks_for_frames_while_it_shows() {
    let host = TestHost::new();
    let mut screen = shown(r#"<Busy name="busy" at="1,1" span="4" caption="Loading" />"#, host.clone());
    Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!(screen.animating(), 1);
    host.set_state("busy", |s| s.visible = Some(false));
    Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!(screen.animating(), 0);
}

#[test]
fn a_press_on_what_an_overlay_opened_from_goes_to_it() {
    let host = InputsHost::new();
    host.values.borrow_mut().insert("size".into(), json!("S"));
    host.offer("size", &[(json!("S"), "Small"), (json!("L"), "Large")]);
    let mut screen = shown(r#"<Dropdown name="size" at="1,1" span="4" bind="size" />"#, host);
    let id = part_named(&screen, "size");
    let place = screen.element(id).unwrap().node().bounds;
    let (x, y) = (place.x + 10.0, place.y + 10.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
    let dropdown = |screen: &Screen| screen.element(id).unwrap().as_any().downcast_ref::<DropdownPart>().unwrap().list();
    assert!(dropdown(&screen).is_some());
    // A second click on its own box closes the list, and the box keeps the focus.
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
    assert!(dropdown(&screen).is_none());
    assert_eq!(screen.focused(), Some(id));
}

#[test]
fn each_system_opens_an_address_with_its_own_program() {
    let flat = |os: Os, address: &str| command_for(os, address).map(|c| (c.program, c.arguments.join(" ")));
    assert_eq!(flat(Os::Linux, "https://example.org"), Some(("xdg-open".into(), "https://example.org".into())));
    assert_eq!(flat(Os::MacOs, "mailto:ada@example.org"), Some(("open".into(), "mailto:ada@example.org".into())));
    assert_eq!(flat(Os::Windows, "https://example.org").map(|c| c.1.ends_with("https://example.org")), Some(true));
    // Only what a link opens is passed on.
    assert_eq!(flat(Os::Linux, "file:///etc/passwd"), None);
    assert_eq!(flat(Os::Windows, "calc.exe"), None);
}

#[test]
fn a_parts_state_look_is_only_what_the_state_sets() {
    let look = ThemeLook::clarity(false);
    let pressed = look.part_own("button", Some("pressed"));
    assert_eq!(pressed.background, Some(look.color("accent_soft")));
    assert_eq!(pressed.radius, None);
    assert_eq!(look.part_own("button", None), LookValues::default());
}
