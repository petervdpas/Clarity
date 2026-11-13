//! A slide-out (conformance/slideouts.json): it slides in over the window
//! from its edge while its state is visible, as high as the window's content,
//! and what is under it stays put; pinned, the window's parts make room for
//! it and a press beside it leaves it open. Escape, a press beside it and its
//! close button close it, and its state hears so. Its inner edge is dragged
//! between its min and max, and its width and pin are kept per window. The
//! focus moves into it and back, it runs `on open` and `on close`, and a
//! screen reader finds it as a dialog named by its caption.

use std::time::{Duration, Instant};

use clarity_ui::access;
use clarity_ui::choices;
use clarity_ui::element::{find_type, ElementId};
use clarity_ui::frame::rules;
use clarity_ui::parts::containers::slideout::{SlideoutButton, SlideoutFrame, SlideoutHandle, SlideoutPart, HANDLE_WIDTH, SLIDE_TIME};
use clarity_ui::testing::part_name;
use clarity_ui::{Element, Key, Modifiers, PointerButton, Rect, Screen};

use crate::core_frame_shared::{click, draw, press, view, window, window_named, FrameHost, HEIGHT, WIDTH};

/// Where the window's content ends: the strip of handles stands at the right edge.
const EDGE: f64 = WIDTH - HANDLE_WIDTH;

const WINDOW: &str = r#"
    <Button name="detailsButton" at="1,1" span="2" caption="Details" />
    <Textbox name="search" at="9,1" span="4" />
    <Slideout name="details" caption="Order" slideWidth="400" resizable="true" min="300" max="600" pinnable="true" span="6" rows="4" onopen="loadDetails" onclose="dropDetails">
        <Textbox name="noteBox" at="1,1" span="6" />
    </Slideout>"#;

/// A screen whose clock stands still at `now`, so the slide is where the test says.
fn still(screen: &mut Screen, now: Instant) {
    screen.now = Box::new(move || now);
}

fn slideout(screen: &Screen) -> &SlideoutPart {
    find_type::<SlideoutPart>(screen.root(), |_| true).expect("a slide-out")
}

fn frame(screen: &Screen) -> &SlideoutFrame {
    screen.find::<SlideoutFrame>().expect("the slide-out lies over the window")
}

fn visible(host: &FrameHost) -> Option<bool> {
    host.state_of("details").visible
}

/// The window with its slide-out open and all the way in.
fn opened(markup: &str) -> (Screen, std::rc::Rc<FrameHost>, Instant) {
    let (mut screen, host) = window(markup, None);
    let start = Instant::now();
    still(&mut screen, start);
    host.set_state("details", |s| s.visible = Some(true));
    draw(&mut screen);
    still(&mut screen, start + SLIDE_TIME + Duration::from_millis(10));
    draw(&mut screen);
    (screen, host, start)
}

/// The handle of a slide-out beside a part: it lies in an overlay of its own.
fn handle_beside(screen: &Screen) -> Rect {
    screen.overlays().iter().find_map(|o| o.content.as_any().downcast_ref::<SlideoutHandle>()).expect("its handle").node().bounds
}

fn id_of(screen: &Screen, name: &str) -> ElementId {
    let top = screen.overlays().iter().find(|o| o.content.as_any().is::<SlideoutFrame>()).map(|o| o.content.as_ref());
    let roots: Vec<&dyn Element> = top.into_iter().chain([screen.root()]).collect();
    roots.into_iter().flat_map(clarity_ui::element::in_order).find(|e| part_name(*e) == Some(name)).map(|e| e.node().id()).expect("the part is there")
}

#[test]
fn it_is_closed_until_its_state_is_visible_and_takes_no_room_but_its_handle() {
    let (mut screen, host) = window(WINDOW, None);
    draw(&mut screen);
    assert!(screen.overlays().is_empty());
    assert_eq!(None, slideout(&screen).opened());
    assert_eq!(Rect::new(0.0, 0.0, EDGE, HEIGHT), view(&screen).body_place());
    host.set_state("details", |s| s.visible = Some(true));
    draw(&mut screen);
    assert_eq!(1, screen.overlays().len());
    host.set_state("details", |s| s.visible = Some(false));
    draw(&mut screen);
    assert!(screen.overlays().is_empty());
}

