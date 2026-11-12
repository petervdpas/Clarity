//! An application's splash screen:
//! the page the app names shows on its own while `on start` runs elsewhere,
//! says what loads in its part `status`, and the main window follows with
//! what `on start` answered. As in Avalonia, the markup says it all: the
//! app names its splash (`<App splash="Welcome" splashFor="1.5">`) and the
//! splash window says how it shows (borderless, centred, on top, its size).

use super::core_windows_fake;

use std::sync::Arc;
use std::time::Duration;

use clarity_runtime::program::Program;
use clarity_runtime::ui::{Level, Message, Text};
use clarity_ui::app_files::Splash;
use clarity_ui::window::WindowStack;
use clarity_ui::{AppFiles, PartHost};
use core_windows_fake::*;

const WELCOME: &str = r#"<Window name="Welcome" width="480" height="280" decorations="none" startup="centerScreen" resizable="false" topmost="true" taskbar="false" frame="none">
  <Label name="status" at="1,1" span="6" caption="Starting" />
</Window>"#;

const APP: &str = r#"<App name="Shop" splash="Welcome" splashFor="1.5"><Menu name="appMenu"><Item caption="App" onclick="fromApp" /></Menu></App>"#;

const SPLASHED: AppFiles = AppFiles { windows: &[("Main", MAIN), ("Welcome", WELCOME)], apps: &[("Shop", APP)], ..FILES };

