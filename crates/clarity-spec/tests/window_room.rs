//! How many rows a window holds at its height under the frame's bars
//! (conformance/window-room.json), as Clarity.UI draws them.

use clarity_spec::layout::{self, Bar};
use serde_json::Value;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../conformance/window-room.json")).unwrap()
}

fn bar(word: &str) -> Bar {
    match word {
        "menu" => Bar::Menu,
        "toolbar" => Bar::Toolbar,
        "ribbon" => Bar::Ribbon,
        "ribbonFolded" => Bar::RibbonFolded,
        "status" => Bar::Status,
        "footerParts" => Bar::FooterParts,
        other => panic!("no bar {other}"),
    }
}

#[test]
fn the_heights_are_the_shared_ones() {
    let file = cases();
    let bars = &file["bars"];
    assert_eq!(layout::MENU_BAR_HEIGHT as u64, bars["menu"].as_u64().unwrap());
    assert_eq!(layout::TOOLBAR_HEIGHT as u64, bars["toolbar"].as_u64().unwrap());
    assert_eq!(layout::RIBBON_TABS_HEIGHT as u64, bars["ribbonTabs"].as_u64().unwrap());
    assert_eq!(layout::RIBBON_BODY_HEIGHT as u64, bars["ribbonBody"].as_u64().unwrap());
    assert_eq!(layout::STATUS_HEIGHT as u64, bars["status"].as_u64().unwrap());
    assert_eq!(layout::FOOTER_PARTS_BEYOND_ROW as u64, bars["footerPartsBeyondRow"].as_u64().unwrap());
    assert_eq!(layout::CONTENT_PADDING as u64, file["padding"].as_u64().unwrap());
    assert_eq!(layout::GAP as u64, file["gap"].as_u64().unwrap());
}

#[test]
fn every_case_of_the_conformance_file() {
    for case in cases()["cases"].as_array().unwrap() {
        let bars: Vec<Bar> = case["bars"].as_array().unwrap().iter().map(|b| bar(b.as_str().unwrap())).collect();
        let held = layout::rows_held(case["height"].as_u64().unwrap() as u32, case["rowHeight"].as_u64().unwrap() as u32, &bars);
        assert_eq!(held as u64, case["rows"].as_u64().unwrap(), "{}", case["name"]);
    }
}

/// A window's own bar replaces the app's of the same kind.
#[test]
fn a_window_s_own_bars_go_before_the_app_s() {
    let module = "module shop\n";
    let app = r#"<App class="shop.Shop" main="Card">
  <Toolbar name="appToolbar"><Item kind="button" caption="A" name="aButton" /></Toolbar>
  <Footer status="Ready" columns="12" />
</App>"#;
    let card = r#"<Window class="shop.Card" title="Card">
  <Menu name="cardMenu"><Item caption="File" name="fileItem" /></Menu>
  <Toolbar name="cardToolbar"><Item kind="button" caption="B" name="bButton" /></Toolbar>
</Window>"#;
    let project = clarity_spec::load_sources(vec![("shop.clarity", module), ("design/Shop.cxaml", app), ("design/Card.cxaml", card)]);
    let errors: Vec<_> = project.diagnostics.iter().filter(|d| d.severity == clarity_spec::Severity::Error).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let app = project.files.iter().flat_map(|f| &f.apps).next().unwrap();
    let page = project.files.iter().flat_map(|f| &f.pages).find(|p| p.name.name == "Card").unwrap();
    assert_eq!(layout::frame_bars(Some(&app.frame), &page.frame), vec![Bar::Menu, Bar::Toolbar, Bar::Status]);
    assert_eq!(layout::frame_bars(None, &page.frame), vec![Bar::Menu, Bar::Toolbar]);
}
