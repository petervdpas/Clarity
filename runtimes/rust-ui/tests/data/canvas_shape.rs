//! What a canvas draws, shape by shape, and how its pointer, toolbar and
//! text tool behave (C# CanvasShapeTests). A canvas 100 by 100 units fills
//! 400 by 400 pixels of an 800 by 400 screen, from 200 across: unit x, y is
//! pixel 200 + 4x, 4y.

use super::canvas_host;

use canvas_host::*;
use clarity_ui::input::{Cursor, Key, KeyInput};
use clarity_ui::parts::data::canvas::Shape;
use clarity_ui::{Element, PointerButton};
use serde_json::json;
use skia_safe::Color;

#[test]
fn a_canvas_draws_an_ellipse_and_a_path_and_an_object_draws_nothing() {
    let (host, mut screen) = canvas("");
    host.draw(
        "pad",
        vec![
            clear("panel"),
            json!({"op": "ellipse", "x": 25, "y": 25, "rx": 20, "ry": 10, "color": "accent"}),
            json!({"op": "path", "points": [10, 70, 90, 70], "color": "error", "width": 4}),
            object(json!(1), 50.0, 0.0, 50.0, 50.0),
        ],
    );
    let mut shot = shot(&mut screen);
    let look = look();
    // Inside the ellipse across, not above it where only its box would be.
    assert_eq!(look.color("accent"), shot.pixel(200 + 4 * 25, 4 * 25));
    assert_eq!(look.color("accent"), shot.pixel(200 + 4 * 40, 4 * 25));
    assert_eq!(look.color("panel"), shot.pixel(200 + 4 * 25, 4 * 12));
    assert_eq!(look.color("error"), shot.pixel(200 + 4 * 50, 4 * 70));
    // The object's box shows the clear under it.
    assert_eq!(look.color("panel"), shot.pixel(200 + 4 * 75, 4 * 25));
    assert_eq!(None, the_canvas(&screen).failed());
}

#[test]
fn a_line_a_ring_and_words_are_drawn_in_their_colors() {
    let (host, mut screen) = canvas("");
    host.draw(
        "pad",
        vec![
            clear("panel"),
            json!({"op": "line", "x1": 10, "y1": 10, "x2": 90, "y2": 10, "color": "accent", "width": 3}),
            json!({"op": "ring", "x": 50, "y": 50, "r": 20, "color": "error", "width": 3}),
            json!({"op": "text", "x": 50, "y": 85, "text": "WWWW", "color": "text", "size": 12}),
        ],
    );
    let mut shot = shot(&mut screen);
    let look = look();
    assert_eq!(look.color("accent"), shot.pixel(200 + 4 * 50, 4 * 10));
    // The ring's line, not its middle.
    assert_eq!(look.color("error"), shot.pixel(200 + 4 * 70, 4 * 50));
    assert_eq!(look.color("panel"), shot.pixel(200 + 4 * 50, 4 * 50));
    // Somewhere along the words' middle is the text color.
    assert!((200 + 4 * 30..200 + 4 * 70).any(|x| shot.pixel(x, 4 * 85) == look.color("text")));
}

#[test]
fn a_canvas_clips_its_words_to_its_units_on_both_sides() {
    let (host, mut screen) = canvas("");
    host.draw(
        "pad",
        vec![
            clear("panel"),
            json!({"op": "text", "x": 98, "y": 50, "text": "WWWW", "color": "text", "size": 12}),
            json!({"op": "text", "x": 2, "y": 20, "text": "WWWW", "color": "text", "size": 12}),
        ],
    );
    let mut shot = shot(&mut screen);
    let text = look().color("text");
    // The units are pixels 200 to 600 across; the screen's margins beside them stay free of the words.
    assert!(!(600..800).any(|x| shot.pixel(x, 4 * 50) == text));
    assert!(!(0..200).any(|x| shot.pixel(x, 4 * 20) == text));
    // Inside the units the words are drawn.
    assert!((500..600).any(|x| shot.pixel(x, 4 * 50) == text));
}

