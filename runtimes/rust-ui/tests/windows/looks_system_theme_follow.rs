//! The system's light or dark choice followed by its change event, read
//! every so often only where there is none (C# HostsParityTests' system
//! theme rows), and the rows of C# SystemThemeTests the reading tests in
//! core_system_theme.rs leave: a program that fails is light, and the theme
//! shown is the theme's .light or .dark JSON over Clarity's own.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use clarity_ui::system_theme::{self, Heard, Listen, Listening, System};
use clarity_ui::{AppFiles, ThemeLook};

/// Listening as the test sees it: what it heard with, and whether it stopped.
#[derive(Default)]
struct Ears {
    heard: Mutex<Option<Heard>>,
    stopped: AtomicBool,
}

/// Stops listening when dropped.
struct Stop(Arc<Ears>);

impl Drop for Stop {
    fn drop(&mut self) {
        self.0.stopped.store(true, Ordering::SeqCst);
    }
}

fn listening(ears: &Arc<Ears>) -> Listen {
    let ears = ears.clone();
    Box::new(move |heard| {
        *ears.heard.lock().unwrap() = Some(heard);
        Ok(Some(Box::new(Stop(ears)) as Listening))
    })
}

#[test]
fn the_system_theme_is_followed_by_its_change_event() {
    let dark = Arc::new(AtomicBool::new(false));
    let asked = dark.clone();
    let ears = Arc::new(Ears::default());
    let (tell, changed) = mpsc::channel();
    // A long poll: only the event can tell it in time.
    let following = system_theme::follow(false, move || asked.load(Ordering::SeqCst), listening(&ears), Duration::from_secs(3600), move |d| {
        let _ = tell.send(d);
    });
    assert!(following.wait_started(Duration::from_secs(5)));
    assert!(following.by_event());
    dark.store(true, Ordering::SeqCst);
    let heard = ears.heard.lock().unwrap().clone().unwrap();
    heard();
    assert_eq!(Ok(true), changed.recv_timeout(Duration::from_secs(5)));
    // Heard again with nothing changed: nothing is told.
    heard();
    assert!(changed.recv_timeout(Duration::from_millis(200)).is_err());
    drop(following);
    assert!(ears.stopped.load(Ordering::SeqCst));
}

#[test]
fn without_a_change_event_the_system_theme_is_read_every_so_often() {
    let dark = Arc::new(AtomicBool::new(false));
    let asked = dark.clone();
    let (tell, changed) = mpsc::channel();
    let none: Listen = Box::new(|_| Ok(None));
    let following = system_theme::follow(false, move || asked.load(Ordering::SeqCst), none, Duration::from_millis(50), move |d| {
        let _ = tell.send(d);
    });
    assert!(following.wait_started(Duration::from_secs(5)));
    assert!(!following.by_event());
    dark.store(true, Ordering::SeqCst);
    assert_eq!(Ok(true), changed.recv_timeout(Duration::from_secs(5)));
}

#[test]
fn a_change_event_that_cannot_be_listened_to_leaves_reading_every_so_often() {
    let failing: Listen = Box::new(|_| Err("no bus".to_string()));
    let following = system_theme::follow(false, || false, failing, Duration::from_secs(60), |_| {});
    assert!(following.wait_started(Duration::from_secs(5)));
    assert!(!following.by_event());
}

#[test]
fn a_program_that_fails_is_light() {
    // A program that is missing or fails answers nothing.
    let failing = |_: &str, _: &[&str]| -> Option<String> { None };
    assert!(!system_theme::dark(System::Linux, None, &failing));
    assert!(!system_theme::dark(System::Windows, None, &failing));
    assert!(!system_theme::dark(System::Macos, None, &failing));
    assert!(!system_theme::dark(System::Other, None, &|_, _| Some("Dark".into())));
}

const NIGHT_OWL: AppFiles = AppFiles {
    themes: &[("night-owl.light", r##"{ "dark": false, "colors": { "accent": "#112233" } }"##), ("night-owl.dark", r##"{ "dark": true, "colors": { "accent": "#445566" } }"##)],
    windows: &[],
    apps: &[],
    model: "{}",
    resources: &[],
    title: "Owl",
    width: 0.0,
    height: 0.0,
};

#[test]
fn the_theme_shown_is_its_light_or_dark_json_over_clarity_s_own() {
    let light = NIGHT_OWL.look(false);
    let dark = NIGHT_OWL.look(true);
    assert_eq!(skia_safe::Color::from_rgb(0x11, 0x22, 0x33), light.color("accent"));
    assert_eq!(skia_safe::Color::from_rgb(0x44, 0x55, 0x66), dark.color("accent"));
    assert!(dark.dark());
    // What the dark one leaves is Clarity's own dark.
    assert_eq!(ThemeLook::clarity(true).color("background"), dark.color("background"));
    // Without a theme of the project, Clarity's own follows the system.
    assert!(AppFiles::default().look(true).dark());
    assert!(!AppFiles::default().look(false).dark());
    // A theme that cannot be read leaves Clarity's own.
    let unread = AppFiles { themes: &[("night-owl.light", "{ not json")], ..NIGHT_OWL };
    assert_eq!(ThemeLook::clarity(false).color("accent"), unread.look(false).color("accent"));
}
