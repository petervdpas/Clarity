//! The ribbon (conformance/ribbons.json), the sidebars
//! (conformance/sidebars.json) and the footer of a window's frame: the
//! ribbon's open tab and folding follow its part state, a sidebar is dragged
//! within its limits, folds to a strip and fills the window, and the
//! footer's status and parts sit at the bottom.

use super::core_frame_shared as shared;

use clarity_ui::frame::{self, rules, sidebar, spec::SidebarSpec};
use clarity_ui::geometry::Rect;
use clarity_ui::input::{Key, Modifiers, PointerButton};
use clarity_ui::window::WindowView;
use clarity_ui::{Element, Painter};
use serde_json::Value;
use shared::*;

const RIBBON: &str = r#"
    <Ribbon name="mainRibbon" collapsible="true">
      <Tab title="Home" name="home">
        <Group title="Clipboard" name="clip">
          <Item name="paste" caption="Paste" large="true" icon="clipboard" key="Ctrl+V" onclick="paste" />
          <Item caption="Cut" onclick="cut" />
        </Group>
        <Group title="Save" name="saving">
          <Item name="save" caption="Save" key="Ctrl+S" onclick="save" />
        </Group>
      </Tab>
      <Tab title="Insert" name="insert">
        <Group title="Pictures"><Item caption="Picture" key="Ctrl+P" onclick="picture" /></Group>
      </Tab>
    </Ribbon>
"#;

const NAV: &str = r#"
    <Sidebar side="left" name="nav" width="280" min="200" max="480" resizable="true" collapsible="true" maximizable="true" columns="4">
      <Button name="find" at="1,1" span="4" caption="Find" key="Ctrl+F" onclick="find" />
    </Sidebar>
"#;

fn conformance(name: &str) -> Value {
    let text = std::fs::read_to_string(format!("{}/../../conformance/{name}", env!("CARGO_MANIFEST_DIR"))).expect("the conformance file reads");
    serde_json::from_str(&text).expect("the conformance file is JSON")
}

fn captions(screen: &clarity_ui::Screen) -> Vec<String> {
    view(screen).ribbon().unwrap().buttons().iter().map(|(i, _)| i.text("caption").unwrap_or("").to_string()).collect()
}

fn ribbon_shown() -> (clarity_ui::Screen, std::rc::Rc<FrameHost>) {
    let (mut screen, host) = window(RIBBON, None);
    host.icons.borrow_mut().insert("clipboard".into(), "M8 4h8v4H8zM6 6H4v14h16V6h-2".into());
    draw(&mut screen);
    (screen, host)
}

// -- The ribbon ----------------------------------------------------------------

#[test]
fn a_ribbon_shows_the_tab_as_every_renderer_does() {
    let cases = conformance("ribbons.json");
    for case in cases["shown"].as_array().unwrap() {
        let visible: Vec<bool> = case["visible"].as_array().unwrap().iter().map(|v| v.as_bool().unwrap()).collect();
        let shown = case["shown"].as_u64().map(|s| s as usize);
        assert_eq!(rules::shown_tab(&visible, case["tab"].as_i64().unwrap()), shown, "{case}");
    }
}

#[test]
fn opening_a_tab_sends_it_as_the_parts_tab() {
    let (mut screen, host) = ribbon_shown();
    let second = view(&screen).ribbon().unwrap().tabs()[1].1;
    click(&mut screen, second);
    assert_eq!(host.state_of("mainRibbon").tab, Some(2));
}

#[test]
fn a_tab_opens_its_groups_and_code_opens_another() {
    let (mut screen, host) = ribbon_shown();
    let mut shot = draw(&mut screen);
    shot.save(shot_path("core-frame-ribbon")).unwrap();
    assert_eq!(view(&screen).ribbon().unwrap().shown_tab(), Some(1));
    assert_eq!(captions(&screen), vec!["Paste", "Cut", "Save"]);
    // A large button is higher, its icon above its caption.
    let buttons: Vec<Rect> = view(&screen).ribbon().unwrap().buttons().iter().map(|(_, r)| *r).collect();
    assert!(buttons[0].height > buttons[1].height);
    host.set_state("mainRibbon", |s| s.tab = Some(2));
    draw(&mut screen);
    assert_eq!(captions(&screen), vec!["Picture"]);
}

