//! What the desktop host does with the system beyond the screen: whatever
//! fails on its loop is said once and the program goes on; a window draws at
//! the system's real content scale; OpenGL 3.3 core first, then older
//! contexts and drawing in software; file dialogs belong to the window,
//! which takes no input meanwhile; what a dialog says as it closes shows
//! below; the pointer's shape, the wheel's notch and the window's icon (C#
//! DesktopPlatformTests, and the icon of CheckFixHostsTests).

#![cfg(feature = "desktop")]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use clarity_runtime::ui::{Level, Message, Text};
use clarity_ui::desktop::fail_safe::{FailSafe, QUIET};
use clarity_ui::desktop::files::{commands_for, Ask, Kind, Os, SystemFilePicker, SystemWindow};
use clarity_ui::desktop::window_system::{first_context, icon, scale_of, software_asked, CONTEXTS, ICON_SIZES};
use clarity_ui::desktop::{blocks_input, hand_down, standard_of, WHEEL_NOTCH};
use clarity_ui::files::FilePicker;
use clarity_ui::input::Cursor;
use winit::window::CursorIcon;

#[test]
fn a_failure_on_the_loop_is_said_once_while_it_repeats_and_the_program_goes_on() {
    let said = Rc::new(RefCell::new(Vec::<String>::new()));
    let kept = said.clone();
    let mut safe = FailSafe::new(move |what| kept.borrow_mut().push(what.to_string()));
    // The panics are the test's own: their default report on the error output is left out.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    assert_eq!(None, safe.run(1000, || panic!("the chart could not draw")));
    // The same failure every frame: said once.
    assert_eq!(None, safe.run(1016, || panic!("the chart could not draw")));
    assert_eq!(None, safe.run(1032, || panic!("the chart could not draw")));
    // Another one is said; the first again once its quiet time is over.
    assert_eq!(None, safe.run(1048, || panic!("{}", "a layout failed")));
    assert_eq!(None, safe.run(1048 + QUIET, || panic!("the chart could not draw")));
    std::panic::set_hook(hook);
    assert_eq!(vec!["the chart could not draw", "a layout failed", "the chart could not draw"], *said.borrow());
    assert_eq!(Some(7), safe.run(9000, || 7));
}

#[test]
fn a_window_draws_at_the_system_s_content_scale() {
    // (window, framebuffer, content scale) -> (scale, window units per screen unit)
    let cases = [
        // Windows and X11 at 100 percent: the window is in pixels, the scale 1.
        ((1100, 760), (1100, 760), 1.0, 1.0, 1.0),
        // X11 or Windows at 150 percent: the framebuffer is the window, the content scale says how large a unit is.
        ((1650, 1140), (1650, 1140), 1.5, 1.5, 1.5),
        ((2200, 1520), (2200, 1520), 2.0, 2.0, 2.0),
        // macOS Retina and Wayland: the system scales the window, the framebuffer is larger.
        ((1100, 760), (2200, 1520), 2.0, 2.0, 1.0),
        // A system that gives no content scale: the framebuffer's ratio.
        ((1100, 760), (2200, 1520), 0.0, 2.0, 1.0),
        ((1100, 760), (1100, 760), 0.0, 1.0, 1.0),
    ];
    for (size, pixels, content, scale, per_unit) in cases {
        let (got_scale, got_per_unit) = scale_of(size, pixels, content);
        assert!((got_scale - scale).abs() < 0.001, "{size:?} {pixels:?} {content}: scale {got_scale}");
        assert!((got_per_unit - per_unit).abs() < 0.001, "{size:?} {pixels:?} {content}: per unit {got_per_unit}");
        // The screen's units: the same window on every system.
        assert!((size.0 as f64 / got_per_unit - 1100.0).abs() < 0.001);
    }
}

