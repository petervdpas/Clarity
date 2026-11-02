//! The containers that hold grids of parts, as the C# tests and the shared
//! cases check them: a panel and a form (frame, header, folding by pointer
//! and keys, a form's submit), a button group, a separator, the scroll view
//! and the scroll box, tabs (conformance/ribbons.json `shown`) and an
//! accordion (conformance/accordions.json). Parts inside are made by their
//! kinds and laid out in the container's own grid.

use std::rc::Rc;

use clarity_ui::element::{find_type, Plate};
use clarity_ui::geometry::Rect;
use clarity_ui::grid::GridItem;
use clarity_ui::input::{Key, KeyInput, Modifiers, PointerButton};
use clarity_ui::painter::Corners;
use clarity_ui::parts::button::ButtonPart;
use clarity_ui::parts::containers::accordion::{self, AccordionPart};
use clarity_ui::parts::containers::buttons::ButtonsPart;
use clarity_ui::parts::containers::panel::{self, PanelPart};
use clarity_ui::parts::containers::scroll::ScrollPart;
use clarity_ui::parts::containers::scroll_view::{ScrollView, BAR_THICKNESS};
use clarity_ui::parts::containers::separator::SeparatorPart;
use clarity_ui::parts::containers::tabs::{self, TabsPart, STRIP_HEIGHT};
use clarity_ui::parts::containers::container;
use clarity_ui::parts::{Part, PartHost};
use clarity_ui::element::GridPanel;
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};
use clarity_ui::theme::ThemeLook;
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

const PANEL: &str = r#"<Panel name="box" at="7,1" span="6" rows="4" caption="Customer"><Label name="inner" at="4,1" span="3" caption="Inner" /></Panel>"#;

#[test]
fn a_panels_parts_are_in_a_grid_as_wide_as_it_spans_under_its_header() {
    let screen = window(PANEL, TestHost::new(), 808.0);
    let panel = first::<PanelPart>(&screen);
    // Twelve columns of 60 with 8 between: the panel is the right half.
    assert_eq!(Rect::new(408.0, 0.0, 400.0, 168.0), panel.node().bounds);
    let inner = screen.element(part_named(&screen, "inner")).unwrap().node().bounds;
    // Its own six columns are the window's: the part at its fourth starts at the window's tenth.
    assert_eq!(612.0, inner.x);
    assert_eq!(196.0, inner.width);
    assert!(inner.y > 20.0);
}

#[test]
fn a_panel_shows_its_caption_in_the_themes_header_and_its_frame() {
    let mut screen = window(PANEL, TestHost::new(), 808.0);
    let look = screen.look().clone();
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    keep(&mut shot, "containers-panel.png");
    assert_eq!(look.part("header", None).background.unwrap(), shot.pixel(800, 6));
    assert_eq!(look.color("strong_border"), shot.pixel(405, 100));
}