#[test]
fn a_hidden_tab_gives_way_and_a_hidden_group_leaves() {
    let (mut screen, host) = ribbon_shown();
    host.set_state("saving", |s| s.visible = Some(false));
    draw(&mut screen);
    assert_eq!(captions(&screen), vec!["Paste", "Cut"]);
    host.set_state("home", |s| s.visible = Some(false));
    draw(&mut screen);
    assert_eq!(view(&screen).ribbon().unwrap().shown_tab(), Some(2));
    assert_eq!(view(&screen).ribbon().unwrap().tabs().len(), 1);
}

#[test]
fn folded_only_the_tabs_show_and_opening_a_tab_unfolds_it() {
    let (mut screen, host) = ribbon_shown();
    let unfolded = view(&screen).body_place().y;
    let first = view(&screen).ribbon().unwrap().tabs()[0].1;
    click_at(&mut screen, first, 2);
    assert_eq!(host.state_of("mainRibbon").collapsed, Some(true));
    draw(&mut screen);
    let ribbon = view(&screen).ribbon().unwrap();
    assert_eq!(ribbon.node().bounds.height, frame::ribbon::TABS_HEIGHT);
    assert!(ribbon.buttons().is_empty());
    assert!(view(&screen).body_place().y < unfolded);
    let second = ribbon.tabs()[1].1;
    click(&mut screen, second);
    let state = host.state_of("mainRibbon");
    assert_eq!((state.collapsed, state.tab), (Some(false), Some(2)));
}

