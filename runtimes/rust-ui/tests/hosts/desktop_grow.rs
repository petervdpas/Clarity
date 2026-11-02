//! A window that names its size and opens in place in the desktop window
//! grows that window to its size, as far as the screen has room, and never
//! makes it smaller than the person left it. A sidebar that opens grows it by
//! the room it takes, so the window's own parts keep theirs (C#
//! DesktopGrowTests).

#![cfg(feature = "desktop")]

#[allow(dead_code)]
use super::core_frame_shared as shared;

use clarity_ui::desktop::grow::{grown, sidebar_grown, sidebars_width};
use clarity_ui::frame::rules::STRIP_WIDTH;
use clarity_ui::window::WindowView;
use shared::*;

#[test]
fn a_larger_window_grows_the_desktop_window() {
    assert_eq!((1100.0, 680.0), grown((900.0, 560.0), Some((1100.0, 680.0)), Some((1920.0, 1080.0))));
}

#[test]
fn it_never_shrinks_and_stays_on_the_screen() {
    assert_eq!((1000.0, 900.0), grown((1000.0, 900.0), Some((800.0, 600.0)), Some((1920.0, 1080.0))));
    assert_eq!((1280.0, 720.0), grown((900.0, 560.0), Some((1600.0, 1000.0)), Some((1280.0, 720.0))));
    // Without a size of its own, or with a screen of unknown size, nothing changes beyond what it asks.
    assert_eq!((900.0, 560.0), grown((900.0, 560.0), None, Some((1920.0, 1080.0))));
    assert_eq!((1100.0, 680.0), grown((900.0, 560.0), Some((1100.0, 680.0)), None));
}

#[test]
fn a_sidebar_that_opens_grows_the_window_by_its_room() {
    // Noughts: 405 wide with its sidebar folded to its strip (28), opened to 320.
    assert_eq!((697.0, 510.0), sidebar_grown((405.0, 510.0), 28.0, 320.0, Some((1920.0, 1080.0))));
    // Folding it again leaves the window as it is; so does the same width.
    assert_eq!((697.0, 510.0), sidebar_grown((697.0, 510.0), 320.0, 28.0, Some((1920.0, 1080.0))));
    assert_eq!((697.0, 510.0), sidebar_grown((697.0, 510.0), 320.0, 320.0, Some((1920.0, 1080.0))));
    // At most as wide as the screen.
    assert_eq!((1000.0, 510.0), sidebar_grown((900.0, 510.0), 28.0, 320.0, Some((1000.0, 1080.0))));
}

#[test]
fn a_window_view_says_how_wide_its_sidebars_are() {
    let (mut screen, host) = window(r#"<Sidebar side="left" name="help" width="320" collapsible="true" collapsed="true"><Text name="rules" at="1,1" span="12" /></Sidebar><Button name="go" at="1,1" span="12" />"#, None);
    // As the generated view model starts it: folded.
    host.set_state("help", |s| s.collapsed = Some(true));
    draw(&mut screen);
    assert_eq!(STRIP_WIDTH, sidebars_width(view(&screen)));
    screen.act_root::<WindowView, _>(|v, cx| v.sidebar_mut(true).unwrap().fold(false, cx));
    assert_eq!(320.0, sidebars_width(view(&screen)));
}
