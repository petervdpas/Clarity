//! Rows of the data area the check found untested (C# CheckFixDataTests): a
//! canvas, chart and display whose drawing fails saying why in place, the
//! bend handles of the link in hand, a vector canvas's dashed frame round the
//! chosen shape and the preview of the one being drawn, a theme colour of
//! two words, a relative link address the hosts refuse, and a gesture whose
//! pointer was taken away dropped instead of committed. (The chart's axis
//! lines are in data_chart.rs, a row dragged when the pointer is taken away
//! in data_tree.rs.) A canvas of 100 by 100 units fills 400 by 400 pixels of
//! an 800 by 400 screen, from 200 across: unit x, y is pixel 200 + 4x, 4y.

use super::canvas_host;

use std::rc::Rc;

use canvas_host::*;
use clarity_ui::parts::containers::markdown::MarkdownPart;
use clarity_ui::parts::data::canvas::Shape;
use clarity_ui::testing::{part_named, window_of, Shot};
use clarity_ui::window::address::{command_for, Os};
use clarity_ui::ThemeLook;
use serde_json::json;
use skia_safe::Color;

// -- Fail safe, per part ----------------------------------------------------------------

#[test]
fn a_canvas_whose_drawing_fails_says_why_in_its_place() {
    let (host, mut screen) = canvas("");
    host.draw("pad", vec![clear("panel"), json!({"op": "image", "x": 10, "y": 10, "width": 30, "height": 30, "picture": "sprite.png"})]);
    host.failing.set(true);
    let mut shot = shot(&mut screen);
    assert_eq!(Some("the picture store is gone".to_string()), the_canvas(&screen).failed());
    // Its red outline, where it is.
    let look = look();
    assert!((0..4).any(|y| toward(shot.pixel(400, y), look.color("error"), look.color("background"))));
}

#[test]
fn a_chart_and_a_display_whose_drawing_fails_say_why_and_the_window_draws_on() {
    let host = Rc::new(CanvasHost::default());
    host.inner.set("orders", json!([{"status": "open", "total": 1}]));
    host.inner.set("total", json!(12));
    let mut screen = window_of(
        r#"<Chart name="totals" at="1,1" span="4" rows="4" bind="orders" by="status" chart="bar" />
           <Display name="shown" at="5,1" span="4" rows="2" bind="total" digits="4" />
           <Label name="beside" at="9,1" span="4" caption="Still here" />"#,
        host.clone(),
    );
    screen.resize(800.0, 400.0, 1.0);
    Shot::of(&mut screen, 800, 400, 1.0);
    host.failing.set(true);
    host.inner.change();
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    let failed = |name: &str| screen.element(part_named(&screen, name)).and_then(|e| e.node().failed());
    assert_eq!(Some("the value is gone".to_string()), failed("totals"));
    assert_eq!(Some("the value is gone".to_string()), failed("shown"));
    // The label beside them still draws its words.
    let label = screen.element(part_named(&screen, "beside")).unwrap().node().bounds;
    let look = screen.look().clone();
    let drawn = (label.x as i32..label.right() as i32).any(|x| (label.y as i32..label.bottom() as i32).any(|y| toward(shot.pixel(x, y), look.color("text"), look.color("background"))));
    assert!(drawn);
}

// -- Bend handles ------------------------------------------------------------------