#[test]
fn it_slides_in_over_the_window_from_its_edge_and_leaves_the_parts_where_they_are() {
    let (mut screen, host) = window(WINDOW, None);
    let start = Instant::now();
    still(&mut screen, start);
    let search = id_of(&screen, "search");
    draw(&mut screen);
    let before = screen.element(search).unwrap().node().bounds;
    host.set_state("details", |s| s.visible = Some(true));
    draw(&mut screen);
    // It starts beyond the right edge, and the screen draws every frame while it slides.
    assert_eq!(EDGE, frame(&screen).node().bounds.x);
    assert_eq!(1, screen.animating());
    still(&mut screen, start + SLIDE_TIME / 2);
    draw(&mut screen);
    assert_eq!(EDGE - 200.0, frame(&screen).node().bounds.x);
    still(&mut screen, start + SLIDE_TIME * 2);
    draw(&mut screen);
    // In: the band at the right of the window's content, as high as it; the parts under it stay put.
    assert_eq!(Rect::new(EDGE - 400.0, 0.0, 400.0, HEIGHT), frame(&screen).node().bounds);
    assert_eq!(0, screen.animating());
    assert_eq!(before, screen.element(search).unwrap().node().bounds);
    assert_eq!(EDGE, view(&screen).body_place().width);
    // It draws over what is under it: its panel, not the window's background.
    let mut shot = draw(&mut screen);
    let panel = screen.look().color("panel");
    assert_eq!(panel, shot.pixel((EDGE - 200.0) as i32, (HEIGHT - 4.0) as i32));
}

#[test]
fn on_the_left_it_comes_in_from_the_left() {
    let (screen, _host, _) = opened(&WINDOW.replace("caption=\"Order\"", "caption=\"Order\" side=\"left\""));
    assert_eq!(Rect::new(HANDLE_WIDTH, 0.0, 400.0, HEIGHT), frame(&screen).node().bounds);
}

#[test]
fn it_lies_below_the_menu_bar_and_above_the_footer() {
    let markup = format!("<Menu><Item caption=\"File\" /></Menu><Footer><Label caption=\"Ready\" at=\"1,1\" span=\"3\" /></Footer>{WINDOW}");
    let (screen, _host, _) = opened(&markup);
    let body = view(&screen).body_place();
    assert!(body.y > 0.0 && body.bottom() < HEIGHT, "{body:?}");
    let place = frame(&screen).node().bounds;
    assert_eq!((place.y, place.height), (body.y, body.height));
}

#[test]
fn escape_or_a_press_beside_it_closes_it_and_its_state_hears() {
    let (mut screen, host, _) = opened(WINDOW);
    press(&mut screen, Key::Escape, Modifiers::NONE);
    assert!(screen.overlays().is_empty());
    assert_eq!(Some(false), visible(&host));
    let (mut screen, host, _) = opened(WINDOW);
    screen.pointer_pressed(100.0, 300.0, PointerButton::Left, 1);
    screen.pointer_released(100.0, 300.0, PointerButton::Left);
    assert!(screen.overlays().is_empty());
    assert_eq!(Some(false), visible(&host));
}

#[test]
fn its_close_button_closes_it_and_its_pin_pins_it() {
    let (mut screen, host, _) = opened(WINDOW);
    let buttons: Vec<(bool, Rect)> = frame(&screen).buttons().iter().map(|b| (b.pins(), b.node().bounds)).collect();
    assert_eq!(vec![true, false], buttons.iter().map(|(pins, _)| *pins).collect::<Vec<_>>());
    // In its header, at the right.
    assert!(buttons[1].1.right() <= WIDTH && buttons[1].1.y < 36.0, "{buttons:?}");
    click(&mut screen, buttons[0].1);
    assert_eq!(Some(true), host.state_of("details").pinned);
    click(&mut screen, buttons[1].1);
    draw(&mut screen);
    assert_eq!(Some(false), visible(&host));
    assert!(screen.overlays().is_empty());
}