#[test]
fn a_window_asks_for_opengl_3_3_core_then_older_contexts_and_can_draw_in_software() {
    assert!(CONTEXTS[0].core);
    assert_eq!((3, 3), (CONTEXTS[0].major, CONTEXTS[0].minor));
    // Down to Windows' own OpenGL 1.1, as over Remote Desktop.
    assert_eq!((1, 1), (CONTEXTS[2].major, CONTEXTS[2].minor));
    assert!(CONTEXTS[1..].iter().all(|c| !c.core));
    assert!(software_asked(|n| (n == "CLARITY_RENDER").then(|| "software".to_string())));
    assert!(software_asked(|n| (n == "CLARITY_RENDER").then(|| " Software ".to_string())));
    assert!(!software_asked(|_| None));
    assert!(!software_asked(|n| (n == "CLARITY_RENDER").then(|| "gpu".to_string())));
    // Drawing in software puts pixels with glDrawPixels, which only a compatibility context has.
    assert_eq!((0, 1, 2), (first_context(false, 0), first_context(true, 0), first_context(true, 2)));
}

#[test]
fn a_file_dialog_belongs_to_the_window_it_was_asked_from() {
    let owner = SystemWindow { x11: 0x3a00007, win32: 0 };
    let ask = Ask { kind: Kind::Open, title: "Choose a file".into(), name: None, filter: None };
    let commands = commands_for(Os::Linux, &ask, Some(owner));
    assert_eq!(vec!["--file-selection", "--title=Choose a file", "--modal", "--attach=60817415"], commands[0].args);
    assert_eq!(vec!["--title", "Choose a file", "--attach", "60817415"], commands[1].args[..4]);
    // The portal names its parent window as x11: and the number in hex.
    assert_eq!("x11:3a00007", owner.portal());
    assert_eq!("", SystemWindow::default().portal());
    // Without a window the dialog belongs to none.
    assert!(!commands_for(Os::Linux, &ask, None)[0].args.contains(&"--modal".to_string()));
}

#[test]
fn while_a_file_dialog_is_open_the_windows_take_no_input() {
    let (release, released) = mpsc::channel::<()>();
    let released = Mutex::new(released);
    let asked: Arc<Mutex<Option<SystemWindow>>> = Arc::default();
    let kept = asked.clone();
    let picker = SystemFilePicker::with(
        Box::new(move |_, owner| {
            *kept.lock().unwrap() = owner;
            let _ = released.lock().unwrap().recv();
            None
        }),
        Box::new(|_| {}),
    );
    picker.set_owner(Some(SystemWindow { x11: 7, win32: 0 }));
    assert!(!picker.showing());
    std::thread::scope(|scope| {
        let choosing = scope.spawn(|| picker.choose_folder("Choose a folder"));
        let until = Instant::now() + Duration::from_secs(5);
        while !picker.showing() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(picker.showing());
        assert!(blocks_input(false, &picker));
        release.send(()).unwrap();
        assert_eq!(None, choosing.join().unwrap());
    });
    assert!(!picker.showing());
    assert!(!blocks_input(false, &picker));
    // A dialog of the window's own blocks it too.
    assert!(blocks_input(true, &picker));
    assert_eq!(Some(7), asked.lock().unwrap().map(|o| o.x11));
}

#[test]
fn what_a_dialog_says_as_it_closes_shows_in_the_window_below() {
    let message = |level, text: &str| Message { level, text: Text::plain(text), toast: None };
    let mut dialog = vec![message(Level::Info, "Kept the note"), message(Level::Warning, "Check the date")];
    let mut below = vec![message(Level::Info, "Earlier")];
    hand_down(&mut dialog, &mut below);
    assert_eq!(vec!["Earlier", "Kept the note", "Check the date"], below.iter().map(|m| m.text.show()).collect::<Vec<_>>());
    assert!(dialog.is_empty());
}