#[test]
fn the_bend_points_of_the_link_in_hand_are_drawn_as_handles() {
    let (host, mut screen) = canvas(r#"onbend="bent""#);
    host.draw("pad", vec![clear("panel"), link(json!([50, 30]), 0.5)]);
    let look = look();
    // The bend point at 50, 30 is pixel 400, 120.
    let mut before = shot(&mut screen);
    assert!(toward(before.pixel(400, 120), look.color("text"), look.color("panel")));
    assert!(!(402..408).any(|x| near(before.pixel(x, 120), look.color("accent"))));
    // The link in hand: a press on it.
    press(&mut screen, 200.0 + 4.0 * 30.0, 4.0 * 40.0);
    release(&mut screen, 200.0 + 4.0 * 30.0, 4.0 * 40.0);
    assert!(host.points.borrow().is_empty());
    let mut after = shot(&mut screen);
    // A handle: the panel's face over the line, ringed in the accent.
    assert_eq!(look.color("panel"), after.pixel(400, 120));
    assert!((402..408).any(|x| near(after.pixel(x, 120), look.color("accent"))));
}

// -- A vector canvas ---------------------------------------------------------------------

fn vector(shapes: String) -> (Rc<CanvasHost>, clarity_ui::Screen) {
    let (host, screen) = canvas(r#"mode="vector" bind="drawing""#);
    host.inner.set("drawing", json!(shapes));
    (host, screen)
}

#[test]
fn the_chosen_shape_shows_in_a_dashed_accent_frame() {
    let (host, mut screen) = vector(drawn(&[json!({"op": "rect", "x": 40, "y": 60, "width": 20, "height": 20, "color": "text"})]));
    let behind = {
        let mut none = shot(&mut screen);
        let behind = none.pixel(200 + 4 * 90, 4 * 95);
        assert_eq!(behind, none.pixel(400, 4 * 60 - 3));
        behind
    };
    clarity_ui::PartHost::select(&*host, "pad", Some(0));
    let mut chosen = shot(&mut screen);
    // The frame runs 3 pixels outside the shape, along its top edge, in dashes.
    let top: Vec<Color> = (4 * 40 + 200 - 3..4 * 40 + 200 - 3 + 4 * 20 + 6).map(|x| chosen.pixel(x, 4 * 60 - 3)).collect();
    assert!(top.iter().any(|p| near(*p, look().color("accent"))));
    assert!(top.iter().any(|p| *p == behind));
}

#[test]
fn the_shape_being_drawn_shows_before_it_is_added() {
    let (host, mut screen) = vector(String::new());
    host.inner.set_state("pad", |s| s.tool = Some("rect".into()));
    press(&mut screen, 200.0 + 4.0 * 30.0, 4.0 * 50.0);
    screen.pointer_moved(200.0 + 4.0 * 60.0, 4.0 * 80.0);
    assert_eq!(look().color("text"), shot(&mut screen).pixel(200 + 4 * 45, 4 * 65));
    assert!(host.actions.borrow().is_empty());
    release(&mut screen, 200.0 + 4.0 * 60.0, 4.0 * 80.0);
    assert_eq!(1, host.actions.borrow().len());
}

// -- Colours -------------------------------------------------------------------------------

/// A role of two words (lcd_text) is the theme's.
#[test]
fn a_colour_of_two_words_is_the_theme_s() {
    let (host, mut screen) = canvas("");
    screen.show(ThemeLook::over(r##"{"colors":{"lcd_text":"#00ff00"}}"##, look()));
    host.draw("pad", vec![rect(0.0, 0.0, 50.0, 50.0, "lcd_text"), rect(50.0, 0.0, 50.0, 50.0, "on_accent")]);
    let mut shot = shot(&mut screen);
    assert_eq!(Color::from_rgb(0, 0xff, 0), shot.pixel(200 + 4 * 25, 4 * 25));
    assert_eq!(look().color("on_accent"), shot.pixel(200 + 4 * 75, 4 * 25));
}

// -- A link's address ----------------------------------------------------------------------------

/// A relative address is no link in a Markdown text, and every host refuses one besides.
#[test]
fn a_relative_link_address_is_opened_by_no_host() {
    let host = Rc::new(CanvasHost::default());
    host.inner.set("notes", json!("Read [the guide](docs/guide.md) or [the site](https://example.org/guide)."));
    let mut screen = window_of(r#"<Markdown name="rules" at="1,1" span="6" rows="4" bind="notes" />"#, host);
    screen.resize(800.0, 400.0, 1.0);
    Shot::of(&mut screen, 800, 400, 1.0);
    let part = screen.find::<MarkdownPart>().expect("the window has its text");
    let mut links: Vec<String> = part.runs().into_iter().filter_map(|r| r.link).collect();
    links.dedup();
    assert_eq!(vec!["https://example.org/guide".to_string()], links);
    for os in [Os::Linux, Os::Windows, Os::MacOs] {
        for relative in ["docs/guide.md", "/about", "../up"] {
            assert_eq!(None, command_for(os, relative));
        }
    }
    assert!(command_for(Os::Linux, "https://example.org/guide").is_some());
}

// -- The pointer taken away ---------------------------------------------------------------------

#[test]
fn a_drag_whose_pointer_is_taken_away_is_dropped_not_committed() {
    let (host, mut screen) = canvas(r#"ondrag="dragged" ondrop="dropped""#);
    press(&mut screen, 240.0, 40.0);
    screen.pointer_moved(280.0, 80.0);
    assert_eq!(vec!["drag".to_string()], host.events());
    screen.pointer_cancelled();
    // The release that may still come is no drop either.
    release(&mut screen, 280.0, 80.0);
    assert_eq!(vec!["drag".to_string()], host.events());
}

#[test]
fn an_object_dragged_when_the_pointer_is_taken_away_stays_where_it_was() {
    let (host, mut screen) = canvas(r#"mode="objects""#);
    host.draw("pad", vec![object(json!("a"), 10.0, 10.0, 20.0, 20.0), rect(10.0, 10.0, 20.0, 20.0, "accent")]);
    press(&mut screen, 260.0, 60.0);
    screen.pointer_moved(300.0, 80.0);
    let x_of = |screen: &clarity_ui::Screen| match the_canvas(screen).shown()[1] {
        Shape::Rect { x, .. } => x,
        _ => f64::NAN,
    };
    assert_eq!(20.0, x_of(&screen));
    screen.pointer_cancelled();
    assert_eq!(10.0, x_of(&screen));
    release(&mut screen, 300.0, 80.0);
    assert!(host.actions.borrow().is_empty());
}

#[test]
fn a_shape_being_drawn_or_a_bend_is_dropped_when_the_pointer_is_taken_away() {
    let (host, mut screen) = vector(String::new());
    host.inner.set_state("pad", |s| s.tool = Some("rect".into()));
    press(&mut screen, 200.0 + 4.0 * 30.0, 4.0 * 50.0);
    screen.pointer_moved(200.0 + 4.0 * 60.0, 4.0 * 80.0);
    screen.pointer_cancelled();
    let mut after = shot(&mut screen);
    assert_eq!(after.pixel(200 + 4 * 90, 4 * 95), after.pixel(200 + 4 * 45, 4 * 65));
    assert!(host.actions.borrow().is_empty());

    let (bends, mut bend_screen) = canvas(r#"onbend="bent""#);
    bends.draw("pad", vec![link(json!([]), 1.0)]);
    let via_of = |screen: &clarity_ui::Screen| {
        the_canvas(screen).shown().into_iter().find_map(|s| if let Shape::Link { via, .. } = s { Some(via) } else { None }).unwrap()
    };
    press(&mut bend_screen, 200.0 + 4.0 * 50.0, 4.0 * 50.0);
    bend_screen.pointer_moved(200.0 + 4.0 * 50.0, 4.0 * 20.0);
    assert!(!via_of(&bend_screen).is_empty());
    bend_screen.pointer_cancelled();
    assert!(via_of(&bend_screen).is_empty());
    assert!(bends.points.borrow().is_empty());
}
