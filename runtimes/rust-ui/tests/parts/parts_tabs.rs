//! Tabs, as the C# TabsPartTests check them: a strip of tabs over the open
//! tab's page, a grid of its parts. The open tab is the part's state's tab,
//! from 1; a tab whose parts are all hidden gives way to the first that
//! shows (conformance/ribbons.json).

use std::rc::Rc;

use clarity_ui::element::find_type;
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::parts::containers::tabs::{self, TabsPart, STRIP_HEIGHT};
use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use clarity_ui::{Element, Screen};
use serde_json::Value;

fn conformance(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance").join(name);
    serde_json::from_str(&std::fs::read_to_string(path).expect("the case file reads")).expect("the case file is JSON")
}

fn window(markup: &str, host: Rc<TestHost>, width: f64) -> Screen {
    let mut screen = window_of(markup, host);
    screen.resize(width, 400.0, 1.0);
    screen
}

fn first<T: Element>(screen: &Screen) -> &T {
    find_type::<T>(screen.root(), |_| true).expect("the part is on the screen")
}

fn click(screen: &mut Screen, x: f64, y: f64) {
    screen.pointer_pressed(x, y, PointerButton::Left, 1);
    screen.pointer_released(x, y, PointerButton::Left);
}

fn keep(shot: &mut Shot, name: &str) {
    if let Ok(dir) = std::env::var("CLARITY_SHOTS") {
        shot.save(std::path::Path::new(&dir).join(name)).expect("the picture is written");
    }
}

const TABS: &str = r#"<Tabs name="pages" at="1,1" span="12" rows="6">
  <Tab title="One"><Label name="first" at="1,1" span="3" caption="First" /></Tab>
  <Tab title="Two"><Label name="second" at="1,1" span="3" caption="Second" /></Tab>
  <Tab title="Three" span="6"><Label name="third" at="4,1" span="3" caption="Third" /></Tab>
</Tabs>"#;

#[test]
fn the_first_tab_is_open_and_only_its_page_shows() {
    let mut screen = window(TABS, TestHost::new(), 808.0);
    let tabs = first::<TabsPart>(&screen);
    assert_eq!(Some(1), tabs.open());
    assert!(tabs.pages()[0].is_shown());
    assert!(!tabs.pages()[1].is_shown());
    assert!(tabs.grids()[0].node().bounds.y > STRIP_HEIGHT);
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    keep(&mut shot, "containers-tabs.png");
}

#[test]
fn a_press_on_a_tab_opens_it_in_the_parts_state() {
    let host = TestHost::new();
    let mut screen = window(TABS, host.clone(), 808.0);
    let second = first::<TabsPart>(&screen).tabs()[1].node().bounds;
    click(&mut screen, second.x + 5.0, second.y + 5.0);
    assert_eq!(Some(2), host.state(Some("pages")).unwrap().tab);
    screen.resize(808.0, 400.0, 1.0);
    let tabs = first::<TabsPart>(&screen);
    assert!(tabs.pages()[1].is_shown() && !tabs.pages()[0].is_shown());
    host.set_state("pages", |s| s.tab = Some(3));
    screen.resize(808.0, 400.0, 1.0);
    let tabs = first::<TabsPart>(&screen);
    assert_eq!(Some(3), tabs.open());
    assert!(tabs.pages()[2].is_shown());
}

#[test]
fn every_page_has_the_parts_columns_as_on_the_web() {
    let screen = window(TABS, TestHost::new(), 808.0);
    let tabs = first::<TabsPart>(&screen);
    assert_eq!(12, tabs.grids()[2].columns);
    assert_eq!(12, tabs.grids()[0].columns);
}

#[test]
fn the_tab_shown_is_the_one_asked_for_else_the_first_that_shows() {
    for case in conformance("ribbons.json")["shown"].as_array().unwrap() {
        let visible: Vec<bool> = case["visible"].as_array().unwrap().iter().map(|v| v.as_bool().unwrap()).collect();
        assert_eq!(case["shown"].as_i64(), tabs::shown_tab(&visible, case["tab"].as_i64().unwrap()), "{case}");
    }
}

#[test]
fn a_tab_whose_parts_are_all_hidden_gives_way() {
    let host = TestHost::new();
    let mut screen = window(TABS, host.clone(), 808.0);
    host.set_state("pages", |s| s.tab = Some(2));
    host.set_state("second", |s| s.visible = Some(false));
    screen.resize(808.0, 400.0, 1.0);
    let tabs = first::<TabsPart>(&screen);
    assert_eq!(Some(1), tabs.open());
    assert!(!tabs.tabs()[1].is_shown());
    assert!(tabs.pages()[0].is_shown());
}