#[test]
fn a_ribbon_that_does_not_fold_stays_open() {
    let (mut screen, host) = window(r#"<Ribbon><Tab title="Home"><Group title="G"><Item caption="A" onclick="a" /></Group></Tab></Ribbon>"#, None);
    draw(&mut screen);
    let tab = view(&screen).ribbon().unwrap().tabs()[0].1;
    click_at(&mut screen, tab, 2);
    draw(&mut screen);
    assert!(!view(&screen).ribbon().unwrap().collapsed());
    let a = view(&screen).ribbon().unwrap().buttons()[0].1;
    click(&mut screen, a);
    assert_eq!(host.log(), vec!["run  click a"]);
}

#[test]
fn an_unnamed_ribbon_keeps_its_tab_itself() {
    let (mut screen, _) = window(r#"<Ribbon collapsible="true"><Tab title="Home"><Group title="G"><Item caption="A" onclick="a" /></Group></Tab><Tab title="More"><Group title="H"><Item caption="B" onclick="b" /></Group></Tab></Ribbon>"#, None);
    draw(&mut screen);
    let more = view(&screen).ribbon().unwrap().tabs()[1].1;
    click(&mut screen, more);
    draw(&mut screen);
    assert_eq!(captions(&screen), vec!["B"]);
}

#[test]
fn a_ribbon_button_runs_its_handler() {
    let (mut screen, host) = ribbon_shown();
    let paste = view(&screen).ribbon().unwrap().buttons()[0].1;
    click(&mut screen, paste);
    assert_eq!(host.log(), vec!["run paste click paste"]);
}

#[test]
fn a_ribbon_buttons_key_runs_it_on_any_tab() {
    let (mut screen, host) = ribbon_shown();
    press(&mut screen, Key::S, Modifiers::CONTROL);
    // On the Insert tab, which is not open.
    press(&mut screen, Key::P, Modifiers::CONTROL);
    assert_eq!(host.log(), vec!["run save click save", "run  click picture"]);
}

#[test]
fn what_code_hid_or_disabled_in_a_ribbon_does_not_answer() {
    let (mut screen, host) = ribbon_shown();
    host.set_state("save", |s| s.enabled = Some(false));
    host.set_state("insert", |s| s.visible = Some(false));
    host.set_state("paste", |s| s.enabled = Some(false));
    draw(&mut screen);
    press(&mut screen, Key::S, Modifiers::CONTROL);
    press(&mut screen, Key::P, Modifiers::CONTROL);
    press(&mut screen, Key::V, Modifiers::CONTROL);
    let paste = view(&screen).ribbon().unwrap().buttons()[0].1;
    click(&mut screen, paste);
    assert!(host.log().is_empty());
}

// -- Sidebars ------------------------------------------------------------------

fn spec(collapsed: bool) -> SidebarSpec {
    SidebarSpec { side: "left".into(), name: Some("nav".into()), width: 240.0, min: None, max: None, resizable: false, collapsible: true, collapsed, maximizable: true, columns: 3, parts: Vec::new() }
}

#[test]
fn a_sidebar_is_dragged_within_its_limits_as_every_renderer_does() {
    let cases = conformance("sidebars.json");
    for case in cases["widths"].as_array().unwrap() {
        let width = rules::sidebar_width(case["dragged"].as_f64().unwrap(), case["min"].as_f64(), case["max"].as_f64());
        assert_eq!(width, case["width"].as_f64().unwrap(), "{case}");
    }
}

#[test]
fn a_sidebar_is_folded_and_fills_the_window_as_every_renderer_says() {
    let cases = conformance("sidebars.json");
    for case in cases["collapsed"].as_array().unwrap() {
        let now = rules::sidebar_now(&spec(case["design"].as_bool().unwrap()), case["part"]["collapsed"].as_bool(), None);
        assert_eq!(now.0, case["collapsed"].as_bool().unwrap(), "{case}");
    }
    for case in cases["maximized"].as_array().unwrap() {
        let now = rules::sidebar_now(&spec(false), None, case["part"]["maximized"].as_bool());
        assert_eq!(now.1, case["maximized"].as_bool().unwrap(), "{case}");
    }
}

#[test]
fn folding_and_filling_are_sent_as_its_part_state() {
    let cases = conformance("sidebars.json");
    for case in cases["set"].as_array().unwrap() {
        let (mut screen, host) = window(NAV, None);
        draw(&mut screen);
        screen.act_root::<WindowView, _>(|v, cx| {
            let nav = v.sidebar_mut(true).unwrap();
            if let Some(folded) = case["collapsed"].as_bool() {
                nav.fold(folded, cx);
            }
            if let Some(filling) = case["maximized"].as_bool() {
                nav.fill(filling, cx);
            }
        });
        let state = host.state_of("nav");
        if let Some(collapsed) = case["part"]["collapsed"].as_bool() {
            assert_eq!(state.collapsed, Some(collapsed), "{case}");
        }
        if let Some(maximized) = case["part"]["maximized"].as_bool() {
            assert_eq!(state.maximized, Some(maximized), "{case}");
        }
    }
}

#[test]
fn a_sidebar_is_its_width_folds_to_a_strip_and_fills_the_window() {
    let (mut screen, host) = window(NAV, None);
    draw(&mut screen).save(shot_path("core-frame-sidebar")).unwrap();
    let nav = view(&screen).sidebar(true).unwrap();
    assert_eq!(nav.node().bounds, Rect::new(0.0, 0.0, 280.0, 600.0));
    assert_eq!(view(&screen).body_place(), Rect::new(280.0, 0.0, 720.0, 600.0));
    let fold = nav.fold_button().unwrap();
    click(&mut screen, fold);
    assert_eq!(host.state_of("nav").collapsed, Some(true));
    draw(&mut screen).save(shot_path("core-frame-sidebar-folded")).unwrap();
    let nav = view(&screen).sidebar(true).unwrap();
    assert_eq!(nav.node().bounds.width, rules::STRIP_WIDTH);
    let find = clarity_ui::testing::part_named(&screen, "find");
    assert!(!screen.shows(find));
    let fold = nav.fold_button().unwrap();
    click(&mut screen, fold);
    draw(&mut screen);
    let fill = view(&screen).sidebar(true).unwrap().fill_button().unwrap();
    click(&mut screen, fill);
    assert_eq!(host.state_of("nav").maximized, Some(true));
    draw(&mut screen).save(shot_path("core-frame-sidebar-filling")).unwrap();
    // It takes the whole width; the window's own parts get none.
    assert_eq!(view(&screen).sidebar(true).unwrap().node().bounds.width, 1000.0);
    assert_eq!(view(&screen).body_place().width, 0.0);
}

#[test]
fn code_folds_it_through_its_part_state() {
    let (mut screen, host) = window(NAV, None);
    host.set_state("nav", |s| s.collapsed = Some(true));
    draw(&mut screen);
    assert_eq!(view(&screen).sidebar(true).unwrap().node().bounds.width, rules::STRIP_WIDTH);
}

#[test]
fn an_unnamed_sidebar_starts_folded_as_designed() {
    let (mut screen, _) = window(r#"<Sidebar side="right" width="300" collapsible="true" collapsed="true" />"#, None);
    draw(&mut screen);
    let side = view(&screen).sidebar(false).unwrap();
    assert_eq!(side.node().bounds, Rect::new(1000.0 - rules::STRIP_WIDTH, 0.0, rules::STRIP_WIDTH, 600.0));
    let fold = side.fold_button().unwrap();
    click(&mut screen, fold);
    draw(&mut screen);
    assert_eq!(view(&screen).sidebar(false).unwrap().node().bounds.width, 300.0);
}

#[test]
fn the_person_drags_its_edge_within_its_limits() {
    let (mut screen, _) = window(NAV, None);
    draw(&mut screen);
    let edge = view(&screen).sidebar(true).unwrap().edge().unwrap();
    let x = edge.x + edge.width / 2.0;
    screen.pointer_pressed(x, 300.0, PointerButton::Left, 1);
    screen.pointer_moved(x + 60.0, 300.0);
    draw(&mut screen);
    assert_eq!(view(&screen).sidebar(true).unwrap().node().bounds.width, 340.0);
    screen.pointer_moved(x + 900.0, 300.0);
    screen.pointer_released(x + 900.0, 300.0, PointerButton::Left);
    draw(&mut screen);
    assert_eq!(view(&screen).sidebar(true).unwrap().node().bounds.width, 480.0);
    assert_eq!(view(&screen).body_place().x, 480.0);
}

#[test]
fn its_parts_sit_on_its_grid_and_answer_their_keys() {
    let (mut screen, host) = window(NAV, None);
    draw(&mut screen);
    let find = clarity_ui::testing::part_named(&screen, "find");
    let place = screen.element(find).unwrap().node().bounds;
    assert_eq!(place.x, sidebar::PADDING);
    assert_eq!(place.width, 280.0 - 2.0 * sidebar::PADDING);
    click(&mut screen, place);
    press(&mut screen, Key::F, Modifiers::CONTROL);
    assert_eq!(host.log(), vec!["run find click find", "run find click find"]);
}

#[test]
fn a_hidden_sidebar_takes_no_room() {
    let (mut screen, host) = window(NAV, None);
    host.set_state("nav", |s| s.visible = Some(false));
    draw(&mut screen);
    assert_eq!(view(&screen).body_place(), Rect::new(0.0, 0.0, 1000.0, 600.0));
    press(&mut screen, Key::F, Modifiers::CONTROL);
    assert!(host.log().is_empty());
}

#[test]
fn the_fill_mark_is_drawn_as_lines_its_way_for_filling_and_giving_back() {
    use skia_safe::{surfaces, Color};
    let look = clarity_ui::ThemeLook::clarity(false);
    let fonts = clarity_ui::fonts::Fonts::new(look.clone());
    for (filling, lit, dark) in [(false, (29, 11), (11, 11)), (true, (11, 11), (29, 11))] {
        let mut surface = surfaces::raster_n32_premul((40, 40)).unwrap();
        surface.canvas().clear(Color::WHITE);
        sidebar::fill_mark(&Painter::new(surface.canvas(), &look, &fonts), Rect::new(0.0, 0.0, 40.0, 40.0), filling, Color::BLACK);
        let pixels = surface.peek_pixels().unwrap();
        assert_ne!(clarity_ui::testing::pixel_at(&pixels, lit.0, lit.1), Color::WHITE, "filling {filling}");
        assert_eq!(clarity_ui::testing::pixel_at(&pixels, dark.0, dark.1), Color::WHITE, "filling {filling}");
    }
}

// -- The footer ----------------------------------------------------------------

#[test]
fn its_status_shows_in_the_language_shown() {
    let (mut screen, host) = window(r#"<Footer status="@ready" columns="12" />"#, None);
    host.base.texts.borrow_mut().insert("ready".into(), "Klaar".into());
    draw(&mut screen);
    let footer = view(&screen).footer().unwrap();
    assert_eq!(footer.status().as_deref(), Some("Klaar"));
    assert_eq!(footer.node().bounds, Rect::new(0.0, 600.0 - frame::footer::STATUS_HEIGHT, 1000.0, frame::footer::STATUS_HEIGHT));
    assert_eq!(view(&screen).body_place().bottom(), 600.0 - frame::footer::STATUS_HEIGHT);
}

#[test]
fn its_parts_sit_on_one_row_beside_the_status() {
    let (mut screen, host) = window(
        r#"<Footer status="Ready" columns="6">
             <Label name="count" at="1,1" span="2" caption="12 items" />
             <Button name="sync" at="5,1" span="2" caption="Sync" onclick="sync" />
           </Footer>"#,
        None,
    );
    draw(&mut screen).save(shot_path("core-frame-footer")).unwrap();
    let footer = view(&screen).footer().unwrap();
    assert_eq!(footer.node().bounds.height, frame::footer::parts_height());
    let status = footer.status_box();
    let label = screen.element(clarity_ui::testing::part_named(&screen, "count")).unwrap().node().bounds;
    let sync = screen.element(clarity_ui::testing::part_named(&screen, "sync")).unwrap().node().bounds;
    assert_eq!(label.y, sync.y);
    assert!(label.x >= status.right());
    assert!(sync.right() <= 1000.0);
    click(&mut screen, sync);
    assert_eq!(host.log(), vec!["run sync click sync"]);
}

#[test]
fn every_region_at_once() {
    let (mut screen, host) = window(
        r#"<Label at="1,1" span="6" caption="The window's own parts" />"#,
        Some(&format!(
            r#"<Menu name="m"><Item caption="File"><Item caption="Open" key="Ctrl+O" onclick="open" /></Item><Item caption="View" /></Menu>
               <Toolbar><Item caption="Save" icon="save" onclick="save" /><Item kind="separator" /><Item caption="Print" onclick="print" /></Toolbar>
               {RIBBON}
               <Sidebar side="left" width="200" collapsible="true"><Label at="1,1" span="3" caption="Left" /></Sidebar>
               <Sidebar side="right" width="180" maximizable="true"><Label at="1,1" span="3" caption="Right" /></Sidebar>
               <Footer status="Ready"><Label at="1,1" span="3" caption="3 items" /></Footer>"#
        )),
    );
    host.icons.borrow_mut().insert("save".into(), "M5 5h14v14H5z".into());
    draw(&mut screen).save(shot_path("core-frame-all")).unwrap();
    let body = view(&screen).body_place();
    assert_eq!(body.x, 200.0);
    assert_eq!(body.right(), 1000.0 - 180.0);
}