#[test]
fn a_form_without_a_caption_has_no_frame_and_its_grid_lines_up_with_the_windows() {
    let screen = window(r#"<Form at="1,2" span="12" rows="3"><Label name="l" at="1,1" span="2" /></Form>"#, TestHost::new(), 808.0);
    let form = first::<PanelPart>(&screen);
    assert!(form.bare());
    assert_eq!(Rect::new(0.0, 44.0, 128.0, 36.0), screen.element(part_named(&screen, "l")).unwrap().node().bounds);
    assert_eq!(0.0, form.around());
}

#[test]
fn a_container_grows_when_a_part_in_it_does() {
    let screen = window(
        r#"<Panel name="a" at="1,1" span="6" rows="3"><Table at="1,1" span="3" rows="2" /></Panel>
           <Panel name="b" at="7,1" span="6" rows="3"><Label at="1,1" span="3" /></Panel>"#,
        TestHost::new(),
        800.0,
    );
    let grows = |name: &str| screen.element(part_named(&screen, name)).and_then(|e| e.as_part()).unwrap().grows();
    // A table grows once the data agent draws it; a label never does.
    assert!(!grows("b"));
    let table_grows = find_type::<PanelPart>(screen.root(), |p| p.base().name() == Some("a")).unwrap().inner().node().children()[0].as_part().unwrap().grows();
    assert_eq!(table_grows, grows("a"));
}

#[test]
fn a_foldable_panel_folds_to_its_header_when_the_person_presses_it() {
    let host = TestHost::new();
    let mut screen = window(&PANEL.replace("caption=", r#"foldable="true" caption="#), host.clone(), 808.0);
    let inner_id = part_named(&screen, "inner");
    let inner = screen.element(inner_id).unwrap().node().bounds;
    assert_eq!(Some(inner_id), screen.at(620.0, inner.y + 5.0));
    click(&mut screen, 500.0, 8.0);
    assert_eq!(Some(true), host.state(Some("box")).unwrap().folded);
    assert!(first::<PanelPart>(&screen).is_folded());
    screen.resize(808.0, 400.0, 1.0);
    // Folded, its parts take no pointer and its place below the header is the window's.
    assert_ne!(Some(inner_id), screen.at(620.0, inner.y + 5.0));
    assert!(first::<PanelPart>(&screen).node().bounds.height < 40.0);
    // A handler unfolds it by its state.
    host.set_state("box", |s| s.folded = Some(false));
    screen.resize(808.0, 400.0, 1.0);
    assert_eq!(Some(inner_id), screen.at(620.0, inner.y + 5.0));
    // A press in its body is no press on its header.
    click(&mut screen, 450.0, 150.0);
    assert_eq!(Some(false), host.state(Some("box")).unwrap().folded);
}

#[test]
fn a_foldable_panel_folds_by_keys_too() {
    let host = TestHost::new();
    let mut screen = window(&PANEL.replace("caption=", r#"foldable="true" caption="#), host.clone(), 808.0);
    let id = screen.root().node().children().iter().find(|c| c.as_any().is::<PanelPart>()).unwrap().node().id();
    screen.focus(Some(id));
    screen.key_pressed(KeyInput::plain(Key::Space));
    assert_eq!(Some(true), host.state(Some("box")).unwrap().folded);
    screen.key_pressed(KeyInput::plain(Key::Enter));
    assert_eq!(Some(false), host.state(Some("box")).unwrap().folded);
    // Tab reaches it: one that does not fold takes no focus.
    let plain = window(PANEL, TestHost::new(), 808.0);
    assert!(!first::<PanelPart>(&plain).focusable());
}

#[test]
fn a_foldable_part_is_folded_as_its_state_says_else_as_designed() {
    let cases = conformance("folding.json");
    for case in cases["cases"].as_array().unwrap() {
        let host = TestHost::new();
        let design = if case["design"].as_bool().unwrap() { r#" folded="true""# } else { "" };
        let named = case["part"].get("folded").and_then(Value::as_bool);
        if let Some(folded) = named {
            host.set_state("box", |s| s.folded = Some(folded));
        }
        let name = if named.is_some() { r#" name="box""# } else { "" };
        let screen = window(&format!(r#"<Panel{name} at="1,1" span="6" rows="3" foldable="true"{design} />"#), host, 800.0);
        assert_eq!(case["folded"].as_bool().unwrap(), first::<PanelPart>(&screen).is_folded(), "{case}");
    }
}

#[test]
fn a_panel_code_hides_draws_nothing() {
    let host = TestHost::new();
    let mut screen = window(PANEL, host.clone(), 808.0);
    host.set_state("box", |s| s.visible = Some(false));
    let background = screen.look().color("background");
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    assert_eq!(background, shot.pixel(800, 6));
}

#[test]
fn a_panel_with_a_header_asks_room_for_it_and_its_grid_gets_its_rows() {
    let rows: String = (1..=3).map(|r| format!(r#"<Label at="1,{r}" span="4" />"#)).collect();
    let screen = window(&format!(r#"<Panel at="1,1" span="12" rows="3" caption="Box">{rows}</Panel>"#), TestHost::new(), 808.0);
    let panel = first::<PanelPart>(&screen);
    assert!(panel.around() > 0.0);
    assert!(panel.inner().node().bounds.height >= panel.inner().content - 0.5);
}

#[test]
fn a_part_in_a_container_is_inside_it_and_one_on_the_window_is_not() {
    assert!(!container::is_inside());
    {
        let _framed = container::framing();
        assert!(container::is_inside());
    }
    assert!(!container::is_inside());
}

#[test]
fn enter_in_an_input_of_a_form_runs_its_submit() {
    let host = TestHost::new();
    let screen = window(r#"<Form name="f" at="1,1" span="12" rows="2" onsubmit="save"><Textbox name="t" at="1,1" span="4" /></Form><Textbox name="out" at="1,4" span="4" />"#, host.clone(), 800.0);
    assert!(panel::submit(screen.root(), part_named(&screen, "t")));
    assert_eq!(vec!["run f submit save"], host.logged());
    // An input in no form submits nothing.
    assert!(!panel::submit(screen.root(), part_named(&screen, "out")));
}

#[test]
fn a_separator_is_a_line_across_or_down_in_the_strong_border_color() {
    let mut screen = window(r#"<Separator name="a" at="1,1" span="12" /><Separator name="d" at="1,2" rows="3" orientation="vertical" />"#, TestHost::new(), 800.0);
    let across = screen.element(part_named(&screen, "a")).unwrap();
    assert!(!across.as_any().downcast_ref::<SeparatorPart>().unwrap().vertical());
    let a = across.node().bounds;
    let d = screen.element(part_named(&screen, "d")).unwrap();
    assert!(d.as_any().downcast_ref::<SeparatorPart>().unwrap().vertical());
    let d = d.node().bounds;
    let line = screen.look().color("strong_border");
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut shot, "containers-separator.png");
    assert_eq!(line, shot.pixel((a.x + a.width / 2.0) as i32, (a.y + a.height / 2.0) as i32));
    assert_eq!(line, shot.pixel((d.x + d.width / 2.0) as i32, (d.y + d.height / 2.0) as i32));
    assert_ne!(line, shot.pixel((a.x + a.width / 2.0) as i32, (a.y + 3.0) as i32));
}

#[test]
fn a_button_group_joins_its_buttons_with_only_its_outer_corners_round() {
    let host = TestHost::new();
    let mut screen = window(
        r#"<Buttons at="1,1" span="4"><Button at="1,1" span="2" name="left" caption="Left" onclick="goLeft" /><Button at="3,1" span="2" name="right" caption="Right" onclick="goRight" /></Buttons>"#,
        host.clone(),
        800.0,
    );
    let group = first::<ButtonsPart>(&screen);
    let pushes: Vec<&ButtonPart> = group.inner().node().children().iter().filter_map(|c| c.as_any().downcast_ref::<ButtonPart>()).collect();
    let (a, b) = (pushes[0].node().bounds, pushes[1].node().bounds);
    assert!((a.right() - b.x).abs() < 0.001);
    assert!((group.node().bounds.right() - b.right()).abs() < 0.001);
    assert_eq!((Corners { left: true, right: false }, Corners { left: false, right: true }), (pushes[0].corners, pushes[1].corners));
    click(&mut screen, a.x + 10.0, a.y + 10.0);
    click(&mut screen, b.x + 10.0, b.y + 10.0);
    assert_eq!(vec!["run left click goLeft", "run right click goRight"], host.logged());
    let mut shot = Shot::of(&mut screen, 800, 100, 1.0);
    keep(&mut shot, "containers-buttons.png");
}

fn tall(content_width: f64, content_height: f64) -> Screen {
    let mut view = ScrollView::new(Box::new(Plate::new("accent", None)));
    view.measure = Some(Box::new(move |_, _, _, _| (content_width, content_height)));
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(view), GridItem::new(1, 1, 1, 1, true));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    screen.resize(200.0, 200.0, 1.0);
    screen
}

fn view(screen: &Screen) -> &ScrollView {
    first::<ScrollView>(screen)
}

#[test]
fn content_taller_than_its_place_scrolls_with_the_wheel_and_stops_at_its_ends() {
    let mut screen = tall(300.0, 1000.0);
    screen.wheel(50.0, 50.0, 0.0, 40.0, Modifiers::NONE);
    assert_eq!(40.0, view(&screen).scroll_y());
    assert_eq!(-40.0, view(&screen).content().node().bounds.y);
    assert_eq!(1000.0, view(&screen).content().node().bounds.height);
    screen.wheel(50.0, 50.0, 0.0, 5000.0, Modifiers::NONE);
    assert_eq!(800.0, view(&screen).scroll_y());
    assert_eq!(800.0, view(&screen).max_scroll_y());
    screen.wheel(50.0, 50.0, 0.0, -5000.0, Modifiers::NONE);
    assert_eq!(0.0, view(&screen).scroll_y());
}

#[test]
fn content_that_fits_does_not_scroll_and_shows_no_bar() {
    let mut screen = tall(200.0, 100.0);
    screen.wheel(50.0, 50.0, 0.0, 40.0, Modifiers::NONE);
    assert_eq!(0.0, view(&screen).scroll_y());
    assert!(view(&screen).bars().iter().all(|b| !b.is_shown()));
    assert_eq!(Some(view(&screen).content().node().id()), screen.at(195.0, 50.0));
}

#[test]
fn content_wider_than_its_place_scrolls_across() {
    let mut screen = tall(500.0, 100.0);
    screen.wheel(50.0, 50.0, 30.0, 0.0, Modifiers::NONE);
    assert_eq!(30.0, view(&screen).scroll_x());
    assert_eq!(-30.0, view(&screen).content().node().bounds.x);
    assert_eq!(300.0, view(&screen).max_scroll_x());
}

#[test]
fn dragging_the_bar_scrolls_as_far_as_the_bar_went() {
    let mut screen = tall(200.0, 1000.0);
    let bar = screen.at(196.0, 10.0);
    assert!(bar.is_some());
    assert_ne!(Some(view(&screen).content().node().id()), bar);
    screen.pointer_pressed(196.0, 10.0, PointerButton::Left, 1);
    screen.pointer_moved(196.0, 90.0);
    screen.pointer_released(196.0, 90.0, PointerButton::Left);
    // 80 of the 160 the thumb can move is half of the 800 the content can.
    assert!((view(&screen).scroll_y() - 400.0).abs() < 0.001);
    screen.resize(200.0, 200.0, 1.0);
    // A press on the track below the thumb pages down.
    click(&mut screen, 196.0, 190.0);
    assert!((view(&screen).scroll_y() - 600.0).abs() < 0.001);
}

#[test]
fn keeping_room_for_the_bar_narrows_the_content_while_the_bar_shows() {
    let made = |height: f64| {
        let mut view = ScrollView::new(Box::new(Plate::new("accent", None))).keeping_bar_room();
        view.measure = Some(Box::new(move |_, w, _, _| (w, height)));
        let mut grid = GridPanel::new(1);
        grid.add(Box::new(view), GridItem::new(1, 1, 1, 1, true));
        let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
        screen.resize(200.0, 200.0, 1.0);
        screen
    };
    let screen = made(1000.0);
    assert_eq!(200.0 - BAR_THICKNESS, view(&screen).content().node().bounds.width);
    assert_eq!(0.0, view(&screen).max_scroll_x());
    let fits = made(100.0);
    assert_eq!(200.0, view(&fits).content().node().bounds.width);
}

#[test]
fn it_shows_its_content_only_inside_its_place() {
    let mut view = ScrollView::new(Box::new(Plate::new("accent", None)));
    view.measure = Some(Box::new(|_, w, _, _| (w, 1000.0)));
    let mut grid = GridPanel::new(1);
    grid.add(Box::new(view), GridItem::new(1, 1, 1, 1, false));
    let mut screen = Screen::new(Box::new(grid), ThemeLook::clarity(false));
    let (accent, background) = (screen.look().color("accent"), screen.look().color("background"));
    let mut shot = Shot::of(&mut screen, 200, 200, 1.0);
    assert_eq!(accent, shot.pixel(20, 20));
    assert_eq!(background, shot.pixel(20, 100));
}

#[test]
fn a_box_of_the_content_scrolls_into_view() {
    let mut screen = tall(300.0, 1000.0);
    let id = view(&screen).node().id();
    let scrolled = |screen: &mut Screen, area: Rect| {
        screen.act(id, |e, cx| e.as_any_mut().downcast_mut::<ScrollView>().unwrap().scroll_into(area, cx));
        view(screen).scroll_y()
    };
    assert_eq!(350.0, scrolled(&mut screen, Rect::new(0.0, 500.0, 10.0, 50.0)));
    assert_eq!(100.0, scrolled(&mut screen, Rect::new(0.0, 100.0, 10.0, 50.0)));
    assert_eq!(100.0, scrolled(&mut screen, Rect::new(0.0, 150.0, 10.0, 50.0)));
}

#[test]
fn a_scroll_box_scrolls_its_parts_when_they_go_further_than_its_rows() {
    let rows: String = (1..=8).map(|r| format!(r#"<Label name="l{r}" at="1,{r}" span="4" caption="Row {r}" />"#)).collect();
    let mut screen = window(&format!(r#"<Scroll at="1,1" span="6" rows="3">{rows}</Scroll>"#), TestHost::new(), 800.0);
    let part = first::<ScrollPart>(&screen);
    assert!(part.view().max_scroll_y() > 0.0);
    assert_eq!(6, part.inner().columns);
    let b = part.node().bounds;
    screen.wheel(b.x + 20.0, b.y + 20.0, 0.0, 1000.0, Modifiers::NONE);
    let part = first::<ScrollPart>(&screen);
    assert_eq!(part.view().max_scroll_y(), part.view().scroll_y());
    let last = screen.element(part_named(&screen, "l8")).unwrap().node().bounds;
    assert!(last.bottom() <= b.bottom() + 0.5);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut shot, "containers-scroll.png");
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

fn accordion_of(sections: usize, open: i64) -> (Screen, Rc<TestHost>) {
    let host = TestHost::new();
    host.set_state("faq", |s| s.open = Some(open));
    let inside: String = (1..=sections).map(|i| format!(r#"<Section title="Section {i}"><Label at="1,1" span="3" caption="Inside {i}" /></Section>"#)).collect();
    let screen = window(&format!(r#"<Accordion name="faq" at="1,1" span="6" rows="8" onchange="opened">{inside}</Accordion>"#), host.clone(), 800.0);
    (screen, host)
}

#[test]
fn the_open_section_shows_as_on_every_renderer() {
    for case in conformance("accordions.json")["shown"].as_array().unwrap() {
        let (screen, _) = accordion_of(case["sections"].as_u64().unwrap() as usize, case["open"].as_i64().unwrap());
        let part = first::<AccordionPart>(&screen);
        let shown = case["shown"].as_i64().unwrap();
        assert_eq!(shown, accordion::shown(part.open(), part.pages().len()), "{case}");
        for (i, page) in part.pages().iter().enumerate() {
            assert_eq!(i as i64 + 1 == shown, page.is_shown());
            assert_eq!(Some(i as i64 + 1 == shown), part.headers()[i].chevron);
        }
    }
}

#[test]
fn a_press_on_a_header_opens_or_folds_as_on_every_renderer() {
    for case in conformance("accordions.json")["press"].as_array().unwrap() {
        let (mut screen, host) = accordion_of(case["sections"].as_u64().unwrap() as usize, case["open"].as_i64().unwrap());
        let header = first::<AccordionPart>(&screen).headers()[case["press"].as_u64().unwrap() as usize - 1].node().bounds;
        click(&mut screen, header.x + 20.0, header.y + 10.0);
        assert_eq!(case["open after"].as_i64(), host.state(Some("faq")).unwrap().open, "{case}");
        assert_eq!(vec!["run faq change opened"], host.logged());
    }
}

#[test]
fn the_open_sections_parts_fill_the_room_under_its_header_and_the_rest_follow() {
    let (mut screen, _) = accordion_of(3, 2);
    let part = first::<AccordionPart>(&screen);
    let headers: Vec<Rect> = part.headers().iter().map(|h| h.node().bounds).collect();
    let page = part.pages()[1].node().bounds;
    assert!(page.y >= headers[1].bottom());
    assert!(headers[2].y >= page.bottom());
    assert!(headers[2].bottom() <= part.node().bounds.bottom() + 0.001);
    let mut shot = Shot::of(&mut screen, 800, 400, 1.0);
    keep(&mut shot, "containers-accordion.png");
}