#[test]
fn a_color_is_the_theme_s_by_name_or_written_out() {
    let (host, mut screen) = canvas("");
    host.draw("pad", vec![rect(0.0, 0.0, 50.0, 50.0, "#101810"), rect(50.0, 0.0, 50.0, 50.0, "no_such_color")]);
    let mut shot = shot(&mut screen);
    assert_eq!(Color::from_rgb(0x10, 0x18, 0x10), shot.pixel(200 + 4 * 25, 4 * 25));
    assert_eq!(look().color("text"), shot.pixel(200 + 4 * 75, 4 * 25));
}

#[test]
fn an_image_shape_draws_its_picture_stretched_to_its_box() {
    let host = std::rc::Rc::new(CanvasHost::default());
    host.inner.resources.borrow_mut().insert("dot.png".into(), png(Color::RED, 4));
    let (host, mut screen) = canvas_on("", host);
    host.draw(
        "pad",
        vec![
            clear("panel"),
            json!({"op": "image", "x": 10, "y": 10, "width": 30, "height": 30, "picture": "dot.png"}),
            json!({"op": "image", "x": 60, "y": 10, "width": 30, "height": 30, "picture": "missing.png"}),
        ],
    );
    let mut shot = shot(&mut screen);
    assert_eq!(Color::RED, shot.pixel(200 + 4 * 25, 4 * 25));
    // A picture it does not carry draws nothing.
    assert_eq!(look().color("panel"), shot.pixel(200 + 4 * 75, 4 * 25));
}

