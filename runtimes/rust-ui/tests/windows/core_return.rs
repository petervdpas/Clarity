//! A window that closes in place gives the screen back to the window below:
//! the first press there goes where it is made, and a note its handler said
//! just before it closed shows in the window below.

use super::core_windows_fake;

use clarity_ui::PointerButton;
use clarity_ui::window::{Apart, WindowStack};
use clarity_ui::Screen;
use core_windows_fake::*;

fn middle(screen: &clarity_ui::Screen, name: &str) -> (f64, f64) {
    let b = screen.element(named(screen, name)).unwrap().node().bounds;
    (b.x + b.width / 2.0, b.y + b.height / 2.0)
}

/// Draws the screen once, as the desktop host does after every event.
fn drawn(screen: &mut clarity_ui::Screen) {
    let mut surface = skia_safe::surfaces::raster_n32_premul((800, 500)).unwrap();
    screen.draw(surface.canvas());
}

#[test]
fn the_first_press_after_a_return_goes_where_it_is_made() {
    let (mut screen, fake) = started();
    drawn(&mut screen);
    click(&mut screen, "openConnection");
    drawn(&mut screen);
    assert_eq!(stack(&screen).windows(), ["Main", "Connection"]);
    let (cx, cy) = middle(&screen, "cancelConnection");
    screen.pointer_moved(cx, cy);
    screen.pointer_pressed(cx, cy, PointerButton::Left, 1);
    screen.pointer_released(cx, cy, PointerButton::Left);
    drawn(&mut screen);
    assert_eq!(stack(&screen).windows(), ["Main"]);
    // One move to Save, then a press and release there, as a person does.
    let (x, y) = middle(&screen, "save");
    screen.pointer_moved(x, y);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
    assert!(fake.calls().iter().any(|c| c == "dispatch save"), "{:?}", fake.calls());
}

#[test]
fn a_note_said_before_a_close_shows_in_the_window_below() {
    let (mut screen, _fake) = started();
    drawn(&mut screen);
    click(&mut screen, "openConnection");
    drawn(&mut screen);
    click(&mut screen, "saveConnection");
    drawn(&mut screen);
    assert_eq!(stack(&screen).windows(), ["Main"]);
    let notes: Vec<String> = stack(&screen).notes().iter().map(|m| m.text.show()).collect();
    assert_eq!(notes, ["Saved"]);
    shot(&mut screen, "note-after-return");
}

/// Only the route marked `in new window` opens a window apart; another
/// route to the same window (a button, `open` in code) opens it in place.
#[test]
fn only_a_route_marked_in_new_window_opens_apart() {
    let (mut screen, _fake) = started();
    stack(&screen).running().set_apart(true);
    click(&mut screen, "openConnection");
    drawn(&mut screen);
    assert_eq!(stack(&screen).windows(), ["Main", "Connection"]);
    assert!(screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap().is_empty());
    click(&mut screen, "cancelConnection");
    drawn(&mut screen);
    assert_eq!(stack(&screen).windows(), ["Main"]);
    click(&mut screen, "openConnectionApart");
    assert_eq!(stack(&screen).windows(), ["Main"]);
    let aparts = screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap();
    let Some(Apart::Window(request)) = aparts.into_iter().next() else { panic!("a window of its own is asked for") };
    assert_eq!(("Connection", false), (request.window.as_str(), request.dialog));
}

/// A note said just before a window of its own closes goes with it to the
/// window that opened it, and shows there.
#[test]
fn a_note_said_before_a_window_of_its_own_closes_shows_in_its_opener() {
    let (mut screen, _fake) = started();
    stack(&screen).running().set_apart(true);
    click(&mut screen, "openConnectionApart");
    let aparts = screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap();
    let Some(Apart::Window(request)) = aparts.into_iter().next() else { panic!("a window of its own is asked for") };
    let apart = WindowStack::apart(stack(&screen).running().clone(), request);
    let mut own = Screen::new(Box::new(apart), FILES.look(false));
    own.resize(800.0, 500.0, 1.0);
    drawn(&mut own);
    click(&mut own, "saveConnection");
    let (ended, closed) = own.act_root::<WindowStack, _>(|s, _| (s.ended(), s.take_closed())).unwrap();
    assert!(ended);
    let closed = closed.expect("the window that opened it is told");
    assert_eq!(closed.said.iter().map(|m| m.text.show()).collect::<Vec<_>>(), ["Saved"]);
    screen.act_root::<WindowStack, _>(move |s, cx| s.apart_closed(closed, cx));
    assert_eq!(stack(&screen).notes().iter().map(|m| m.text.show()).collect::<Vec<_>>(), ["Saved"]);
}

