//! A window's life as the desktop host leans on it (C# CheckFixWindowTests,
//! UnconnectedWindowTests, AppScreenTests, and the parts that live with
//! their window: TimerPartTests, SoundPartTests): the stack ends when its
//! last window closes and not before; the system's close closes the dialogs
//! over the window first, each hearing its `on close`, and tells the window
//! that opened it; while the databases are not open what keeps the
//! connection shows first; the host's file dialogs reach every window's
//! parts; a timer stops under a window opened in place and runs again when
//! it is back, and leaves out a tick while a long call runs.

use super::core_windows_fake;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clarity_runtime::program::{Program, Unconnected};
use clarity_runtime::ui::Text;
use clarity_ui::files::{ChosenFile, FileFilter, FilePicker};
use clarity_ui::markup;
use clarity_ui::parts::timer::TimerPart;
use clarity_ui::window::{ApartWindow, Running, WindowStack};
use clarity_ui::{AppFiles, Key, KeyInput, PartHost, Screen};
use core_windows_fake::*;
use serde_json::Map;

/// Closes the window on top as its `close` does.
fn close_top(screen: &mut Screen) {
    let host = stack(screen).current().unwrap().clone();
    host.run(None, &markup::handler_of("click", "close"));
    screen.act_root::<WindowStack, _>(|_, _| ());
}

#[test]
fn the_stack_ends_when_its_last_window_closes_and_not_before() {
    let (mut screen, fake) = started();
    click(&mut screen, "openConnection");
    assert_eq!(vec!["Main", "Connection"], stack(&screen).windows());
    fake.calls();
    close_top(&mut screen);
    assert!(!stack(&screen).ended());
    assert_eq!(vec!["Main"], stack(&screen).windows());
    // The one below is told once that it is on top again.
    assert_eq!(vec!["close Connection", "return Main"], fake.calls());
    close_top(&mut screen);
    assert!(stack(&screen).ended());
    assert_eq!(vec!["close Main"], fake.calls());
}

#[test]
fn the_systems_close_closes_the_dialogs_over_the_window_first_each_hearing_on_close() {
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let running = Running::new(program, FILES);
    let request = ApartWindow { window: "Main".into(), with: Map::new(), opener: 7, asked: None, dialog: false };
    let mut screen = Screen::new(Box::new(WindowStack::apart(running, request)), FILES.look(false));
    screen.resize(800.0, 500.0, 1.0);
    click(&mut screen, "openPick");
    assert_eq!(1, screen.overlays().iter().filter(|o| o.modal).count());
    fake.calls();
    screen.act_root::<WindowStack, _>(|stack, cx| stack.closing(cx));
    assert_eq!(vec!["close Pick", "return Main", "close Main"], fake.calls());
    assert!(!screen.overlays().iter().any(|o| o.modal));
    assert!(stack(&screen).ended());
    // The window that opened its desktop window is told.
    let closed = screen.act_root::<WindowStack, _>(|stack, _| stack.take_closed()).flatten().expect("its opener is told");
    assert_eq!(7, closed.opener);
}

// -- While the databases are not open ---------------------------------------------

fn unconnected(handler: Option<&str>, window: bool) -> Fake {
    let window = window.then(|| "Connection".to_string());
    Fake { unconnected: Some(Unconnected { handler: handler.map(str::to_string), window, problem: Text::plain("server down") }), ..Fake::default() }
}

#[test]
fn a_setting_changed_after_the_window_closed_does_not_count() {
    let (mut screen, fake) = started_with(unconnected(None, true));
    screen.act_root::<WindowStack, _>(|_, _| ());
    // The error said first, then the window closes unchanged.
    screen.key_pressed(KeyInput::plain(Key::Enter));
    close_top(&mut screen);
    assert!(stack(&screen).ended());
    fake.settings.lock().unwrap().insert("connection".into(), "kept".into());
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert!(stack(&screen).ended());
    assert!(!fake.calls().contains(&"connect".to_string()));
}

#[test]
fn still_not_connected_the_window_shows_again_saying_why() {
    let (mut screen, fake) = started_with(unconnected(None, true));
    screen.act_root::<WindowStack, _>(|_, _| ());
    screen.key_pressed(KeyInput::plain(Key::Enter));
    click(&mut screen, "keep");
    fake.calls();
    close_top(&mut screen);
    // The program could not connect: what keeps the connection shows again with its why.
    assert_eq!(vec!["close Connection", "connect", "open Connection"], fake.calls());
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!(vec!["Connection"], stack(&screen).windows());
    assert!(!stack(&screen).ended());
    assert_eq!("still no database", stack(&screen).said()[0].text.show());
}

