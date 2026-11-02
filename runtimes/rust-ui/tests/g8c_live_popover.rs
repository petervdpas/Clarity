//! What the every-part project showed live on a popover: opened by the
//! button its `for` names, it lies under that button (not at its own place
//! in the grid), and a press on the button is no press outside it.

use clarity_ui::parts::PartHost;
use clarity_ui::testing::{part_named, window_of, Shot, TestHost};

const WINDOW: &str = r#"
    <Button name="filterButton" at="3,7" span="2" caption="Filter" />
    <Popover name="filters" at="1,1" span="4" for="filterButton"><Checkbox name="namedBox" at="1,1" span="4" caption="Only named" /></Popover>"#;

#[test]
fn a_popover_lies_under_the_part_its_for_names() {
    let host = TestHost::new();
    host.set_state("filters", |s| s.visible = Some(false));
    let mut screen = window_of(WINDOW, host.clone());
    let _ = Shot::of(&mut screen, 808, 400, 1.0);
    host.set_state("filters", |s| s.visible = Some(true));
    let mut shot = Shot::of(&mut screen, 808, 400, 1.0);
    let button_id = part_named(&screen, "filterButton");
    let button = screen.element(button_id).unwrap().node().bounds;
    assert_eq!(1, screen.overlays().len());
    let overlay = &screen.overlays()[0];
    let place = overlay.content.node().bounds;
    assert_eq!(button.x, place.x, "at the button's left");
    assert_eq!(button.bottom() + 4.0, place.y, "just under the button");
    assert_eq!(Some(button_id), overlay.anchor, "a press on the button is no press outside");
    shot.save("/tmp/claude-1000/-home-peter-Projects-Clarity/271a7793-55cd-491f-b56c-9578fe570b4e/scratchpad/shots/g8c-core-popover.png").unwrap();
    assert_eq!(Some(true), host.state(Some("filters")).unwrap().visible);
}