/// A handler's `open ... in new window` opens a desktop window of its own,
/// with its values, where the host gives them; else in place.
#[test]
fn a_handler_s_open_in_new_window_opens_apart_with_its_values() {
    fn opens_apart(reply: &mut clarity_runtime::ui::Reply) {
        let with = serde_json::Map::from_iter([("year".to_string(), serde_json::json!(2026))]);
        reply.open.push(clarity_runtime::ui::OpenRequest { window: "Connection".into(), with, new_window: true });
    }
    let (mut screen, fake) = started();
    stack(&screen).running().set_apart(true);
    *fake.then.lock().unwrap() = Some(opens_apart);
    click(&mut screen, "save");
    assert_eq!(stack(&screen).windows(), ["Main"]);
    let aparts = screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap();
    let Some(Apart::Window(request)) = aparts.into_iter().next() else { panic!("a window of its own is asked for") };
    assert_eq!(("Connection", false), (request.window.as_str(), request.dialog));
    assert_eq!(Some(&serde_json::json!(2026)), request.with.get("year"));
    // Where the host gives no windows of their own, it opens in place.
    let (mut screen, fake) = started();
    *fake.then.lock().unwrap() = Some(opens_apart);
    click(&mut screen, "save");
    assert_eq!(stack(&screen).windows(), ["Main", "Connection"]);
}

/// A handler that quits by code ends the program, from a window of its own too.
#[test]
fn a_handler_that_quits_by_code_ends_the_program_from_any_window() {
    fn quits(reply: &mut clarity_runtime::ui::Reply) {
        reply.quit = true;
    }
    let (mut screen, fake) = started();
    *fake.then.lock().unwrap() = Some(quits);
    click(&mut screen, "save");
    assert!(stack(&screen).quitting());
    assert!(stack(&screen).ended());
    // From a window of its own: its stack quits, which ends the desktop's every window.
    let (mut screen, fake) = started();
    stack(&screen).running().set_apart(true);
    click(&mut screen, "openConnectionApart");
    let aparts = screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap();
    let Some(Apart::Window(request)) = aparts.into_iter().next() else { panic!("a window of its own is asked for") };
    let mut own = Screen::new(Box::new(WindowStack::apart(stack(&screen).running().clone(), request)), FILES.look(false));
    own.resize(800.0, 500.0, 1.0);
    drawn(&mut own);
    *fake.then.lock().unwrap() = Some(quits);
    click(&mut own, "keep");
    assert!(own.act_root::<WindowStack, _>(|s, _| s.quitting()).unwrap());
}

/// A handler that opens the keys window by code shows it over the window,
/// or asks for it in a window of its own where the host gives them.
#[test]
fn a_handler_that_opens_the_keys_window_by_code_shows_it() {
    fn keys(reply: &mut clarity_runtime::ui::Reply) {
        reply.keys = true;
    }
    let (mut screen, fake) = started();
    *fake.then.lock().unwrap() = Some(keys);
    click(&mut screen, "save");
    assert!(screen.find::<clarity_ui::dialogs::KeysWindow>().is_some());
    let (mut screen, fake) = started();
    stack(&screen).running().set_apart(true);
    *fake.then.lock().unwrap() = Some(keys);
    click(&mut screen, "save");
    let aparts = screen.act_root::<WindowStack, _>(|s, _| s.take_aparts()).unwrap();
    assert!(matches!(aparts.as_slice(), [Apart::Keys]));
}