#[test]
fn a_handler_that_opens_nothing_leaves_it_to_the_window_that_keeps_the_connection() {
    let (mut screen, fake) = started_with(unconnected(Some("nothing"), true));
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!(r#"answer nothing ["server down"]"#, fake.calls()[0]);
    assert_eq!(vec!["Connection"], stack(&screen).windows());
    assert_eq!(vec!["server down"], stack(&screen).said().iter().map(|m| m.text.show()).collect::<Vec<_>>());
}

#[test]
fn the_apps_handler_opens_what_shows_first_and_says_nothing_itself() {
    let (mut screen, _) = started_with(unconnected(Some("unconnected"), false));
    screen.act_root::<WindowStack, _>(|_, _| ());
    assert_eq!(vec!["Connection"], stack(&screen).windows());
    assert!(stack(&screen).said().is_empty());
}

#[test]
fn without_a_window_to_show_the_application_ends() {
    let (screen, _) = started_with(unconnected(None, false));
    assert!(stack(&screen).windows().is_empty());
    assert!(stack(&screen).ended());
}

// -- The host's file dialogs ------------------------------------------------------

struct Picker;

impl FilePicker for Picker {
    fn picks_folders(&self) -> bool {
        true
    }
    fn open_file(&self, _title: &str, _filter: Option<&FileFilter>) -> Option<ChosenFile> {
        None
    }
    fn save_file(&self, _title: &str, _name: &str, _bytes: &[u8]) -> Option<String> {
        None
    }
    fn choose_folder(&self, _title: &str) -> Option<String> {
        Some("/chosen".into())
    }
    fn show(&self, _name: &str, _bytes: &[u8]) {}
}

#[test]
fn the_hosts_file_dialogs_reach_every_windows_parts() {
    clarity_ui::files::set_system(Arc::new(Picker));
    let (mut screen, _) = started();
    click(&mut screen, "openConnection");
    assert!(stack(&screen).current().is_some());
    assert_eq!(Some("/chosen".to_string()), clarity_ui::files::system().choose_folder("Export"));
    clarity_ui::files::set_system(Arc::new(clarity_ui::files::NoFilePicker));
}

// -- A timer lives with its window ----------------------------------------------------

const TICKING: &str = r#"<Window name="Main">
  <Timer name="clock" at="1,1" ontick="tick" />
  <Button name="openConnection" at="1,2" span="3" caption="Connection" opens="Connection" />
  <Button name="long" at="4,2" span="3" caption="Long" onclick="long" />
</Window>"#;

const TICKING_FILES: AppFiles = AppFiles { windows: &[("Main", TICKING), ("Connection", CONNECTION)], ..FILES };

fn ticking(fake: Fake) -> (Screen, Arc<Fake>) {
    let fake = Arc::new(fake);
    let program: Arc<dyn Program> = fake.clone();
    let mut screen = Screen::new(Box::new(WindowStack::start(program, TICKING_FILES)), FILES.look(false));
    screen.resize(800.0, 500.0, 1.0);
    (screen, fake)
}

fn timer(screen: &Screen) -> Option<&TimerPart> {
    screen.find::<TimerPart>()
}

/// Time goes on until the timer's next tick, which is due then.
fn tick(screen: &mut Screen) {
    let at = timer(screen).and_then(TimerPart::next).unwrap_or_else(|| Instant::now() + Duration::from_secs(2));
    screen.now = Box::new(move || at);
    screen.wake_due(at);
}

#[test]
fn a_timer_stops_under_a_window_opened_in_place_and_starts_when_it_is_back() {
    let (mut screen, fake) = ticking(Fake::default());
    tick(&mut screen);
    assert_eq!(1, fake.calls().iter().filter(|c| *c == "dispatch tick").count());
    click(&mut screen, "openConnection");
    assert!(timer(&screen).is_none(), "its window is parked under the one on top");
    screen.wake_due(Instant::now() + Duration::from_secs(5));
    assert!(!fake.calls().contains(&"dispatch tick".to_string()));
    close_top(&mut screen);
    assert!(timer(&screen).unwrap().running());
    tick(&mut screen);
    assert_eq!(1, fake.calls().iter().filter(|c| *c == "dispatch tick").count());
}

#[test]
fn a_tick_while_a_long_call_runs_is_left_out() {
    let (mut screen, fake) = ticking(Fake { watches: true, ..Fake::default() });
    click(&mut screen, "long");
    let until = Instant::now() + Duration::from_secs(10);
    while !fake.long_reached.load(Ordering::SeqCst) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
    fake.calls();
    tick(&mut screen);
    assert!(fake.calls().is_empty(), "no handler runs while the long call does");
    let watch = fake.watch.lock().unwrap().clone().unwrap();
    watch.stop.store(true, Ordering::SeqCst);
    screen.finish_later();
    assert!(!stack(&screen).long_call());
    tick(&mut screen);
    assert_eq!(vec!["dispatch tick"], fake.calls());
}

#[test]
fn a_timer_has_no_place_in_the_window() {
    let (mut screen, fake) = ticking(Fake::default());
    let timer = timer(&screen).unwrap();
    assert!(!clarity_ui::Element::is_shown(timer));
    let b = clarity_ui::Element::node(timer).bounds;
    assert_eq!((0.0, 0.0), (b.width, b.height));
    fake.calls();
    // The button in its cell takes the click.
    click(&mut screen, "openConnection");
    assert_eq!(vec!["open Connection"], fake.calls());
}