#[test]
fn the_files_say_which_page_is_the_splash_and_how_long_it_shows() {
    assert_eq!(SPLASHED.splash(), Some(Splash { page: "Welcome".into(), at_least: Duration::from_millis(1500) }));
    let quick = AppFiles { apps: &[("Shop", r#"<App name="Shop" splash="Welcome"></App>"#)], ..SPLASHED };
    assert_eq!(quick.splash().map(|s| s.at_least), Some(Duration::ZERO), "without a time it closes once the main window shows");
    assert_eq!(FILES.splash(), None, "a splash is optional");
    let uncarried = AppFiles { windows: &[("Main", MAIN)], ..SPLASHED };
    assert_eq!(uncarried.splash(), None, "a page the program does not carry is no splash");
}

#[test]
fn the_splash_shows_alone_and_says_what_loads() {
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let running = WindowStack::prepare(program, SPLASHED);
    let splash = WindowStack::splash(running, "Welcome");
    assert_eq!(splash.windows(), vec!["Welcome".to_string()]);
    assert_eq!(fake.calls(), vec!["open Welcome".to_string()], "the app's on start is not run by the splash");
    splash.show_status(Text::plain("Loading the catalogue"));
    let host = splash.current().unwrap();
    assert_eq!(host.state(Some("status")).and_then(|s| s.caption), Some(Text::plain("Loading the catalogue")));
}

#[test]
fn the_main_window_follows_with_what_on_start_said() {
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let running = WindowStack::prepare(program.clone(), SPLASHED);
    // As the host runs it, off the screen's thread while the splash shows.
    let mut reply = program.start_app();
    if let Ok(reply) = reply.as_mut() {
        reply.messages.push(Message { level: Level::Info, text: Text::plain("Catalogue loaded") });
    }
    let main = WindowStack::started(running, reply);
    assert_eq!(main.windows(), vec!["Main".to_string()]);
    assert_eq!(fake.calls(), vec!["start".to_string(), "open Main".to_string()]);
}

/// In the browser the splash page fills the page itself: the same screen
/// shows it until `on start` answered, then the main window takes its place.
#[test]
fn in_place_the_main_window_takes_the_splash_s_place() {
    let fake = Arc::new(Fake::default());
    let program: Arc<dyn Program> = fake.clone();
    let quick = AppFiles { apps: &[("Shop", r#"<App name="Shop" splash="Welcome"></App>"#)], ..SPLASHED };
    let mut driven = clarity_ui::headless::Driven::start_splashed(program, quick, None);
    let mut shown = Vec::new();
    for _ in 0..200 {
        let windows = driven.screen().root().as_any().downcast_ref::<WindowStack>().unwrap().windows();
        if shown.last() != Some(&windows) {
            shown.push(windows.clone());
        }
        if windows == vec!["Main".to_string()] {
            break;
        }
        driven.step(&serde_json::json!({ "do": "settle", "ms": 20 }));
    }
    assert_eq!(shown, vec![vec!["Welcome".to_string()], vec!["Main".to_string()]]);
    let calls = fake.calls();
    assert_eq!(calls.iter().filter(|c| *c == "start").count(), 1, "{calls:?}");
}

/// The splash window says how it shows, as an Avalonia window does in its
/// XAML, and opens at the size it gives, not the app's running size.
#[test]
fn the_splash_window_says_how_it_shows() {
    let look = SPLASHED.window_look("Welcome");
    assert_eq!(look, clarity_ui::markup::WindowLook { size: Some((480.0, 280.0)), decorations: false, centre: true, resizable: false, topmost: true, taskbar: false, app_frame: false });
    assert_eq!(SPLASHED.window_look("Main"), clarity_ui::markup::WindowLook::default(), "a window that says nothing is the system's own");
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    let running = WindowStack::prepare(program, SPLASHED);
    assert_eq!(WindowStack::splash(running, "Welcome").wanted_size(), Some((480.0, 280.0)));
}

/// A splash shows its own parts only, never the app's frame (its menu, ribbon or footer), as a dialog does.
#[test]
fn the_splash_shows_no_frame_of_the_app() {
    assert!(clarity_ui::frame::app_frame_of(&SPLASHED, "Main").is_some(), "the main window shows the app's frame");
    assert!(clarity_ui::frame::app_frame_of(&SPLASHED, "Welcome").is_none(), "the splash does not: its own markup says frame=none");
    let unnamed = AppFiles { apps: &[("Shop", r#"<App name="Shop"><Menu name="appMenu"><Item caption="App" onclick="fromApp" /></Menu></App>"#)], ..SPLASHED };
    assert!(clarity_ui::frame::app_frame_of(&unnamed, "Welcome").is_none(), "whatever the app's markup says");
}

/// What code says loads (`splash.status`) shows on the splash while it does.
#[test]
fn the_splash_shows_what_code_says_loads() {
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    clarity_runtime::splash::set_status(Text::plain("Loading the lists"));
    let mut driven = clarity_ui::headless::Driven::start_splashed(program, SPLASHED, None);
    driven.step(&serde_json::json!({ "do": "settle", "ms": 400 }));
    let stack = driven.screen().root().as_any().downcast_ref::<WindowStack>().unwrap();
    assert_eq!(stack.windows(), vec!["Welcome".to_string()], "the splash shows its 1.5 seconds");
    let status = stack.current().unwrap().state(Some("status")).and_then(|s| s.caption);
    assert_eq!(status, Some(Text::plain("Loading the lists")));
}

/// A page's own exact grid (`cell="10"` on its markup): square cells of 10
/// pixels on a 560 px window, without gaps or padding, rows that keep their
/// height even beside a picture, so one step left is as long as one step down.
#[test]
fn an_exact_grid_lays_parts_on_square_cells() {
    const FINE: &str = r#"<Window name="Welcome" cell="10" columns="56" frame="none">
  <Image name="logo" at="4,3" span="16" rows="16" source="logo.png" />
  <Heading name="titleHeading" at="24,5" span="30" rows="4" caption="Todo" />
</Window>"#;
    let files = AppFiles { windows: &[("Main", MAIN), ("Welcome", FINE)], ..SPLASHED };
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    let running = WindowStack::prepare(program, files);
    let stack = WindowStack::splash(running, "Welcome");
    let mut screen = clarity_ui::Screen::new(Box::new(stack), files.look(false));
    screen.resize(560.0, 280.0, 1.0);
    let driven = clarity_ui::headless::Driven::over(screen);
    let layout = driven.layout();
    let at = |name: &str| -> Vec<f64> {
        let part = layout["parts"].as_array().unwrap().iter().find(|p| p["name"] == name).unwrap_or_else(|| panic!("{name}: {layout:#?}"));
        part["at"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap().round()).collect()
    };
    assert_eq!(at("logo"), vec![30.0, 20.0, 160.0, 160.0], "column 4 is 30 px in, row 3 is 20 px down, 16 cells are 160 px");
    assert_eq!(at("titleHeading"), vec![230.0, 40.0, 300.0, 40.0]);
}

/// A window without a title of its own shows the app's title, as the app's
/// markup says it (`<App title="@todoTitle">`), its key in the person's
/// language; a window with its own shows that; without either, the project's name.
#[test]
fn a_window_without_its_own_title_shows_the_app_s() {
    let program: Arc<dyn Program> = Arc::new(Fake::default());
    let titled = AppFiles { apps: &[("Shop", r#"<App name="Shop" title="Shop for all"></App>"#)], model: r#"{"project":{"files":[{"pages":[{"name":{"name":"Main"},"kind":"window"},{"name":{"name":"Own"},"kind":"window","title":"Its own"}]}]}}"#, ..FILES };
    let running = WindowStack::prepare(program.clone(), titled);
    assert_eq!(running.title_of("Main"), "Shop for all");
    assert_eq!(running.title_of("Own"), "Its own");
    let untitled = AppFiles { apps: &[("Shop", r#"<App name="Shop"></App>"#)], ..titled };
    let running = WindowStack::prepare(program, untitled);
    assert_eq!(running.title_of("Main"), FILES.title, "the project's name");
}