#[test]
fn pinned_it_stays_open_beside_the_parts_which_make_room() {
    let (mut screen, host, _) = opened(WINDOW);
    host.set_state("details", |s| s.pinned = Some(true));
    draw(&mut screen);
    assert_eq!(Rect::new(0.0, 0.0, EDGE - 400.0, HEIGHT), view(&screen).body_place());
    assert_eq!(Rect::new(EDGE - 400.0, 0.0, 400.0, HEIGHT), frame(&screen).node().bounds);
    // The parts are laid out in what is left: the search box ends before it.
    let search = screen.element(id_of(&screen, "search")).unwrap().node().bounds;
    assert!(search.right() <= EDGE - 400.0, "{search:?}");
    // A press beside it reaches the parts there, and it stays open.
    let button = screen.element(id_of(&screen, "detailsButton")).unwrap().node().bounds;
    click(&mut screen, button);
    assert_eq!(1, screen.overlays().len());
    assert_eq!(Some(true), visible(&host));
    // Escape with the focus beside it leaves it open; with the focus in it, closes it.
    press(&mut screen, Key::Escape, Modifiers::NONE);
    assert_eq!(1, screen.overlays().len());
    let note = id_of(&screen, "noteBox");
    screen.focus(Some(note));
    screen.focus(clarity_ui::element::focus_takers(screen.element(note).unwrap()).first().copied());
    press(&mut screen, Key::Escape, Modifiers::NONE);
    assert!(screen.overlays().is_empty());
    // Unpinned again, the parts have the whole window back.
    host.set_state("details", |s| {
        s.pinned = Some(false);
        s.visible = Some(true);
    });
    draw(&mut screen);
    assert_eq!(EDGE, view(&screen).body_place().width);
}