#[test]
fn on_a_canvas_that_zooms_the_clear_fills_the_whole_view() {
    let (host, mut screen) = canvas(r#"zoom="true""#);
    host.draw("pad", vec![clear("accent")]);
    let mut shot = shot(&mut screen);
    // Beside the units too: the sheet has no edges.
    assert_eq!(look().color("accent"), shot.pixel(50, 200));
}

// -- The pointer ---------------------------------------------------------------

#[test]
fn a_press_moved_less_than_four_pixels_is_still_a_click_and_one_let_go_outside_is_none() {
    let (host, mut screen) = canvas(r#"onclick="clicked" ondrag="dragged""#);
    press(&mut screen, 240.0, 40.0);
    screen.pointer_moved(241.0, 41.0);
    release(&mut screen, 241.0, 41.0);
    let points = host.points.borrow();
    assert_eq!(1, points.len());
    assert_eq!(("click", json!(10), json!(10)), (points[0].1.as_str(), points[0].2["x"].clone(), points[0].2["y"].clone()));
    // Let go off the part, above it: nothing.
    let (other, mut other_screen) = canvas(r#"onclick="clicked""#);
    press(&mut other_screen, 240.0, 40.0);
    release(&mut other_screen, 240.0, -10.0);
    assert!(other.points.borrow().is_empty());
}

#[test]
fn a_drag_without_a_drag_handler_pans_a_canvas_that_zooms() {
    let (host, mut screen) = canvas(r#"zoom="true" onclick="clicked""#);
    let whole = the_canvas(&screen).viewport();
    press(&mut screen, 400.0, 200.0);
    screen.pointer_moved(380.0, 180.0);
    release(&mut screen, 380.0, 180.0);
    let now = the_canvas(&screen).viewport();
    assert!((whole.x + 20.0 / whole.scale - now.x).abs() < 1e-6);
    assert!((whole.y + 20.0 / whole.scale - now.y).abs() < 1e-6);
    assert!(host.points.borrow().is_empty());
}

#[test]
fn a_double_click_on_a_bend_point_takes_it_away() {
    let (host, mut screen) = canvas(r#"onbend="bent""#);
    host.draw("pad", vec![link(json!([50, 30]), 1.0)]);
    // The link in hand first, then a double click on its point at 50, 30.
    press(&mut screen, 200.0 + 4.0 * 30.0, 4.0 * 40.0);
    release(&mut screen, 200.0 + 4.0 * 30.0, 4.0 * 40.0);
    screen.pointer_pressed(400.0, 120.0, PointerButton::Left, 2);
    release(&mut screen, 400.0, 120.0);
    let points = host.points.borrow();
    assert_eq!(1, points.len());
    assert_eq!(("bend", json!("a-b")), (points[0].1.as_str(), points[0].2["bent"].clone()));
    assert_eq!(json!([]), points[0].2["via"]);
}

#[test]
fn a_press_on_a_shape_drawn_over_a_link_drags_and_does_not_bend() {
    let (host, mut screen) = canvas(r#"onbend="bent" ondrag="dragged""#);
    host.draw("pad", vec![link(json!([]), 1.0), rect(40.0, 40.0, 20.0, 20.0, "panel")]);
    press(&mut screen, 400.0, 200.0);
    screen.pointer_moved(400.0, 120.0);
    release(&mut screen, 400.0, 120.0);
    assert_eq!(vec!["drag".to_string()], host.events());
}

#[test]
fn the_hand_shows_over_a_canvas_with_a_click_and_its_buttons() {
    let (_, screen) = canvas(r#"onclick="clicked""#);
    assert_eq!(Some(Cursor::Hand), the_canvas(&screen).cursor_at(400.0, 200.0));
    let (_, screen) = canvas(r#"zoom="true""#);
    let plain = the_canvas(&screen);
    assert_eq!(None, plain.cursor_at(400.0, 200.0));
    let (_, back, _) = plain.zoom_buttons();
    assert_eq!(Some(Cursor::Hand), plain.cursor_at(back.x + 5.0, back.y + 5.0));
}

// -- Modes ---------------------------------------------------------------------

#[test]
fn a_press_on_a_toolbar_button_picks_its_tool_and_tools_names_the_ones_shown() {
    let (host, mut screen) = canvas(r#"mode="vector" bind="drawing" tools="select, rect""#);
    host.inner.set("drawing", json!(""));
    assert_eq!(vec!["select", "rect"], the_canvas(&screen).tools_shown());
    let rect = the_canvas(&screen).tool_buttons(screen.fonts(), screen.look())[1].1;
    press(&mut screen, rect.x + 4.0, rect.y + 4.0);
    release(&mut screen, rect.x + 4.0, rect.y + 4.0);
    assert_eq!("rect", the_canvas(&screen).tool_in_hand());
    assert!(host.actions.borrow().is_empty());
    // The tool code names, when the canvas does not have it, is its first.
    host.inner.set_state("pad", |s| s.tool = Some("ellipse".into()));
    assert_eq!("select", the_canvas(&screen).tool_in_hand());
}

#[test]
fn a_readonly_canvas_shows_its_toolbar_disabled_and_a_press_picks_nothing() {
    let (host, mut screen) = canvas(r#"mode="vector" bind="drawing" readonly="true""#);
    host.inner.set("drawing", json!(""));
    let (other, mut editable) = canvas(r#"mode="vector" bind="drawing""#);
    other.inner.set("drawing", json!(""));
    let select = the_canvas(&screen).tool_buttons(screen.fonts(), screen.look())[0].1;
    let mut dimmed = shot(&mut screen);
    let mut normal = shot(&mut editable);
    let accent = look().color("accent");
    // The tool in hand shows in the accent only while the person may use it.
    let (x, y) = (select.x as i32 + 3, select.bottom() as i32 - 3);
    assert_eq!(accent, normal.pixel(x, y));
    assert_ne!(accent, dimmed.pixel(x, y));
    let line = the_canvas(&screen).tool_buttons(screen.fonts(), screen.look())[1].1;
    press(&mut screen, line.x + 4.0, line.y + 4.0);
    assert_eq!("select", the_canvas(&screen).tool_in_hand());
    assert_eq!(None, the_canvas(&screen).cursor_at(line.x + 4.0, line.y + 4.0));
    assert_eq!(Some(Cursor::Hand), the_canvas(&editable).cursor_at(line.x + 4.0, line.y + 4.0));
}

#[test]
fn a_press_beside_every_object_drags_nothing() {
    let (host, mut screen) = canvas(r#"mode="objects""#);
    host.draw("pad", vec![object(json!(7), 10.0, 10.0, 20.0, 20.0), rect(10.0, 10.0, 20.0, 20.0, "accent")]);
    press(&mut screen, 200.0 + 4.0 * 60.0, 4.0 * 60.0);
    screen.pointer_moved(200.0 + 4.0 * 70.0, 4.0 * 70.0);
    release(&mut screen, 200.0 + 4.0 * 70.0, 4.0 * 70.0);
    assert!(host.actions.borrow().is_empty());
}

#[test]
fn a_drag_goes_on_while_the_drawing_is_drawn_anew() {
    let (host, mut screen) = canvas(r#"mode="objects""#);
    host.draw("pad", vec![object(json!("a"), 10.0, 10.0, 20.0, 20.0), rect(10.0, 10.0, 20.0, 20.0, "accent")]);
    press(&mut screen, 260.0, 60.0);
    screen.pointer_moved(280.0, 60.0);
    // A timer's handler draws anew, another object first: the drag stays on "a".
    host.draw("pad", vec![object(json!("b"), 60.0, 60.0, 20.0, 20.0), object(json!("a"), 10.0, 10.0, 20.0, 20.0), rect(10.0, 10.0, 20.0, 20.0, "accent")]);
    screen.pointer_moved(300.0, 80.0);
    let shown = the_canvas(&screen).shown();
    assert!(matches!(shown[2], Shape::Rect { x, .. } if x == 20.0), "{shown:?}");
    release(&mut screen, 300.0, 80.0);
    assert_eq!(vec![json!({"move": {"id": "a", "x": 20, "y": 15}})], *host.actions.borrow());
}

#[test]
fn a_press_on_no_shape_with_select_chooses_none() {
    let (host, mut screen) = canvas(r#"mode="vector" bind="drawing""#);
    host.inner.set("drawing", json!(drawn(&[json!({"op": "rect", "x": 10, "y": 10, "width": 10, "height": 10, "color": "accent"})])));
    host.inner.set_state("pad", |s| s.tool = Some("select".into()));
    clarity_ui::PartHost::select(&*host, "pad", Some(0));
    press(&mut screen, 200.0 + 4.0 * 80.0, 4.0 * 80.0);
    release(&mut screen, 200.0 + 4.0 * 80.0, 4.0 * 80.0);
    assert_eq!(None, the_canvas(&screen).chosen());
    assert!(host.actions.borrow().is_empty());
}

// -- The text tool ---------------------------------------------------------------

fn typing() -> (std::rc::Rc<CanvasHost>, clarity_ui::Screen) {
    let (host, mut screen) = canvas(r#"mode="vector" bind="drawing""#);
    host.inner.set("drawing", json!(""));
    host.inner.set_state("pad", |s| s.tool = Some("text".into()));
    press(&mut screen, 240.0, 40.0);
    release(&mut screen, 240.0, 40.0);
    (host, screen)
}

#[test]
fn the_text_tool_types_in_a_text_input_whose_caret_moves() {
    let (host, mut screen) = typing();
    assert!(the_canvas(&screen).typing_editor().is_some());
    // The canvas has the keyboard and hands it to its text input.
    assert_eq!(Some(the_canvas(&screen).node().id()), screen.focused());
    screen.text_typed("Hlo");
    // Left twice and type between: a caret that moves, as in every text box.
    screen.key_pressed(KeyInput::plain(Key::Left));
    screen.key_pressed(KeyInput::plain(Key::Left));
    screen.text_typed("el");
    assert_eq!(Some("Hello".to_string()), the_canvas(&screen).typing());
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert!(the_canvas(&screen).typing_editor().is_none());
    let actions = host.actions.borrow();
    assert_eq!(1, actions.len());
    assert_eq!(json!("Hello"), actions[0]["add"]["text"]);
    // The canvas has the keyboard still, for Delete.
    assert_eq!(Some(the_canvas(&screen).node().id()), screen.focused());
}

#[test]
fn escape_drops_the_text_being_typed() {
    let (host, mut screen) = typing();
    screen.text_typed("Gone");
    screen.key_pressed(KeyInput::plain(Key::Escape));
    assert_eq!(None, the_canvas(&screen).typing());
    assert!(host.actions.borrow().is_empty());
}
