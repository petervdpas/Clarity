//! The status item: a status bar's panel. It says its caption (as code set
//! it, else as written), with its icon, a gauge of the number it binds at
//! its right end (as full as a progress bar, conformance/small-parts.json),
//! a check mark in the theme's success color while code sets it checked,
//! and a warning in the theme's warning color while code sets an error on
//! it, the error then being its tooltip. A click runs its handler.

use std::rc::Rc;

use clarity_runtime::ui::Text;
use clarity_ui::parts::containers::small::StatusItemPart;
use clarity_ui::testing::{near, part_named, window_of, Shot, TestHost};
use clarity_ui::{Rect, Screen};
use serde_json::json;
use skia_safe::Color;

const ITEMS: &str = r#"<StatusItem at="1,1" span="4" name="progressItem" caption="@progressCaption" bind="done" max="200" onclick="progressClicked" />
<StatusItem at="5,1" span="4" name="reviewItem" caption="@reviewCaption" />"#;

fn screen(host: Rc<TestHost>) -> Screen {
    host.texts.borrow_mut().insert("progressCaption".into(), "Progress".into());
    host.texts.borrow_mut().insert("reviewCaption".into(), "Reviewed".into());
    let mut screen = window_of(ITEMS, host);
    screen.resize(808.0, 300.0, 1.0);
    screen
}

fn part<'a>(screen: &'a Screen, name: &str) -> &'a StatusItemPart {
    screen.element(part_named(screen, name)).and_then(|e| e.as_any().downcast_ref::<StatusItemPart>()).expect("a status item")
}

fn inked(shot: &mut Shot, place: Rect, color: Color) -> bool {
    (place.y as i32..place.bottom() as i32).any(|y| (place.x as i32..place.right() as i32).any(|x| near(shot.pixel(x, y), color, 30)))
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

#[test]
fn it_says_its_caption_and_what_code_set() {
    let host = TestHost::new();
    let screen = screen(host.clone());
    assert_eq!("Progress", part(&screen, "progressItem").says());
    host.set_state("progressItem", |s| s.caption = Some(Text::plain("Progress 60%")));
    assert_eq!("Progress 60%", part(&screen, "progressItem").says());
}

#[test]
fn its_gauge_is_as_full_as_the_number_it_binds_of_its_max() {
    let host = TestHost::new();
    host.set("done", json!(100));
    let mut screen = screen(host.clone());
    assert_eq!(Some(0.5), part(&screen, "progressItem").full());
    assert_eq!(None, part(&screen, "reviewItem").full(), "an item that binds nothing has no gauge");
    let gauge = part(&screen, "progressItem").places().gauge.expect("a gauge at its right end");
    let look = screen.look().clone();
    let mut shot = Shot::of(&mut screen, 808, 300, 1.0);
    let middle = (gauge.y + gauge.height / 2.0) as i32;
    assert_eq!(look.color("accent"), shot.pixel((gauge.x + gauge.width / 4.0) as i32, middle));
    assert_eq!(look.color("raised"), shot.pixel((gauge.x + gauge.width * 3.0 / 4.0) as i32, middle));
    keep(&mut shot, "status-item-gauge.png");
}

#[test]
fn checked_it_shows_a_check_mark_and_with_an_error_a_warning_that_is_its_tooltip() {
    let host = TestHost::new();
    let mut screen = screen(host.clone());
    let look = screen.look().clone();
    let mark = part(&screen, "reviewItem").places().mark;
    let mut shot = Shot::of(&mut screen, 808, 300, 1.0);
    assert!(!inked(&mut shot, mark, look.color("success")) && !inked(&mut shot, mark, look.color("warning")), "no mark to start");

    host.set_state("reviewItem", |s| s.checked = Some(true));
    assert!(part(&screen, "reviewItem").checked());
    let mut shot = Shot::of(&mut screen, 808, 300, 1.0);
    assert!(inked(&mut shot, mark, look.color("success")), "a check mark in the success color");
    keep(&mut shot, "status-item-checked.png");

    host.set_state("reviewItem", |s| s.error = Some(Text::plain("Changed since review")));
    assert_eq!(Some("Changed since review".to_string()), part(&screen, "reviewItem").warning());
    let mut shot = Shot::of(&mut screen, 808, 300, 1.0);
    assert!(inked(&mut shot, mark, look.color("warning")), "a warning outweighs the check");
    assert!(!inked(&mut shot, mark, look.color("success")));
    let b = screen.element(part_named(&screen, "reviewItem")).unwrap().node().bounds;
    assert_eq!(Some("Changed since review".to_string()), screen.element(part_named(&screen, "reviewItem")).unwrap().tooltip_at(b.x + 4.0, b.y + 4.0));
    keep(&mut shot, "status-item-warning.png");

    host.set_state("reviewItem", |s| s.error = Some(Text::plain("")));
    assert_eq!(None, part(&screen, "reviewItem").warning(), "an empty error clears the warning");
}

#[test]
fn a_click_runs_its_handler() {
    let host = TestHost::new();
    let mut screen = screen(host.clone());
    let _ = Shot::of(&mut screen, 808, 300, 1.0);
    let b = screen.element(part_named(&screen, "progressItem")).unwrap().node().bounds;
    screen.click(b.x + 10.0, b.y + b.height / 2.0);
    assert!(host.log.borrow().iter().any(|l| l.starts_with("run progressItem click")), "{:?}", host.log.borrow());
}
