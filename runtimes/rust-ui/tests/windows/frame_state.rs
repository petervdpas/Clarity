//! What the frame's regions keep in the part state for the code, and what a
//! sidebar takes from the catalog: a ribbon's open tab (conformance
//! ribbons.json), a sidebar's folding and filling the window, side by side,
//! and the fields a sidebar's markup leaves out.

use super::core_frame_shared as shared;

use clarity_runtime::ui::Ui;
use clarity_ui::frame::FrameSpec;
use clarity_ui::markup;
use clarity_ui::window::WindowView;
use serde_json::Value;
use shared::*;

const RIBBON: &str = r#"
    <Ribbon name="mainRibbon" collapsible="true">
      <Tab title="Home" name="home"><Group title="Clipboard"><Item name="cut" caption="Cut" onclick="cut" /></Group></Tab>
      <Tab title="Insert" name="insert"><Group title="Pictures"><Item caption="Picture" onclick="picture" /></Group></Tab>
    </Ribbon>
"#;

const NAV: &str = r#"
    <Sidebar side="left" name="nav" width="280" collapsible="true" maximizable="true" columns="4">
      <Button name="find" at="1,1" span="4" caption="Find" onclick="find" />
    </Sidebar>
"#;

fn conformance(name: &str) -> Value {
    let text = std::fs::read_to_string(format!("{}/../../conformance/{name}", env!("CARGO_MANIFEST_DIR"))).expect("the conformance file reads");
    serde_json::from_str(&text).expect("the conformance file is JSON")
}

#[test]
fn its_part_state_carries_the_open_tab() {
    let cases = conformance("ribbons.json");
    for case in cases["tab"].as_array().unwrap() {
        let mut ui = Ui::default();
        if let Some(tab) = case["part"]["tab"].as_i64() {
            ui.set_tab("mainRibbon", tab);
        }
        assert_eq!(ui.tab("mainRibbon"), case["tab"].as_i64().unwrap(), "{case}");
    }
    for case in cases["set"].as_array().unwrap() {
        let mut ui = Ui::default();
        ui.set_tab("mainRibbon", case["tab"].as_i64().unwrap());
        assert_eq!(ui.tab("mainRibbon"), case["part"]["tab"].as_i64().unwrap(), "{case}");
    }
    // Opening a tab sends it as the part's tab.
    let (mut screen, host) = window(RIBBON, None);
    draw(&mut screen);
    let second = view(&screen).ribbon().unwrap().tabs()[1].1;
    click(&mut screen, second);
    assert_eq!(host.state_of("mainRibbon").tab, Some(2));
}

#[test]
fn a_sidebars_part_state_carries_whether_it_is_folded_and_fills_the_window() {
    let mut ui = Ui::default();
    ui.set_collapsed("nav", true);
    ui.set_maximized("nav", true);
    assert!(ui.collapsed("nav", false) && ui.maximized("nav"));
    // Folded and filling at once, both are kept: shown again, it fills the window again.
    let (mut screen, host) = window(NAV, None);
    draw(&mut screen);
    screen.act_root::<WindowView, _>(|v, cx| {
        let nav = v.sidebar_mut(true).unwrap();
        nav.fold(true, cx);
        nav.fill(true, cx);
    });
    let state = host.state_of("nav");
    assert_eq!((state.collapsed, state.maximized), (Some(true), Some(true)));
}

#[test]
fn a_sidebar_takes_what_its_markup_leaves_out_from_the_catalog() {
    let defaults = markup::sidebar_defaults("right");
    let read = |text: &str| FrameSpec::read(&clarity_ui::xml::parse(&format!("<Window>{text}</Window>")).unwrap().children);
    let spec = read(r#"<Sidebar side="right" name="info" min="120" />"#).right.unwrap();
    assert_eq!(Some(spec.width), defaults["width"].as_f64());
    assert_eq!(Some(spec.columns as f64), defaults["columns"].as_f64());
    assert_eq!(spec.min, Some(120.0));
    assert_eq!(spec.max, None);
    assert!(!spec.resizable);
    let written = read(r#"<Sidebar side="left" width="300" columns="2" resizable="true" />"#).left.unwrap();
    assert_eq!((written.width, written.columns, written.resizable), (300.0, 2, true));
}

/// An element that says when it woke.
struct Sleeper {
    node: clarity_ui::Node,
    woke: std::rc::Rc<std::cell::Cell<bool>>,
}

impl clarity_ui::Element for Sleeper {
    clarity_ui::element_node!();

    fn on_wake(&mut self, _cx: &mut clarity_ui::Cx) {
        self.woke.set(true);
    }
}

#[test]
fn a_menu_item_that_names_an_element_to_wake_wakes_it_once_it_ran() {
    use clarity_ui::frame::{FrameMenus, MenuEntry};
    let woke = std::rc::Rc::new(std::cell::Cell::new(false));
    let ran = std::rc::Rc::new(std::cell::Cell::new(false));
    let sleeper = Sleeper { node: clarity_ui::Node::new(), woke: woke.clone() };
    let id = sleeper.node.id();
    let mut screen = clarity_ui::Screen::new(Box::new(sleeper), clarity_ui::ThemeLook::clarity(false));
    screen.resize(400.0, 300.0, 1.0);
    let did = ran.clone();
    let entry = MenuEntry { caption: "Copy".into(), enabled: true, run: Some(std::rc::Rc::new(move || did.set(true))), wake: Some(id), ..MenuEntry::default() };
    screen.act(id, |_, cx| FrameMenus::new().open((id, 0), vec![entry], clarity_ui::Rect::new(10.0, 10.0, 0.0, 0.0), None, cx));
    let list = screen.overlays()[0].content.node().bounds;
    screen.click(list.x + 20.0, list.y + 10.0);
    assert!(ran.get());
    assert!(!woke.get());
    screen.wake_due(std::time::Instant::now() + std::time::Duration::from_secs(1));
    assert!(woke.get());
}
