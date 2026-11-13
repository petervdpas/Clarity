//! How many rows a window holds at its height under the frame's bars
//! (conformance/window-room.json): Clarity.UI draws the bars at the shared
//! heights, so a window holds the rows the checker counts, and one more scrolls.

use clarity_ui::frame::{footer, menu_bar, ribbon, toolbar};
use clarity_ui::markup;
use clarity_ui::parts::PartHost;
use clarity_ui::testing::Shot;
use clarity_ui::window::view::{WindowView, CONTENT_PADDING};
use clarity_ui::{Screen, ThemeLook};
use serde_json::Value;
use std::rc::Rc;

use crate::core_frame_shared::FrameHost;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../conformance/window-room.json")).unwrap()
}

#[test]
fn the_bars_are_drawn_at_the_shared_heights() {
    let file = cases();
    let bars = &file["bars"];
    let height = |key: &str| bars[key].as_f64().unwrap();
    assert_eq!(menu_bar::HEIGHT, height("menu"));
    assert_eq!(toolbar::HEIGHT, height("toolbar"));
    assert_eq!(ribbon::TABS_HEIGHT, height("ribbonTabs"));
    assert_eq!(ribbon::BODY_HEIGHT, height("ribbonBody"));
    assert_eq!(footer::STATUS_HEIGHT, height("status"));
    assert_eq!(footer::parts_height() - clarity_ui::grid::row_height(), height("footerPartsBeyondRow"));
    assert_eq!(CONTENT_PADDING, file["padding"].as_f64().unwrap());
    assert_eq!(clarity_ui::grid::GAP, file["gap"].as_f64().unwrap());
}

/// The markup of a bar as the case names it.
fn bar(word: &str) -> &'static str {
    match word {
        "menu" => r#"<Menu name="mainMenu"><Item caption="File" name="fileItem" /></Menu>"#,
        "toolbar" => r#"<Toolbar name="mainToolbar"><Item kind="button" caption="A" name="aButton" /></Toolbar>"#,
        "ribbon" => r#"<Ribbon name="mainRibbon"><Tab title="Home"><Group title="G"><Item caption="A" onclick="a" /></Group></Tab></Ribbon>"#,
        "ribbonFolded" => r#"<Ribbon name="mainRibbon" collapsible="true" collapsed="true"><Tab title="Home"><Group title="G"><Item caption="A" onclick="a" /></Group></Tab></Ribbon>"#,
        "status" => r#"<Footer status="Ready" columns="12" />"#,
        "footerParts" => r#"<Footer status="Ready" columns="6"><Label name="footerLabel" at="1,1" span="2" caption="Saved" /></Footer>"#,
        other => panic!("no bar {other}"),
    }
}

/// How far a window `height` high with the case's bars scrolls when a part reaches row `row`.
fn scroll_room(case: &Value, row: u64) -> f64 {
    let bars: String = case["bars"].as_array().unwrap().iter().map(|b| bar(b.as_str().unwrap())).collect();
    let markup = format!(r#"<Window name="Main" rowHeight="{}">{bars}<Label name="last" at="1,{row}" span="2" caption="Last" /></Window>"#, case["rowHeight"]);
    let spec = markup::read(&markup).expect("the window's markup reads");
    let host: Rc<dyn PartHost> = FrameHost::new();
    let mut screen = Screen::new(Box::new(WindowView::new(&spec, host)), ThemeLook::clarity(false));
    Shot::of(&mut screen, 1000, case["height"].as_u64().unwrap() as i32, 1.0);
    screen.root().as_any().downcast_ref::<WindowView>().expect("a window").max_scroll_y
}

#[test]
fn a_window_holds_the_rows_of_every_case_and_one_more_scrolls() {
    for case in cases()["cases"].as_array().unwrap() {
        let rows = case["rows"].as_u64().unwrap();
        assert_eq!(scroll_room(case, rows), 0.0, "{}: {rows} rows fit", case["name"]);
        assert!(scroll_room(case, rows + 1) > 0.0, "{}: {} rows scroll", case["name"], rows + 1);
    }
}
