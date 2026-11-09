//! A window's content (DESIGN 5.33): one view at a time, the one code picks
//! (the part's state's tab, from 1). It has no look of its own: no strip of
//! tabs, no frame, no fill; the view's parts start at its top.

use clarity_ui::element::find_type;
use clarity_ui::parts::containers::tabs::TabsPart;
use clarity_ui::testing::{window_of, TestHost};
use clarity_ui::{Element, Screen};

const CONTENT: &str = r#"<Content name="main" at="1,1" span="12" rows="6">
  <Tab columns="6"><Label name="domain" at="1,1" span="3" caption="Domain" /></Tab>
  <Tab columns="6"><Label name="project" at="1,1" span="3" caption="Project" /></Tab>
</Content>"#;

fn content(screen: &Screen) -> &TabsPart {
    find_type::<TabsPart>(screen.root(), |_| true).expect("the content is on the screen")
}

#[test]
fn content_shows_the_view_code_picks_and_nothing_of_its_own() {
    let host = TestHost::new();
    let mut screen = window_of(CONTENT, host.clone());
    screen.resize(808.0, 400.0, 1.0);
    let part = content(&screen);
    assert!(part.tabs().is_empty(), "no strip of tabs");
    assert_eq!(Some(1), part.open());
    // The view's parts start where the content starts.
    assert_eq!(part.node().bounds.y, part.pages()[0].node().bounds.y);
    host.set_state("main", |s| s.tab = Some(2));
    screen.resize(808.0, 400.0, 1.0);
    let part = content(&screen);
    assert!(part.pages()[1].is_shown() && !part.pages()[0].is_shown());
}
