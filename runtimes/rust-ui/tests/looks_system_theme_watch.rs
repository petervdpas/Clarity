//! The system's light or dark followed while the program runs, where the
//! window system says nothing of a change (winit on X11): asked again every
//! few seconds off the screen's thread, as C#'s SystemTheme.Watch does; a
//! change is told once, and dropping the watch stops it.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use clarity_ui::system_theme;

#[test]
fn a_change_of_the_system_s_dark_is_told_once() {
    let dark = Arc::new(AtomicBool::new(false));
    let asked = dark.clone();
    let (tell, heard) = mpsc::channel();
    let _watch = system_theme::watch_with(false, Duration::from_millis(10), move || asked.load(Ordering::SeqCst), move |d| {
        let _ = tell.send(d);
    });
    assert!(heard.recv_timeout(Duration::from_millis(100)).is_err(), "nothing changed, nothing is told");
    dark.store(true, Ordering::SeqCst);
    assert_eq!(Ok(true), heard.recv_timeout(Duration::from_secs(2)));
    assert!(heard.recv_timeout(Duration::from_millis(100)).is_err(), "told once");
    dark.store(false, Ordering::SeqCst);
    assert_eq!(Ok(false), heard.recv_timeout(Duration::from_secs(2)));
}

#[test]
fn dropping_the_watch_stops_it() {
    let asks = Arc::new(AtomicUsize::new(0));
    let counted = asks.clone();
    let watch = system_theme::watch_with(false, Duration::from_millis(10), move || {
        counted.fetch_add(1, Ordering::SeqCst);
        false
    }, |_| {});
    std::thread::sleep(Duration::from_millis(60));
    drop(watch);
    std::thread::sleep(Duration::from_millis(30));
    let after = asks.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(60));
    assert_eq!(after, asks.load(Ordering::SeqCst));
}
