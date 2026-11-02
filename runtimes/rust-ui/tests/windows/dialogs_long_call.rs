//! A long call (DESIGN 5.12), as the C# LongCallBarTests have it: while a
//! handler runs off the screen's thread the window keeps drawing, shows how
//! far the call is with a bar and Stop at the bottom right, and takes no
//! other handler; Stop stops once and then greys; once the handler ended
//! the bar goes. A long call shows and waits only in its own window stack.

use super::dialogs_fake;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clarity_ui::dialogs::long_call::Said;
use clarity_ui::dialogs::LongCallBar;
use clarity_ui::input::{Key, KeyInput};
use clarity_ui::window::WindowStack;
use clarity_ui::{Element, Screen, ThemeLook};
use dialogs_fake::*;
use serde_json::json;

fn bar_of(said: Option<(i64, i64)>) -> (LongCallBar, Arc<AtomicBool>) {
    let said: Said = Arc::new(Mutex::new(said));
    let stop = Arc::new(AtomicBool::new(false));
    (LongCallBar::new(said, stop.clone()), stop)
}

fn wait_for(what: impl Fn() -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    while !what() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn the_bar(screen: &Screen) -> Option<&LongCallBar> {
    screen.find::<LongCallBar>()
}

#[test]
fn the_bar_shows_how_far_and_stop_stops() {
    let (bar, stop) = bar_of(Some((30, 120)));
    let mut screen = Screen::new(Box::new(bar), ThemeLook::clarity(false));
    screen.resize(600.0, 400.0, 1.0);
    let mut picture = clarity_ui::testing::Shot::of(&mut screen, 600, 400, 1.0);
    picture.save(format!("{SHOTS}/r1-dialogs-long-bar.png")).unwrap();
    let bar = screen.root().as_any().downcast_ref::<LongCallBar>().unwrap();
    assert_eq!(0.25, bar.fraction());
    assert_eq!("25%", bar.said_text());
    assert_eq!("Stop", bar.stop_button().text);
    // At the bottom right, clear of the messages' notes at the top.
    let note = bar.note();
    assert!(note.right() > 500.0 && note.bottom() > 300.0, "{note:?}");
    let stop_at = bar.stop_button().node().bounds;
    screen.click(stop_at.x + 4.0, stop_at.y + 4.0);
    assert!(stop.load(Ordering::SeqCst));
    // Stop greys once pressed: neither a press nor Escape stops again.
    let bar = screen.root().as_any().downcast_ref::<LongCallBar>().unwrap();
    assert!(bar.stopped());
    assert!(!bar.stop_button().usable());
    let id = bar.stop_button().node().id();
    assert!(!screen.enabled(id));
    let (again, again_stop) = bar_of(Some((1, 2)));
    let mut other = Screen::new(Box::new(again), ThemeLook::clarity(false));
    other.resize(600.0, 400.0, 1.0);
    // Escape stops a call whose Stop was not pressed.
    other.key_pressed(KeyInput::plain(Key::Escape));
    assert!(again_stop.load(Ordering::SeqCst));
    let again = other.root().as_any().downcast_ref::<LongCallBar>().unwrap();
    assert!(!again.stop_button().usable());
}

#[test]
fn stop_once_pressed_stops_nothing_more() {
    let said: Said = Arc::new(Mutex::new(Some((1, 4))));
    let stop = Arc::new(AtomicBool::new(false));
    let mut screen = Screen::new(Box::new(LongCallBar::new(said, stop.clone())), ThemeLook::clarity(false));
    screen.resize(600.0, 400.0, 1.0);
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(stop.load(Ordering::SeqCst));
    let bar = screen.root().as_any().downcast_ref::<LongCallBar>().unwrap();
    let at = bar.stop_button().node().bounds;
    assert!(bar.stopped());
    // A second press, by the pointer or Escape, does nothing: the button takes nothing now.
    screen.click(at.x + 4.0, at.y + 4.0);
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert!(screen.root().as_any().downcast_ref::<LongCallBar>().unwrap().stopped());
}

#[test]
fn a_call_that_says_nothing_yet_shows_an_empty_bar() {
    let (bar, _) = bar_of(None);
    assert_eq!(0.0, bar.fraction());
    assert_eq!("", bar.said_text());
    assert!(!bar.says());
}

#[test]
fn a_long_call_shows_the_bar_over_the_window_and_stops() {
    let (mut screen, fake) = started_with(Fake { watches: true, ..Fake::default() });
    fake.calls();
    click(&mut screen, "long");
    wait_for(|| fake.long_reached.load(Ordering::SeqCst));
    // The screen draws on; the bar shows what the call said.
    shot(&mut screen, "long-call");
    let bar = the_bar(&screen).expect("the bar lies over the window");
    assert_eq!(0.25, bar.fraction());
    assert!(screen.overlays().iter().any(|o| o.modal));
    // No other handler runs meanwhile.
    click(&mut screen, "save");
    assert_eq!(vec!["dispatch long"], fake.calls());
    let stop = the_bar(&screen).unwrap().stop_button().node().id();
    click_at(&mut screen, stop);
    screen.finish_later();
    assert_eq!(json!(true), stack(&screen).current().unwrap().state_now().data["stopped"]);
    // Once the handler ended, the bar is gone.
    assert!(the_bar(&screen).is_none());
    assert!(!stack(&screen).long_call());
}

/// Two window stacks (two desktop programs'): a long call in one shows its
/// bar only there, the other's handlers run on, and the other's Escape
/// stops nothing of it. A desktop window of its own shares its opener's call.
#[test]
fn a_long_call_shows_and_waits_only_in_its_own_stack() {
    let (mut first, waiting) = started_with(Fake { watches: true, ..Fake::default() });
    let (mut second, other) = started_with(Fake { watches: true, ..Fake::default() });
    waiting.calls();
    other.calls();
    click(&mut first, "long");
    wait_for(|| waiting.long_reached.load(Ordering::SeqCst));
    shot(&mut first, "long-call-first");
    assert!(the_bar(&first).is_some());
    assert!(stack(&first).running().calling());
    // The other stack's handlers run, and it shows no bar.
    click(&mut second, "save");
    second.finish_later();
    assert_eq!(vec!["dispatch save"], other.calls());
    assert!(the_bar(&second).is_none());
    assert!(!stack(&second).running().calling());
    // Its Escape stops nothing of the first's call.
    second.key_pressed(KeyInput::plain(Key::Escape));
    std::thread::sleep(Duration::from_millis(50));
    assert!(stack(&first).long_call());
    assert!(!the_bar(&first).unwrap().stopped());
    // A desktop window of its own shares its opener's call: its handlers wait too.
    let running = stack(&first).running().clone();
    let request = clarity_ui::window::ApartWindow { window: "Main".into(), with: Default::default(), opener: 0, asked: None, dialog: false };
    let mut own = Screen::new(Box::new(WindowStack::apart(running, request)), FILES.look(false));
    own.resize(800.0, 500.0, 1.0);
    waiting.calls();
    click(&mut own, "save");
    assert!(!waiting.calls().contains(&"dispatch save".to_string()));
    // The first's own Stop stops it.
    let stop = the_bar(&first).unwrap().stop_button().node().id();
    click_at(&mut first, stop);
    first.finish_later();
    assert!(the_bar(&first).is_none());
    assert_eq!(json!(true), stack(&first).current().unwrap().state_now().data["stopped"]);
}

/// The bar asks the call how far it is each time it looks, and tells it
/// Stop once: plain calls by the watch's number, which a program of any
/// language answers.
#[test]
fn a_watching_bar_asks_how_far_and_tells_stop_once() {
    use std::cell::Cell;
    use std::rc::Rc;
    let at = Rc::new(Cell::new(None::<(i64, i64)>));
    let stops = Rc::new(Cell::new(0));
    let (asks, told) = (at.clone(), stops.clone());
    let bar = LongCallBar::watching(Rc::new(move || asks.get()), Rc::new(move || told.set(told.get() + 1)));
    assert!(!bar.says());
    at.set(Some((3, 4)));
    assert_eq!(0.75, bar.fraction());
    let mut screen = Screen::new(Box::new(bar), ThemeLook::clarity(false));
    screen.resize(600.0, 400.0, 1.0);
    clarity_ui::testing::Shot::of(&mut screen, 600, 400, 1.0);
    let stop_at = screen.root().as_any().downcast_ref::<LongCallBar>().unwrap().stop_button().node().bounds;
    screen.click(stop_at.x + 4.0, stop_at.y + 4.0);
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert_eq!(1, stops.get());
    assert!(screen.root().as_any().downcast_ref::<LongCallBar>().unwrap().stopped());
}