#[test]
fn its_inner_edge_is_dragged_within_its_limits_and_the_width_is_kept_per_window() {
    let host = FrameHost::new();
    *host.window.borrow_mut() = Some("Orders".into());
    choices::forget("slideout.Orders.details");
    let mut screen = window_named(WINDOW, None, host.clone(), "Orders");
    host.set_state("details", |s| s.visible = Some(true));
    draw(&mut screen);
    still(&mut screen, Instant::now() + Duration::from_secs(5));
    draw(&mut screen);
    let edge = frame(&screen).edge().expect("a resizable slide-out has an edge");
    assert_eq!(EDGE - 400.0, edge.x);
    let (x, y) = (edge.x + 2.0, 300.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x - 100.0, y);
    screen.pointer_released(x - 100.0, y, PointerButton::Left);
    draw(&mut screen);
    assert_eq!(500.0, frame(&screen).node().bounds.width);
    // Past its max it stops there, as a sidebar does (conformance/slideouts.json).
    let edge = frame(&screen).edge().unwrap();
    screen.pointer_pressed(edge.x + 2.0, y, PointerButton::Left, 1);
    screen.pointer_moved(edge.x - 400.0, y);
    screen.pointer_released(edge.x - 400.0, y, PointerButton::Left);
    draw(&mut screen);
    assert_eq!(600.0, frame(&screen).node().bounds.width);
    assert_eq!(Some(r#"{"pinned":false,"size":600}"#.to_string()), choices::kept("slideout.Orders.details"));
    // The window opened again has the width the person left, and the pin.
    let pin = frame(&screen).buttons()[0].node().bounds;
    click(&mut screen, pin);
    let host = FrameHost::new();
    *host.window.borrow_mut() = Some("Orders".into());
    let mut again = window_named(WINDOW, None, host.clone(), "Orders");
    host.set_state("details", |s| s.visible = Some(true));
    draw(&mut again);
    still(&mut again, Instant::now() + Duration::from_secs(5));
    draw(&mut again);
    assert_eq!(600.0, frame(&again).node().bounds.width);
    assert_eq!(EDGE - 600.0, view(&again).body_place().width);
    choices::forget("slideout.Orders.details");
}

#[test]
fn the_widths_are_a_sidebars() {
    let file: serde_json::Value = serde_json::from_str(include_str!("../../../../conformance/slideouts.json")).unwrap();
    for case in file["widths"].as_array().unwrap() {
        let width = rules::sidebar_width(case["dragged"].as_f64().unwrap(), case["min"].as_f64(), case["max"].as_f64());
        assert_eq!(case["width"].as_f64().unwrap(), width, "{case}");
    }
}

#[test]
fn the_focus_moves_into_it_and_back_and_it_runs_open_and_close() {
    let (mut screen, host) = window(WINDOW, None);
    let button = id_of(&screen, "detailsButton");
    draw(&mut screen);
    screen.focus(Some(button));
    still(&mut screen, Instant::now());
    host.set_state("details", |s| s.visible = Some(true));
    draw(&mut screen);
    let note = id_of(&screen, "noteBox");
    let focused = screen.focused().expect("something has the focus");
    assert!(clarity_ui::element::find(screen.element(note).unwrap(), focused).is_some(), "the note box has the focus");
    // Its on open runs once the screen laid it out, as an event.
    screen.wake_due(Instant::now() + Duration::from_secs(1));
    assert!(host.log().iter().any(|l| l == "run details open loadDetails"), "{:?}", host.log());
    // Tab stays in it while it has the focus.
    let order = screen.tab_order();
    assert!(order.iter().all(|t| frame(&screen).node().id() == *t || clarity_ui::element::find(frame(&screen), *t).is_some()), "{order:?}");
    press(&mut screen, Key::Escape, Modifiers::NONE);
    draw(&mut screen);
    assert_eq!(Some(button), screen.focused());
    screen.wake_due(Instant::now() + Duration::from_secs(1));
    assert!(host.log().iter().any(|l| l == "run details close dropDetails"), "{:?}", host.log());
    // Each runs once.
    assert_eq!(1, host.log().iter().filter(|l| l.contains("loadDetails")).count());
}

#[test]
fn a_screen_reader_finds_it_as_a_dialog_named_by_its_caption() {
    let (screen, _host, _) = opened(WINDOW);
    let tree = access::tree(&screen);
    let node = tree.named(access::Role::Dialog, "Order").expect("a dialog named Order");
    assert_eq!(frame(&screen).node().id(), node.id);
    // Its buttons say what they do.
    let said: Vec<String> = frame(&screen).buttons().iter().map(|b: &&SlideoutButton| b.tooltip_at(0.0, 0.0).unwrap_or_default()).collect();
    assert_eq!(vec!["Pin it open".to_string(), "Close".to_string()], said);
    assert!(tree.named(access::Role::Button, "Close").is_some());
}

fn handle(screen: &Screen) -> Rect {
    screen.find::<SlideoutHandle>().expect("a slide-out with a caption has a handle").node().bounds
}

#[test]
fn its_handle_stands_on_its_edge_and_opens_and_closes_it() {
    let (mut screen, host) = window(WINDOW, None);
    draw(&mut screen);
    // A tab at the top of the strip at the right edge, as long as its caption.
    let tab = handle(&screen);
    assert_eq!((EDGE, 0.0, HANDLE_WIDTH), (tab.x, tab.y, tab.width));
    assert!(tab.height > 2.0 * 14.0, "{tab:?}");
    // A click slides it out; it lies beside the strip, which stays in sight.
    click(&mut screen, tab);
    draw(&mut screen);
    assert_eq!(Some(true), visible(&host));
    assert_eq!(1, screen.overlays().len());
    still(&mut screen, Instant::now() + Duration::from_secs(5));
    draw(&mut screen);
    assert_eq!(EDGE, frame(&screen).node().bounds.right());
    assert_eq!(tab, handle(&screen));
    // Another click on it slides it back: the press is no press beside it that would open it again.
    click(&mut screen, tab);
    draw(&mut screen);
    assert_eq!(Some(false), visible(&host));
    assert!(screen.overlays().is_empty());
}

#[test]
fn its_handle_takes_the_focus_and_enter_opens_it() {
    let (mut screen, host) = window(WINDOW, None);
    draw(&mut screen);
    let id = screen.find::<SlideoutHandle>().unwrap().node().id();
    screen.focus(Some(id));
    press(&mut screen, Key::Enter, Modifiers::NONE);
    draw(&mut screen);
    assert_eq!(Some(true), visible(&host));
}

#[test]
fn the_handles_of_one_side_stand_one_under_the_other_and_one_opens_while_the_other_closes() {
    let markup = format!("{WINDOW}<Slideout name=\"diagram\" caption=\"Diagram\" span=\"6\" rows=\"4\"><Textbox name=\"diagramBox\" at=\"1,1\" span=\"6\" /></Slideout>");
    let (mut screen, host) = window(&markup, None);
    draw(&mut screen);
    let tabs: Vec<Rect> = clarity_ui::element::in_order(screen.root()).into_iter().filter_map(|e| e.as_any().downcast_ref::<SlideoutHandle>()).map(|h| h.node().bounds).collect();
    assert_eq!(2, tabs.len());
    assert_eq!(tabs[0].bottom(), tabs[1].y);
    assert_eq!(EDGE, view(&screen).body_place().width);
    click(&mut screen, tabs[0]);
    draw(&mut screen);
    assert_eq!(Some(true), visible(&host));
    still(&mut screen, Instant::now() + Duration::from_secs(5));
    draw(&mut screen);
    click(&mut screen, tabs[1]);
    draw(&mut screen);
    assert_eq!(Some(false), visible(&host));
    assert_eq!(Some(true), host.state_of("diagram").visible);
}

#[test]
fn one_with_an_empty_caption_has_no_handle() {
    let (mut screen, _host) = window(&WINDOW.replace(" caption=\"Order\"", " caption=\"\""), None);
    draw(&mut screen);
    assert!(screen.find::<SlideoutHandle>().is_none());
    assert_eq!(WIDTH, view(&screen).body_place().width);
}

#[test]
fn a_screen_reader_finds_its_handle_as_a_button_that_says_whether_it_is_out() {
    let (mut screen, host) = window(WINDOW, None);
    draw(&mut screen);
    let tree = access::tree(&screen);
    assert_eq!(Some(false), tree.named(access::Role::Button, "Order").expect("the handle").states.expanded);
    host.set_state("details", |s| s.visible = Some(true));
    draw(&mut screen);
    let tree = access::tree(&screen);
    assert_eq!(Some(true), tree.named(access::Role::Button, "Order").unwrap().states.expanded);
}

/// The window's slide-out at `side`, `slideHeight` 200 high where it lies across the window.
fn at_side(side: &str) -> String {
    WINDOW.replace("caption=\"Order\" slideWidth=\"400\"", &format!("caption=\"Order\" side=\"{side}\" slideHeight=\"200\""))
}

#[test]
fn at_the_top_it_comes_down_under_its_strip_as_wide_as_the_content() {
    let (mut screen, host) = window(&at_side("top"), None);
    let start = Instant::now();
    still(&mut screen, start);
    draw(&mut screen);
    // Its handle stands at the left of a strip along the top, as long as its caption; the parts start under it.
    let tab = handle(&screen);
    assert_eq!((0.0, 0.0, HANDLE_WIDTH), (tab.x, tab.y, tab.height));
    assert!(tab.width > 2.0 * 14.0, "{tab:?}");
    assert_eq!(Rect::new(0.0, HANDLE_WIDTH, WIDTH, HEIGHT - HANDLE_WIDTH), view(&screen).body_place());
    host.set_state("details", |s| s.visible = Some(true));
    draw(&mut screen);
    still(&mut screen, start + SLIDE_TIME / 2);
    draw(&mut screen);
    // Half way it shows its lower half, just under the strip.
    assert_eq!(Rect::new(0.0, HANDLE_WIDTH, WIDTH, 100.0), frame(&screen).node().bounds);
    still(&mut screen, start + SLIDE_TIME * 2);
    draw(&mut screen);
    assert_eq!(Rect::new(0.0, HANDLE_WIDTH, WIDTH, 200.0), frame(&screen).node().bounds);
}

#[test]
fn at_the_bottom_it_comes_up_and_its_inner_edge_is_dragged_up() {
    let (screen, _host, _) = opened(&at_side("bottom"));
    let body = view(&screen).body_place();
    assert_eq!(Rect::new(0.0, 0.0, WIDTH, HEIGHT - HANDLE_WIDTH), body);
    assert_eq!(Rect::new(0.0, body.bottom() - 200.0, WIDTH, 200.0), frame(&screen).node().bounds);
    let mut screen = screen;
    let edge = frame(&screen).edge().expect("a resizable slide-out has an edge");
    assert_eq!((body.bottom() - 200.0, WIDTH), (edge.y, edge.width));
    let (x, y) = (300.0, edge.y + 2.0);
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_moved(x, y - 150.0);
    screen.pointer_released(x, y - 150.0, PointerButton::Left);
    draw(&mut screen);
    assert_eq!(350.0, frame(&screen).node().bounds.height);
}

#[test]
fn at_the_bottom_its_handles_stand_on_top_of_the_footer_one_beside_the_other() {
    let markup = format!("<Footer status=\"Ready\"><Label caption=\"Saved\" at=\"1,1\" span=\"3\" /></Footer>{}<Slideout name=\"erd\" caption=\"ERD\" side=\"bottom\" span=\"6\" rows=\"4\"><Textbox name=\"erdBox\" at=\"1,1\" span=\"6\" /></Slideout>", at_side("bottom"));
    let (mut screen, host) = window(&markup, None);
    draw(&mut screen);
    let footer = view(&screen).footer().expect("a footer").node().bounds;
    let tabs: Vec<Rect> = clarity_ui::element::in_order(screen.root()).into_iter().filter_map(|e| e.as_any().downcast_ref::<SlideoutHandle>()).map(|h| h.node().bounds).collect();
    assert_eq!(2, tabs.len());
    // In a strip just above the footer, from its left, one beside the other; the footer stays whole.
    assert_eq!((tabs[0].x, tabs[0].bottom(), tabs[0].height), (0.0, footer.y, HANDLE_WIDTH));
    assert_eq!(tabs[0].right(), tabs[1].x);
    assert_eq!((footer.x, footer.width), (0.0, WIDTH));
    // The window's parts end above the strip, and the slide-out comes up from it.
    assert_eq!(footer.y - HANDLE_WIDTH, view(&screen).body_place().bottom());
    click(&mut screen, tabs[1]);
    draw(&mut screen);
    still(&mut screen, Instant::now() + Duration::from_secs(5));
    draw(&mut screen);
    assert_eq!(Some(true), host.state_of("erd").visible);
    assert_eq!(footer.y - HANDLE_WIDTH, frame(&screen).node().bounds.bottom());
}

#[test]
fn pinned_at_the_top_it_pushes_the_parts_down() {
    let (mut screen, _host, _) = opened(&at_side("top"));
    let pin = frame(&screen).buttons()[0].node().bounds;
    click(&mut screen, pin);
    draw(&mut screen);
    assert_eq!(Rect::new(0.0, HANDLE_WIDTH + 200.0, WIDTH, HEIGHT - HANDLE_WIDTH - 200.0), view(&screen).body_place());
}

#[test]
fn a_press_on_a_canvas_in_it_gives_the_canvas_the_focus_so_ctrl_z_takes_back_its_bend() {
    let (mut screen, _host, _) = opened(
        r#"<Slideout name="details" caption="Diagram" slideWidth="500" span="6" rows="6">
            <Radio name="kindBox" at="1,1" span="3" />
            <Canvas name="board" at="1,2" span="6" rows="5" undoScope="board" ondraw="drawn" onbend="bent" />
        </Slideout>"#,
    );
    let board = id_of(&screen, "board");
    let b = screen.element(board).unwrap().node().bounds;
    screen.pointer_pressed(b.x + b.width / 2.0, b.y + b.height / 2.0, PointerButton::Left, 1);
    screen.pointer_released(b.x + b.width / 2.0, b.y + b.height / 2.0, PointerButton::Left);
    assert_eq!(Some(board), screen.focused(), "the canvas pressed has the focus, not the first part of the slide-out");
}

/// `beside chapters`: in a panel, against the list's right edge, with its
/// handle there and not at the window's; it slides out over what lies beside
/// the list, as high as the list, and a click on its handle opens it.
#[test]
fn beside_a_part_it_lies_against_that_part_in_any_container() {
    let markup = r#"
    <Panel name="desk" at="1,1" span="12" rows="6">
        <List name="chapters" at="1,1" span="4" rows="6" />
        <Textbox name="text" at="5,1" span="8" />
        <Slideout name="details" caption="Chapter" side="right" beside="chapters" slideWidth="300" pinnable="true" span="6" rows="4">
            <Textbox name="titleBox" at="1,1" span="6" />
        </Slideout>
    </Panel>"#;
    let (mut screen, host, _) = opened(markup);
    // No strip of handles at the window's edge: its handle is on the list.
    assert_eq!(WIDTH, view(&screen).body_place().width);
    let list = screen.root().node().bounds;
    let list = clarity_ui::element::in_order(screen.root()).into_iter().find(|e| part_name(*e) == Some("chapters")).map(|e| e.node().bounds).unwrap_or(list);
    let handle = handle_beside(&screen);
    assert_eq!((handle.x, handle.y, handle.width), (list.right(), list.y, HANDLE_WIDTH));
    let band = frame(&screen).node().bounds;
    assert_eq!((band.x, band.width), (list.right() + HANDLE_WIDTH, 300.0));
    // As high as the panel's grid the list stands in, not only the list.
    let panel = clarity_ui::element::in_order(screen.root()).into_iter().find(|e| part_name(*e) == Some("desk")).map(|e| e.node().bounds).expect("the panel");
    assert!(band.y <= list.y && band.bottom() >= list.bottom() && band.bottom() <= panel.bottom() + 0.5, "{band:?} in {panel:?}");
    // Its handle closes it and opens it again.
    click(&mut screen, handle);
    draw(&mut screen);
    assert_eq!(Some(false), visible(&host));
    click(&mut screen, handle);
    draw(&mut screen);
    assert_eq!(Some(true), visible(&host));
}

/// Beside a list in a narrow splitter pane, it lies over the pane beside it, not over the list.
#[test]
fn beside_a_part_in_a_narrow_pane_it_lies_over_the_next_pane() {
    let markup = r#"
    <Splitter name="split" at="1,1" span="12" rows="6" horizontal="true">
        <Pane span="3">
            <List name="chapters" at="1,1" span="3" rows="6" />
            <Slideout name="details" caption="Chapter" side="right" beside="chapters" slideWidth="300" span="6" rows="4">
                <Textbox name="titleBox" at="1,1" span="6" />
            </Slideout>
        </Pane>
        <Pane>
            <Textbox name="text" at="1,1" span="9" />
        </Pane>
    </Splitter>"#;
    let (screen, _, _) = opened(markup);
    let list = clarity_ui::element::in_order(screen.root()).into_iter().find(|e| part_name(*e) == Some("chapters")).map(|e| e.node().bounds).expect("the list");
    // Its handle lies over everything, so the narrow pane does not cut it off: just outside the list's edge.
    let handle = handle_beside(&screen);
    assert_eq!((handle.x, handle.y), (list.right(), list.y));
    let band = frame(&screen).node().bounds;
    assert_eq!((band.x, band.width), (list.right() + HANDLE_WIDTH, 300.0));
}

/// What lies beside the part makes room for its handle, as the window's parts
/// make room for the handles at its edge: in the same grid, and in the next pane.
#[test]
fn what_lies_beside_the_part_makes_room_for_its_handle() {
    let part_at = |screen: &Screen, name: &str| clarity_ui::element::in_order(screen.root()).into_iter().find(|e| part_name(*e) == Some(name)).map(|e| e.node().bounds).expect(name);
    let same_grid = r#"
    <Panel name="desk" at="1,1" span="12" rows="6">
        <List name="chapters" at="1,1" span="4" rows="6" />
        <Textbox name="text" at="5,1" span="8" />
        <Slideout name="details" caption="Chapter" side="right" beside="chapters" slideWidth="300" span="6" rows="4">
            <Textbox name="titleBox" at="1,1" span="6" />
        </Slideout>
    </Panel>"#;
    let (mut screen, _, _) = opened(same_grid);
    draw(&mut screen);
    let handle = handle_beside(&screen);
    let text = part_at(&screen, "text");
    assert!(text.x >= handle.right() - 0.5, "the text box {text:?} starts after the handle {handle:?}");
    assert!(part_at(&screen, "chapters").right() <= handle.x + 0.5, "the list keeps its place");
    let next_pane = r#"
    <Splitter name="split" at="1,1" span="12" rows="6" horizontal="true">
        <Pane span="3">
            <List name="chapters" at="1,1" span="3" rows="6" />
            <Slideout name="details" caption="Chapter" side="right" beside="chapters" slideWidth="300" span="6" rows="4">
                <Textbox name="titleBox" at="1,1" span="6" />
            </Slideout>
        </Pane>
        <Pane>
            <Textbox name="text" at="1,1" span="9" rows="4" />
        </Pane>
    </Splitter>"#;
    let (mut screen, _, _) = opened(next_pane);
    draw(&mut screen);
    let handle = handle_beside(&screen);
    let text = part_at(&screen, "text");
    assert!(text.x >= handle.right() - 0.5, "the text box {text:?} in the next pane starts after the handle {handle:?}");
}

/// In a folded accordion section its part is not laid out: its handle and its panel go with it.
#[test]
fn beside_a_part_in_a_folded_section_it_is_not_there() {
    let markup = r#"
    <Accordion name="parts" at="1,1" span="12" rows="6">
        <Section caption="Chapters">
            <List name="chapters" at="1,1" span="4" rows="6" />
            <Slideout name="details" caption="Chapter" side="right" beside="chapters" slideWidth="300" span="6" rows="4">
                <Textbox name="titleBox" at="1,1" span="6" />
            </Slideout>
        </Section>
        <Section caption="Research">
            <Textbox name="other" at="1,1" span="6" />
        </Section>
    </Accordion>"#;
    let (mut screen, host, _) = opened(markup);
    assert!(handle_shown(&screen));
    host.set_state("parts", |s| s.open = Some(2));
    draw(&mut screen);
    draw(&mut screen);
    assert!(!handle_shown(&screen), "the handle went with its folded section");
    assert!(!frame(&screen).is_shown(), "and so did its panel");
}

fn handle_shown(screen: &Screen) -> bool {
    screen.overlays().iter().find_map(|o| o.content.as_any().downcast_ref::<SlideoutHandle>()).is_some_and(|h| h.is_shown())
}

/// A slide-out lies over the window from its edge: its rows take no room in
/// the window's content, so a tall one gives the window nothing to scroll
/// (a window that scrolled for it moved its parts up and left a blank band).
#[test]
fn a_tall_slide_out_gives_the_window_nothing_to_scroll() {
    let tall = r#"
    <Button name="detailsButton" at="1,1" span="2" caption="Details" />
    <Slideout name="details" at="1,1" caption="Order" slideWidth="400" span="6" rows="60">
        <Textbox name="noteBox" at="1,1" span="6" />
    </Slideout>"#;
    let (mut screen, _host) = window(tall, None);
    draw(&mut screen);
    assert_eq!(view(&screen).max_scroll_y, 0.0, "the window does not scroll for its slide-out");
}

/// The maintenance app's main window: a toolbar, a sidebar at the left and
/// a slide-out at the right beside a content whose view is one memo. At 756
/// pixels, 15 rows fit under the toolbar (16 were 2 pixels too many: the
/// window scrolled and cut off the memo's bottom edge); the content grows to fill it.
#[test]
fn the_maintenance_window_fits_its_rows_and_its_memo_ends_inside() {
    let main = |rows: u32| format!(r#"
    <Toolbar name="mainToolbar">
        <Item kind="button" caption="Projects" name="projectsButton" />
    </Toolbar>
    <Sidebar side="left" name="ledgerSidebar" width="420" min="280" max="800" resizable="true" columns="6">
        <List name="projectList" at="1,1" span="6" rows="3" />
    </Sidebar>
    <Content name="main" at="1,1" span="12" rows="{rows}">
        <Tab columns="12"><Memo name="entryText" at="1,1" span="12" rows="{rows}" markdown="true" /></Tab>
    </Content>
    <Slideout name="reviewPanel" at="1,1" span="6" rows="{rows}" caption="Review" side="right" slideWidth="480">
        <Label name="note" at="1,1" span="6" caption="Note" />
    </Slideout>"#);
    let measured = |rows: u32| {
        let (mut screen, _host) = window(&main(rows), None);
        clarity_ui::testing::Shot::of(&mut screen, 1280, 756, 1.0);
        let memo = screen.element(clarity_ui::testing::part_named(&screen, "entryText")).map(|m| m.node().bounds).expect("the memo");
        (view(&screen).max_scroll_y, memo)
    };
    assert!(measured(16).0 > 0.0, "16 rows are more than 756 pixels hold under a toolbar");
    let (scroll, memo) = measured(15);
    assert_eq!(scroll, 0.0, "15 rows fit");
    assert!(memo.bottom() <= 756.0 - 12.0 + 0.5, "the memo ends at {} above the window's padding", memo.bottom());
}
