//! The trend (DESIGN 5.26), as on the web: the last values of one number,
//! taken every so many seconds on the screen's clock while it shows, drawn
//! as the line chart with the newest at the right.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use clarity_ui::parts::data::chart::MarkKind;
use clarity_ui::parts::data::trend::TrendPart;
use clarity_ui::testing::{window_of, Shot, TestHost};
use clarity_ui::{Element, Screen, ThemeLook};
use serde_json::json;

/// A window with a trend, its host set up by `setup` before the window is first laid out (when a trend takes its first value).
fn trend_with(extra: &str, setup: impl FnOnce(&TestHost)) -> (Screen, Rc<TestHost>) {
    let host = TestHost::new();
    setup(&host);
    let mut screen = window_of(&format!(r#"<Trend name="load" at="1,1" span="6" rows="4" bind="machine.load" {extra} />"#), host.clone());
    screen.resize(800.0, 400.0, 1.0);
    (screen, host)
}

fn trend(extra: &str) -> (Screen, Rc<TestHost>) {
    trend_with(extra, |_| {})
}

fn the_trend(screen: &mut Screen) -> &mut TrendPart {
    let id = screen.find::<TrendPart>().map(|t| t.node().id()).expect("the window has its trend");
    screen.element_mut(id).and_then(|e| e.as_any_mut().downcast_mut::<TrendPart>()).unwrap()
}

#[test]
fn it_keeps_the_last_values_it_took_that_are_numbers() {
    let (mut screen, host) = trend(r#"points="3""#);
    for value in [json!(1), json!("2.5"), json!(null), json!("x"), json!(4), json!(5)] {
        host.set("machine.load", value);
        the_trend(&mut screen).take();
    }
    assert_eq!(&[2.5, 4.0, 5.0], the_trend(&mut screen).history());
    assert_eq!(3, the_trend(&mut screen).points());
}

#[test]
fn it_takes_a_value_when_first_drawn_and_then_every_so_many_seconds() {
    let host = TestHost::new();
    host.set("machine.load", json!(10));
    let mut screen = window_of(r#"<Trend name="load" at="1,1" span="6" rows="4" bind="machine.load" every="0.5" max="100" />"#, host.clone());
    let start = Instant::now();
    let now = Rc::new(Cell::new(start));
    let clock = now.clone();
    screen.now = Box::new(move || clock.get());
    assert_eq!(0.5, the_trend(&mut screen).every());
    assert_eq!(Some(100.0), the_trend(&mut screen).max());
    Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!(&[10.0], the_trend(&mut screen).history());
    // It does not draw every frame: it wakes when the next value is due.
    assert_eq!(0, screen.animating());
    assert_eq!(Some(start + Duration::from_millis(500)), screen.next_wake());
    host.set("machine.load", json!(20));
    now.set(start + Duration::from_millis(300));
    Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!(&[10.0], the_trend(&mut screen).history());
    now.set(start + Duration::from_millis(500));
    screen.take_redraw();
    screen.wake_due(now.get());
    assert!(screen.take_redraw());
    Shot::of(&mut screen, 800, 400, 1.0);
    assert_eq!(&[10.0, 20.0], the_trend(&mut screen).history());
}

#[test]
fn it_keeps_sixty_values_a_second_apart_without_points_or_every() {
    let (mut screen, host) = trend("");
    assert_eq!(60, the_trend(&mut screen).points());
    assert_eq!(1.0, the_trend(&mut screen).every());
    for i in 0..70 {
        host.set("machine.load", json!(i));
        the_trend(&mut screen).take();
    }
    assert_eq!(60, the_trend(&mut screen).history().len());
    assert_eq!(Some(&69.0), the_trend(&mut screen).history().last());
}

#[test]
fn its_values_are_drawn_as_the_line_chart() {
    let (mut screen, _host) = trend_with("", |host| host.set("machine.load", json!(5)));
    the_trend(&mut screen).take();
    the_trend(&mut screen).take();
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    shot.save("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/data-trend-line.png").unwrap();
    let marks = the_trend(&mut screen).marks();
    assert_eq!(3, marks.len());
    assert!(marks.iter().all(|m| m.kind == MarkKind::Point));
    let (x, y) = marks[1].center();
    assert_eq!(ThemeLook::clarity(false).color("accent"), shot.pixel(x as i32, y as i32));
}

#[test]
fn a_hidden_trend_takes_nothing_and_wakes_no_more() {
    let (mut screen, _host) = trend_with("", |host| {
        host.set("machine.load", json!(5));
        host.set_state("load", |s| s.visible = Some(false));
    });
    Shot::of(&mut screen, 800, 400, 1.0);
    assert!(the_trend(&mut screen).history().is_empty());
    assert_eq!(None, screen.next_wake());
}