#[test]
fn arrow_keys_move_between_tabs() {
    let host = TestHost::new();
    let mut screen = window(TABS, host.clone(), 808.0);
    let (one, two) = {
        let tabs = first::<TabsPart>(&screen);
        (tabs.tabs()[0].node().id(), tabs.tabs()[1].node().id())
    };
    screen.focus(Some(one));
    screen.key_pressed(KeyInput::plain(Key::Right));
    assert_eq!(Some(2), host.state(Some("pages")).unwrap().tab);
    assert_eq!(Some(two), screen.focused());
    screen.key_pressed(KeyInput::plain(Key::Left));
    screen.key_pressed(KeyInput::plain(Key::Left));
    assert_eq!(Some(1), host.state(Some("pages")).unwrap().tab);
    assert_eq!(Some(one), screen.focused());
}

#[test]
fn a_page_whose_parts_go_further_than_its_room_scrolls() {
    let rows: String = (1..=8).map(|r| format!(r#"<Label name="l{r}" at="1,{r}" span="4" />"#)).collect();
    let mut screen = window(&format!(r#"<Tabs name="pages" at="1,1" span="12" rows="3"><Tab title="One">{rows}</Tab></Tabs>"#), TestHost::new(), 808.0);
    // A place lower than the tabs ask for, as in a window too low for them.
    let id = first::<TabsPart>(&screen).node().id();
    screen.act(id, |e, cx| e.arrange(Rect::new(0.0, 0.0, 808.0, 124.0), cx));
    let v = first::<TabsPart>(&screen).views()[0].node().bounds;
    assert!(first::<TabsPart>(&screen).views()[0].max_scroll_y() > 0.0);
    screen.wheel(v.x + 20.0, v.y + 20.0, 0.0, 1000.0, Modifiers::NONE);
    let view = first::<TabsPart>(&screen).views()[0];
    assert_eq!(view.max_scroll_y(), view.scroll_y());
    let last = screen.element(part_named(&screen, "l8")).unwrap().node().bounds;
    assert!(last.bottom() <= v.bottom() + 0.5);
}

#[test]
fn tabs_ask_room_for_their_strip_so_a_page_of_their_own_rows_needs_no_scrolling() {
    let rows: String = (1..=3).map(|r| format!(r#"<Label at="1,{r}" span="4" />"#)).collect();
    let screen = window(&format!(r#"<Tabs name="pages" at="1,1" span="12" rows="3"><Tab title="One">{rows}</Tab></Tabs>"#), TestHost::new(), 808.0);
    let tabs = first::<TabsPart>(&screen);
    assert!(tabs.around() >= STRIP_HEIGHT);
    assert_eq!(0.0, tabs.views()[0].max_scroll_y());
}

/// A tab with columns of its own (a view keeps its grid, `columns`) lays its parts out on them: a part in its last columns stays
/// inside the tabs, though they span fewer of the window's columns.
#[test]
fn a_tab_with_columns_of_its_own_keeps_its_parts_inside() {
    let host = TestHost::new();
    let markup = r#"<Tabs name="parts" at="1,1" span="9" rows="4"><Tab title="View" columns="12"><Button name="saveButton" at="11,1" span="2" /></Tab><Tab title="Own"><Button name="ownButton" at="8,1" span="2" /></Tab></Tabs>"#;
    let mut screen = window(markup, host, 1200.0);
    Shot::of(&mut screen, 1200, 400, 1.0);
    let tabs = screen.element(part_named(&screen, "parts")).unwrap().node().bounds;
    let save = screen.element(part_named(&screen, "saveButton")).unwrap().node().bounds;
    assert!(save.x + save.width <= tabs.x + tabs.width + 0.5, "the button at 11,1 of 12 ends at {} but the tabs end at {}", save.x + save.width, tabs.x + tabs.width);
    assert!(save.width > 0.0);
}

/// Tabs with every option: closing (each tab's x, but a permanent one's), cycling and available.
const CLOSING: &str = r#"<Tabs name="pages" at="1,1" span="12" rows="6" closing="true" cycling="true" available="true" onclose="pageClosed">
  <Tab title="One" permanent="true"><Label name="first" at="1,1" span="3" caption="First" /></Tab>
  <Tab title="Two"><Label name="second" at="1,1" span="3" caption="Second" /></Tab>
  <Tab title="Three"><Label name="third" at="1,1" span="3" caption="Third" /></Tab>
</Tabs>"#;

fn tab_buttons(screen: &Screen) -> Vec<&tabs::TabButton> {
    first::<TabsPart>(screen).tabs().into_iter().filter_map(|t| t.as_any().downcast_ref::<tabs::TabButton>()).collect()
}

#[test]
fn a_tab_closes_with_its_x_and_a_permanent_one_has_none() {
    let host = TestHost::new();
    let mut screen = window(CLOSING, host.clone(), 808.0);
    let buttons = tab_buttons(&screen);
    assert!(buttons[0].close_mark().is_none(), "a permanent tab has no x");
    let x = buttons[1].close_mark().expect("a tab of tabs that close has an x");
    click(&mut screen, x.x + x.width / 2.0, x.y + x.height / 2.0);
    let tabs = first::<TabsPart>(&screen);
    assert_eq!(tabs.closed(), vec![2], "the second tab is closed");
    assert!(!tabs.tabs()[1].is_shown(), "its tab is gone from the strip");
    assert!(host.logged().iter().any(|l| l == "run pages close pageClosed"), "the part's close runs: {:?}", host.logged());
    let plain = window(TABS, TestHost::new(), 808.0);
    assert!(tab_buttons(&plain).iter().all(|t| t.close_mark().is_none()), "without closing, no tab has an x");
}

#[test]
fn closing_the_open_tab_opens_the_next_and_code_brings_one_back() {
    let host = TestHost::new();
    let mut screen = window(CLOSING, host.clone(), 808.0);
    host.set_state("pages", |s| s.tab = Some(2));
    screen.resize(808.0, 400.0, 1.0);
    let x = tab_buttons(&screen)[1].close_mark().unwrap();
    click(&mut screen, x.x + x.width / 2.0, x.y + x.height / 2.0);
    screen.resize(808.0, 400.0, 1.0);
    assert_eq!(Some(3), first::<TabsPart>(&screen).open(), "the tab after it opens");
    // `pages.reopen(2)` is `pages.tab = 2`: opening a closed tab brings it back.
    host.set_state("pages", |s| s.tab = Some(2));
    screen.resize(808.0, 400.0, 1.0);
    let tabs = first::<TabsPart>(&screen);
    assert!(tabs.closed().is_empty(), "back");
    assert_eq!(Some(2), tabs.open());
}

#[test]
fn tabs_that_do_not_fit_are_gone_through_with_the_triangles() {
    let host = TestHost::new();
    let mut screen = window(CLOSING, host.clone(), 808.0);
    assert!(first::<TabsPart>(&screen).cycle_buttons().is_none(), "all fit: no triangles");
    screen.resize(150.0, 400.0, 1.0);
    let (before, next) = first::<TabsPart>(&screen).cycle_buttons().expect("too narrow: the triangles show");
    assert!(next.x > before.x, "the tab before, then the next");
    click(&mut screen, next.x + next.width / 2.0, next.y + next.height / 2.0);
    assert_eq!(Some(2), host.state(Some("pages")).unwrap().tab);
    click(&mut screen, before.x + before.width / 2.0, before.y + before.height / 2.0);
    click(&mut screen, before.x + before.width / 2.0, before.y + before.height / 2.0);
    assert_eq!(Some(3), host.state(Some("pages")).unwrap().tab, "round from the first to the last");
    screen.resize(150.0, 400.0, 1.0);
    let tabs = first::<TabsPart>(&screen);
    assert!(tabs.tabs()[2].is_shown(), "the open tab is in view");
    let plain = window(TABS, TestHost::new(), 150.0);
    assert!(first::<TabsPart>(&plain).cycle_buttons().is_none(), "without cycling, no triangles");
}

#[test]
fn the_dropdown_lists_the_closed_tabs_and_brings_one_back() {
    let host = TestHost::new();
    let mut screen = window(CLOSING, host.clone(), 808.0);
    assert!(first::<TabsPart>(&screen).available_box().is_none(), "nothing closed: no dropdown");
    let x = tab_buttons(&screen)[2].close_mark().unwrap();
    click(&mut screen, x.x + x.width / 2.0, x.y + x.height / 2.0);
    screen.resize(808.0, 400.0, 1.0);
    let place = first::<TabsPart>(&screen).available_box().expect("a closed tab: the dropdown shows");
    click(&mut screen, place.x + 5.0, place.y + 5.0);
    screen.key_pressed(KeyInput::plain(Key::Down));
    screen.key_pressed(KeyInput::plain(Key::Enter));
    screen.resize(808.0, 400.0, 1.0);
    let tabs = first::<TabsPart>(&screen);
    assert!(tabs.closed().is_empty(), "picked, it is back");
    assert_eq!(Some(3), tabs.open(), "and open");
}

#[test]
fn tabs_with_their_options_draw_their_x_triangles_and_dropdown() {
    let host = TestHost::new();
    let mut screen = window(CLOSING, host.clone(), 420.0);
    let x = tab_buttons(&screen)[2].close_mark().unwrap();
    click(&mut screen, x.x + x.width / 2.0, x.y + x.height / 2.0);
    screen.resize(210.0, 200.0, 1.0);
    let tabs = first::<TabsPart>(&screen);
    assert!(tabs.cycle_buttons().is_some() && tabs.available_box().is_some(), "both show at this width with a tab closed: {:?} {:?} {:?} {:?}", tabs.closed(), tabs.cycle_buttons(), tabs.available_box(), tabs.node().bounds);
    let mut shot = Shot::of(&mut screen, 210, 200, 1.0);
    keep(&mut shot, "containers-tabs-options.png");
}