#[test]
fn the_pointer_takes_the_system_s_shape_of_what_is_under_it() {
    assert_eq!(CursorIcon::Pointer, standard_of(Cursor::Hand));
    assert_eq!(CursorIcon::Text, standard_of(Cursor::Text));
    assert_eq!(CursorIcon::EwResize, standard_of(Cursor::ResizeLeftRight));
    assert_eq!(CursorIcon::NsResize, standard_of(Cursor::ResizeUpDown));
    assert_eq!(CursorIcon::NwseResize, standard_of(Cursor::ResizeCorner));
    assert_eq!(CursorIcon::Move, standard_of(Cursor::Move));
    assert_eq!(CursorIcon::Crosshair, standard_of(Cursor::Crosshair));
    assert_eq!(CursorIcon::NotAllowed, standard_of(Cursor::NotAllowed));
    assert_eq!(CursorIcon::Default, standard_of(Cursor::Default));
}

#[test]
fn a_notch_of_the_wheel_is_fifty() {
    assert_eq!(50.0, WHEEL_NOTCH);
}

#[test]
fn a_window_shows_the_application_s_icon_or_clarity_s() {
    // No icon of the application: Clarity's own, at every size the system picks from.
    let own = icon(None);
    assert_eq!(ICON_SIZES.to_vec(), own.iter().map(|i| i.size).collect::<Vec<_>>());
    assert!(own.iter().all(|i| i.rgba.len() == (i.size * i.size * 4) as usize));
    // Its pixels are drawn, not empty.
    assert!(own.iter().all(|i| i.rgba.chunks(4).any(|p| p[3] > 0)));
    // Data that is no picture leaves Clarity's.
    assert_eq!(own, icon(Some(b"not a png")));
    // A small picture of the application's: no size larger than it, 16 at least.
    let mut surface = skia_safe::surfaces::raster_n32_premul((32, 32)).unwrap();
    surface.canvas().clear(skia_safe::Color::from_rgb(200, 30, 30));
    let png = surface.image_snapshot().encode(None, skia_safe::EncodedImageFormat::PNG, 100).unwrap();
    let app = icon(Some(png.as_bytes()));
    assert_eq!(vec![16, 32], app.iter().map(|i| i.size).collect::<Vec<_>>());
    assert_eq!(&[200, 30, 30, 255], &app[1].rgba[0..4]);
    // An SVG icon is drawn sharp at every size the system picks from.
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><rect width="24" height="24" fill="#1e64c8"/></svg>"##;
    let drawn = icon(Some(svg));
    assert_eq!(ICON_SIZES.to_vec(), drawn.iter().map(|i| i.size).collect::<Vec<_>>());
    assert_eq!(&[30, 100, 200, 255], &drawn[4].rgba[0..4]);
}

#[test]
fn a_picker_is_the_parts_file_dialogs() {
    let picker = SystemFilePicker::with(Box::new(|_, _| None), Box::new(|_| {}));
    assert!(picker.picks_folders());
}

/// The splash screen lies in the middle of the screen and finishes once
/// `on start` answered, but not sooner than the app asks; then the main window shows.
#[test]
fn the_splash_lies_in_the_middle_and_stays_its_time() {
    use clarity_ui::splash::{centred, closes, next_look, STATUS_EVERY};
    use std::time::Duration;
    assert_eq!(centred((0, 0), (1920, 1080), (480, 280)), (720, 400));
    assert_eq!(centred((1920, 0), (1280, 1024), (480, 280)), (2320, 372), "on the second screen");
    assert_eq!(centred((0, 0), (400, 200), (480, 280)), (0, 0), "a screen smaller than the splash shows its corner");
    let two = Duration::from_secs(2);
    assert!(!closes(false, Duration::from_secs(5), two), "not before on start answered");
    assert!(!closes(true, Duration::from_millis(500), two), "not before its time");
    assert!(closes(true, two, two));
    assert!(closes(true, Duration::from_millis(10), Duration::ZERO), "without a time it finishes once on start answered");
    assert_eq!(next_look(Duration::from_millis(500), two, false), STATUS_EVERY);
    assert_eq!(next_look(Duration::from_millis(500), two, true), STATUS_EVERY, "the status is still asked for while it shows");
    assert_eq!(next_look(Duration::from_millis(1950), two, true), Duration::from_millis(50), "and it is looked at when its time is up");
}
